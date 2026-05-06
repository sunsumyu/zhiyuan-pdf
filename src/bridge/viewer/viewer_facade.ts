// Viewer facade — frozen v1 TS bindings. See docs/api-contract.md.

import { getWasmApi } from '../shared/wasm_loader';

export type StubResult = { implemented: boolean; error: string };

function call<T>(name: string, ...args: unknown[]): T | null {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try { return args.length ? fn(...args) : fn(); } catch { return null; }
}

// Stable
export function facadeViewerReadSession(): unknown { return call('viewerFacadeReadSession'); }
export function facadeViewerResetSession(): void { call('viewerFacadeResetSession'); }
export function facadeViewerSetDocument(path: string | null, pageCount: number, initialZoom: number): void {
    call('viewerFacadeSetDocument', path, pageCount, initialZoom);
}
export function facadeViewerSetCurrentPage(pageIndex: number): void { call('viewerFacadeSetCurrentPage', pageIndex); }
export function facadeViewerSetCurrentZoom(zoom: number): void { call('viewerFacadeSetCurrentZoom', zoom); }
export function facadeViewerSetPageSize(w: number, h: number): void { call('viewerFacadeSetPageSize', w, h); }
export function facadeViewerNavigatePrev(): unknown { return call('viewerFacadeNavigatePrev'); }
export function facadeViewerNavigateNext(): unknown { return call('viewerFacadeNavigateNext'); }
export function facadeViewerApplyZoomSelection(zoom: number): unknown { return call('viewerFacadeApplyZoomSelection', zoom); }

// Stubs
export function facadeViewerGoToPage(index: number, anchor?: string): StubResult | null {
    return call('viewerFacadeGoToPage', index, anchor ?? null);
}
export function facadeViewerGoToNamedDestination(name: string): StubResult | null {
    return call('viewerFacadeGoToNamedDestination', name);
}
export function facadeViewerSetPresentationMode(enabled: boolean): StubResult | null {
    return call('viewerFacadeSetPresentationMode', enabled);
}
export function facadeViewerSetLayoutMode(mode: string): StubResult | null {
    return call('viewerFacadeSetLayoutMode', mode);
}

