import { ensureWasmInitialized, getWasmApi } from '../shared/wasm_loader';
import { invalidateVectorPageCache, isAbortedRenderRequest, resolveVectorPageBundle } from './vector_page_bundle';
import { updateTextLayer } from './text_layer';
import {
    applyViewportCanvasFrame,
    clearVectorCanvasHost,
    ensureVectorCanvasHost,
    getExistingVectorCanvasHost,
    getRenderBufferCanvas,
    presentViewportCanvas,
    presentViewportCanvasFromSource,
    stageViewportCanvasFromSource,
    type VectorHostRefs,
    VECTOR_CANVAS_ID,
    VECTOR_CONTAINER_ID,
} from './vector_canvas_host';
import {
    clearVectorFrameCache,
    deleteViewportFrameCacheKeys,
    readViewportFrameCache,
    writeViewportFrameCache,
} from './vector_frame_cache';
import { logPdfLayoutTrace } from './layout_trace';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import { getMainCanvasTransformOwner } from './canvas_transform_owner';
import { getDetailOverlayOwner } from './detail_overlay_owner';
import {
    gestureDirectRenderBudget,
    isGestureDirectRenderEligible,
    renderGestureViewportPatchDirect,
} from './gesture_direct_render';
import { createRenderWasmApi, type RenderExecutionPlan, type RenderLayerRuntimePlan } from './render_wasm_api';
import type { VectorWorkerRequest, VectorWorkerResponse } from './vector_worker';

let vectorWorker: Worker | null = null;
let msgIdCounter = 0;
/** Monotonic counter of direct-render attempts — drives PROF sampling (ADR-0026). */
let gestureDirectRenderSeq = 0;
const pendingVectorTasks = new Map<number, { resolve: (bitmap: ImageBitmap, workerMs?: number, recvDelayMs?: number) => void; reject: (err: any) => void }>();

let workerLastPath: string | null = null;
let workerLastPageIndex: number | null = null;
let workerLastRevision: number | null = null;

function ensureVectorWorker(): Worker {
    if (!vectorWorker) {
        vectorWorker = new Worker(new URL('./vector_worker.ts', import.meta.url), { type: 'module' });
        vectorWorker.onmessage = (e: MessageEvent<VectorWorkerResponse>) => {
            const msg = e.data;
            if (msg.type === 'RENDER_DONE') {
                const task = pendingVectorTasks.get(msg.msgId);
                if (task) {
                    pendingVectorTasks.delete(msg.msgId);
                    task.resolve(msg.bitmap, msg.workerMs, msg.recvDelayMs);
                }
            } else if (msg.type === 'ERROR') {
                const task = pendingVectorTasks.get(msg.msgId as number);
                if (task) {
                    pendingVectorTasks.delete(msg.msgId as number);
                    task.reject(new Error(msg.error));
                }
            }
        };
        vectorWorker.postMessage({ type: 'INIT_WASM' } as VectorWorkerRequest);
    }
    return vectorWorker;
}

export { invalidateVectorPageCache };
export { VECTOR_CANVAS_ID, VECTOR_CONTAINER_ID };

export type VectorRenderResult = {
    width: number;
    height: number;
    displayWidth?: number;
    displayHeight?: number;
    aborted?: boolean;
    pendingPresents?: VectorLayerPresent[];
};

export type VectorCommitOptions = {
    beforePresent?: () => void;
    /**
     * Display zoom of the frame being committed — the zoom space the presenter
     * just installed the main canvas box in. Handed to CanvasTransformOwner as
     * the new box space (ADR-0010). No default: a commit without it leaves the
     * owner's boxZoom unchanged, which is exactly right for a commit whose
     * presents were skipped.
     */
    displayZoom?: number;
    /**
     * Live visual zoom read at commit time. The owner re-derives the canvas
     * transform as `scale(visual / boxZoom)` from the box space it just
     * recorded — never from a guess (lastRendered/renderZoom). Without this,
     * the canvas shows `page × displayZoom` (transform stale) for one
     * compositor frame while tiles sit at `page × visualZoom` (the 2026-09-30
     * double-exposure during zoom-in reknock presents).
     */
    getVisualZoom?: () => number;
};

export type VectorLayerPresent = {
    sourceCanvas: HTMLCanvasElement;
    viewportWidth: number;
    viewportHeight: number;
    useViewportTile: boolean;
    viewportLeft: number;
    viewportTop: number;
    showDetailOverlay: boolean;
    retainDetailOverlay: boolean;
};

export type RenderZoomPlan = {
    displayZoom: number;
    renderZoom: number;
    baseRenderZoom: number;
    baseCacheZoom: number;
    detailCacheZoom: number;
    baseCacheKey: string;
    detailCacheKey: string;
    cssScale: number;
    useViewportTile: boolean;
    previewSettled?: boolean;
    allowRenderDuringPreview?: boolean;
    showDetailOverlay?: boolean;
    reuseActiveBaseLayer?: boolean;
    renderBaseLayer?: boolean;
    preferProgressiveBase?: boolean;
    reuseActiveDetailTile?: boolean;
    renderDetailLayer?: boolean;
    preferProgressiveDetail?: boolean;
    tileLeft?: number;
    tileTop?: number;
    tileWidth?: number;
    tileHeight?: number;
    prepareVisibleLayout?: boolean;
    renderReason?: string;
};

