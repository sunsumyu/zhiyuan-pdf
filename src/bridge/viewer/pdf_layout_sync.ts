import { logPdfLayoutTrace } from '../render/layout_trace';
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
        const rect = scrollContainer.getBoundingClientRect();
        const wasm = deps.getWasmApi();
        logPdfLayoutTrace('layout.sync.before', {
            displayZoom,
            renderedZoom,
            safeDisplayZoom,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
            viewportWidth: scrollContainer.clientWidth || rect.width || 0,
            viewportHeight: scrollContainer.clientHeight || rect.height || 0,
            layoutOverride: layoutOverride ?? null,
            zoomState: deps.readZoomState(),
        });
        const layout = wasm.syncHostLayout?.({
            displayZoom: safeDisplayZoom,
            renderZoom: renderedZoom > 0 ? renderedZoom : safeDisplayZoom,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
            viewportWidth: scrollContainer.clientWidth || rect.width || 0,
            viewportHeight: scrollContainer.clientHeight || rect.height || 0,
            layoutOverride: layoutOverride ? {
                hostWidth: layoutOverride.hostWidth,
                hostHeight: layoutOverride.hostHeight,
                contentLeft: layoutOverride.contentLeft,
                contentTop: layoutOverride.contentTop,
            } : null,
        }) ?? null;

        const domWidth = Number.isFinite(layout?.domWidth) ? layout.domWidth : deps.getPageWidth() * (renderedZoom > 0 ? renderedZoom : safeDisplayZoom);
        const domHeight = Number.isFinite(layout?.domHeight) ? layout.domHeight : deps.getPageHeight() * (renderedZoom > 0 ? renderedZoom : safeDisplayZoom);
        const displayWidth = Number.isFinite(layout?.displayWidth) ? layout.displayWidth : deps.getPageWidth() * safeDisplayZoom;
        const displayHeight = Number.isFinite(layout?.displayHeight) ? layout.displayHeight : deps.getPageHeight() * safeDisplayZoom;
        const hostWidth = Number.isFinite(layout?.hostWidth) ? layout.hostWidth : displayWidth;
        const hostHeight = Number.isFinite(layout?.hostHeight) ? layout.hostHeight : displayHeight;
        const contentLeft = Number.isFinite(layout?.contentLeft) ? layout.contentLeft : 0;
        const contentTop = Number.isFinite(layout?.contentTop) ? layout.contentTop : 0;
        const cssScale = Number.isFinite(layout?.cssScale) ? layout.cssScale : (renderedZoom > 0 ? safeDisplayZoom / renderedZoom : 1.0);

        wrapper.style.display = 'block';
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
            // CSS transform is NOT set here — callers manage it explicitly
            // via applyPreviewFrame (preview) or applyCommittedFrame (settle).
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
        scrollContainer.style.scrollbarGutter = 'stable both-edges';
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
