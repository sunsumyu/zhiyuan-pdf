import { targetInvokeV3 } from '../shared/wasm_loader';
import { logPdfLayoutTrace } from './layout_trace';

// ── GPU 渲染缓存 ────────────────────────────────────────────────────────────

interface GpuCacheEntry {
    bitmap: ImageBitmap;
    lastAccessed: number;
    sizeBytes: number;
}

const gpuPageCache = new Map<string, GpuCacheEntry>();

/** 最大缓存页数 */
const MAX_GPU_CACHE_ENTRIES = 20;

/** 最大缓存内存（500 MB），超过时触发 LRU 淘汰 */
const MAX_GPU_CACHE_BYTES = 500 * 1024 * 1024;

/** 单页最大像素尺寸限制（前端侧） */
export const GPU_PAGE_MAX_WIDTH = 8192;
export const GPU_PAGE_MAX_HEIGHT = 8192;

/** GPU 渲染超时时间（毫秒） */
export const GPU_RENDER_TIMEOUT_MS = 15000;

/** 瓦片大小（像素） */
const TILE_SIZE = 4096;

function getGpuCacheKey(
    path: string,
    pageIndex: number,
    width: number,
    height: number,
    zoom: number,
    documentRevision?: number,
): string {
    return `${path}::${pageIndex}::${Math.round(width)}::${Math.round(height)}::${zoom.toFixed(2)}::${documentRevision ?? 0}`;
}

function getCacheEntrySize(entry: GpuCacheEntry): number {
    // ImageBitmap 内存 ≈ width * height * 4 bytes (RGBA)
    const w = entry.bitmap.width;
    const h = entry.bitmap.height;
    return w * h * 4;
}

function getTotalCacheSize(): number {
    let total = 0;
    for (const entry of gpuPageCache.values()) {
        total += getCacheEntrySize(entry);
    }
    return total;
}

/** 按 LRU 策略淘汰缓存，直到满足内存限制 */
function evictGpuCacheIfNeeded(): void {
    // 先按条目数淘汰
    while (gpuPageCache.size > MAX_GPU_CACHE_ENTRIES) {
        evictOldestEntry();
    }

    // 再按内存限制淘汰
    let totalSize = getTotalCacheSize();
    while (totalSize > MAX_GPU_CACHE_BYTES && gpuPageCache.size > 0) {
        evictOldestEntry();
        totalSize = getTotalCacheSize();
    }
}



function evictOldestEntry(): void {
    let oldestKey: string | null = null;
    let oldestTime = Infinity;

    for (const [key, entry] of gpuPageCache) {
        if (entry.lastAccessed < oldestTime) {
            oldestTime = entry.lastAccessed;
            oldestKey = key;
        }
    }

    if (oldestKey !== null) {
        const entry = gpuPageCache.get(oldestKey);
        if (entry) {
            try { entry.bitmap.close(); } catch {}
        }
        gpuPageCache.delete(oldestKey);
    }
}

function touchCacheEntry(key: string): void {
    const entry = gpuPageCache.get(key);
    if (entry) {
        entry.lastAccessed = Date.now();
    }
}

// ── 瓦片渲染 ───────────────────────────────────────────────────────────────

interface TileSpec {
    x: number;
    y: number;
    width: number;
    height: number;
}

/**
 * 判断是否需要分块渲染（超过 GPU texture limit）
 */
function needsTiling(width: number, height: number): boolean {
    return width > GPU_PAGE_MAX_WIDTH || height > GPU_PAGE_MAX_HEIGHT;
}

/**
 * 计算瓦片布局
 */
function computeTiles(totalWidth: number, totalHeight: number): TileSpec[] {
    const tiles: TileSpec[] = [];
    const cols = Math.ceil(totalWidth / TILE_SIZE);
    const rows = Math.ceil(totalHeight / TILE_SIZE);

    for (let row = 0; row < rows; row++) {
        for (let col = 0; col < cols; col++) {
            const x = col * TILE_SIZE;
            const y = row * TILE_SIZE;
            const width = Math.min(TILE_SIZE, totalWidth - x);
            const height = Math.min(TILE_SIZE, totalHeight - y);
            tiles.push({ x, y, width, height });
        }
    }

    return tiles;
}