function logRenderChain(node: string, details: Record<string, unknown>): void {
    emitPdfDiagnostic('render-chain', node, details, { verboseOnly: true });
}

const renderApi = createRenderWasmApi(() => getWasmApi());

function isFrameCurrent(frameToken?: number): boolean {
    if (!Number.isFinite(frameToken as number)) return true;
    try {
        return renderApi.isRenderFrameCurrent(frameToken as number);
    } catch {
        return true;
    }
}

function abortStaleFrameIfNeeded(
    frameToken: number | undefined,
    node: string,
    details: Record<string, unknown>,
): boolean {
    if (isFrameCurrent(frameToken)) return false;
    renderApi.cancelProgressiveRender();
    logRenderChain(node, {
        ...details,
        frameToken,
    });
    return true;
}

export function cancelWorkerRender(): void {
    if (vectorWorker) {
        try { vectorWorker.postMessage({ type: 'CANCEL_RENDER' } as VectorWorkerRequest); } catch {}
    }
}

/**
 * Whether the worker's page context already matches (path, page, revision) —
 * read-only peek used to decide if images must be re-cloned before claiming.
 */
function workerContextMatches(path: string, pageIndex: number, revision: number): boolean {
    return (
        workerLastPath === path &&
        workerLastPageIndex === pageIndex &&
        workerLastRevision === revision
    );
}

/**
 * Claim the worker's page context for (path, page, revision). Must be called
 * synchronously right before worker.postMessage — an await between claim and
 * post lets another render claim the worker first and the stale isSamePage
 * flag would render the wrong page's content.
 */
function claimWorkerPageContext(path: string, pageIndex: number, revision: number): boolean {
    const isSamePage = workerContextMatches(path, pageIndex, revision);
    workerLastPath = path;
    workerLastPageIndex = pageIndex;
    workerLastRevision = revision;
    return isSamePage;
}

export function clearVectorHost(): void {
    logPdfLayoutTrace('vector-host.clear.before');
    try {
        renderApi.cancelProgressiveRender();
        renderApi.resetFrameCache();
    } catch {
    }
    clearVectorCanvasHost();
    invalidateVectorPageCache();
    clearVectorFrameCache();
    gestureDirectRenderBudget.reset();
    workerLastPath = null;
    workerLastPageIndex = null;
    workerLastRevision = null;
    logPdfLayoutTrace('vector-host.clear.after');
}

export function invalidateVectorRenderCache(): void {
    logPdfLayoutTrace('vector-host.invalidate-cache.before');
    try {
        renderApi.cancelProgressiveRender();
        renderApi.resetFrameCache();
    } catch {
    }
    invalidateVectorPageCache();
    clearVectorFrameCache();
    workerLastPath = null;
    workerLastPageIndex = null;
    workerLastRevision = null;
    logPdfLayoutTrace('vector-host.invalidate-cache.after');
}

export function ensureVectorHost(): VectorHostRefs | null {
    return ensureVectorCanvasHost();
}

