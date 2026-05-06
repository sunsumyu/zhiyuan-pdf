// Zoom facade — frozen v1 TS bindings. See docs/api-contract.md.

import { getWasmApi } from '../shared/wasm_loader';

export type StubResult = { implemented: boolean; error: string };

function call<T>(name: string, ...args: unknown[]): T | null {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try { return args.length ? fn(...args) : fn(); } catch { return null; }
}

// Stable
export function facadeZoomReadState(): unknown { return call('zoomFacadeReadState'); }
export function facadeZoomReset(initialZoom: number): void { call('zoomFacadeReset', initialZoom); }
export function facadeZoomSetTarget(targetZoom: number): void { call('zoomFacadeSetTarget', targetZoom); }
export function facadeZoomMarkRendered(renderedZoom: number): void { call('zoomFacadeMarkRendered', renderedZoom); }

// Stubs
export function facadeZoomAnimateTo(target: number, durationMs: number): StubResult | null {
    return call('zoomFacadeAnimateTo', target, durationMs);
}
export function facadeZoomFitPage(): StubResult | null { return call('zoomFacadeFitPage'); }
export function facadeZoomFitWidth(): StubResult | null { return call('zoomFacadeFitWidth'); }
export function facadeZoomActualSize(): StubResult | null { return call('zoomFacadeActualSize'); }
export function facadeZoomZoomAtPoint(target: number, x: number, y: number): StubResult | null {
    return call('zoomFacadeZoomAtPoint', target, x, y);
}

