import type { VectorWorkerRequest, VectorWorkerResponse } from '../vector_worker';

let vectorWorker: Worker | null = null;
let msgIdCounter = 0;

export const pendingVectorTasks = new Map<
    number,
    { resolve: (bitmap: ImageBitmap) => void; reject: (err: any) => void }
>();

export let workerLastPath: string | null = null;
export let workerLastPageIndex: number | null = null;
export let workerLastRevision: number | null = null;

export function clearWorkerLastState(): void {
    workerLastPath = null;
    workerLastPageIndex = null;
    workerLastRevision = null;
}

export function ensureVectorWorker(): Worker {
    if (!vectorWorker) {
        vectorWorker = new Worker(new URL('../vector_worker.ts', import.meta.url), { type: 'module' });
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

export async function runWorkerRender(params: {
    path: string | undefined;
    pageIndex: number | undefined;
    revision: number | undefined;
    model: any;
    paintPlan: any;
    zoom: number;
    dpr: number;
    viewportLeft: number;
    viewportTop: number;
    viewportWidth: number;
    viewportHeight: number;
    clonedImageCacheMap: Map<string, ImageBitmap>;
    transferList: Transferable[];
    width: number;
    height: number;
    budgetMs: number;
    maxItems: number;
    useProgressive: boolean;
}): Promise<ImageBitmap> {
    const worker = ensureVectorWorker();
    const msgId = ++msgIdCounter;

    const isSamePage =
        params.path !== undefined &&
        params.pageIndex !== undefined &&
        params.revision !== undefined &&
        workerLastPath === params.path &&
        workerLastPageIndex === params.pageIndex &&
        workerLastRevision === params.revision;

    if (params.path !== undefined && params.pageIndex !== undefined && params.revision !== undefined) {
        workerLastPath = params.path;
        workerLastPageIndex = params.pageIndex;
        workerLastRevision = params.revision;
    }

    const promise = new Promise<ImageBitmap>((resolve, reject) => {
        pendingVectorTasks.set(msgId, { resolve, reject });
    });

    worker.postMessage({
        type: 'RENDER_PAGE',
        msgId,
        isSamePage,
        modelJson: isSamePage ? undefined : JSON.stringify(params.model ?? {}),
        paintPlanJson: isSamePage ? undefined : JSON.stringify(params.paintPlan ?? {}),
        zoom: params.zoom ?? 1.0,
        dpr: params.dpr,
        viewportLeft: params.viewportLeft ?? 0,
        viewportTop: params.viewportTop ?? 0,
        viewportWidth: params.viewportWidth ?? params.model?.width ?? 0,
        viewportHeight: params.viewportHeight ?? params.model?.height ?? 0,
        imageCacheMap: params.clonedImageCacheMap,
        width: params.width,
        height: params.height,
        budgetMs: params.budgetMs,
        maxItems: params.maxItems,
        useProgressive: params.useProgressive,
    }, params.transferList);

    return promise;
}
