import { logPdfLayoutTrace } from '../render/layout_trace';
import { getPresentationSurfaceOwner } from '../render/presentation_surface_owner';
import { getViewportGeometry } from './viewport_geometry';
import {
    getScrollContainer,
    getVectorContainer,
    getWrapper,
    MIN_ZOOM,
} from './pdf_viewer_dom';
import type { WasmModule } from '../shared/wasm_loader';

type LayoutOverride = {
    hostWidth: number;
    hostHeight: number;
    contentLeft: number;
    contentTop: number;
    scrollLeft: number;
    scrollTop: number;
} | null;

type LayoutSyncDeps = {
    getWasmApi: () => WasmModule;
    getPageWidth: () => number;
    getPageHeight: () => number;
    readZoomState: () => { currentZoom: number; targetZoom: number; visualZoom: number; lastRenderedZoom: number };
};

export function createLayoutSync(deps: LayoutSyncDeps) {
    function syncLayoutBox(
        displayZoom: number,
        renderedZoom: number,
        layoutOverride?: LayoutOverride,
    ): void {
        const wrapper = getWrapper();
        const container = getVectorContainer();
        const scrollContainer = getScrollContainer();
        if (!wrapper || !scrollContainer) return;

        const safeDisplayZoom = Math.max(displayZoom, MIN_ZOOM);
        // ADR-0014: viewport geometry comes from its single owner (cached
        // within a frame) instead of a fresh getBoundingClientRect here.
        const vp = getViewportGeometry().read();
        const wasm = deps.getWasmApi();
        logPdfLayoutTrace('layout.sync.before', {
            displayZoom,
            renderedZoom,
            safeDisplayZoom,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
            viewportWidth: vp.width,
            viewportHeight: vp.height,
            layoutOverride: layoutOverride ?? null,
            zoomState: deps.readZoomState(),
        });
        const layout = wasm.syncHostLayout?.({
            displayZoom: safeDisplayZoom,
            renderZoom: renderedZoom > 0 ? renderedZoom : safeDisplayZoom,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
            viewportWidth: vp.width,
            viewportHeight: vp.height,
            layoutOverride: layoutOverride ? {
                hostWidth: layoutOverride.hostWidth,
                hostHeight: layoutOverride.hostHeight,
                contentLeft: layoutOverride.contentLeft,
                contentTop: layoutOverride.contentTop,
            } : null,
        }) ?? null;

        // Rust owns fallback layout computation — no TS-side formulas.
        let fallback = null as { domWidth: number; domHeight: number; displayWidth: number; displayHeight: number; hostWidth: number; hostHeight: number; contentLeft: number; contentTop: number; cssScale: number } | null;
        try {
            fallback = wasm.resolveLayoutFallback?.({
                pageWidth: deps.getPageWidth(),
                pageHeight: deps.getPageHeight(),
                displayZoom: safeDisplayZoom,
            }) ?? null;
        } catch { /* WASM not available */ }
        const domWidth = Number.isFinite(layout?.domWidth) ? layout.domWidth : (fallback?.domWidth ?? deps.getPageWidth());
        const domHeight = Number.isFinite(layout?.domHeight) ? layout.domHeight : (fallback?.domHeight ?? deps.getPageHeight());
        const displayWidth = Number.isFinite(layout?.displayWidth) ? layout.displayWidth : (fallback?.displayWidth ?? deps.getPageWidth());
        const displayHeight = Number.isFinite(layout?.displayHeight) ? layout.displayHeight : (fallback?.displayHeight ?? deps.getPageHeight());
        const hostWidth = Number.isFinite(layout?.hostWidth) ? layout.hostWidth : (fallback?.hostWidth ?? displayWidth);
        const hostHeight = Number.isFinite(layout?.hostHeight) ? layout.hostHeight : (fallback?.hostHeight ?? displayHeight);
        const contentLeft = Number.isFinite(layout?.contentLeft) ? layout.contentLeft : (fallback?.contentLeft ?? 0);
        const contentTop = Number.isFinite(layout?.contentTop) ? layout.contentTop : (fallback?.contentTop ?? 0);

        // Visibility is owned by PresentationSurfaceOwner (ADR-0011): re-assert
        // that a document is on screen. Geometry below stays here (size ≠
        // visibility); the owner's show is idempotent and never hides.
        getPresentationSurfaceOwner().showDocument();
        wrapper.style.position = 'relative';
        wrapper.style.width = `${hostWidth}px`;
        wrapper.style.height = `${hostHeight}px`;
        wrapper.style.margin = '0';
        wrapper.style.textAlign = 'left';
        wrapper.style.transform = '';
        wrapper.style.transformOrigin = '0 0';

        if (container) {
            container.style.position = 'absolute';
            container.style.left = `${contentLeft}px`;
            container.style.top = `${contentTop}px`;
            container.style.width = `${domWidth}px`;
            container.style.height = `${domHeight}px`;
            container.style.margin = '0';
            container.style.transformOrigin = '0 0';
        }

        const rasterCanvas = document.getElementById('pdf-render-target') as HTMLCanvasElement | null;
        if (rasterCanvas) {
            rasterCanvas.style.position = 'absolute';
            rasterCanvas.style.left = `${contentLeft}px`;
            rasterCanvas.style.top = `${contentTop}px`;
            rasterCanvas.style.width = `${domWidth}px`;
            rasterCanvas.style.height = `${domHeight}px`;
            rasterCanvas.style.margin = '0';
            rasterCanvas.style.transformOrigin = '0 0';
            // CSS transform is NOT set here — callers manage it explicitly.
        }

        scrollContainer.style.overflowX = 'auto';
        scrollContainer.style.overflowY = 'auto';
        scrollContainer.style.textAlign = 'left';
        scrollContainer.style.padding = '0';
        scrollContainer.style.position = 'relative';
        logPdfLayoutTrace('layout.sync.after', {
            displayZoom,
            renderedZoom,
            safeDisplayZoom,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
            layout: {
                displayWidth,
                displayHeight,
                hostWidth,
                hostHeight,
                contentLeft,
                contentTop,
            },
            layoutOverride: layoutOverride ?? null,
            zoomState: deps.readZoomState(),
        });
    }

    return { syncLayoutBox };
}
