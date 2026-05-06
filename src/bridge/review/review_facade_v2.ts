// Review facade v2 — frozen v1 TS bindings against `crate::review::facade`.
// (named *_v2.ts to avoid clashing with the pre-existing `bridge/find_facade.ts`
//  which already exports `ReviewFacadeResult`-typed helpers via raw wasm.)
// See docs/api-contract.md.

import { getWasmApi } from '../shared/wasm_loader';

export type StubResult = { implemented: boolean; error: string };

function call<T>(name: string, ...args: unknown[]): T | null {
    const api = getWasmApi();
    const fn = (api as any)[name];
    if (typeof fn !== 'function') return null;
    try { return args.length ? fn(...args) : fn(); } catch { return null; }
}

// Stable
export function facadeReviewReadFeed(): unknown { return call('reviewFacadeReadFeed'); }
export function facadeReviewAccept(patchKey: string): unknown { return call('reviewFacadeAccept', patchKey); }
export function facadeReviewReject(patchKey: string): unknown { return call('reviewFacadeReject', patchKey); }
export function facadeReviewAcceptAll(): unknown { return call('reviewFacadeAcceptAll'); }
export function facadeReviewRejectAll(): unknown { return call('reviewFacadeRejectAll'); }

// Stubs
export function facadeReviewExportReport(format: string): StubResult | null {
    return call('reviewFacadeExportReport', format);
}
export function facadeReviewReadFilteredFeed(filter: unknown): StubResult | null {
    return call('reviewFacadeReadFilteredFeed', filter);
}

