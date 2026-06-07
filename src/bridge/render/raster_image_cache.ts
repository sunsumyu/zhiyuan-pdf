import { emitPdfDiagnostic } from '../shared/diagnostics';

const RASTER_MEMORY_BUDGET_BYTES = 128 * 1024 * 1024; // 128MB

let currentRasterMemoryBytes = 0;

const decodedRasterImages = new Map<string, ImageBitmap>();
const inflightRasterImages = new Map<string, Promise<ImageBitmap | null>>();

type RasterWarmOptions = {
    role?: 'current' | 'preview' | 'prefetch' | 'unknown';
    pageIndex?: number;
};

function summarizeRasterSrc(src: string): string {
    if (src.startsWith('data:')) {
        return `${src.slice(0, 32)}...len${src.length}`;
    }
    try {
        const url = new URL(src);
        return `${url.protocol}//${url.host}${url.pathname}`;
    } catch {
        return src.length > 96 ? `${src.slice(0, 93)}...` : src;
    }
}

function logRasterCache(event: string, src: string, fields: Record<string, unknown> = {}): void {
    emitPdfDiagnostic('cache', `raster-image.${event}`, {
        ...fields,
        src: summarizeRasterSrc(src),
        cacheSize: decodedRasterImages.size,
        currentMemoryMB: Math.round(currentRasterMemoryBytes / 1024 / 1024),
        budgetMB: Math.round(RASTER_MEMORY_BUDGET_BYTES / 1024 / 1024),
    });
}

function getBitmapByteSize(bitmap: ImageBitmap): number {
    // 4 bytes per pixel for RGBA
    return bitmap.width * bitmap.height * 4;
}

function rememberRasterImage(
    src: string,
    bitmap: ImageBitmap,
    options: RasterWarmOptions = {},
): ImageBitmap {
    const existing = decodedRasterImages.get(src);
    if (existing) {
        currentRasterMemoryBytes -= getBitmapByteSize(existing);
    }
    
    decodedRasterImages.delete(src);
    decodedRasterImages.set(src, bitmap);
    currentRasterMemoryBytes += getBitmapByteSize(bitmap);
    
    // Evict oldest until we fit in budget (always keep at least 1 item to prevent thrashing)
    while (currentRasterMemoryBytes > RASTER_MEMORY_BUDGET_BYTES && decodedRasterImages.size > 1) {
        const oldestEntry = decodedRasterImages.entries().next().value;
        if (!oldestEntry) break;
        const [oldestKey, oldestBitmap] = oldestEntry;
        
        currentRasterMemoryBytes -= getBitmapByteSize(oldestBitmap);
        decodedRasterImages.delete(oldestKey);
        
        // Explicitly release GPU memory
        oldestBitmap.close();
        logRasterCache('evict', oldestKey, {
            role: options.role ?? 'unknown',
            pageIndex: options.pageIndex,
        });
    }
    logRasterCache('store', src, {
        role: options.role ?? 'unknown',
        pageIndex: options.pageIndex,
        naturalWidth: bitmap.width,
        naturalHeight: bitmap.height,
    });
    return bitmap;
}

export function readDecodedRasterImage(src: string): ImageBitmap | null {
    const cached = decodedRasterImages.get(src);
    if (!cached) return null;
    decodedRasterImages.delete(src);
    decodedRasterImages.set(src, cached);
    return cached;
}

export function warmRasterImage(
    src: string,
    options: RasterWarmOptions = {},
): Promise<ImageBitmap | null> {
    const cached = readDecodedRasterImage(src);
    if (cached) {
        logRasterCache('hit', src, {
            role: options.role ?? 'unknown',
            pageIndex: options.pageIndex,
        });
        return Promise.resolve(cached);
    }

    const inflight = inflightRasterImages.get(src);
    if (inflight) {
        logRasterCache('inflight', src, {
            role: options.role ?? 'unknown',
            pageIndex: options.pageIndex,
        });
        return inflight;
    }

    const task = (async () => {
        const startedAt = performance.now();
        try {
            logRasterCache('miss', src, {
                role: options.role ?? 'unknown',
                pageIndex: options.pageIndex,
            });
            
            // fetch blob and decode into ImageBitmap off main thread
            const response = await fetch(src);
            if (!response.ok) {
                throw new Error(`fetch failed: ${response.status}`);
            }
            const blob = await response.blob();
            const bitmap = await createImageBitmap(blob);
            
            return rememberRasterImage(src, bitmap, options);
        } catch (error) {
            logRasterCache('decode-failed', src, {
                role: options.role ?? 'unknown',
                pageIndex: options.pageIndex,
                error: String(error),
            });
            return null;
        } finally {
            logRasterCache('decode-end', src, {
                role: options.role ?? 'unknown',
                pageIndex: options.pageIndex,
                elapsedMs: performance.now() - startedAt,
            });
            inflightRasterImages.delete(src);
        }
    })();

    inflightRasterImages.set(src, task);
    return task;
}

export function clearRasterImageCache(): void {
    emitPdfDiagnostic('cache', 'raster-image.clear', {
        cacheSize: decodedRasterImages.size,
        inflightSize: inflightRasterImages.size,
    });
    for (const bitmap of decodedRasterImages.values()) {
        bitmap.close();
    }
    decodedRasterImages.clear();
    inflightRasterImages.clear();
    currentRasterMemoryBytes = 0;
}
