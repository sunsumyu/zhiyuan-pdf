import { emitPdfDiagnostic } from '../shared/diagnostics';

const RASTER_IMAGE_CACHE_MAX = 15;

const decodedRasterImages = new Map<string, HTMLImageElement>();
const inflightRasterImages = new Map<string, Promise<HTMLImageElement | null>>();

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
        maxSize: RASTER_IMAGE_CACHE_MAX,
    });
}

function rememberRasterImage(
    src: string,
    image: HTMLImageElement,
    options: RasterWarmOptions = {},
): HTMLImageElement {
    decodedRasterImages.delete(src);
    decodedRasterImages.set(src, image);
    while (decodedRasterImages.size > RASTER_IMAGE_CACHE_MAX) {
        const oldest = decodedRasterImages.keys().next().value;
        if (!oldest) break;
        decodedRasterImages.delete(oldest);
        logRasterCache('evict', oldest, {
            role: options.role ?? 'unknown',
            pageIndex: options.pageIndex,
        });
    }
    logRasterCache('store', src, {
        role: options.role ?? 'unknown',
        pageIndex: options.pageIndex,
        naturalWidth: image.naturalWidth,
        naturalHeight: image.naturalHeight,
    });
    return image;
}

function waitForImageLoad(image: HTMLImageElement): Promise<void> {
    if (image.complete && image.naturalWidth > 0) {
        return Promise.resolve();
    }
    return new Promise((resolve, reject) => {
        image.onload = () => resolve();
        image.onerror = () => reject(new Error('raster image load failed'));
    });
}

export function readDecodedRasterImage(src: string): HTMLImageElement | null {
    const cached = decodedRasterImages.get(src);
    if (!cached) return null;
    decodedRasterImages.delete(src);
    decodedRasterImages.set(src, cached);
    return cached;
}

export function warmRasterImage(
    src: string,
    options: RasterWarmOptions = {},
): Promise<HTMLImageElement | null> {
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
        const image = new Image();
        image.decoding = 'async';
        image.src = src;
        try {
            logRasterCache('miss', src, {
                role: options.role ?? 'unknown',
                pageIndex: options.pageIndex,
            });
            if (typeof image.decode === 'function') {
                await image.decode();
            } else {
                await waitForImageLoad(image);
            }
            return rememberRasterImage(src, image, options);
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
    decodedRasterImages.clear();
    inflightRasterImages.clear();
}
