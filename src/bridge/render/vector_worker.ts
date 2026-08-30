import { ensureWasmInitialized, getWasmApi } from '../shared/wasm_loader';
import { createRenderWasmApi } from './render_wasm_api';

export type VectorWorkerRequest =
    | { type: 'INIT_WASM' }
    | {
        type: 'RENDER_PAGE';
        msgId: number;
        isSamePage: boolean;
        modelJson?: string;
        paintPlanJson?: string;
        zoom: number;
        dpr: number;
        viewportLeft: number;
        viewportTop: number;
        viewportWidth: number;
        viewportHeight: number;
        /**
         * Page images — only sent when the worker's page context changed.
         * When omitted the worker reuses its stored map, so tile renders on
         * an already-loaded page do not re-clone every bitmap per tile.
         */
        imageCacheMap?: Map<string, ImageBitmap>;
        width: number;
        height: number;
        budgetMs: number;
        maxItems: number;
        useProgressive: boolean;
      }
    | { type: 'CANCEL_RENDER' };

export type VectorWorkerResponse = 
    | { type: 'INIT_DONE' }
    | { type: 'RENDER_DONE'; msgId: number; bitmap: ImageBitmap; aborted?: boolean }
    | { type: 'ERROR'; msgId?: number; error: string };

let renderCancelled = false;
// Worker-owned copy of the current page's images. Replaced only when a
// RENDER_PAGE message carries a new map (page/bundle changed); tile renders
// on the same context reuse it without re-transferring bitmaps.
let storedImageCacheMap: Map<string, ImageBitmap> = new Map();

self.onmessage = async (e: MessageEvent<VectorWorkerRequest>) => {
    try {
        const msg = e.data;
        if (msg.type === 'CANCEL_RENDER') {
            renderCancelled = true;
        } else if (msg.type === 'INIT_WASM') {
            await ensureWasmInitialized();
            self.postMessage({ type: 'INIT_DONE' });
        } else if (msg.type === 'RENDER_PAGE') {
            renderCancelled = false;
            await ensureWasmInitialized();
            const wasm = createRenderWasmApi(getWasmApi);

            if (msg.imageCacheMap) {
                for (const old of storedImageCacheMap.values()) {
                    try { old.close(); } catch {}
                }
                storedImageCacheMap = msg.imageCacheMap;
            }
            if (msg.isSamePage) {
                wasm.updatePageViewport(
                    msg.zoom,
                    msg.dpr,
                    msg.viewportLeft,
                    msg.viewportTop,
                    msg.viewportWidth,
                    msg.viewportHeight
                );
            } else {
                wasm.initPageContext(
                    msg.modelJson ?? '{}',
                    msg.paintPlanJson ?? '{}',
                    msg.zoom,
                    msg.dpr,
                    msg.viewportLeft,
                    msg.viewportTop,
                    msg.viewportWidth,
                    msg.viewportHeight
                );
            }
            
            const canvas = new OffscreenCanvas(msg.width, msg.height);

            if (msg.useProgressive) {
                // progressive render
                const start = wasm.startProgressiveRender();
                if (!start?.started) {
                    wasm.renderPageOffscreen(canvas, storedImageCacheMap, msg.dpr);
                } else {
                    let guard = 0;
                    while (guard < 4000) {
                        if (renderCancelled) {
                            wasm.cancelProgressiveRender();
                            (self as any).postMessage({ type: 'RENDER_DONE', msgId: msg.msgId, bitmap: null, aborted: true });
                            return;
                        }
                        const step = wasm.stepProgressiveRenderOffscreen(
                            canvas,
                            storedImageCacheMap,
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
                wasm.renderPageOffscreen(canvas, storedImageCacheMap, msg.dpr);
            }
            
            const bitmap = canvas.transferToImageBitmap();
            (self as any).postMessage({ type: 'RENDER_DONE', msgId: msg.msgId, bitmap }, [bitmap]);
        }
    } catch (err) {
        console.error('[VectorWorker] Error:', err);
        self.postMessage({ type: 'ERROR', msgId: (e.data as any).msgId, error: String(err) });
    }
};
