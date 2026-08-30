import { logPdfLayoutTrace } from './layout_trace';
import { createRenderWasmApi } from './render_wasm_api';
import { emitPdfDiagnostic } from '../shared/diagnostics';

export type RustFramePlan = {
    renderReason: string;
    prepareVisibleLayout?: boolean;
    displayZoom: number;
    renderZoom: number;
    baseRenderZoom: number;
    baseCacheZoom: number;
    detailCacheZoom: number;
    baseCacheKey: string;
    detailCacheKey: string;
    cssScale: number;
    useViewportTile: boolean;
    previewSettled: boolean;
    allowRenderDuringPreview: boolean;
    showDetailOverlay: boolean;
    reuseActiveBaseLayer: boolean;
    renderBaseLayer: boolean;
    preferProgressiveBase: boolean;
    reuseActiveDetailTile: boolean;
    renderDetailLayer: boolean;
    preferProgressiveDetail: boolean;
    hostWidth: number;
    hostHeight: number;
    contentLeft: number;
    contentTop: number;
    scrollLeft: number;
    scrollTop: number;
    tileLeft: number;
    tileTop: number;
    tileWidth: number;
    tileHeight: number;
};

export type RustPreviewFrame = {
    settled: boolean;
    visualZoom: number;
    renderedBaseZoom: number;
    cssScale: number;
    previewPresent: {
        translateX: number;
        translateY: number;
        cssScale: number;
    };
    framePlan: RustFramePlan;
};

export type RustRenderFrame = {
    frameToken: number;
    framePlan: RustFramePlan;
};

export type RustRenderTransition = {
    accepted: boolean;
    nextFrame: RustRenderFrame | null;
};

export type RustRenderCommitResult = {
    accepted: boolean;
    nextFrame: RustRenderFrame | null;
    pageWidth: number;
    pageHeight: number;
};

export type RustViewportRefreshDecision = {
    shouldRefresh: boolean;
    delayMs: number;
};

export type RustLayoutFallback = {
    domWidth: number;
    domHeight: number;
    displayWidth: number;
    displayHeight: number;
    hostWidth: number;
    hostHeight: number;
    contentLeft: number;
    contentTop: number;
    cssScale: number;
};

export type RustFitToWidthResult = {
    fitZoom: number;
    shouldFit: boolean;
};

export type RustLayerExecutionPlan = {
    skipRender: boolean;
    renderBaseLayer: boolean;
    renderDetailLayer: boolean;
    showDetailOverlay: boolean;
    retainDetailOverlayDuringBase: boolean;
};

export type RustLayerPresentDecision = {
    showDetailOverlay: boolean;
    retainDetailOverlay: boolean;
};

type FramePlanAdapterDeps = {
    getWasmApi: () => any;
    getScrollContainer: () => HTMLElement | null;
    getPageWidth: () => number;
    getPageHeight: () => number;
    getMaxZoom: () => number;
    getMaxCanvasDim: () => number;
};

export type RenderReason = 'default' | 'navigation' | 'zoom' | 'editorVisibility' | 'documentMutation';

export type FramePlanAdapter = {
    buildRenderRequest: (displayZoom: number, renderReason?: RenderReason) => Record<string, number | string | boolean>;
    peek: (displayZoom: number, renderReason?: RenderReason) => RustFramePlan | null;
    take: (displayZoom: number, renderReason?: RenderReason) => RustFramePlan | null;
    stepPreview: (displayZoom: number, timestampMs?: number) => RustPreviewFrame | null;
    resolveViewportRefresh: (displayZoom: number, timestampMs?: number) => RustViewportRefreshDecision | null;
    resolveHostScrollRefresh: (displayZoom: number, timestampMs?: number) => RustViewportRefreshDecision | null;
    scheduleRender: (displayZoom: number, renderReason?: RenderReason) => RustRenderFrame | null;
    settleRender: (frameToken: number | null, renderedZoom: number) => RustRenderTransition | null;
    abortRender: (frameToken: number | null) => RustRenderTransition | null;
    commitRenderResult: (frameToken: number, renderedZoom: number, pageWidth: number, pageHeight: number) => RustRenderCommitResult | null;
    isImmediateMutationFrame: (renderReason: string) => boolean;
    resolveFitToWidth: (viewportWidth: number, pageWidth: number) => RustFitToWidthResult | null;
    scheduleRenderFollowUp: (renderedDisplayZoom: number) => RustRenderFrame | null;
    resolveLayerExecutionPlan: (bundleChanged: boolean, framePlan: RustFramePlan) => RustLayerExecutionPlan | null;
    resolveLayerPresentDecision: (useDetailLayer: boolean, framePlan: RustFramePlan) => RustLayerPresentDecision | null;
    isRenderFrameCurrent: (frameToken: number | null) => boolean;
    queueRenderLoopFrame: (frame: RustRenderFrame | null) => RustRenderFrame | null;
    advanceRenderLoopFrame: (frame: RustRenderFrame | null) => RustRenderFrame | null;
};