export function commitVectorRenderResult(result: VectorRenderResult, options: VectorCommitOptions = {}): void {
    const pendingPresents = result.pendingPresents ?? [];
    let preparedVisibleFrame = false;
    const prepareVisibleFrame = (): void => {
        if (preparedVisibleFrame) return;
        preparedVisibleFrame = true;
        options.beforePresent?.();
    };

    if (pendingPresents.length === 0) {
        prepareVisibleFrame();
        return;
    }

    const refs = getExistingVectorCanvasHost();
    if (!refs) return;

    // CRITICAL ORDER:
    // 1. prepareVisibleFrame → syncLayoutBox: updates container CSS dimensions while still hidden (display:none)
    // 2. presentViewportCanvasFromSource: writes new pixels to canvas + updates mainCanvas CSS box
    // 3. presentViewportCanvas: makes container display:block with correct pixels + correct CSS dims
    prepareVisibleFrame();
    let basePresented = false;
    for (const pending of pendingPresents) {
        presentViewportCanvasFromSource(
            refs,
            pending.sourceCanvas,
            pending.viewportWidth,
            pending.viewportHeight,
            pending.useViewportTile,
            pending.viewportLeft,
            pending.viewportTop,
        );
        presentViewportCanvas(refs, {
            showDetailOverlay: pending.showDetailOverlay,
            retainDetailOverlay: pending.retainDetailOverlay,
        });
        // The base layer (useViewportTile=false) is the present that re-boxes
        // the MAIN canvas — to page × displayZoom. Detail presents only touch
        // backCanvas, so they must not move the owner's box space.
        if (!pending.useViewportTile) basePresented = true;
    }
    // ADR-0010: hand the presenter's new main-canvas box space to the single
    // owner, then re-apply the live visual in the same JS turn. This replaces
    // the old `presentScale` guess (visual/renderZoom) — the box space is the
    // frame's displayZoom, and the owner is the only writer.
    if (basePresented && options.displayZoom != null) {
        const owner = getMainCanvasTransformOwner(refs.mainCanvas);
        owner.presentFrame(options.displayZoom);
        const visual = options.getVisualZoom?.();
        if (visual != null) owner.sync(visual);
    }

    // ADR-0024: the detail overlay is a zoom-driven surface too. Whenever a
    // viewport-tile (detail) present landed, record its rect in the frame's
    // displayZoom space and write the visual-space mapping in this same turn —
    // the third surface under the ADR-0009 unified present formula. Without
    // this, a patch rendered at a mid-gesture band freezes at its commit-time
    // box while the visual keeps moving (the 2026-10-03 stale-patch video).
    let detailRect: { left: number; top: number; width: number; height: number } | null = null;
    for (const pending of pendingPresents) {
        if (pending.useViewportTile) {
            detailRect = {
                left: pending.viewportLeft,
                top: pending.viewportTop,
                width: pending.viewportWidth,
                height: pending.viewportHeight,
            };
        }
    }
    if (detailRect && options.displayZoom != null) {
        const visual = options.getVisualZoom?.();
        getDetailOverlayOwner(refs.backCanvas).present(
            detailRect,
            options.displayZoom,
            visual != null ? visual : options.displayZoom,
        );
    }

    logRenderChain('ts.deferred-present.commit', {
        layerCount: pendingPresents.length,
        width: result.width,
        height: result.height,
    });
}

async function renderVectorPage(path: string, pageIndex: number, zoom: number): Promise<VectorRenderResult> {
    return renderVectorPageWithPlan(path, pageIndex, {
        displayZoom: zoom,
        renderZoom: zoom,
        baseRenderZoom: zoom,
        baseCacheZoom: zoom,
        detailCacheZoom: zoom,
        baseCacheKey: '',
        detailCacheKey: '',
        cssScale: 1.0,
        useViewportTile: false,
        preferProgressiveBase: false,
    });
}

