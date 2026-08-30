import { ensureVectorHost } from '../render/vector_host';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import type { FramePlanAdapter } from '../render/frame_plan';
import type { ViewerSessionAdapter } from './viewer_session';

import type { WasmModule } from '../shared/wasm_loader';

type GeometryProbeDeps = {
    ensureWasmInitialized: () => Promise<any>;
    getWasmApi: () => WasmModule;
    viewerSession: ViewerSessionAdapter;
    framePlanAdapter: FramePlanAdapter;
    getZoomState: () => { targetZoom: number; visualZoom: number; lastRenderedZoom: number };
    getScrollContainer: () => HTMLElement | null;
    getVectorContainer: () => HTMLElement | null;
    syncLayoutBox: (
        displayZoom: number,
        renderedZoom: number,
        layoutOverride?: {
            hostWidth: number;
            hostHeight: number;
            contentLeft: number;
            contentTop: number;
            scrollLeft: number;
            scrollTop: number;
        } | null,
    ) => void;
    syncZoomSelect: () => void;
    showWrapper: () => void;
    setPageDimensions: (pageWidth: number, pageHeight: number) => void;
    getPageWidth: () => number;
    getPageHeight: () => number;
    clampZoom: (zoom: number) => number;
    getMaxZoom: () => number;
};

type GeometryProbeSnapshot = {
    pageWidth: number;
    pageHeight: number;
    zoomState: { targetZoom: number; visualZoom: number; lastRenderedZoom: number };
    pagePoint: { x: number; y: number };
    clientPoint: { x: number; y: number };
    containerRect: { left: number; top: number; width: number; height: number };
    scrollOffsets: { left: number; top: number };
};

type GeometryProbeApi = {
    init: (pageWidth?: number, pageHeight?: number, zoom?: number) => Promise<GeometryProbeSnapshot | null>;
    snapshot: (pageRatioX?: number, pageRatioY?: number) => GeometryProbeSnapshot | null;
    wheelAtClient: (clientX: number, clientY: number, deltaY: number) => GeometryProbeSnapshot | null;
};

function projectSnapshot(
    deps: GeometryProbeDeps,
    pageRatioX = 0.32,
    pageRatioY = 0.22,
): GeometryProbeSnapshot | null {
    const container = deps.getVectorContainer();
    const scrollContainer = deps.getScrollContainer();
    if (!container || !scrollContainer) return null;

    const session = deps.viewerSession.read();
    const pageWidth = session.pageWidth || deps.getPageWidth();
    const pageHeight = session.pageHeight || deps.getPageHeight();
    const rect = container.getBoundingClientRect();
    const scaleX = pageWidth > 0 ? rect.width / pageWidth : 1;
    const scaleY = pageHeight > 0 ? rect.height / pageHeight : 1;
    const pageX = Math.max(0, Math.min(pageWidth, pageWidth * pageRatioX));
    const pageY = Math.max(0, Math.min(pageHeight, pageHeight * pageRatioY));

    return {
        pageWidth,
        pageHeight,
        zoomState: deps.getZoomState(),
        pagePoint: { x: pageX, y: pageY },
        clientPoint: {
            x: rect.left + pageX * scaleX,
            y: rect.top + pageY * scaleY,
        },
        containerRect: {
            left: rect.left,
            top: rect.top,
            width: rect.width,
            height: rect.height,
        },
        scrollOffsets: {
            left: scrollContainer.scrollLeft,
            top: scrollContainer.scrollTop,
        },
    };
}

function applyRenderPlan(
    deps: GeometryProbeDeps,
    displayZoom: number,
    renderZoom: number,
    plan: {
        hostWidth: number;
        hostHeight: number;
        contentLeft: number;
        contentTop: number;
        scrollLeft: number;
        scrollTop: number;
    } | null | undefined,
): void {
    const container = deps.getVectorContainer();
    const scrollContainer = deps.getScrollContainer();
    if (!container || !scrollContainer) return;

    deps.syncLayoutBox(displayZoom, renderZoom, plan ?? null);

    if (plan) {
        scrollContainer.scrollLeft = plan.scrollLeft;
        scrollContainer.scrollTop = plan.scrollTop;
    }
}

