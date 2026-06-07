import type {
    RustLayerExecutionPlan,
    RustLayerPresentDecision,
    RustPreviewFrame,
    RustPreviewHostStepResult,
    RustPreviewTickDecision,
    RustRenderCommitResult,
    RustRenderFrame,
    RustRenderTransition,
    RustViewportRefreshDecision,
    RustWheelRenderDecision,
    RustWheelZoomHostResult,
} from './frame_plan';

type GetWasmApi = () => any;

type ProgressiveRenderStart = {
    started?: boolean;
    totalItems?: number;
};

type ProgressiveRenderPolicy = {
    useProgressive?: boolean;
    budgetMs?: number;
    maxItems?: number;
};

export type RenderLayerRuntimePlan = {
    useDetailLayer: boolean;
    cacheKey: string;
    renderZoom: number;
    preferProgressive: boolean;
    showDetailOverlay: boolean;
    retainDetailOverlay: boolean;
};

export type RenderExecutionPlan = {
    skipRender: boolean;
    baseLayer: RenderLayerRuntimePlan | null;
    detailLayer: RenderLayerRuntimePlan | null;
};

type ProgressiveRenderStep = {
    active?: boolean;
    completed?: boolean;
};

type FrameCacheStoreResult = {
    evictedKeys?: string[];
};

export type RenderWasmApi = {
    resolveFramePlan: (frameRequest: Record<string, unknown>) => RustRenderFrame['framePlan'] | null;
    takeFramePlan: (frameRequest: Record<string, unknown>) => RustRenderFrame['framePlan'] | null;
    stepZoomFramePlan: (frameRequest: Record<string, unknown>) => RustPreviewFrame | null;
    resolveViewportRefresh: (frameRequest: Record<string, unknown>) => RustViewportRefreshDecision | null;
    resolveHostScrollRefresh: (frameRequest: Record<string, unknown>) => RustViewportRefreshDecision | null;
    scheduleRenderFrame: (frameRequest: Record<string, unknown>) => RustRenderFrame | null;
    markRenderedZoom: (renderedZoom: number) => void;
    settleRenderFrame: (frameToken: number, renderedZoom: number) => RustRenderTransition | null;
    abortRenderFrame: (frameToken: number) => RustRenderTransition | null;
    commitRenderResult: (
        frameToken: number,
        renderedZoom: number,
        pageWidth: number,
        pageHeight: number,
    ) => RustRenderCommitResult | null;
    resolveWheelRenderDecision: (request: Record<string, boolean | number>) => RustWheelRenderDecision | null;
    resolvePreviewTickDecision: (request: Record<string, boolean | number>) => RustPreviewTickDecision | null;
    scheduleRenderFollowUp: (renderedDisplayZoom: number, frameRequest: Record<string, unknown>) => RustRenderFrame | null;
    handleWheelZoomHost: (request: Record<string, unknown>) => RustWheelZoomHostResult | null;
    stepPreviewHost: (request: Record<string, unknown>) => RustPreviewHostStepResult | null;
    setWheelRenderPending: (pending: boolean) => void;
    getWheelRenderPending: () => boolean;
    queueCommittedFrame: (frame: Record<string, unknown>) => void;
    takeReadyCommittedFrame: () => Record<string, unknown> | null;
    isRenderFrameCurrent: (frameToken: number) => boolean;
    queueRenderLoopFrame: (frame: RustRenderFrame | null) => RustRenderFrame | null;
    advanceRenderLoopFrame: (frame: RustRenderFrame | null) => RustRenderFrame | null;
    resolveLayerExecutionPlan: (bundleChanged: boolean, framePlan: unknown) => RustLayerExecutionPlan | null;
    resolveRenderExecutionPlan: (bundleChanged: boolean, framePlan: unknown) => RenderExecutionPlan | null;
    resolveLayerPresentDecision: (useDetailLayer: boolean, framePlan: unknown) => RustLayerPresentDecision | null;
    cancelProgressiveRender: () => void;
    resetFrameCache: () => void;
    initPageContext: (
        modelJson: string,
        paintPlanJson: string,
        zoom: number,
        dpr: number,
        viewportLeft: number,
        viewportTop: number,
        viewportWidth: number,
        viewportHeight: number,
    ) => void;
    updatePageViewport: (
        zoom: number,
        dpr: number,
        viewportLeft: number,
        viewportTop: number,
        viewportWidth: number,
        viewportHeight: number,
    ) => void;
    touchFrameCacheEntry: (useViewportTile: boolean, cacheKey: string) => boolean | null;
    storeFrameCacheEntry: (useViewportTile: boolean, cacheKey: string) => FrameCacheStoreResult | null;
    startProgressiveRender: () => ProgressiveRenderStart | null;
    renderPage: (renderTargetId: string, imageCacheMap: Map<string, HTMLImageElement>) => void;
    renderPageOffscreen: (canvasJs: OffscreenCanvas, imageCacheMap: Map<string, ImageBitmap>, dpr: number) => void;
    resolveProgressiveRenderPolicy: (request: Record<string, unknown>) => ProgressiveRenderPolicy | null;
    stepProgressiveRender: (
        renderTargetId: string,
        imageCacheMap: Map<string, HTMLImageElement>,
        budgetMs: number,
        maxItems: number,
    ) => ProgressiveRenderStep | null;
    stepProgressiveRenderOffscreen: (
        canvasJs: OffscreenCanvas,
        imageCacheMap: Map<string, ImageBitmap>,
        budgetMs: number,
        maxItems: number,
        dpr: number,
    ) => ProgressiveRenderStep | null;
};

