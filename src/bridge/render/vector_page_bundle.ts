import { getWasmApi, targetInvokeV3 } from '../shared/wasm_loader';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import { logPdfLayoutTrace } from './layout_trace';
import { createPagePresentationRuntimeAdapter } from '../viewer/page_presentation_runtime';
import { createViewerSessionAdapter } from '../viewer/viewer_session';
import type { PagePresentationRuntimeAdapter } from '../viewer/page_presentation_runtime';
import type { ViewerSessionAdapter } from '../viewer/viewer_session';

export type VectorPageBundle = {
    path: string;
    pageIndex: number;
    documentRevision: number;
    model: any;
    paintPlan: any;
    imageCacheMap: Map<string, ImageBitmap>;
};

type VectorPageBundleResolution = {
    bundle: VectorPageBundle;
    bundleChanged: boolean;
};

const PAGE_CACHE_MAX = 15;
const pageBundleCache: VectorPageBundle[] = [];
let pagePresentationRuntime: PagePresentationRuntimeAdapter = createPagePresentationRuntimeAdapter({
    getWasmApi: () => getWasmApi() as any,
});
let viewerSession: ViewerSessionAdapter = createViewerSessionAdapter({
    getWasmApi: () => getWasmApi() as any,
    getFallbackPageWidth: () => 595,
    getFallbackPageHeight: () => 842,
});

export function configureVectorPageBundleRuntime(deps: {
    pagePresentationRuntime: PagePresentationRuntimeAdapter;
    viewerSession: ViewerSessionAdapter;
}): void {
    pagePresentationRuntime = deps.pagePresentationRuntime;
    viewerSession = deps.viewerSession;
}

function isFrameCurrent(frameToken?: number): boolean {
    if (frameToken === undefined || !Number.isFinite(frameToken)) return true;
    try {
        const wasm: any = getWasmApi();
        return wasm?.is_render_frame_current?.(frameToken) !== false;
    } catch {
        return true;
    }
}

function getCurrentPageIndex(): number | null {
    try {
        const w = window as any;
        if (typeof w.__getCurrentPage === 'function') {
            return w.__getCurrentPage();
        }
    } catch {}
    return null;
}

function resolveAssetRole(pageIndex: number, frameToken?: number, explicitRole?: 'current' | 'prefetch'): 'current' | 'prefetch' {
    if (explicitRole) return explicitRole;
    if (frameToken !== undefined && Number.isFinite(frameToken)) return 'current';
    return getCurrentPageIndex() === pageIndex ? 'current' : 'prefetch';
}

function admitPageAsset(pageIndex: number, role: 'current' | 'prefetch', assetKind: string, node: string): boolean {
    const decision = pagePresentationRuntime.admitPageAsset(pageIndex, role, assetKind);
    if (!decision.accepted) {
        logPdfLayoutTrace(node, {
            pageIndex,
            role,
            assetKind,
            rejectReason: decision.rejectReason,
            snapshot: decision.snapshot,
        });
    }
    return decision.accepted;
}

function findCachedBundle(path: string, pageIndex: number, currentRevision: number): VectorPageBundle | null {
    const idx = pageBundleCache.findIndex(b => b.path === path && b.pageIndex === pageIndex);
    if (idx < 0) return null;
    
    const hit = pageBundleCache[idx];
    if (hit.documentRevision !== currentRevision) {
        // Revision mismatch! Document has been mutated. Evict stale cache.
        pageBundleCache.splice(idx, 1);
        logPdfLayoutTrace('page-bundle.evict-stale-revision', {
            path,
            pageIndex,
            staleRevision: hit.documentRevision,
            currentRevision,
        });
        return null;
    }
    
    // Move to front (most recently used)
    pageBundleCache.splice(idx, 1);
    pageBundleCache.unshift(hit);
    return hit;
}

function insertCachedBundle(bundle: VectorPageBundle): void {
    // Remove existing entry for same page if present
    const idx = pageBundleCache.findIndex(b => b.path === bundle.path && b.pageIndex === bundle.pageIndex);
    if (idx >= 0) pageBundleCache.splice(idx, 1);
    pageBundleCache.unshift(bundle);
    while (pageBundleCache.length > PAGE_CACHE_MAX) {
        pageBundleCache.pop();
    }
}