export function createViewerGeometryProbe(deps: GeometryProbeDeps): GeometryProbeApi {
    async function init(pageWidth = 595, pageHeight = 842, zoom = 1.0): Promise<GeometryProbeSnapshot | null> {
        emitPdfDiagnostic('geometry-probe', 'init', { pageWidth, pageHeight, zoom }, { verboseOnly: true });
        await deps.ensureWasmInitialized();
        deps.setPageDimensions(pageWidth, pageHeight);
        deps.viewerSession.setDocument('__geometry_probe__', 1, zoom);
        deps.viewerSession.setPageDimensions(pageWidth, pageHeight);
        deps.getWasmApi().resetZoomState?.(zoom);
        deps.showWrapper();
        ensureVectorHost();

        const container = deps.getVectorContainer();
        if (container) {
            container.style.background = 'repeating-linear-gradient(0deg, #ffffff, #ffffff 23px, #f1f5ff 24px)';
            container.style.outline = '1px solid rgba(83, 141, 211, 0.35)';
        }

        const plan = deps.framePlanAdapter.peek(zoom);
        applyRenderPlan(deps, zoom, plan?.renderZoom ?? zoom, plan);
        deps.syncZoomSelect();
        return projectSnapshot(deps);
    }

    function snapshot(pageRatioX = 0.32, pageRatioY = 0.22): GeometryProbeSnapshot | null {
        return projectSnapshot(deps, pageRatioX, pageRatioY);
    }

    function wheelAtClient(clientX: number, clientY: number, deltaY: number): GeometryProbeSnapshot | null {
        const scrollContainer = deps.getScrollContainer();
        if (!scrollContainer) return null;

        const rect = scrollContainer.getBoundingClientRect();
        const zoomState = deps.getZoomState();
        const currentDisplayZoom = zoomState.visualZoom > 0 ? zoomState.visualZoom : zoomState.targetZoom;
        const pageWidth = deps.getPageWidth();
        const pageHeight = deps.getPageHeight();
        emitPdfDiagnostic('geometry-probe', 'wheel', { pageWidth, pageHeight, clientX, clientY, deltaY }, { verboseOnly: true });
        const wheelPlan = deps.getWasmApi().resolveWheelZoom?.({
            deltaY,
            viewportX: clientX - rect.left,
            viewportY: clientY - rect.top,
            viewportWidth: scrollContainer.clientWidth || rect.width || 0,
            viewportHeight: scrollContainer.clientHeight || rect.height || 0,
            pageWidth,
            pageHeight,
            scrollLeft: scrollContainer.scrollLeft,
            scrollTop: scrollContainer.scrollTop,
            contentWidth: pageWidth * currentDisplayZoom,
            contentHeight: pageHeight * currentDisplayZoom,
            targetZoom: zoomState.targetZoom,
            minZoom: 0.1,
            maxZoom: deps.getMaxZoom(),
        });

        const nextZoom = deps.clampZoom(Number(wheelPlan?.targetZoom || zoomState.targetZoom));
        // Instant jump: this probe renders synchronously with no RAF animation,
        // so visual_zoom must snap to target or css_scale inverts on commit.
        deps.getWasmApi().setTargetZoomInstant?.(nextZoom);
        deps.viewerSession.setCurrentZoom(nextZoom);

        const plan = deps.framePlanAdapter.take(nextZoom) ?? deps.framePlanAdapter.peek(nextZoom);
        const renderZoom = plan?.renderZoom ?? nextZoom;
        applyRenderPlan(deps, nextZoom, renderZoom, plan);
        deps.getWasmApi().markRenderedZoom?.(renderZoom);
        deps.syncZoomSelect();
        return projectSnapshot(deps);
    }

    return {
        init,
        snapshot,
        wheelAtClient,
    };
}



