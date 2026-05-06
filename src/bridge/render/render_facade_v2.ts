// Render facade v2 — frozen v1 TS bindings against `crate::render::wasm_facade`.
// (named *_v2.ts to avoid clashing with existing bridge/render_facade.ts)
// See docs/api-contract.md.

import { getWasmApi } from '../shared/wasm_loader';

export type StubResult = { implemented: boolean; error: string };

function call<T>(name: string, ...args: unknown[]): T | null {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try { return args.length ? fn(...args) : fn(); } catch { return null; }
}

// Stable — progressive lifecycle
export function facadeRenderStartProgressive(): unknown { return call('renderFacadeStartProgressive'); }
export function facadeRenderStepProgressive(
    canvasId: string,
    imageCache: unknown,
    budgetMs: number,
    maxItems: number,
): unknown {
    return call('renderFacadeStepProgressive', canvasId, imageCache, budgetMs, maxItems);
}
export function facadeRenderCancelProgressive(): void { call('renderFacadeCancelProgressive'); }
export function facadeRenderRenderPage(canvasId: string, imageCache: unknown): void {
    call('renderFacadeRenderPage', canvasId, imageCache);
}

// Stable — frame
export function facadeRenderCommitResult(
    frameToken: number,
    renderedZoom: number,
    pageWidth: number,
    pageHeight: number,
): unknown {
    return call('renderFacadeCommitResult', frameToken, renderedZoom, pageWidth, pageHeight);
}
export function facadeRenderAbortFrame(frameToken: number): unknown { return call('renderFacadeAbortFrame', frameToken); }
export function facadeRenderIsFrameCurrent(frameToken: number): boolean {
    return !!call<boolean>('renderFacadeIsFrameCurrent', frameToken);
}

// Stable — cache
export function facadeRenderTouchCache(isDetail: boolean, key: string): boolean {
    return !!call<boolean>('renderFacadeTouchCache', isDetail, key);
}
export function facadeRenderStoreCache(isDetail: boolean, key: string): unknown {
    return call('renderFacadeStoreCache', isDetail, key);
}
export function facadeRenderResetCache(): void { call('renderFacadeResetCache'); }

// Stubs
export function facadeRenderSnapshotPng(dpi: number): StubResult | null {
    return call('renderFacadeSnapshotPng', dpi);
}
export function facadeRenderPrewarmCache(pageIndex: number): StubResult | null {
    return call('renderFacadePrewarmCache', pageIndex);
}
export function facadeRenderSetQuality(preset: 'draft' | 'normal' | 'high'): StubResult | null {
    return call('renderFacadeSetQuality', preset);
}
export function facadeRenderSetDebugOverlay(enabled: boolean): StubResult | null {
    return call('renderFacadeSetDebugOverlay', enabled);
}