export function invalidateVectorPageCache(): void {
    logPdfLayoutTrace('page-bundle.invalidate', {
        cacheSize: pageBundleCache.length,
    });
    pageBundleCache.length = 0;
}

function summarizeText(value: unknown, limit = 48): string {
    const text = typeof value === 'string' ? value : '';
    return text.length > limit ? `${text.slice(0, limit)}...` : text;
}

function summarizeVectorModel(model: any): Record<string, unknown> {
    const objects = Array.isArray(model?.objects) ? model.objects : [];
    const textObjects = objects.filter((object: any) => object?.type === 'text' || Array.isArray(object?.runs));
    const sampleRuns = textObjects
        .flatMap((object: any) => Array.isArray(object?.runs)
            ? object.runs.map((run: any) => ({ objectId: object.id, run }))
            : [])
        .slice(0, 8)
        .map(({ objectId, run }: any) => ({
            objectId,
            text: summarizeText(run?.text),
            x: run?.tx,
            y: run?.ty,
            origins: Array.isArray(run?.charOrigins) ? run.charOrigins.length : 0,
            firstOrigin: Array.isArray(run?.charOrigins) ? run.charOrigins[0] : null,
            lastOrigin: Array.isArray(run?.charOrigins) ? run.charOrigins[run.charOrigins.length - 1] : null,
            font: run?.fontName,
        }));
    return {
        pageIndex: model?.pageIndex,
        width: model?.width,
        height: model?.height,
        objectCount: objects.length,
        textObjectCount: textObjects.length,
        sampleRuns,
    };
}

function summarizePaintPlan(plan: any): Record<string, unknown> {
    const regions = Array.isArray(plan?.regions) ? plan.regions : [];
    const paragraphs = regions.flatMap((region: any) => Array.isArray(region?.paragraphs) ? region.paragraphs : []);
    const sampleParagraphs = paragraphs.slice(0, 8).map((paragraph: any) => ({
        id: paragraph?.id,
        regionId: paragraph?.regionId,
        runCount: Array.isArray(paragraph?.runs) ? paragraph.runs.length : 0,
        text: Array.isArray(paragraph?.runs)
            ? summarizeText(paragraph.runs.map((run: any) => run?.text ?? '').join(''))
            : '',
        firstRun: Array.isArray(paragraph?.runs) && paragraph.runs[0]
            ? {
                text: summarizeText(paragraph.runs[0].text),
                x: paragraph.runs[0].originX,
                y: paragraph.runs[0].originY,
                origins: Array.isArray(paragraph.runs[0].charOrigins) ? paragraph.runs[0].charOrigins.length : 0,
                font: paragraph.runs[0]?.resolvedFont?.renderFamily,
            }
            : null,
    }));
    return {
        pageIndex: plan?.pageIndex,
        width: plan?.width,
        height: plan?.height,
        regionCount: regions.length,
        paragraphCount: paragraphs.length,
        sampleParagraphs,
    };
}

async function loadImageCacheMapForPage(
    modelObjects: any[],
    frameToken?: number,
    pageIndex?: number,
    assetRole: 'current' | 'prefetch' = 'current',
): Promise<Map<string, ImageBitmap>> {
    const imageCacheMap = new Map<string, ImageBitmap>();
    const imageObjects = modelObjects.filter((o: any) => {
        const typeLower = String(o?.type).toLowerCase();
        return typeLower === 'image' && o?.id;
    });
    
    await Promise.all(
        imageObjects.map(
            (obj: any) =>
                new Promise<void>(async (resolve) => {
                    // Check before launching each image fetch request
                    if (frameToken !== undefined && !isFrameCurrent(frameToken)) {
                        resolve();
                        return;
                    }
                    if (pageIndex !== undefined) {
                        if (!admitPageAsset(pageIndex, assetRole, 'imageCache', 'page-bundle.image-cache.rejected')) {
                            resolve();
                            return;
                        }
                    }
                    try {
                        const id = obj.id;
                        const res = await fetch(`http://pdfasset.localhost/${id}`);
                        if (res.ok) {
                            const blob = await res.blob();
                            const bitmap = await createImageBitmap(blob);
                            imageCacheMap.set(id, bitmap);
                        }
                    } catch (e) {
                        console.error('Failed to load image cache', e);
                    }
                    resolve();
                }),
        ),
    );
    return imageCacheMap;
}