export async function renderVectorPageWithPlan(
    path: string,
    pageIndex: number,
    plan: RenderZoomPlan,
    frameToken?: number,
): Promise<VectorRenderResult> {
    logPdfLayoutTrace('vector-render.begin', {
        path,
        pageIndex,
        frameToken,
        plan,
    });
    await ensureWasmInitialized();
    const refs = ensureVectorHost();
    if (!refs) {
        throw new Error('pdf-content-wrapper not found');
    }

    let bundleResolution;
    try {
        bundleResolution = await resolveVectorPageBundle(path, pageIndex, frameToken);
    } catch (e: any) {
        const errMsg = typeof e === 'string' ? e : e?.message;
        if (
            (typeof errMsg === 'string' && isAbortedRenderRequest(errMsg)) ||
            (frameToken !== undefined && !isFrameCurrent(frameToken))
        ) {
            return {
                width: 0,
                height: 0,
                aborted: true,
            };
        }
        throw e;
    }
    const { bundle, bundleChanged } = bundleResolution;
    const { model, paintPlan, imageCacheMap } = bundle;
    logPdfLayoutTrace('vector-render.bundle-resolved', {
        path,
        pageIndex,
        frameToken,
        bundleChanged,
        modelWidth: model?.width,
        modelHeight: model?.height,
        paintPlanWidth: paintPlan?.width,
        paintPlanHeight: paintPlan?.height,
        imageCount: imageCacheMap.size,
        plan,
    });

    const dpr = window.devicePixelRatio || 1;
    const displayWidth = model.width * plan.displayZoom;
    const displayHeight = model.height * plan.displayZoom;

    // FORCE DOUBLE-BUFFERING: Globally force deferring presentation of onscreen canvases.
    // This preserves the old page's pixels on screen until the new page is 100% rendered offscreen,
    // avoiding intermediate flashing and canvas aspect-ratio stretching.
    const deferVisiblePresent = true;

    const isPipelineStale = (): boolean => {
        try {
            const w = window as any;
            if (typeof w.__getCurrentPage === 'function') {
                const currentPage = w.__getCurrentPage();
                if (currentPage !== null && currentPage !== pageIndex) {
                    return true;
                }
            }
        } catch {}
        return false;
    };

    if (isPipelineStale()) {
        console.log(`[PDF-DIAG] Pipeline pre-emptively aborted before canvas-frame setup for page ${pageIndex}`);
        return {
            width: model.width,
            height: model.height,
            aborted: true,
        };
    }

    const pendingPresents: VectorLayerPresent[] = [];

    let viewportLeft = 0;
    let viewportTop = 0;
    let viewportWidth = displayWidth;
    let viewportHeight = displayHeight;

    if (plan.showDetailOverlay) {
        if (
            Number.isFinite(plan.tileLeft) &&
            Number.isFinite(plan.tileTop) &&
            Number.isFinite(plan.tileWidth) &&
            Number.isFinite(plan.tileHeight)
        ) {
            viewportLeft = Math.max(0, plan.tileLeft || 0);
            viewportTop = Math.max(0, plan.tileTop || 0);
            viewportWidth = Math.max(1, plan.tileWidth || viewportWidth);
            viewportHeight = Math.max(1, plan.tileHeight || viewportHeight);
        }
    }

    if (isPipelineStale() || abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.before-canvas-frame', {
        pageIndex,
        displayZoom: plan.displayZoom,
        renderZoom: plan.renderZoom,
        baseRenderZoom: plan.baseRenderZoom,
        renderReason: (plan as any).renderReason,
    })) {
        return {
            width: model.width,
            height: model.height,
            aborted: true,
        };
    }

    // Canvas CSS box: computed by WASM (resolveCanvasCssBox) — single source
    // for the "canvas at baseRenderZoom units inside the display-zoom
    // container" geometry. If WASM is unavailable, degrade to the display box.
    const canvasBox =
        renderApi.resolveCanvasCssBox({
            displayWidth,
            displayHeight,
            displayZoom: plan.displayZoom,
            baseRenderZoom: plan.baseRenderZoom,
        }) ?? { domWidth: displayWidth, domHeight: displayHeight };

    applyViewportCanvasFrame(refs, {
        displayZoom: plan.displayZoom,
        baseRenderZoom: plan.baseRenderZoom,
        displayWidth,
        displayHeight,
        domBoxWidth: canvasBox.domWidth,
        domBoxHeight: canvasBox.domHeight,
        viewportLeft,
        viewportTop,
        viewportWidth,
        viewportHeight,
        dpr,
    }, !!plan.useViewportTile, deferVisiblePresent);
    logPdfLayoutTrace('vector-render.canvas-frame-applied', {
        path,
        pageIndex,
        frameToken,
        deferVisiblePresent,
        displayWidth,
        displayHeight,
        viewportLeft,
        viewportTop,
        viewportWidth,
        viewportHeight,
        plan,
    });
    if (deferVisiblePresent) {
        logRenderChain('ts.canvas-frame.defer-visible', {
            pageIndex,
            displayZoom: plan.displayZoom,
            renderZoom: plan.renderZoom,
            displayWidth,
            displayHeight,
            renderReason: (plan as any).renderReason,
        });
    }

    const renderLayer = async (
        layerPlan: RenderLayerRuntimePlan,
        layerViewportLeft: number,
        layerViewportTop: number,
        layerViewportWidth: number,
        layerViewportHeight: number,
    ): Promise<{ aborted?: boolean }> => {
        const layerUseViewportTile = !!layerPlan.useDetailLayer;
        const layerCacheKey = layerPlan.cacheKey;
        const layerRenderZoom = layerPlan.renderZoom;
        // ADR-0026: the gesture reknock patch renders synchronously on the
        // main thread. Eligibility is the frame plan's decision (core stays
        // the authority); this is only the wiring.
        const isGestureDirectRender = isGestureDirectRenderEligible(plan, layerUseViewportTile);

        if (isPipelineStale() || abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.before-layer', {
            pageIndex,
            useViewportTile: layerUseViewportTile,
            cacheKey: layerCacheKey,
        })) {
            return { aborted: true };
        }

        logRenderChain('ts.layer.begin', {
            pageIndex,
            bundleChanged,
            useViewportTile: layerUseViewportTile,
            layerRenderZoom,
            displayZoom: plan.displayZoom,
            baseRenderZoom: plan.baseRenderZoom,
            viewportLeft: layerViewportLeft,
            viewportTop: layerViewportTop,
            viewportWidth: layerViewportWidth,
            viewportHeight: layerViewportHeight,
            cacheKey: layerCacheKey,
        });

        if (bundleChanged) {
            logRenderChain('ts.page-context.init', {
                pageIndex,
                modelWidth: model.width,
                modelHeight: model.height,
                paintRegionCount: Array.isArray(paintPlan?.regions) ? paintPlan.regions.length : 0,
                zoom: layerRenderZoom,
                dpr,
            });
            renderApi.initPageContext(
                JSON.stringify(model),
                JSON.stringify(paintPlan),
                layerRenderZoom,
                dpr,
                layerViewportLeft,
                layerViewportTop,
                layerViewportWidth,
                layerViewportHeight,
            );
        } else {
            logRenderChain('ts.page-context.viewport', {
                pageIndex,
                zoom: layerRenderZoom,
                dpr,
                viewportLeft: layerViewportLeft,
                viewportTop: layerViewportTop,
                viewportWidth: layerViewportWidth,
                viewportHeight: layerViewportHeight,
            });
            renderApi.updatePageViewport(
                layerRenderZoom,
                dpr,
                layerViewportLeft,
                layerViewportTop,
                layerViewportWidth,
                layerViewportHeight,
            );
        }

        // Editor overlay renders must bypass the frame cache because the
        // cache key does not encode overlay/editor state.  A stale bitmap
        // from the initial page load would show the un-suppressed original
        // text instead of the edited replacement.
        const isOverlayRender =
            (plan as any).renderReason === 'editorVisibility' ||
            (plan as any).renderReason === 'documentMutation';
        // ADR-0026: direct-render frames bypass the READ too — a quantized
        // band hit would skip a fresh render (reuse = choosing blur). The
        // WRITE below is kept: settle reuse semantics are unchanged.
        const cachedFrame = (isOverlayRender || isGestureDirectRender)
            ? null
            : readViewportFrameCache(layerCacheKey);
        if (cachedFrame) {
            const cacheKnown = renderApi.touchFrameCacheEntry(
                layerUseViewportTile,
                layerCacheKey,
            );
            if (cacheKnown === false) {
                logRenderChain('ts.frame-cache.drop-stale', {
                    pageIndex,
                    useViewportTile: layerUseViewportTile,
                    cacheKey: layerCacheKey,
                });
                deleteViewportFrameCacheKeys([layerCacheKey]);
            } else {
                renderApi.cancelProgressiveRender();
                logRenderChain('ts.frame-cache.hit', {
                    pageIndex,
                    useViewportTile: layerUseViewportTile,
                    cacheKey: layerCacheKey,
                });
                if (isPipelineStale() || abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.before-cache-present', {
                    pageIndex,
                    useViewportTile: layerUseViewportTile,
                    cacheKey: layerCacheKey,
                })) {
                    return { aborted: true };
                }
                if (deferVisiblePresent) {
                    pendingPresents.push({
                        sourceCanvas: cachedFrame,
                        viewportWidth: layerViewportWidth,
                        viewportHeight: layerViewportHeight,
                        useViewportTile: layerUseViewportTile,
                        viewportLeft: layerViewportLeft,
                        viewportTop: layerViewportTop,
                        showDetailOverlay: !!layerPlan.showDetailOverlay,
                        retainDetailOverlay: !!layerPlan.retainDetailOverlay,
                    });
                    logRenderChain('ts.deferred-present.queue-cache', {
                        pageIndex,
                        useViewportTile: layerUseViewportTile,
                        cacheKey: layerCacheKey,
                    });
                } else {
                    stageViewportCanvasFromSource(
                        refs,
                        cachedFrame,
                        layerViewportWidth,
                        layerViewportHeight,
                        layerUseViewportTile,
                        layerViewportLeft,
                        layerViewportTop,
                    );
                    presentViewportCanvas(refs, {
                        showDetailOverlay: !!layerPlan.showDetailOverlay,
                        retainDetailOverlay: !!layerPlan.retainDetailOverlay,
                    });
                }
                return {};
            }
        }

        const progressiveResult = await renderViewportProgressiveIfNeeded(
            refs,
            imageCacheMap as any,
            layerUseViewportTile,
            frameToken,
            !!layerPlan.preferProgressive,
            path,
            pageIndex,
            model,
            paintPlan,
            layerPlan.renderZoom,
            layerViewportLeft,
            layerViewportTop,
            layerViewportWidth,
            layerViewportHeight,
            bundle.documentRevision,
            isOverlayRender,
            isGestureDirectRender,
        );

        if (progressiveResult?.aborted) {
            logRenderChain('ts.layer.aborted', {
                pageIndex,
                useViewportTile: layerUseViewportTile,
                cacheKey: layerCacheKey,
            });
            return { aborted: true };
        }

        if (isPipelineStale() || abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.before-layer-present', {
            pageIndex,
            useViewportTile: layerUseViewportTile,
            cacheKey: layerCacheKey,
        })) {
            return { aborted: true };
        }

        const renderedBuffer = getRenderBufferCanvas(refs, layerUseViewportTile);
        const cacheStoreResult = renderApi.storeFrameCacheEntry(
            layerUseViewportTile,
            layerCacheKey,
        ) as { evictedKeys?: string[] } | null | undefined;
        writeViewportFrameCache(layerCacheKey, renderedBuffer);
        deleteViewportFrameCacheKeys(cacheStoreResult?.evictedKeys ?? []);
        logRenderChain('ts.layer.rendered', {
            pageIndex,
            useViewportTile: layerUseViewportTile,
            cacheKey: layerCacheKey,
            evictedKeys: cacheStoreResult?.evictedKeys ?? [],
        });
        if (deferVisiblePresent) {
            pendingPresents.push({
                sourceCanvas: renderedBuffer,
                viewportWidth: layerViewportWidth,
                viewportHeight: layerViewportHeight,
                useViewportTile: layerUseViewportTile,
                viewportLeft: layerViewportLeft,
                viewportTop: layerViewportTop,
                showDetailOverlay: !!layerPlan.showDetailOverlay,
                retainDetailOverlay: !!layerPlan.retainDetailOverlay,
            });
            logRenderChain('ts.deferred-present.queue-rendered', {
                pageIndex,
                useViewportTile: layerUseViewportTile,
                cacheKey: layerCacheKey,
            });
        } else {
            stageViewportCanvasFromSource(
                refs,
                renderedBuffer,
                layerViewportWidth,
                layerViewportHeight,
                layerUseViewportTile,
                layerViewportLeft,
                layerViewportTop,
            );
            presentViewportCanvas(refs, {
                showDetailOverlay: !!layerPlan.showDetailOverlay,
                retainDetailOverlay: !!layerPlan.retainDetailOverlay,
            });
        }
        return {};
    };

    const executionPlan = renderApi.resolveRenderExecutionPlan(
        bundleChanged,
        plan,
    ) as RenderExecutionPlan | null | undefined;

    const shouldSkipRender = executionPlan?.skipRender ?? (!bundleChanged && !plan.renderBaseLayer && plan.renderDetailLayer === false);
    const baseLayerPlan = executionPlan?.baseLayer ?? null;
    const detailLayerPlan = executionPlan?.detailLayer ?? null;
    const shouldRenderBaseLayer = !!baseLayerPlan;
    const shouldRenderDetailLayer = !!detailLayerPlan;

    logRenderChain('ts.layer-plan', {
        pageIndex,
        bundleChanged,
        shouldSkipRender,
        shouldRenderBaseLayer,
        shouldRenderDetailLayer,
        displayZoom: plan.displayZoom,
        renderZoom: plan.renderZoom,
        baseRenderZoom: plan.baseRenderZoom,
        cssScale: plan.cssScale,
        useViewportTile: plan.useViewportTile,
        showDetailOverlay: plan.showDetailOverlay,
    });
    if ((plan as any).renderReason === 'editorVisibility' || (plan as any).renderReason === 'documentMutation') {
        emitPdfDiagnostic('present', 'mutation-frame', {
            renderReason: (plan as any).renderReason,
            useViewportTile: !!plan.useViewportTile,
            renderBaseLayer: shouldRenderBaseLayer,
            renderDetailLayer: shouldRenderDetailLayer,
            showDetailOverlay: !!plan.showDetailOverlay,
            baseRetainDetailOverlay: !!baseLayerPlan?.retainDetailOverlay,
            detailRetainDetailOverlay: !!detailLayerPlan?.retainDetailOverlay,
        });
    }

    if (shouldSkipRender) {
        renderApi.cancelProgressiveRender();
        logRenderChain('ts.render.skip', {
            pageIndex,
            bundleChanged,
            displayZoom: plan.displayZoom,
            renderZoom: plan.renderZoom,
        });
        if (abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.skip-render', {
            pageIndex,
            displayZoom: plan.displayZoom,
            renderZoom: plan.renderZoom,
        })) {
            return {
                width: model.width,
                height: model.height,
                aborted: true,
            };
        }
        return {
            width: model.width,
            height: model.height,
            pendingPresents,
        };
    }

    if (shouldRenderBaseLayer) {
        const baseLayerResult = await renderLayer(baseLayerPlan!, 0, 0, displayWidth, displayHeight);
        if (baseLayerResult.aborted) {
            return {
                width: model.width,
                height: model.height,
                aborted: true,
            };
        }
    }

    if (shouldRenderDetailLayer) {
        const detailLayerResult = await renderLayer(
            detailLayerPlan!,
            viewportLeft,
            viewportTop,
            viewportWidth,
            viewportHeight,
        );
        if (detailLayerResult.aborted) {
            return {
                width: model.width,
                height: model.height,
                aborted: true,
            };
        }
    }

    logPdfLayoutTrace('vector-render.done', {
        path,
        pageIndex,
        frameToken,
        bundleChanged,
        pendingPresentCount: pendingPresents.length,
        modelWidth: model.width,
        modelHeight: model.height,
        plan,
    });
    updateTextLayer(path, pageIndex, model, plan.displayZoom);

    return {
        width: model.width,
        height: model.height,
        displayWidth,
        displayHeight,
        pendingPresents,
    };
}

