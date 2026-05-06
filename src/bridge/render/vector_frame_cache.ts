import { logPdfLayoutTrace } from './layout_trace';

const viewportFrameCache = new Map<string, HTMLCanvasElement>();

function cloneCanvas(source: HTMLCanvasElement): HTMLCanvasElement {
    const snapshot = document.createElement('canvas');
    snapshot.width = source.width;
    snapshot.height = source.height;
    const ctx = snapshot.getContext('2d', { alpha: false });
    if (ctx) {
        ctx.drawImage(source, 0, 0);
    }
    return snapshot;
}

export function clearVectorFrameCache(): void {
    logPdfLayoutTrace('frame-cache.clear', {
        sizeBefore: viewportFrameCache.size,
    });
    viewportFrameCache.clear();
}

export function readViewportFrameCache(key: string): HTMLCanvasElement | null {
    const cached = viewportFrameCache.get(key) ?? null;
    logPdfLayoutTrace(cached ? 'frame-cache.hit' : 'frame-cache.miss', {
        key,
        size: viewportFrameCache.size,
        canvasWidth: cached?.width,
        canvasHeight: cached?.height,
    });
    return cached;
}

export function writeViewportFrameCache(key: string, sourceCanvas: HTMLCanvasElement): void {
    logPdfLayoutTrace('frame-cache.write.before', {
        key,
        sizeBefore: viewportFrameCache.size,
        sourceCanvasWidth: sourceCanvas.width,
        sourceCanvasHeight: sourceCanvas.height,
    });
    viewportFrameCache.set(key, cloneCanvas(sourceCanvas));
    logPdfLayoutTrace('frame-cache.write.after', {
        key,
        sizeAfter: viewportFrameCache.size,
    });
}

export function deleteViewportFrameCacheKeys(keys: string[]): void {
    if (keys.length > 0) {
        logPdfLayoutTrace('frame-cache.delete.before', {
            keys,
            sizeBefore: viewportFrameCache.size,
        });
    }
    for (const key of keys) {
        if (!key) continue;
        viewportFrameCache.delete(key);
    }
    if (keys.length > 0) {
        logPdfLayoutTrace('frame-cache.delete.after', {
            keys,
            sizeAfter: viewportFrameCache.size,
        });
    }
}