export async function resolveVectorPageBundle(
    path: string,
    pageIndex: number,
    frameToken?: number,
    explicitRole?: 'current' | 'prefetch',
): Promise<VectorPageBundleResolution> {
    const currentRevision = viewerSession.read().documentRevision;
    const cached = findCachedBundle(path, pageIndex, currentRevision);
    if (cached) {
        logPdfLayoutTrace('page-bundle.reuse', {
            path,
            pageIndex,
            modelWidth: cached.model?.width,
            modelHeight: cached.model?.height,
        });
        return { bundle: cached, bundleChanged: false };
    }

    {
        logPdfLayoutTrace('page-bundle.load.before', {
            path,
            pageIndex,
            cacheSize: pageBundleCache.length,
        });

        const assetRole = resolveAssetRole(pageIndex, frameToken, explicitRole);

        // Rust owns page asset admission. TS keeps only the interrupt point.
        if (!admitPageAsset(pageIndex, assetRole, 'vectorModel', 'page-bundle.load.rejected.before-ipc')) {
            throw new Error('stale frame');
        }

        const pageAssetBundle = await targetInvokeV3('read_page_asset_bundle', {
            path,
            pageIndex,
            targetZoom: 1.0,
            requestRole: assetRole,
            documentRevision: viewerSession.read().documentRevision,
        });
        const model = pageAssetBundle?.model;
        const paintPlan = pageAssetBundle?.paintPlan;

        // Preemption guard after IPC returns.
        if (!admitPageAsset(pageIndex, assetRole, 'paintPlan', 'page-bundle.load.rejected.after-ipc')) {
            throw new Error('stale frame');
        }
        if (frameToken !== undefined && !isFrameCurrent(frameToken)) {
            console.log(`[PDF-DIAG] Aborting bundle load for page ${pageIndex} due to stale frame`);
            throw new Error('stale frame');
        }

        // Skip read_images IPC if model already has inline image data
        const modelObjects = Array.isArray(model?.objects) ? model.objects : [];


        // Preemption guard before image cache loading.
        if (!admitPageAsset(pageIndex, assetRole, 'imageCache', 'page-bundle.load.rejected.before-images')) {
            throw new Error('stale frame');
        }

        const hasInlineImages = modelObjects.some((o: any) => {
            const typeLower = String(o?.type).toLowerCase();
            return typeLower === 'image' && o?.dataUrl;
        });
        const imageCacheMap = hasInlineImages
            ? new Map<string, ImageBitmap>()
            : await loadImageCacheMapForPage(modelObjects, frameToken, pageIndex, assetRole);

        if (frameToken !== undefined && !isFrameCurrent(frameToken)) {
            throw new Error('stale frame');
        }

        const objTypes = modelObjects.reduce((acc: Record<string, number>, o: any) => {
            const typeLower = String(o?.type).toLowerCase();
            const t = typeLower === 'image' ? 'Image' : typeLower === 'text' ? 'Text' : typeLower === 'path' ? 'Path' : 'Unknown';
            acc[t] = (acc[t] || 0) + 1;
            return acc;
        }, {} as Record<string, number>);
        logPdfLayoutTrace('page-bundle.asset-result', {
            pageIndex,
            objectCount: modelObjects.length,
            types: objTypes,
            width: model?.width,
            height: model?.height,
            imageMapSize: imageCacheMap.size,
            hasInlineImages,
        });

        const newBundle: VectorPageBundle = {
            path,
            pageIndex,
            documentRevision: currentRevision,
            model,
            paintPlan,
            imageCacheMap,
        };
        insertCachedBundle(newBundle);

        // Phase 3.1: eagerly hydrate WASM HOST_PAGE_STATE so click-to-edit
        // (which reads paint_plan) works even before the first render frame.
        try {
            const wasm: any = getWasmApi();
            if (wasm?.init_page_context) {
                const dpr = typeof window !== 'undefined' && Number.isFinite(window.devicePixelRatio)
                    ? window.devicePixelRatio
                    : 1;
                wasm.init_page_context(
                    JSON.stringify(model),
                    JSON.stringify(paintPlan),
                    1.0,
                    dpr,
                    0,
                    0,
                    model?.width ?? 0,
                    model?.height ?? 0,
                );
                const regionCount = Array.isArray(paintPlan?.regions) ? paintPlan.regions.length : 0;
                const paragraphCount = Array.isArray(paintPlan?.regions)
                    ? paintPlan.regions.reduce((acc: number, r: any) => acc + (Array.isArray(r?.paragraphs) ? r.paragraphs.length : 0), 0)
                    : 0;
                logPdfLayoutTrace('page-bundle.wasm-hydrated', {
                    pageIndex,
                    regionCount,
                    paragraphCount,
                    modelWidth: model?.width,
                    modelHeight: model?.height,
                });
            } else {
                console.warn('[EDITOR-DIAG] wasm.init_page_context unavailable');
            }
        } catch (err) {
            console.error('[EDITOR-DIAG] page-bundle.wasm-hydrate-error', { pageIndex, error: String(err) });
        }

        emitPdfDiagnostic('render-bundle', 'load', {
            path,
            pageIndex,
            model: summarizeVectorModel(model),
            paintPlan: summarizePaintPlan(paintPlan),
            imageCount: imageCacheMap.size,
        }, { verboseOnly: true });
        logPdfLayoutTrace('page-bundle.load.after', {
            path,
            pageIndex,
            modelWidth: model?.width,
            modelHeight: model?.height,
            paintPlanWidth: paintPlan?.width,
            paintPlanHeight: paintPlan?.height,
            imageCount: imageCacheMap.size,
        });
    }

    const resolvedBundle = findCachedBundle(path, pageIndex, currentRevision);
    if (!resolvedBundle) {
        throw new Error('vector page bundle unavailable after initialization');
    }

    return {
        bundle: resolvedBundle,
        bundleChanged: true,
    };
}