async function renderViewportProgressiveIfNeeded(
    refs: VectorHostRefs,
    imageCacheMap: Map<string, ImageBitmap>,
    useViewportTile: boolean,
    frameToken?: number,
    preferProgressiveLayer?: boolean,
    path?: string,
    pageIndex?: number,
    model?: any,
    paintPlan?: any,
    zoom?: number,
    viewportLeft?: number,
    viewportTop?: number,
    viewportWidth?: number,
    viewportHeight?: number,
    revision?: number,
    isOverlayRender?: boolean,
    isGestureDirectRender?: boolean,
): Promise<{ aborted?: boolean } | null> {
    const isProgressivePipelineStale = (): boolean => {
        if (path === undefined || pageIndex === undefined) return false;
        try {
            const w = window as any;
            if (typeof w.__getCurrentPage === 'function') {
                const currentPage = w.__getCurrentPage();
                if (currentPage !== null && currentPage !== pageIndex) {
                    return true;
                }
            }
        } catch {}
        return false;
    };

    if (isProgressivePipelineStale()) {
        renderApi.cancelProgressiveRender();
        return { aborted: true };
    }

    // ── ADR-0026: gesture direct render ─────────────────────────────────
    // The reknock viewport patch renders synchronously on the main thread:
    // no progressive task, no worker round trip, no await (an await would
    // reopen the stale-frame window ADR-0022's checkpoints exist for). The
    // pixels still come from the wasm CanvasRenderer into the reused stage
    // buffer; the caller's store/present flow continues unchanged.
    if (isGestureDirectRender) {
        renderApi.cancelProgressiveRender();
        const directTarget = getRenderBufferCanvas(refs, useViewportTile);
        const directDpr = window.devicePixelRatio || 1;
        const directResult = renderGestureViewportPatchDirect({
            renderTarget: directTarget,
            imageCacheMap,
            dpr: directDpr,
            renderPageOffscreen: (canvas, map, d) => renderApi.renderPageOffscreen(canvas, map, d),
            budget: gestureDirectRenderBudget,
            seq: ++gestureDirectRenderSeq,
            onProf: (fields) => emitPdfDiagnostic('PROF', 'reknock-phase-timing', fields),
        });
        logRenderChain(directResult.skipped
            ? 'ts.layer.gesture-direct-render.skip'
            : 'ts.layer.gesture-direct-render', {
            pageIndex,
            costMs: directResult.costMs,
            w: directTarget.width,
            h: directTarget.height,
        });
        return null;
    }

    // Phase timing (2026-09-28 perf probe): wasm prep on the main thread vs
    // the worker round-trip vs the blit. If roundMs >> workerMs the main
    // thread was blocked between postMessage and the reply — worker-bound if
    // they match.
    const tWasm0 = performance.now();
    const start = renderApi.startProgressiveRender() as
        | { started?: boolean; totalItems?: number }
        | null
        | undefined;

    const renderTarget = getRenderBufferCanvas(refs, useViewportTile);

    if (
        isProgressivePipelineStale() || (
            Number.isFinite(frameToken as number) &&
            !renderApi.isRenderFrameCurrent(frameToken as number)
        )
    ) {
        renderApi.cancelProgressiveRender();
        return { aborted: true };
    }

    if (isOverlayRender) {
        logRenderChain('ts.layer.main-thread-render', { pageIndex, useViewportTile });
        renderApi.cancelProgressiveRender();
        const canvas = new OffscreenCanvas(renderTarget.width, renderTarget.height);
        const dpr = window.devicePixelRatio || 1;
        renderApi.renderPageOffscreen(canvas, imageCacheMap, dpr);
        const ctx = renderTarget.getContext('2d');
        if (ctx) {
            ctx.clearRect(0, 0, renderTarget.width, renderTarget.height);
            ctx.drawImage(canvas, 0, 0);
        }
        return null;
    }

    const totalItems = Number.isFinite(start?.totalItems) ? Number(start?.totalItems) : 0;
    const policy = renderApi.resolveProgressiveRenderPolicy({
        useViewportTile: useViewportTile,
        preferProgressiveLayer: !!preferProgressiveLayer,
        totalItems,
    }) as { useProgressive?: boolean; budgetMs?: number; maxItems?: number } | null | undefined;

    const useProgressive = !!start?.started && !!policy?.useProgressive;

    renderApi.cancelProgressiveRender(); // Cancel main thread render, worker will do it.
    const wasmMs = Math.round((performance.now() - tWasm0) * 10) / 10;

    const worker = ensureVectorWorker();
    const msgId = ++msgIdCounter;

    // Clone page images only when the worker's context differs — repeat
    // renders on the same page/bundle reuse the worker's stored map. The
    // claim itself must sit synchronously right before postMessage.
    const willReuseWorkerImages =
        path !== undefined &&
        pageIndex !== undefined &&
        revision !== undefined &&
        workerContextMatches(path, pageIndex, revision);

    const clonedImageCacheMap = new Map<string, ImageBitmap>();
    const transferList: Transferable[] = [];
    if (!willReuseWorkerImages && imageCacheMap && imageCacheMap.size > 0) {
        await Promise.all(
            Array.from(imageCacheMap.entries()).map(async ([key, bmp]) => {
                const clone = await createImageBitmap(bmp);
                clonedImageCacheMap.set(key, clone);
                transferList.push(clone);
            })
        );
    }

    let workerMsObserved = -1;
    let recvDelayObserved = -1;
    const promise = new Promise<ImageBitmap>((resolve, reject) => {
        pendingVectorTasks.set(msgId, {
            resolve: (bitmap, workerMs, recvDelayMs) => {
                workerMsObserved = workerMs ?? -1;
                recvDelayObserved = recvDelayMs ?? -1;
                resolve(bitmap);
            },
            reject,
        });
    });

    const dpr = window.devicePixelRatio || 1;

    const isSamePage =
        path !== undefined &&
        pageIndex !== undefined &&
        revision !== undefined &&
        claimWorkerPageContext(path, pageIndex, revision);

    const tRound0 = performance.now();
    worker.postMessage({
        type: 'RENDER_PAGE',
        msgId,
        postedAt: tRound0,
        isSamePage,
        modelJson: isSamePage ? undefined : JSON.stringify(model ?? {}),
        paintPlanJson: isSamePage ? undefined : JSON.stringify(paintPlan ?? {}),
        zoom: zoom ?? 1.0,
        dpr: dpr,
        viewportLeft: viewportLeft ?? 0,
        viewportTop: viewportTop ?? 0,
        viewportWidth: viewportWidth ?? model?.width ?? 0,
        viewportHeight: viewportHeight ?? model?.height ?? 0,
        imageCacheMap: isSamePage ? undefined : clonedImageCacheMap,
        width: renderTarget.width,
        height: renderTarget.height,
        budgetMs: Number.isFinite(policy?.budgetMs) ? Number(policy?.budgetMs) : 1.6,
        maxItems: Number.isFinite(policy?.maxItems) ? Number(policy?.maxItems) : 8,
        useProgressive
    }, transferList);

    let bitmap: ImageBitmap;
    try {
        bitmap = await promise;
    } catch (err) {
        console.error('Worker render failed', err);
        return { aborted: true };
    }
    const roundMs = Math.round((performance.now() - tRound0) * 10) / 10;

    if (
        isProgressivePipelineStale() || (
            Number.isFinite(frameToken as number) &&
            !renderApi.isRenderFrameCurrent(frameToken as number)
        )
    ) {
        return { aborted: true };
    }

    const tBlit0 = performance.now();
    const ctx = renderTarget.getContext('2d');
    if (ctx) {
        // clear just in case
        ctx.clearRect(0, 0, renderTarget.width, renderTarget.height);
        ctx.drawImage(bitmap, 0, 0);
    }
    bitmap.close();
    const blitMs = Math.round((performance.now() - tBlit0) * 10) / 10;
    emitPdfDiagnostic('PROF', 'reknock-phase-timing', {
        wasmMs,
        roundMs,
        workerMs: workerMsObserved,
        recvDelayMs: recvDelayObserved,
        blitMs,
        w: renderTarget.width,
        h: renderTarget.height,
        prog: useProgressive,
        tile: useViewportTile,
    });

    return null;
}

