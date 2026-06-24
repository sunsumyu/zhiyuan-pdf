import { ensureWasmInitialized, getWasmApi } from '../../shared/wasm_loader';
import { resolveVectorPageBundle } from '../vector_page_bundle';
import { updateTextLayer } from '../text_layer';
import {
    applyViewportCanvasFrame,
    ensureVectorCanvasHost,
    getExistingVectorCanvasHost,
    getRenderBufferCanvas,
    presentViewportCanvas,
    stageViewportCanvasFromSource,
    type VectorHostRefs,
} from '../vector_canvas_host';
import {
    deleteViewportFrameCacheKeys,
    readViewportFrameCache,
    writeViewportFrameCache,
} from '../vector_frame_cache';
import { logPdfLayoutTrace } from '../layout_trace';
import { emitPdfDiagnostic } from '../../shared/diagnostics';
import { createRenderWasmApi, type RenderExecutionPlan, type RenderLayerRuntimePlan } from '../render_wasm_api';
import { runWorkerRender } from './worker_client';

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

const renderApi = createRenderWasmApi(() => getWasmApi());

function logRenderChain(node: string, details: Record<string, unknown>): void {
    emitPdfDiagnostic('render-chain', node, details, { verboseOnly: true });
}

export function isFrameCurrent(frameToken?: number): boolean {
    if (!Number.isFinite(frameToken as number)) return true;
    try {
        return renderApi.isRenderFrameCurrent(frameToken as number);
    } catch {
        return true;
    }
}

export function abortStaleFrameIfNeeded(
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
    const refs = ensureVectorCanvasHost();
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

    // FORCE DOUBLE-BUFFERING
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

    const dpr = window.devicePixelRatio || 1;

    let bitmap: ImageBitmap;
    try {
        bitmap = await runWorkerRender({
            path,
            pageIndex,
            revision,
            model,
            paintPlan,
            zoom: zoom ?? 1.0,
            dpr,
            viewportLeft: viewportLeft ?? 0,
            viewportTop: viewportTop ?? 0,
            viewportWidth: viewportWidth ?? model?.width ?? 0,
            viewportHeight: viewportHeight ?? model?.height ?? 0,
            clonedImageCacheMap,
            transferList,
            width: renderTarget.width,
            height: renderTarget.height,
            budgetMs: Number.isFinite(policy?.budgetMs) ? Number(policy?.budgetMs) : 1.6,
            maxItems: Number.isFinite(policy?.maxItems) ? Number(policy?.maxItems) : 8,
            useProgressive,
        });
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
        ctx.clearRect(0, 0, renderTarget.width, renderTarget.height);
        ctx.drawImage(bitmap, 0, 0);
    }
    bitmap.close();

    return null;
}