export function createRenderWasmApi(getWasmApi: GetWasmApi): RenderWasmApi {
    return {
        resolveFramePlan(frameRequest) {
            return getWasmApi().resolve_frame_plan?.(frameRequest) ?? null;
        },
        takeFramePlan(frameRequest) {
            return getWasmApi().take_frame_plan?.(frameRequest) ?? null;
        },
        stepZoomFramePlan(frameRequest) {
            return getWasmApi().step_zoom_frame_plan?.(frameRequest) ?? null;
        },
        resolveViewportRefresh(frameRequest) {
            return getWasmApi().resolve_viewport_refresh?.(frameRequest) ?? null;
        },
        resolveHostScrollRefresh(frameRequest) {
            return getWasmApi().resolve_host_scroll_refresh?.(frameRequest) ?? null;
        },
        scheduleRenderFrame(frameRequest) {
            return getWasmApi().schedule_render_frame?.(frameRequest) ?? null;
        },
        markRenderedZoom(renderedZoom) {
            getWasmApi().mark_rendered_zoom?.(renderedZoom);
        },
        settleRenderFrame(frameToken, renderedZoom) {
            return getWasmApi().settle_render_frame?.(frameToken, renderedZoom) ?? null;
        },
        abortRenderFrame(frameToken) {
            return getWasmApi().abort_render_frame?.(frameToken) ?? null;
        },
        commitRenderResult(frameToken, renderedZoom, pageWidth, pageHeight) {
            return getWasmApi().commit_render_result?.(
                frameToken,
                renderedZoom,
                pageWidth,
                pageHeight,
            ) ?? null;
        },
        resolveWheelRenderDecision(request) {
            return getWasmApi().resolve_wheel_render_decision?.(request) ?? null;
        },
        resolvePreviewTickDecision(request) {
            return getWasmApi().resolve_preview_tick_decision?.(request) ?? null;
        },
        scheduleRenderFollowUp(renderedDisplayZoom, frameRequest) {
            return getWasmApi().schedule_render_follow_up?.(
                renderedDisplayZoom,
                frameRequest,
            ) ?? null;
        },
        handleWheelZoomHost(request) {
            return getWasmApi().handle_wheel_zoom_host?.(request) ?? null;
        },
        stepPreviewHost(request) {
            return getWasmApi().step_preview_host?.(request) ?? null;
        },
        setWheelRenderPending(pending) {
            getWasmApi().set_wheel_render_pending?.(pending);
        },
        getWheelRenderPending() {
            return !!getWasmApi().get_wheel_render_pending?.();
        },
        queueCommittedFrame(frame) {
            getWasmApi().queue_committed_frame?.(frame);
        },
        takeReadyCommittedFrame() {
            return getWasmApi().take_ready_committed_frame?.() ?? null;
        },
        isRenderFrameCurrent(frameToken) {
            return getWasmApi().is_render_frame_current?.(frameToken) !== false;
        },
        queueRenderLoopFrame(frame) {
            return getWasmApi().queue_render_loop_frame?.(frame) ?? frame;
        },
        advanceRenderLoopFrame(frame) {
            return getWasmApi().advance_render_loop_frame?.(frame) ?? frame;
        },
        resolveLayerExecutionPlan(bundleChanged, framePlan) {
            return getWasmApi().resolve_layer_execution_plan?.(bundleChanged, framePlan) ?? null;
        },
        resolveRenderExecutionPlan(bundleChanged, framePlan) {
            return getWasmApi().resolve_render_execution_plan?.(bundleChanged, framePlan) ?? null;
        },
        resolveLayerPresentDecision(useDetailLayer, framePlan) {
            return getWasmApi().resolve_layer_present_decision?.(useDetailLayer, framePlan) ?? null;
        },
        cancelProgressiveRender() {
            getWasmApi().cancel_progressive_render?.();
        },
        resetFrameCache() {
            getWasmApi().reset_frame_cache?.();
        },
        initPageContext(modelJson, paintPlanJson, zoom, dpr, viewportLeft, viewportTop, viewportWidth, viewportHeight) {
            getWasmApi().init_page_context?.(
                modelJson,
                paintPlanJson,
                zoom,
                dpr,
                viewportLeft,
                viewportTop,
                viewportWidth,
                viewportHeight,
            );
        },
        updatePageViewport(zoom, dpr, viewportLeft, viewportTop, viewportWidth, viewportHeight) {
            getWasmApi().update_page_viewport?.(
                zoom,
                dpr,
                viewportLeft,
                viewportTop,
                viewportWidth,
                viewportHeight,
            );
        },
        touchFrameCacheEntry(useViewportTile, cacheKey) {
            return getWasmApi().touch_frame_cache_entry?.(useViewportTile, cacheKey) ?? null;
        },
        storeFrameCacheEntry(useViewportTile, cacheKey) {
            return getWasmApi().store_frame_cache_entry?.(useViewportTile, cacheKey) ?? null;
        },
        startProgressiveRender() {
            return getWasmApi().start_progressive_render?.() ?? null;
        },
        renderPage(renderTargetId, imageCacheMap) {
            getWasmApi().render_page?.(renderTargetId, imageCacheMap);
        },
        renderPageOffscreen(canvasJs, imageCacheMap, dpr) {
            getWasmApi().render_page_offscreen?.(canvasJs, imageCacheMap, dpr);
        },
        resolveProgressiveRenderPolicy(request) {
            return getWasmApi().resolve_progressive_render_policy?.(request) ?? null;
        },
        stepProgressiveRender(renderTargetId, imageCacheMap, budgetMs, maxItems) {
            return getWasmApi().step_progressive_render?.(
                renderTargetId,
                imageCacheMap,
                budgetMs,
                maxItems,
            ) ?? null;
        },
        stepProgressiveRenderOffscreen(canvasJs, imageCacheMap, budgetMs, maxItems, dpr) {
            return getWasmApi().step_progressive_render_offscreen?.(
                canvasJs,
                imageCacheMap,
                budgetMs,
                maxItems,
                dpr,
            ) ?? null;
        },
    };
}


