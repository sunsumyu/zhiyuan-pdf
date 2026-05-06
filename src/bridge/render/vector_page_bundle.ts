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

let cachedPageBundle: VectorPageBundle | null = null;

export function invalidateVectorPageCache(): void {
    logPdfLayoutTrace('page-bundle.invalidate', {
        hadCache: !!cachedPageBundle,
        cachedPath: cachedPageBundle?.path,
        cachedPageIndex: cachedPageBundle?.pageIndex,
        cachedWidth: cachedPageBundle?.model?.width,
        cachedHeight: cachedPageBundle?.model?.height,
    });
    cachedPageBundle = null;
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

async function loadImageCacheMap(path: string): Promise<Map<string, HTMLImageElement>> {
    const imageCacheRaw: Record<string, string> = await targetInvokeV3('read_images', { path });
    const imageCacheMap = new Map<string, HTMLImageElement>();
    await Promise.all(
        Object.entries(imageCacheRaw).map(
            ([id, dataUrl]) =>
                new Promise<void>((resolve) => {
                    const img = new Image();
                    img.onload = () => {
                        imageCacheMap.set(id, img);
                        resolve();
                    };
                    img.onerror = () => resolve();
                    img.src = dataUrl;
                }),
        ),
    );
    return imageCacheMap;
}

export async function resolveVectorPageBundle(
    path: string,
    pageIndex: number,
): Promise<VectorPageBundleResolution> {
    const bundleChanged =
        !cachedPageBundle ||
        cachedPageBundle.path !== path ||
        cachedPageBundle.pageIndex !== pageIndex;

    if (bundleChanged) {
        logPdfLayoutTrace('page-bundle.load.before', {
            path,
            pageIndex,
            hadCache: !!cachedPageBundle,
            cachedPath: cachedPageBundle?.path,
            cachedPageIndex: cachedPageBundle?.pageIndex,
        });
        const [model, paintPlan] = await Promise.all([
            targetInvokeV3('read_vector', {
                path,
                pageIndex,
                targetZoom: 1.0,
            }),
            targetInvokeV3('read_glyph_plan', { path, pageIndex }),
        ]);
        const imageCacheMap = await loadImageCacheMap(path);

        cachedPageBundle = {
            path,
            pageIndex,
            model,
            paintPlan,
            imageCacheMap,
        };

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
    } else {
        logPdfLayoutTrace('page-bundle.reuse', {
            path,
            pageIndex,
            modelWidth: cachedPageBundle?.model?.width,
            modelHeight: cachedPageBundle?.model?.height,
        });
    }

    if (!cachedPageBundle) {
        throw new Error('vector page bundle unavailable after initialization');
    }

    return {
        bundle: cachedPageBundle,
        bundleChanged,
    };
}


