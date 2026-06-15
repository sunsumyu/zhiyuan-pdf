import { ensureWasmInitialized, getWasmApi } from '../shared/wasm_loader';
import { invalidateVectorPageCache, resolveVectorPageBundle } from './vector_page_bundle';
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
import { createRenderWasmApi, type RenderExecutionPlan, type RenderLayerRuntimePlan } from './render_wasm_api';
import type { VectorWorkerRequest, VectorWorkerResponse } from './vector_worker';

let vectorWorker: Worker | null = null;
let msgIdCounter = 0;
const pendingVectorTasks = new Map<number, { resolve: (bitmap: ImageBitmap) => void; reject: (err: any) => void }>();

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
                    task.resolve(msg.bitmap);
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
            errMsg === 'stale frame' ||
            (typeof errMsg === 'string' && errMsg.includes('stale page asset request')) ||
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

    applyViewportCanvasFrame(refs, {
        displayZoom: plan.displayZoom,
        baseRenderZoom: plan.baseRenderZoom,
        displayWidth,
        displayHeight,
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
        const cachedFrame = isOverlayRender ? null : readViewportFrameCache(layerCacheKey);
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

    const worker = ensureVectorWorker();
    const msgId = ++msgIdCounter;
    
    const clonedImageCacheMap = new Map<string, ImageBitmap>();
    const transferList: Transferable[] = [];
    if (imageCacheMap && imageCacheMap.size > 0) {
        await Promise.all(
            Array.from(imageCacheMap.entries()).map(async ([key, bmp]) => {
                const clone = await createImageBitmap(bmp);
                clonedImageCacheMap.set(key, clone);
                transferList.push(clone);
            })
        );
    }

    const promise = new Promise<ImageBitmap>((resolve, reject) => {
        pendingVectorTasks.set(msgId, { resolve, reject });
    });
    
    const dpr = window.devicePixelRatio || 1;

    const isSamePage =
        path !== undefined &&
        pageIndex !== undefined &&
        revision !== undefined &&
        workerLastPath === path &&
        workerLastPageIndex === pageIndex &&
        workerLastRevision === revision;

    if (path !== undefined && pageIndex !== undefined && revision !== undefined) {
        workerLastPath = path;
        workerLastPageIndex = pageIndex;
        workerLastRevision = revision;
    }

    worker.postMessage({
        type: 'RENDER_PAGE',
        msgId,
        isSamePage,
        modelJson: isSamePage ? undefined : JSON.stringify(model ?? {}),
        paintPlanJson: isSamePage ? undefined : JSON.stringify(paintPlan ?? {}),
        zoom: zoom ?? 1.0,
        dpr: dpr,
        viewportLeft: viewportLeft ?? 0,
        viewportTop: viewportTop ?? 0,
        viewportWidth: viewportWidth ?? model?.width ?? 0,
        viewportHeight: viewportHeight ?? model?.height ?? 0,
        imageCacheMap: clonedImageCacheMap,
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

    if (
        isProgressivePipelineStale() || (
            Number.isFinite(frameToken as number) &&
            !renderApi.isRenderFrameCurrent(frameToken as number)
        )
    ) {
        return { aborted: true };
    }

    const ctx = renderTarget.getContext('2d');
    if (ctx) {
        // clear just in case
        ctx.clearRect(0, 0, renderTarget.width, renderTarget.height);
        ctx.drawImage(bitmap, 0, 0);
    }
    bitmap.close();
    
    return null;
}

