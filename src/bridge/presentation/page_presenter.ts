import { hideVectorCanvasHostForPreview } from '../render/vector_canvas_host';
import { clearVectorHost } from '../render/vector_host';
import { readDecodedRasterImage, warmRasterImage } from '../render/raster_image_cache';
import { emitPdfDiagnostic } from '../shared/diagnostics';

export type RasterSurfaceRole = 'current' | 'preview' | 'prefetch' | 'unknown';

export type RasterSurfaceOptions = {
    hideVectorOnly?: boolean;
    role?: RasterSurfaceRole;
    pageIndex?: number;
};

type PreparedRasterSurface = {
    src: string;
    pageWidth?: number;
    pageHeight?: number;
    displayZoom?: number;
    role: RasterSurfaceRole;
    pageIndex?: number;
    preparedElapsedMs: number;
};

type PagePresenterDeps = {
    getWrapper: () => HTMLElement | null;
    getRasterTarget: () => HTMLCanvasElement | null;
    getEmptyState: () => HTMLElement | null;
    clearEditorOverlay: () => void;
};

function logPresent(event: string, fields: Record<string, unknown> = {}): void {
    emitPdfDiagnostic('present', event, fields);
}

export function createPagePresenter(deps: PagePresenterDeps) {
    async function prepareRasterSurface(
        src: string,
        pageWidth?: number,
        pageHeight?: number,
        displayZoom?: number,
        options: RasterSurfaceOptions = {},
    ): Promise<PreparedRasterSurface | null> {
        const startedAt = performance.now();
        const role = options.role ?? 'current';
        const cached = readDecodedRasterImage(src);
        if (cached) {
            logPresent('raster.prepare.cache-hit', {
                role,
                pageIndex: options.pageIndex,
            });
        } else if (role === 'current') {
            emitPdfDiagnostic('PROF', 'current-raster-miss-critical-path', {
                role,
                pageIndex: options.pageIndex,
            }, { level: 'WARN', layer: 'PERF' });
        }

        const image = cached ?? await warmRasterImage(src, {
            role,
            pageIndex: options.pageIndex,
        });

        const preparedElapsedMs = performance.now() - startedAt;
        if (!image) {
            logPresent('raster.prepare.failed', {
                role,
                pageIndex: options.pageIndex,
                elapsedMs: preparedElapsedMs,
            });
            return null;
        }

        logPresent('raster.prepare.ready', {
            role,
            pageIndex: options.pageIndex,
            elapsedMs: preparedElapsedMs,
            naturalWidth: image.width,
            naturalHeight: image.height,
        });

        return {
            src,
            pageWidth,
            pageHeight,
            displayZoom,
            role,
            pageIndex: options.pageIndex,
            preparedElapsedMs,
        };
    }

    function commitRasterSurface(
        surface: PreparedRasterSurface,
        options: RasterSurfaceOptions = {},
    ): boolean {
        const canvas = deps.getRasterTarget();
        const wrapper = deps.getWrapper();
        const emptyState = deps.getEmptyState();
        if (!canvas || !wrapper) return false;

        const bitmap = readDecodedRasterImage(surface.src);
        if (!bitmap) return false;

        const commitStartedAt = performance.now();
        if (options.hideVectorOnly) {
            hideVectorCanvasHostForPreview();
        } else {
            clearVectorHost();
        }
        deps.clearEditorOverlay();

        canvas.style.display = 'block';
        
        // Match internal resolution to bitmap
        if (canvas.width !== bitmap.width || canvas.height !== bitmap.height) {
            canvas.width = bitmap.width;
            canvas.height = bitmap.height;
        }
        const ctx = canvas.getContext('2d');
        if (ctx) {
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            ctx.drawImage(bitmap, 0, 0);
        }

        if (
            surface.pageWidth &&
            surface.pageWidth > 0 &&
            surface.pageHeight &&
            surface.pageHeight > 0
        ) {
            const zoom = surface.displayZoom && surface.displayZoom > 0
                ? surface.displayZoom
                : 1;
            const cssW = Math.round(surface.pageWidth * zoom);
            const cssH = Math.round(surface.pageHeight * zoom);
            canvas.style.width = cssW + 'px';
            canvas.style.height = cssH + 'px';
            wrapper.style.width = cssW + 'px';
            wrapper.style.height = cssH + 'px';
        }

        wrapper.style.display = 'block';
        if (emptyState) emptyState.style.display = 'none';

        logPresent('raster.commit', {
            role: surface.role,
            pageIndex: surface.pageIndex,
            elapsedMs: performance.now() - commitStartedAt,
            preparedElapsedMs: surface.preparedElapsedMs,
        });
        return true;
    }

    async function presentRaster(
        src: string,
        pageWidth?: number,
        pageHeight?: number,
        displayZoom?: number,
        options: RasterSurfaceOptions = {},
    ): Promise<boolean> {
        const surface = await prepareRasterSurface(src, pageWidth, pageHeight, displayZoom, options);
        if (!surface) return false;
        if ((options.role ?? 'current') === 'prefetch') return true;
        return commitRasterSurface(surface, options);
    }

    /**
     * ready-only commit: 仅在 raster image 已解码命中 cache 时提交，否则打性能违规日志并返回 false。
     * 用于 preview-first 路径：current miss 不等待 decode，vector render 在后台继续进行。
     * 这样可以消除 40-50ms decode 阻塞当前可见路径。
     */
    function commitReadySurfaceOrFallback(
        src: string,
        pageWidth?: number,
        pageHeight?: number,
        displayZoom?: number,
        options: RasterSurfaceOptions = {},
    ): boolean {
        const role = options.role ?? 'current';
        const cached = readDecodedRasterImage(src);
        if (cached) {
            logPresent('raster.ready-only.cache-hit', {
                role,
                pageIndex: options.pageIndex,
            });
            const surface: PreparedRasterSurface = {
                src,
                pageWidth,
                pageHeight,
                displayZoom,
                role,
                pageIndex: options.pageIndex,
                preparedElapsedMs: 0,
            };
            return commitRasterSurface(surface, options);
        }

        // cache miss: 不等待，打性能违规日志
        emitPdfDiagnostic('PROF', 'current-raster-miss-ready-only-fallback', {
            role,
            pageIndex: options.pageIndex,
        }, { level: 'WARN', layer: 'PERF' });
        return false;
    }

    return {
        prepareRasterSurface,
        commitRasterSurface,
        commitReadySurfaceOrFallback,
        presentRaster,
    };
}