export function createFramePlanAdapter(deps: FramePlanAdapterDeps): FramePlanAdapter {
    const renderApi = createRenderWasmApi(deps.getWasmApi);

    function buildRequest(displayZoom: number, renderReason: RenderReason = 'default'): Record<string, number | string | boolean> {
        const scrollContainer = deps.getScrollContainer();
        const rect = scrollContainer?.getBoundingClientRect();
        
        const vw = scrollContainer?.clientWidth || rect?.width || 0;
        const vh = scrollContainer?.clientHeight || rect?.height || 0;
        const dpr = window.devicePixelRatio || 1;

        logPdfLayoutTrace('frame.request.build', {
            displayZoom,
            renderReason,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
            viewportWidth: vw,
            viewportHeight: vh,
            scrollLeft: scrollContainer?.scrollLeft || 0,
            scrollTop: scrollContainer?.scrollTop || 0,
            dpr,
        });

        return {
            displayZoom,
            renderReason,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
            viewportWidth: vw,
            viewportHeight: vh,
            scrollLeft: scrollContainer?.scrollLeft || 0,
            scrollTop: scrollContainer?.scrollTop || 0,
            devicePixelRatio: dpr,
            maxZoom: deps.getMaxZoom(),
            maxCanvasDim: deps.getMaxCanvasDim(),
            timestampMs: performance.now(),
            // CRITICAL FIX: During mutation, we MUST prevent the zoom-jump.
            // Force the layout engine to treat the current displayZoom as the rendered scale
            // to avoid transient CSS scale explosions.
            forceStaticRenderScale: renderReason === 'documentMutation',
        };
    }

    function buildRenderRequest(displayZoom: number, renderReason: RenderReason = 'default'): Record<string, number | string | boolean> {
        return buildRequest(displayZoom, renderReason);
    }

    function peek(displayZoom: number, renderReason: RenderReason = 'default'): RustFramePlan | null {
        try {
            return renderApi.resolveFramePlan(buildRequest(displayZoom, renderReason)) as RustFramePlan;
        } catch (err) {
            emitPdfDiagnostic('RENDER', 'peek.error', { error: String(err) }, { level: 'ERROR' });
            return null;
        }
    }

    function take(displayZoom: number, renderReason: RenderReason = 'default'): RustFramePlan | null {
        try {
            return renderApi.takeFramePlan(buildRequest(displayZoom, renderReason)) as RustFramePlan;
        } catch {
            return null;
        }
    }

    function stepPreview(displayZoom: number, timestampMs?: number): RustPreviewFrame | null {
        try {
            const request = buildRequest(displayZoom, 'zoom');
            if (Number.isFinite(timestampMs as number)) {
                request.timestampMs = timestampMs as number;
            }
            return renderApi.stepZoomFramePlan(request) as RustPreviewFrame;
        } catch {
            return null;
        }
    }

    function resolveViewportRefresh(displayZoom: number, timestampMs?: number): RustViewportRefreshDecision | null {
        try {
            const request = buildRequest(displayZoom, 'zoom');
            if (Number.isFinite(timestampMs as number)) {
                request.timestampMs = timestampMs as number;
            }
            return renderApi.resolveViewportRefresh(request) as RustViewportRefreshDecision;
        } catch {
            return null;
        }
    }

    function resolveHostScrollRefresh(displayZoom: number, timestampMs?: number): RustViewportRefreshDecision | null {
        try {
            const request = buildRequest(displayZoom, 'zoom');
            if (Number.isFinite(timestampMs as number)) {
                request.timestampMs = timestampMs as number;
            }
            return renderApi.resolveHostScrollRefresh(request) as RustViewportRefreshDecision;
        } catch {
            return null;
        }
    }

    function scheduleRender(displayZoom: number, renderReason: RenderReason = 'default'): RustRenderFrame | null {
        try {
            return renderApi.scheduleRenderFrame(buildRequest(displayZoom, renderReason)) as RustRenderFrame;
        } catch (err) {
            emitPdfDiagnostic('RENDER', 'scheduleRender.error', { error: String(err) }, { level: 'ERROR' });
            return null;
        }
    }

    function settleRender(frameToken: number | null, renderedZoom: number): RustRenderTransition | null {
        if (!frameToken || !Number.isFinite(frameToken)) {
            try {
                renderApi.markRenderedZoom(renderedZoom);
                return { accepted: true, nextFrame: null };
            } catch {
                return null;
            }
        }
        try {
            return renderApi.settleRenderFrame(frameToken, renderedZoom) as RustRenderTransition;
        } catch {
            return null;
        }
    }

    function abortRender(frameToken: number | null): RustRenderTransition | null {
        if (!frameToken || !Number.isFinite(frameToken)) {
            return { accepted: false, nextFrame: null };
        }
        try {
            return renderApi.abortRenderFrame(frameToken) as RustRenderTransition;
        } catch {
            return null;
        }
    }

    function commitRenderResult(
        frameToken: number,
        renderedZoom: number,
        pageWidth: number,
        pageHeight: number,
    ): RustRenderCommitResult | null {
        try {
            return renderApi.commitRenderResult(
                frameToken,
                renderedZoom,
                pageWidth,
                pageHeight,
            ) as RustRenderCommitResult;
        } catch {
            return null;
        }
    }

    function isImmediateMutationFrame(renderReason: string): boolean {
        try {
            return renderApi.isImmediateMutationFrame(renderReason);
        } catch {
            return renderReason === 'editorVisibility' || renderReason === 'documentMutation';
        }
    }

    function resolveFitToWidth(viewportWidth: number, pageWidth: number): RustFitToWidthResult | null {
        try {
            return renderApi.resolveFitToWidth(viewportWidth, pageWidth) as RustFitToWidthResult;
        } catch {
            return null;
        }
    }

    function scheduleRenderFollowUp(renderedDisplayZoom: number): RustRenderFrame | null {
        try {
            return renderApi.scheduleRenderFollowUp(
                renderedDisplayZoom,
                buildRequest(renderedDisplayZoom, 'zoom'),
            ) as RustRenderFrame;
        } catch {
            return null;
        }
    }

    function isRenderFrameCurrent(frameToken: number | null): boolean {
        if (!frameToken || !Number.isFinite(frameToken)) return true;
        try {
            return renderApi.isRenderFrameCurrent(frameToken);
        } catch {
            return true;
        }
    }

    function queueRenderLoopFrame(frame: RustRenderFrame | null): RustRenderFrame | null {
        try {
            return renderApi.queueRenderLoopFrame(frame) as RustRenderFrame;
        } catch {
            return frame;
        }
    }

    function advanceRenderLoopFrame(frame: RustRenderFrame | null): RustRenderFrame | null {
        try {
            return renderApi.advanceRenderLoopFrame(frame) as RustRenderFrame;
        } catch {
            return frame;
        }
    }

    function resolveLayerExecutionPlan(bundleChanged: boolean, framePlan: RustFramePlan): RustLayerExecutionPlan | null {
        try {
            return renderApi.resolveLayerExecutionPlan(
                bundleChanged,
                framePlan,
            ) as RustLayerExecutionPlan;
        } catch {
            return null;
        }
    }

    function resolveLayerPresentDecision(useDetailLayer: boolean, framePlan: RustFramePlan): RustLayerPresentDecision | null {
        try {
            return renderApi.resolveLayerPresentDecision(
                useDetailLayer,
                framePlan,
            ) as RustLayerPresentDecision;
        } catch {
            return null;
        }
    }

    return {
        buildRenderRequest,
        peek,
        take,
        stepPreview,
        resolveViewportRefresh,
        resolveHostScrollRefresh,
        scheduleRender,
        settleRender,
        abortRender,
        commitRenderResult,
        scheduleRenderFollowUp,
        resolveLayerExecutionPlan,
        resolveLayerPresentDecision,
        isImmediateMutationFrame,
        resolveFitToWidth,
        isRenderFrameCurrent,
        queueRenderLoopFrame,
        advanceRenderLoopFrame,
    };
}

