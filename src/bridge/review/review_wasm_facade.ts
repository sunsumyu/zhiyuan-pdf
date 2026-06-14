// Review WASM bridge — delegates to `ReviewSession` struct API (P2 of session-API plan).

import { getWasmApi } from '../shared/wasm_loader';
import type { WasmModule } from '../shared/wasm_loader';
import type { ReviewSession } from '../../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui';

// ── Singleton ReviewSession instance ──────────────────────────────────

let _session: ReviewSession | null = null;

function getReviewSession(): ReviewSession | null {
    if (!_session) {
        const api = getWasmApi();
        if (typeof api?.ReviewSession === 'function') {
            _session = new api.ReviewSession();
        }
    }
    return _session;
}

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

function callMethod<T>(method: string, ...args: unknown[]): T | null {
    const session = getReviewSession();
    const sessionRecord = session as unknown as Record<string, unknown>;
    const fn = sessionRecord?.[method];
    if (typeof fn !== 'function') return null;
    try {
        return (fn as (...a: unknown[]) => T).apply(session, args);
    } catch {
        return null;
    }
}

// `locateChange` is not yet on `ReviewSession`; fall back to the raw wasm export.
function callRawWasm<T>(fnName: string, arg?: unknown): T | null {
    const api = getWasmApi() as unknown as Record<string, unknown>;
    const fn = api?.[fnName];
    if (typeof fn !== 'function') return null;
    try {
        return arg !== undefined ? (fn as (a?: unknown) => T)(arg) : (fn as () => T)();
    } catch {
        return null;
    }
}

// ── Review API ──

export function getReviewFeed(): ReviewFeedResult | null {
    return callMethod<ReviewFeedResult>('readFeed');
}

export function acceptChange(patchKey: string): ReviewFacadeResult | null {
    const r = callMethod<{ changed: boolean }>('accept', patchKey);
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function rejectChange(patchKey: string): ReviewFacadeResult | null {
    const r = callMethod<{ changed: boolean }>('reject', patchKey);
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function acceptAllChanges(): ReviewFacadeResult | null {
    const r = callMethod<{ changed: boolean }>('acceptAll');
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function rejectAllChanges(): ReviewFacadeResult | null {
    const r = callMethod<{ changed: boolean }>('rejectAll');
    return r ? { changed: r.changed, feed: null, locateResult: null, renderFrame: null } : null;
}

export function locateChange(patchKey: string): ReviewLocateResult | null {
    return callRawWasm<ReviewLocateResult>('locate_review_change', patchKey);
}
