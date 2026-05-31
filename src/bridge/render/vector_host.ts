import { ensureWasmInitialized, getWasmApi } from '../shared/wasm_loader';
import { invalidateVectorPageCache, resolveVectorPageBundle } from './vector_page_bundle';
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

export { invalidateVectorPageCache };
export { VECTOR_CANVAS_ID, VECTOR_CONTAINER_ID };

export type VectorRenderResult = {
    width: number;
    height: number;
    aborted?: boolean;
    pendingPresents?: VectorLayerPresent[];
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
    logPdfLayoutTrace('vector-host.invalidate-cache.after');
}

export function ensureVectorHost(): VectorHostRefs | null {
    return ensureVectorCanvasHost();
}

export function commitVectorRenderResult(result: VectorRenderResult): void {
    const pendingPresents = result.pendingPresents ?? [];
    if (pendingPresents.length === 0) return;
    const refs = getExistingVectorCanvasHost();
    if (!refs) return;

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
        if (e?.message === 'stale frame' || (frameToken !== undefined && !isFrameCurrent(frameToken))) {
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
    const deferVisiblePresent = plan.prepareVisibleLayout === false;
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

    if (abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.before-canvas-frame', {
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

        if (abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.before-layer', {
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
                if (abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.before-cache-present', {
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
            imageCacheMap,
            layerUseViewportTile,
            frameToken,
            !!layerPlan.preferProgressive,
        );

        if (progressiveResult?.aborted) {
            logRenderChain('ts.layer.aborted', {
                pageIndex,
                useViewportTile: layerUseViewportTile,
                cacheKey: layerCacheKey,
            });
            return { aborted: true };
        }

        if (abortStaleFrameIfNeeded(frameToken, 'ts.frame.stale.before-layer-present', {
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
    return {
        width: model.width,
        height: model.height,
        pendingPresents,
    };
}

async function renderViewportProgressiveIfNeeded(
    refs: VectorHostRefs,
    imageCacheMap: Map<string, HTMLImageElement>,
    useViewportTile: boolean,
    frameToken?: number,
    preferProgressiveLayer?: boolean,
): Promise<{ aborted?: boolean } | null> {
    const start = renderApi.startProgressiveRender() as
        | { started?: boolean; totalItems?: number }
        | null
        | undefined;
    const renderTarget = getRenderBufferCanvas(refs, useViewportTile);
    const renderTargetId = renderTarget.id;

    if (
        Number.isFinite(frameToken as number) &&
        !renderApi.isRenderFrameCurrent(frameToken as number)
    ) {
        renderApi.cancelProgressiveRender();
        return { aborted: true };
    }

    if (!start?.started) {
        renderApi.renderPage(renderTargetId, imageCacheMap);
        if (
            Number.isFinite(frameToken as number) &&
            !renderApi.isRenderFrameCurrent(frameToken as number)
        ) {
            renderApi.cancelProgressiveRender();
            return { aborted: true };
        }
        return null;
    }

    const totalItems = Number.isFinite(start.totalItems) ? Number(start.totalItems) : 0;
    const policy = renderApi.resolveProgressiveRenderPolicy({
        useViewportTile: useViewportTile,
        preferProgressiveLayer: !!preferProgressiveLayer,
        totalItems,
    }) as
        | { useProgressive?: boolean; budgetMs?: number; maxItems?: number }
        | null
        | undefined;

    if (!policy?.useProgressive) {
        renderApi.cancelProgressiveRender();
        renderApi.renderPage(renderTargetId, imageCacheMap);
        if (
            Number.isFinite(frameToken as number) &&
            !renderApi.isRenderFrameCurrent(frameToken as number)
        ) {
            renderApi.cancelProgressiveRender();
            return { aborted: true };
        }
        return null;
    }

    let guard = 0;
    while (guard < 4000) {
        if (
            Number.isFinite(frameToken as number) &&
            !renderApi.isRenderFrameCurrent(frameToken as number)
        ) {
            renderApi.cancelProgressiveRender();
            return { aborted: true };
        }

        const budgetMs = Number.isFinite(policy.budgetMs) ? Number(policy.budgetMs) : 1.6;
        const maxItems = Number.isFinite(policy.maxItems) ? Number(policy.maxItems) : 8;

        const step = renderApi.stepProgressiveRender(
            renderTargetId,
            imageCacheMap,
            budgetMs,
            maxItems,
        ) as
            | { active?: boolean; completed?: boolean }
            | null
            | undefined;

        if (!step?.active || step.completed) {
            return null;
        }

        guard += 1;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }

    renderApi.cancelProgressiveRender();
    throw new Error('progressive render guard exceeded');
}

