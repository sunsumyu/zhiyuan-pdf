import { getWasmApi, targetInvokeV3 } from '../shared/wasm_loader';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import { logPdfLayoutTrace } from './layout_trace';

export type VectorPageBundle = {
    path: string;
    pageIndex: number;
    model: any;
    paintPlan: any;
    imageCacheMap: Map<string, HTMLImageElement>;
};

type VectorPageBundleResolution = {
    bundle: VectorPageBundle;
    bundleChanged: boolean;
};

const PAGE_CACHE_MAX = 5;
const pageBundleCache: VectorPageBundle[] = [];

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

function findCachedBundle(path: string, pageIndex: number): VectorPageBundle | null {
    const idx = pageBundleCache.findIndex(b => b.path === path && b.pageIndex === pageIndex);
    if (idx < 0) return null;
    // Move to front (most recently used)
    const [hit] = pageBundleCache.splice(idx, 1);
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
): Promise<Map<string, HTMLImageElement>> {
    const imageCacheMap = new Map<string, HTMLImageElement>();
    const imageObjects = modelObjects.filter((o: any) => {
        const typeLower = String(o?.type).toLowerCase();
        return typeLower === 'image' && o?.id;
    });
    
    await Promise.all(
        imageObjects.map(
            (obj: any) =>
                new Promise<void>((resolve) => {
                    // Check before launching each image fetch request
                    if (frameToken !== undefined && !isFrameCurrent(frameToken)) {
                        resolve();
                        return;
                    }
                    if (pageIndex !== undefined) {
                        const currentPage = getCurrentPageIndex();
                        if (currentPage !== null && Math.abs(currentPage - pageIndex) > 1) {
                            resolve();
                            return;
                        }
                    }
                    const id = obj.id;
                    const img = new Image();
                    img.onload = () => {
                        imageCacheMap.set(id, img);
                        resolve();
                    };
                    img.onerror = () => resolve();
                    // Load directly from the fast, zero-overhead custom protocol
                    img.src = `http://pdfasset.localhost/${id}`;
                }),
        ),
    );
    return imageCacheMap;
}

export async function resolveVectorPageBundle(
    path: string,
    pageIndex: number,
    frameToken?: number,
): Promise<VectorPageBundleResolution> {
    const cached = findCachedBundle(path, pageIndex);
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
        const [model, paintPlan] = await Promise.all([
            targetInvokeV3('read_vector', {
                path,
                pageIndex,
                targetZoom: 1.0,
            }),
            targetInvokeV3('read_glyph_plan', { path, pageIndex }),
        ]);

        // Early-Abort check after IPC returns
        if (frameToken !== undefined && !isFrameCurrent(frameToken)) {
            console.log(`[PDF-DIAG] Aborting bundle load for page ${pageIndex} due to stale frame`);
            throw new Error('stale frame');
        }
        const currentPage = getCurrentPageIndex();
        if (currentPage !== null && Math.abs(currentPage - pageIndex) > 1) {
            console.log(`[PDF-DIAG] Aborting prefetch/load for page ${pageIndex} because currentPage is ${currentPage}`);
            throw new Error('stale frame');
        }

        // Skip read_images IPC if model already has inline image data
        const modelObjects = Array.isArray(model?.objects) ? model.objects : [];
        const hasInlineImages = modelObjects.some((o: any) => {
            const typeLower = String(o?.type).toLowerCase();
            return typeLower === 'image' && o?.dataUrl;
        });
        const imageCacheMap = hasInlineImages
            ? new Map<string, HTMLImageElement>()
            : await loadImageCacheMapForPage(modelObjects, frameToken, pageIndex);

        if (frameToken !== undefined && !isFrameCurrent(frameToken)) {
            throw new Error('stale frame');
        }

        const objTypes = modelObjects.reduce((acc: Record<string, number>, o: any) => {
            const typeLower = String(o?.type).toLowerCase();
            const t = typeLower === 'image' ? 'Image' : typeLower === 'text' ? 'Text' : typeLower === 'path' ? 'Path' : 'Unknown';
            acc[t] = (acc[t] || 0) + 1;
            return acc;
        }, {} as Record<string, number>);
        console.log('[PDF-DIAG] read_vector result:', {
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
                console.log('[EDITOR-DIAG] page-bundle.wasm-hydrated', {
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

    const resolvedBundle = findCachedBundle(path, pageIndex);
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
    const targets: number[] = [];
    if (currentPage + 1 < pageCount) targets.push(currentPage + 1);
    if (currentPage - 1 >= 0) targets.push(currentPage - 1);

    for (const target of targets) {
        if (findCachedBundle(path, target)) continue;
        // Fire and forget — don't block current render
        resolveVectorPageBundle(path, target).catch(() => {});
    }
}


