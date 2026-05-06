// Review WASM facade — review types and API calls extracted from find_facade.ts.

import { getWasmApi } from '../shared/wasm_loader';

// ── Review types ──

export type ReviewChangeEntry = {
    patchKey: string;
    pageIndex: number;
    regionId: string;
    kind: string | null;
    originalText: string;
    currentText: string;
    source: string;
};

export type ReviewFeedResult = {
    revision: number;
    pendingCount: number;
    changes: ReviewChangeEntry[];
};

export type ReviewLocateResult = {
    pageIndex: number;
    regionId: string;
    kind: string | null;
    originalText: string;
};

export type ReviewFacadeResult = {
    changed: boolean;
    feed: ReviewFeedResult | null;
    locateResult: ReviewLocateResult | null;
    renderFrame: unknown | null;
};

// ── Helpers ──

function callWasm<T>(fnName: string, arg?: unknown): T | null {
    const api = getWasmApi();
    const fn = (api as any)[fnName];
    if (typeof fn !== 'function') return null;
    try {
        return arg !== undefined ? fn(arg) : fn();
    } catch {
        return null;
    }
}

// ── Review API ──

export function getReviewFeed(): ReviewFeedResult | null {
    return callWasm<ReviewFeedResult>('reviewFacadeReadFeed');
}

export function acceptChange(patchKey: string): ReviewFacadeResult | null {
    const r = callWasm<{ changed: boolean }>('reviewFacadeAccept', patchKey);
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function rejectChange(patchKey: string): ReviewFacadeResult | null {
    const r = callWasm<{ changed: boolean }>('reviewFacadeReject', patchKey);
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function acceptAllChanges(): ReviewFacadeResult | null {
    const r = callWasm<{ changed: boolean }>('reviewFacadeAcceptAll');
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function rejectAllChanges(): ReviewFacadeResult | null {
    const r = callWasm<{ changed: boolean }>('reviewFacadeRejectAll');
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function locateChange(patchKey: string): ReviewLocateResult | null {
    return callWasm<ReviewLocateResult>('locate_review_change', patchKey);
}