/**
 * 渲染单个瓦片
 */
async function renderTile(
    path: string,
    pageIndex: number,
    tile: TileSpec,
    zoom: number,
    documentRevision?: number,
): Promise<ImageBitmap | null> {
    const base64Png = await targetInvokeV3('render_page_to_image', {
        path,
        pageIndex,
        zoom,
        width: Math.round(tile.width),
        height: Math.round(tile.height),
        documentRevision,
    });

    if (!base64Png || typeof base64Png !== 'string') {
        return null;
    }

    const binary = atob(base64Png);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
    }

    const blob = new Blob([bytes], { type: 'image/png' });
    return createImageBitmap(blob);
}

/**
 * 分块渲染大页面，将多个瓦片拼接成完整页面
 */
async function renderTiledPage(
    path: string,
    pageIndex: number,
    width: number,
    height: number,
    zoom: number,
    documentRevision?: number,
): Promise<ImageBitmap | null> {
    const tiles = computeTiles(width, height);
    logPdfLayoutTrace('gpu-render.tiled', {
        path, pageIndex, width, height, zoom, tileCount: tiles.length,
    });

    // 并行渲染所有瓦片
    const tileBitmaps = await Promise.all(
        tiles.map(tile =>
            renderTile(path, pageIndex, tile, zoom, documentRevision)
                .catch(() => null),
        ),
    );

    // 创建 offscreen canvas 拼接瓦片
    const canvas = new OffscreenCanvas(width, height);
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    for (let i = 0; i < tiles.length; i++) {
        const bitmap = tileBitmaps[i];
        if (!bitmap) continue;
        const tile = tiles[i];
        ctx.drawImage(bitmap, tile.x, tile.y);
        bitmap.close(); // 释放单个瓦片
    }

    return canvas.transferToImageBitmap();
}

// ── 主渲染 API ────────────────────────────────────────────────────────────

/**
 * 使用后端 vello GPU 矢量渲染器将单页 PDF 渲染为 ImageBitmap。
 * 返回 null 表示渲染失败或 aborted。
 *
 * 特性：
 * - 自动超时（15 秒）
 * - LRU 缓存管理
 * - 超大页面自动分块渲染
 */
export async function renderGpuPage(
    path: string,
    pageIndex: number,
    width: number,
    height: number,
    zoom: number,
    _frameToken?: number,
    documentRevision?: number,
): Promise<ImageBitmap | null> {
    const cacheKey = getGpuCacheKey(path, pageIndex, width, height, zoom, documentRevision);
    const cached = gpuPageCache.get(cacheKey);
    if (cached) {
        touchCacheEntry(cacheKey);
        return cached.bitmap;
    }

    const renderPromise = renderGpuPageInternal(
        path, pageIndex, width, height, zoom, documentRevision,
    );

    return Promise.race([
        renderPromise,
        new Promise<null>((_, reject) => {
            setTimeout(() => {
                reject(new Error('GPU render timeout'));
            }, GPU_RENDER_TIMEOUT_MS);
        }),
    ]).catch((e) => {
        console.error('[GPU-RENDER] Failed:', e);
        return null;
    });
}

async function renderGpuPageInternal(
    path: string,
    pageIndex: number,
    width: number,
    height: number,
    zoom: number,
    documentRevision?: number,
): Promise<ImageBitmap | null> {
    const cacheKey = getGpuCacheKey(path, pageIndex, width, height, zoom, documentRevision);

    // 超大页面：分块渲染
    if (needsTiling(width, height)) {
        const bitmap = await renderTiledPage(path, pageIndex, width, height, zoom, documentRevision);
        if (bitmap) {
            evictGpuCacheIfNeeded();
            gpuPageCache.set(cacheKey, {
                bitmap,
                lastAccessed: Date.now(),
                sizeBytes: bitmap.width * bitmap.height * 4,
            });
        }
        return bitmap;
    }

    // 常规渲染（单块）
    const base64Png = await targetInvokeV3('render_page_to_image', {
        path,
        pageIndex,
        zoom,
        width: Math.round(width),
        height: Math.round(height),
        documentRevision,
    });

    if (!base64Png || typeof base64Png !== 'string') {
        console.error('[GPU-RENDER] Empty response from render_page_to_image');
        return null;
    }

    const binary = atob(base64Png);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
    }

    const blob = new Blob([bytes], { type: 'image/png' });
    const bitmap = await createImageBitmap(blob);

    evictGpuCacheIfNeeded();
    gpuPageCache.set(cacheKey, {
        bitmap,
        lastAccessed: Date.now(),
        sizeBytes: bitmap.width * bitmap.height * 4,
    });

    logPdfLayoutTrace('gpu-render.success', { path, pageIndex, width, height, zoom });
    return bitmap;
}

