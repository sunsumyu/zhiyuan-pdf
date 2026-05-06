import { getWasmApi } from '../shared/wasm_loader';

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

export type VectorRenderRequest = {
    path: string;
    pageIndex: number;
    plan: RenderZoomPlan;
    frameToken?: number;
};

export type VectorRenderLayer = {
    cacheKey: string;
    renderZoom: number;
    useDetailLayer: boolean;
    viewportLeft: number;
    viewportTop: number;
    viewportWidth: number;
    viewportHeight: number;
};

export type VectorRenderResult = {
    width: number;
    height: number;
    aborted: boolean;
    pendingPresents: VectorLayerPresent[];
};

export type VectorLayerPresent = {
    sourceCanvasId: string;
    viewportWidth: number;
    viewportHeight: number;
    useViewportTile: boolean;
    viewportLeft: number;
    viewportTop: number;
    showDetailOverlay: boolean;
    retainDetailOverlay: boolean;
};

export type RenderCacheInvalidateRequest = {
    path: string;
    pageIndex?: number;
    clearFrameCache: boolean;
    clearPageCache: boolean;
};

export type RenderFacadeResult = {
    changed: boolean;
    renderResult: VectorRenderResult | null;
    renderFrame: unknown | null;
};

function callFacade<T>(fnName: string, arg?: unknown): T | null {
    const api = getWasmApi();
    const fn = (api as any)[fnName];
    if (typeof fn !== 'function') return null;
    try {
        return arg !== undefined ? fn(arg) : fn();
    } catch {
        return null;
    }
}

export function facadeRenderPage(request: VectorRenderRequest): VectorRenderResult | null {
    return callFacade<VectorRenderResult>('renderFacadePage', request);
}

export function facadeRenderLayer(request: VectorRenderLayer): RenderFacadeResult | null {
    return callFacade<RenderFacadeResult>('renderFacadeLayer', request);
}

export function facadeInvalidateCache(request: RenderCacheInvalidateRequest): RenderFacadeResult | null {
    return callFacade<RenderFacadeResult>('renderFacadeInvalidateCache', request);
}

export function facadeClearHost(): RenderFacadeResult | null {
    return callFacade<RenderFacadeResult>('renderFacadeClearHost');
}

export function facadeCancelProgressive(): RenderFacadeResult | null {
    return callFacade<RenderFacadeResult>('renderFacadeCancelProgressive');
}

export function facadeIsFrameCurrent(frameToken: number): boolean | null {
    return callFacade<boolean>('renderFacadeIsFrameCurrent', frameToken);
}

