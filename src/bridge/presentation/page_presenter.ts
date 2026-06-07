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
    getRasterTarget: () => HTMLImageElement | null;
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
            naturalWidth: image.naturalWidth,
            naturalHeight: image.naturalHeight,
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
        const img = deps.getRasterTarget();
        const wrapper = deps.getWrapper();
        const emptyState = deps.getEmptyState();
        if (!img || !wrapper) return false;

        const commitStartedAt = performance.now();
        if (options.hideVectorOnly) {
            hideVectorCanvasHostForPreview();
        } else {
            clearVectorHost();
        }
        deps.clearEditorOverlay();

        img.style.display = 'block';
        if (img.src !== surface.src) {
            img.decoding = 'async';
            img.src = surface.src;
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
            img.style.width = cssW + 'px';
            img.style.height = cssH + 'px';
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

    return {
        prepareRasterSurface,
        commitRasterSurface,
        presentRaster,
    };
}
