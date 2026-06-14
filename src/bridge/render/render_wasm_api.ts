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

import type { WasmModule } from '../shared/wasm_loader';

type GetWasmApi = () => WasmModule;

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
            return getWasmApi().resolveFramePlan?.(frameRequest) ?? null;
        },
        takeFramePlan(frameRequest) {
            return getWasmApi().takeFramePlan?.(frameRequest) ?? null;
        },
        stepZoomFramePlan(frameRequest) {
            return getWasmApi().stepZoomFramePlan?.(frameRequest) ?? null;
        },
        resolveViewportRefresh(frameRequest) {
            return getWasmApi().resolveViewportRefresh?.(frameRequest) ?? null;
        },
        resolveHostScrollRefresh(frameRequest) {
            return getWasmApi().resolveHostScrollRefresh?.(frameRequest) ?? null;
        },
        scheduleRenderFrame(frameRequest) {
            return getWasmApi().scheduleRenderFrame?.(frameRequest) ?? null;
        },
        markRenderedZoom(renderedZoom) {
            getWasmApi().markRenderedZoom?.(renderedZoom);
        },
        settleRenderFrame(frameToken, renderedZoom) {
            return getWasmApi().settleRenderFrame?.(frameToken, renderedZoom) ?? null;
        },
        abortRenderFrame(frameToken) {
            return getWasmApi().abortRenderFrame?.(frameToken) ?? null;
        },
        commitRenderResult(frameToken, renderedZoom, pageWidth, pageHeight) {
            return getWasmApi().commitRenderResult?.(
                frameToken,
                renderedZoom,
                pageWidth,
                pageHeight,
            ) ?? null;
        },
        resolveWheelRenderDecision(request) {
            return getWasmApi().resolveWheelRenderDecision?.(request) ?? null;
        },
        resolvePreviewTickDecision(request) {
            return getWasmApi().resolvePreviewTickDecision?.(request) ?? null;
        },
        scheduleRenderFollowUp(renderedDisplayZoom, frameRequest) {
            return getWasmApi().scheduleRenderFollowUp?.(
                renderedDisplayZoom,
                frameRequest,
            ) ?? null;
        },
        handleWheelZoomHost(request) {
            return getWasmApi().handleWheelZoomHost?.(request) ?? null;
        },
        stepPreviewHost(request) {
            return getWasmApi().stepPreviewHost?.(request) ?? null;
        },
        setWheelRenderPending(pending) {
            getWasmApi().setWheelRenderPending?.(pending);
        },
        getWheelRenderPending() {
            return !!getWasmApi().getWheelRenderPending?.();
        },
        queueCommittedFrame(frame) {
            getWasmApi().queueCommittedFrame?.(frame);
        },
        takeReadyCommittedFrame() {
            return getWasmApi().takeReadyCommittedFrame?.() ?? null;
        },
        isRenderFrameCurrent(frameToken) {
            return getWasmApi().isRenderFrameCurrent?.(frameToken) !== false;
        },
        queueRenderLoopFrame(frame) {
            return getWasmApi().queueRenderLoopFrame?.(frame) ?? frame;
        },
        advanceRenderLoopFrame(frame) {
            return getWasmApi().advanceRenderLoopFrame?.(frame) ?? frame;
        },
        resolveLayerExecutionPlan(bundleChanged, framePlan) {
            return getWasmApi().resolveLayerExecutionPlan?.(bundleChanged, framePlan) ?? null;
        },
        resolveRenderExecutionPlan(bundleChanged, framePlan) {
            return getWasmApi().resolveRenderExecutionPlan?.(bundleChanged, framePlan) ?? null;
        },
        resolveLayerPresentDecision(useDetailLayer, framePlan) {
            return getWasmApi().resolveLayerPresentDecision?.(useDetailLayer, framePlan) ?? null;
        },
        cancelProgressiveRender() {
            getWasmApi().cancelProgressiveRender?.();
        },
        resetFrameCache() {
            getWasmApi().resetFrameCache?.();
        },
        initPageContext(modelJson, paintPlanJson, zoom, dpr, viewportLeft, viewportTop, viewportWidth, viewportHeight) {
            getWasmApi().initPageContext?.(
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
            getWasmApi().updatePageViewport?.(
                zoom,
                dpr,
                viewportLeft,
                viewportTop,
                viewportWidth,
                viewportHeight,
            );
        },
        touchFrameCacheEntry(useViewportTile, cacheKey) {
            return getWasmApi().touchFrameCacheEntry?.(useViewportTile, cacheKey) ?? null;
        },
        storeFrameCacheEntry(useViewportTile, cacheKey) {
            return getWasmApi().storeFrameCacheEntry?.(useViewportTile, cacheKey) ?? null;
        },
        startProgressiveRender() {
            return getWasmApi().startProgressiveRender?.() ?? null;
        },
        renderPage(renderTargetId, imageCacheMap) {
            getWasmApi().renderPage?.(renderTargetId, imageCacheMap);
        },
        renderPageOffscreen(canvasJs, imageCacheMap, dpr) {
            getWasmApi().renderPageOffscreen?.(canvasJs, imageCacheMap, dpr);
        },
        resolveProgressiveRenderPolicy(request) {
            return getWasmApi().resolveProgressiveRenderPolicy?.(request) ?? null;
        },
        stepProgressiveRender(renderTargetId, imageCacheMap, budgetMs, maxItems) {
            return getWasmApi().stepProgressiveRender?.(
                renderTargetId,
                imageCacheMap,
                budgetMs,
                maxItems,
            ) ?? null;
        },
        stepProgressiveRenderOffscreen(canvasJs, imageCacheMap, budgetMs, maxItems, dpr) {
            return getWasmApi().stepProgressiveRenderOffscreen?.(
                canvasJs,
                imageCacheMap,
                budgetMs,
                maxItems,
                dpr,
            ) ?? null;
        },
    };
}


