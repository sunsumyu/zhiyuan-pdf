import { ensureWasmInitialized, getWasmApi } from '../shared/wasm_loader';
import { createRenderWasmApi } from './render_wasm_api';

export type VectorWorkerRequest = 
    | { type: 'INIT_WASM' }
    | { 
        type: 'RENDER_PAGE'; 
        msgId: number;
        modelJson: string; 
        paintPlanJson: string; 
        zoom: number; 
        dpr: number; 
        viewportLeft: number; 
        viewportTop: number; 
        viewportWidth: number; 
        viewportHeight: number; 
        imageCacheMap: Map<string, ImageBitmap>;
        width: number;
        height: number;
        budgetMs: number;
        maxItems: number;
        useProgressive: boolean;
      };

export type VectorWorkerResponse = 
    | { type: 'INIT_DONE' }
    | { type: 'RENDER_DONE'; msgId: number; bitmap: ImageBitmap; aborted?: boolean }
    | { type: 'ERROR'; msgId?: number; error: string };

self.onmessage = async (e: MessageEvent<VectorWorkerRequest>) => {
    try {
        const msg = e.data;
        if (msg.type === 'INIT_WASM') {
            await ensureWasmInitialized();
            self.postMessage({ type: 'INIT_DONE' });
        } else if (msg.type === 'RENDER_PAGE') {
            await ensureWasmInitialized();
            const wasm = createRenderWasmApi(getWasmApi);
            
            wasm.initPageContext(
                msg.modelJson,
                msg.paintPlanJson,
                msg.zoom,
                msg.dpr,
                msg.viewportLeft,
                msg.viewportTop,
                msg.viewportWidth,
                msg.viewportHeight
            );
            
            const canvas = new OffscreenCanvas(msg.width, msg.height);

            if (msg.useProgressive) {
                // progressive render
                const start = wasm.startProgressiveRender();
                if (!start?.started) {
                    wasm.renderPageOffscreen(canvas, msg.imageCacheMap, msg.dpr);
                } else {
                    let guard = 0;
                    while (guard < 4000) {
                        const step = wasm.stepProgressiveRenderOffscreen(
                            canvas,
                            msg.imageCacheMap,
                            msg.budgetMs,
                            msg.maxItems,
                            msg.dpr
                        );
                        if (!step?.active || step.completed) {
                            break;
                        }
                        guard++;
                        // yield to event loop
                        await new Promise(r => setTimeout(r, 0));
                    }
                    if (guard >= 4000) {
                        wasm.cancelProgressiveRender();
                        throw new Error('progressive render guard exceeded in worker');
                    }
                }
            } else {
                wasm.renderPageOffscreen(canvas, msg.imageCacheMap, msg.dpr);
            }
            
            const bitmap = canvas.transferToImageBitmap();
            (self as any).postMessage({ type: 'RENDER_DONE', msgId: msg.msgId, bitmap }, [bitmap]);
        }
    } catch (err) {
        console.error('[VectorWorker] Error:', err);
        self.postMessage({ type: 'ERROR', msgId: (e.data as any).msgId, error: String(err) });
    }
};
