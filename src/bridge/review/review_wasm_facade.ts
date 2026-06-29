// Review WASM bridge — delegates to `ReviewSession` struct API (P2 of session-API plan).

import { getWasmApi } from '../shared/wasm_loader';
import type { ReviewSession } from '../../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui';

// ── Singleton ReviewSession instance ──────────────────────────────────

let _session: ReviewSession | null = null;

function getReviewSession(): ReviewSession {
    if (!_session) {
        const api = getWasmApi();
        if (typeof api?.ReviewSession === 'function') {
            _session = new api.ReviewSession();
        }
    }
    if (!_session) {
        throw new Error('ReviewSession WASM API is unavailable');
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

function callMethod<T>(method: string, ...args: unknown[]): T {
    const session = getReviewSession();
    const sessionRecord = session as unknown as Record<string, unknown>;
    const fn = sessionRecord?.[method];
    if (typeof fn !== 'function') {
        throw new Error(`Required ReviewSession method is unavailable: ${method}`);
    }
    return (fn as (...a: unknown[]) => T).apply(session, args);
}

// ── Review API ──

export function getReviewFeed(): ReviewFeedResult | null {
    return callMethod<ReviewFeedResult>('readFeed');
}

export function acceptChange(patchKey: string): ReviewFacadeResult {
    const r = callMethod<{ changed: boolean }>('accept', patchKey);
    return { changed: r.changed, feed: null, locateResult: null, renderFrame: null };
}

export function rejectChange(patchKey: string): ReviewFacadeResult {
    const r = callMethod<{ changed: boolean }>('reject', patchKey);
    return { changed: r.changed, feed: null, locateResult: null, renderFrame: null };
}

export function acceptAllChanges(): ReviewFacadeResult {
    const r = callMethod<{ changed: boolean }>('acceptAll');
    return { changed: r.changed, feed: null, locateResult: null, renderFrame: null };
}

export function rejectAllChanges(): ReviewFacadeResult {
    const r = callMethod<{ changed: boolean }>('rejectAll');
    return { changed: r.changed, feed: null, locateResult: null, renderFrame: null };
}

export function locateChange(patchKey: string): ReviewLocateResult | null {
    return callMethod<ReviewLocateResult>('locate', patchKey);
}
