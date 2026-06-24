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
        options.beforePresent?.();
    };

    if (pendingPresents.length === 0) {
        prepareVisibleFrame();
        return;
    }

    const refs = getExistingVectorCanvasHost();
    if (!refs) return;

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

    emitPdfDiagnostic('render-chain', 'ts.deferred-present.commit', {
        layerCount: pendingPresents.length,
        width: result.width,
        height: result.height,
    }, { verboseOnly: true });
}