// ── 缓存查询 ────────────────────────────────────────────────────────────────

export function findGpuPageInCache(
    path: string,
    pageIndex: number,
    width: number,
    height: number,
    zoom: number,
    documentRevision?: number,
): ImageBitmap | null {
    const cacheKey = getGpuCacheKey(path, pageIndex, width, height, zoom, documentRevision);
    const entry = gpuPageCache.get(cacheKey);
    if (entry) {
        touchCacheEntry(cacheKey);
        return entry.bitmap;
    }
    return null;
}

export function invalidateGpuPageCache(path: string, pageIndex: number): void {
    for (const key of gpuPageCache.keys()) {
        if (key.startsWith(`${path}::${pageIndex}::`)) {
            const entry = gpuPageCache.get(key);
            if (entry) {
                try { entry.bitmap.close(); } catch {}
            }
            gpuPageCache.delete(key);
        }
    }
}

export function clearGpuPageCache(): void {
    for (const entry of gpuPageCache.values()) {
        try { entry.bitmap.close(); } catch {}
    }
    gpuPageCache.clear();
}

/** 获取当前缓存统计信息（用于调试） */
export function getGpuCacheStats(): { entries: number; totalBytes: number } {
    return {
        entries: gpuPageCache.size,
        totalBytes: getTotalCacheSize(),
    };
}

// ── 预渲染 ─────────────────────────────────────────────────────────────────

/** 预渲染指定页面到 GPU 缓存（后台、不阻塞） */
export function prefetchGpuPage(
    path: string,
    pageIndex: number,
    width: number,
    height: number,
    zoom: number,
    documentRevision?: number,
): void {
    const cacheKey = getGpuCacheKey(path, pageIndex, width, height, zoom, documentRevision);
    if (gpuPageCache.has(cacheKey)) return;

    renderGpuPage(path, pageIndex, width, height, zoom, undefined, documentRevision)
        .then(() => {
            logPdfLayoutTrace('gpu-render.prefetch.success', { path, pageIndex, width, height, zoom });
        })
        .catch(() => {
            // 预渲染失败静默处理
        });
}

// ── 异步重新渲染 ───────────────────────────────────────────────────────────

interface AsyncRenderCallback {
    onComplete: (bitmap: ImageBitmap) => void;
    onError: () => void;
}

const pendingAsyncRenders = new Map<string, AbortController>();

/**
 * 异步重新渲染页面（用于编辑 commit 后）。
 * 返回 Promise，resolve 时新的 ImageBitmap 已准备好。
 */
export async function asyncReRenderGpuPage(
    path: string,
    pageIndex: number,
    width: number,
    height: number,
    zoom: number,
    documentRevision?: number,
): Promise<ImageBitmap | null> {
    const cacheKey = getGpuCacheKey(path, pageIndex, width, height, zoom, documentRevision);

    // 取消同一页面的旧异步渲染
    const existing = pendingAsyncRenders.get(cacheKey);
    if (existing) {
        existing.abort();
    }

    const controller = new AbortController();
    pendingAsyncRenders.set(cacheKey, controller);

    try {
        // 先使缓存失效，确保获取最新版本
        invalidateGpuPageCache(path, pageIndex);

        const bitmap = await renderGpuPage(path, pageIndex, width, height, zoom, undefined, documentRevision);
        return bitmap;
    } finally {
        pendingAsyncRenders.delete(cacheKey);
    }
}