/** Prefetch adjacent page bundles in background (non-blocking). */
export function prefetchAdjacentPages(path: string, currentPage: number, pageCount: number): void {
    const decision = pagePresentationRuntime.decideAdjacentPrefetch(currentPage, pageCount);
    if (!decision.allowed) {
        logPdfLayoutTrace('page-bundle.prefetch.rejected', {
            path,
            currentPage,
            pageCount,
            rejectReason: decision.rejectReason,
            snapshot: decision.snapshot,
        });
        return;
    }

    for (const target of decision.targets) {
        if (target.assetKind === 'preview') continue;
        prefetchVectorPage(path, target.pageIndex);
    }
}

/**
 * 单页 vector bundle 预热，不重新调用 decideAdjacentPrefetch。
 * 由调用方负责确保页码已通过 Rust 准入决策。
 */
export function hasVectorPageBundle(path: string, pageIndex: number): boolean {
    const resolvedBundle = findCachedBundle(path, pageIndex, viewerSession.read().documentRevision);
    return !!resolvedBundle;
}

export function prefetchVectorPage(path: string, pageIndex: number): void {
    if (findCachedBundle(path, pageIndex, viewerSession.read().documentRevision)) return;
    // Fire and forget — 不阻塞当前渲染
    resolveVectorPageBundle(path, pageIndex, undefined, 'prefetch').catch(() => {});
}

export function isPageBundleCached(path: string, pageIndex: number): boolean {
    return pageBundleCache.some(b => b.path === path && b.pageIndex === pageIndex && b.documentRevision === viewerSession.read().documentRevision);
}