export type TileRegionRenderParams = {
    path: string;
    pageIndex: number;
    /** Render zoom — the visual zoom the tile bitmap is rendered at. */
    zoom: number;
    dpr: number;
    /** Display-space rectangle of the tile region (clipped to the page). */
    regionLeft: number;
    regionTop: number;
    regionWidth: number;
    regionHeight: number;
    /** Device-pixel size of the output bitmap. */
    bitmapWidth: number;
    bitmapHeight: number;
};

/**
 * Render a single tile region through the vector worker and resolve with the
 * ImageBitmap. The worker processes messages sequentially, so a tile render
 * never interleaves with an in-flight full-page render; it only queues after
 * it. Page images are transferred to the worker once per page context — tile
 * renders on an already-loaded page reuse the worker's stored map.
 */
export async function renderTileRegion(params: TileRegionRenderParams): Promise<ImageBitmap> {
    const { bundle } = await resolveVectorPageBundle(params.path, params.pageIndex);
    const worker = ensureVectorWorker();

    // Clone page images BEFORE claiming the worker context: the claim must be
    // immediately followed by postMessage with no await in between.
    const transferList: Transferable[] = [];
    let imageCacheMap: Map<string, ImageBitmap> | undefined;
    if (!workerContextMatches(params.path, params.pageIndex, bundle.documentRevision)) {
        imageCacheMap = new Map();
        await Promise.all(
            Array.from(bundle.imageCacheMap.entries()).map(async ([key, bmp]) => {
                const clone = await createImageBitmap(bmp);
                imageCacheMap!.set(key, clone);
                transferList.push(clone);
            }),
        );
    }

    const msgId = ++msgIdCounter;
    const isSamePage = claimWorkerPageContext(params.path, params.pageIndex, bundle.documentRevision);
    const promise = new Promise<ImageBitmap>((resolve, reject) => {
        pendingVectorTasks.set(msgId, { resolve, reject });
    });

    worker.postMessage({
        type: 'RENDER_PAGE',
        msgId,
        isSamePage,
        modelJson: isSamePage ? undefined : JSON.stringify(bundle.model),
        paintPlanJson: isSamePage ? undefined : JSON.stringify(bundle.paintPlan),
        zoom: params.zoom,
        dpr: params.dpr,
        viewportLeft: params.regionLeft,
        viewportTop: params.regionTop,
        viewportWidth: params.regionWidth,
        viewportHeight: params.regionHeight,
        imageCacheMap,
        width: params.bitmapWidth,
        height: params.bitmapHeight,
        budgetMs: 1.6,
        maxItems: 8,
        useProgressive: false,
    }, transferList);

    return promise;
}

