import { getWasmApi } from '../../shared/wasm_loader';
import { invalidateVectorPageCache } from '../vector_page_bundle';
import {
    clearVectorCanvasHost,
    ensureVectorCanvasHost,
    getExistingVectorCanvasHost,
    presentViewportCanvas,
    presentViewportCanvasFromSource,
    VECTOR_CANVAS_ID,
    VECTOR_CONTAINER_ID,
} from '../vector_canvas_host';
import {
    clearVectorFrameCache,
} from '../vector_frame_cache';
import { logPdfLayoutTrace } from '../layout_trace';
import { emitPdfDiagnostic } from '../../shared/diagnostics';
import { createRenderWasmApi } from '../render_wasm_api';
import { clearWorkerLastState } from './worker_client';
import { renderVectorPageWithPlan } from './layer_render';
import { clearGpuPageCache } from '../gpu_page_renderer';

export { invalidateVectorPageCache };
export { VECTOR_CANVAS_ID, VECTOR_CONTAINER_ID };
export { renderVectorPageWithPlan };
export type { VectorRenderResult, VectorCommitOptions, VectorLayerPresent, RenderZoomPlan } from './layer_render';

const renderApi = createRenderWasmApi(() => getWasmApi());

export function clearVectorHost(): void {
    logPdfLayoutTrace('vector-host.clear.before');
    try {
        renderApi.cancelProgressiveRender();
        renderApi.resetFrameCache();
    } catch {}
    clearVectorCanvasHost();
    clearGpuPageCache();
    invalidateVectorPageCache();
    clearVectorFrameCache();
    clearWorkerLastState();
    logPdfLayoutTrace('vector-host.clear.after');
}

export function invalidateVectorRenderCache(): void {
    logPdfLayoutTrace('vector-host.invalidate-cache.before');
    try {
        renderApi.cancelProgressiveRender();
        renderApi.resetFrameCache();
    } catch {}
    clearGpuPageCache();
    invalidateVectorPageCache();
    clearVectorFrameCache();
    clearWorkerLastState();
    logPdfLayoutTrace('vector-host.invalidate-cache.after');
}

export function ensureVectorHost(): any {
    return ensureVectorCanvasHost();
}

export function commitVectorRenderResult(result: any, options: any = {}): void {
    const pendingPresents = result.pendingPresents ?? [];
    let preparedVisibleFrame = false;
    const prepareVisibleFrame = (): void => {
        if (preparedVisibleFrame) return;
        preparedVisibleFrame = true;
        const refs = ensureVectorCanvasHost();
        if (!refs) return;

        if (result.mainLayer) {
            presentViewportCanvasFromSource(
                refs,
                result.mainLayer.canvas,
                result.mainLayer.destWidth,
                result.mainLayer.destHeight,
                false,
                result.mainLayer.destX,
                result.mainLayer.destY,
            );
        }

        for (const present of pendingPresents) {
            presentViewportCanvasFromSource(
                refs,
                present.sourceCanvas,
                present.viewportWidth,
                present.viewportHeight,
                present.useViewportTile,
                present.viewportLeft,
                present.viewportTop,
            );
            presentViewportCanvas(refs, {
                showDetailOverlay: present.showDetailOverlay,
                retainDetailOverlay: present.retainDetailOverlay,
            });
        }
    };

    if (pendingPresents.length > 0) {
        options.onPendingPresent?.({
            pendingCount: pendingPresents.length,
            prepareVisibleFrame,
        });
    }

    prepareVisibleFrame();

    for (const present of pendingPresents) {
        if (present.target === 'preview') {
            options.onPreviewPresent?.(present);
        }
    }
}
