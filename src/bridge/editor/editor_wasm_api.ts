// ─────────────────────────────────────────────────────────────────────────────
// editor_wasm_api.ts — document & review session bridge.
//
// Editor-specific logic has been fully migrated to EditorSession (./api.ts).
// This file delegates document-mutation and review-feed calls to
// `DocumentSession` / `ReviewSession` struct APIs (P1/P2 of session-API plan).
// ─────────────────────────────────────────────────────────────────────────────

import type { RustRenderFrame } from '../render/frame_plan';
import type { WasmModule } from '../shared/wasm_loader';
import type { DocumentSession, ReviewSession } from '../../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui';

type GetWasmApi = () => WasmModule;

// ── Session singletons ─────────────────────────────────────────

let _documentSession: DocumentSession | null = null;
let _reviewSession: ReviewSession | null = null;

function getDocumentSession(getWasmApi: GetWasmApi): DocumentSession {
    if (!_documentSession) {
        const api = getWasmApi();
        if (typeof api?.DocumentSession === 'function') {
            _documentSession = new api.DocumentSession();
        }
    }
    if (!_documentSession) {
        throw new Error('DocumentSession WASM API is unavailable');
    }
    return _documentSession;
}

function getReviewSession(getWasmApi: GetWasmApi): ReviewSession {
    if (!_reviewSession) {
        const api = getWasmApi();
        if (typeof api?.ReviewSession === 'function') {
            _reviewSession = new api.ReviewSession();
        }
    }
    if (!_reviewSession) {
        throw new Error('ReviewSession WASM API is unavailable');
    }
    return _reviewSession;
}

// ─── Shared types (preserved for external callers) ───────────────────────────

export type RegionTextReplaceRequest = {
    pageIndex: number;
    regionId: string;
    kind: string;
    originalText: string;
    query: string;
    replacement: string;
    replaceAllOccurrences?: boolean;
};

export type RegionTextReplaceResult = {
    appliedCount: number;
    skippedCount: number;
    renderFrame?: RustRenderFrame | null;
};

export type DocumentRefreshResult = {
    revision?: number;
    renderFrame?: RustRenderFrame | null;
};

export type ReviewChangeEntry = {
    patchKey: string;
    pageIndex: number;
    regionId: string;
    source: string;
    kind?: string | null;
    originalText: string;
    currentText: string;
};

export type ReviewFeedResult = {
    revision: number;
    pendingCount: number;
    changes: ReviewChangeEntry[];
};

export type AcceptReviewChangeResult = {
    changed: boolean;
    revision: number;
    patchKey: string;
};

export type RejectReviewChangeResult = AcceptReviewChangeResult;

export type ReviewBulkChangeResult = {
    changed: boolean;
    revision: number;
    affectedPatchCount: number;
};

// ─── Compact public surface ──────────────────────────────────────────────────

export type EditorWasmApi = {
    /** document.applyPatch */
    applyDocumentPatch: (patch: unknown) => void;
    /** document.buildRegionPatch */
    buildRegionTextPatch: (
        pageIndex: number,
        regionId: string,
        kind: string,
        originalText: string,
        newText: string,
    ) => unknown | null;
    /** document.applyRegionReplacements */
    applyRegionTextReplacements: (
        replacements: RegionTextReplaceRequest[],
        frameRequest: Record<string, unknown>,
    ) => RegionTextReplaceResult;
    /** document.requestRefresh */
    requestDocumentRefresh: (
        source: string,
        frameRequest: Record<string, unknown>,
    ) => DocumentRefreshResult;
    /** review.readFeed */
    getReviewFeed: () => ReviewFeedResult | null;
    /** review.accept */
    acceptReviewChange: (patchKey: string) => AcceptReviewChangeResult | null;
    /** review.reject */
    rejectReviewChange: (patchKey: string) => RejectReviewChangeResult | null;
    /** review.acceptAll */
    acceptAllReviewChanges: () => ReviewBulkChangeResult | null;
    /** review.rejectAll */
    rejectAllReviewChanges: () => ReviewBulkChangeResult | null;
    /** editor.saveSession */
    saveSession: (path: string, pageIndex: number) => Promise<unknown>;
};

function callMethod<T>(target: unknown, method: string, ...args: unknown[]): T {
    const targetRecord = target as Record<string, unknown> | null | undefined;
    const fn = targetRecord?.[method];
    if (typeof fn !== 'function') {
        throw new Error(`Required WASM method is unavailable: ${method}`);
    }
    return (fn as (...a: unknown[]) => T).apply(target, args);
}

export function createEditorWasmApi(getWasmApi: GetWasmApi): EditorWasmApi {
    return {
        applyDocumentPatch(patch: unknown): void {
            getDocumentSession(getWasmApi).applyPatch(patch);
        },

        buildRegionTextPatch(pageIndex, regionId, kind, originalText, newText) {
            return callMethod<unknown>(
                getDocumentSession(getWasmApi),
                'buildRegionPatch',
                pageIndex,
                regionId,
                kind,
                originalText,
                newText,
            );
        },

        applyRegionTextReplacements(replacements, frameRequest) {
            return callMethod<RegionTextReplaceResult>(
                getDocumentSession(getWasmApi),
                'applyRegionReplacements',
                replacements,
                frameRequest,
            );
        },

        requestDocumentRefresh(source, frameRequest) {
            return callMethod<DocumentRefreshResult>(
                getDocumentSession(getWasmApi),
                'requestRefresh',
                source,
                frameRequest,
            );
        },

        getReviewFeed() {
            return callMethod<ReviewFeedResult>(getReviewSession(getWasmApi), 'readFeed');
        },

        acceptReviewChange(patchKey) {
            return callMethod<AcceptReviewChangeResult>(
                getReviewSession(getWasmApi),
                'accept',
                patchKey,
            );
        },

        rejectReviewChange(patchKey) {
            return callMethod<RejectReviewChangeResult>(
                getReviewSession(getWasmApi),
                'reject',
                patchKey,
            );
        },

        acceptAllReviewChanges() {
            return callMethod<ReviewBulkChangeResult>(getReviewSession(getWasmApi), 'acceptAll');
        },

        rejectAllReviewChanges() {
            return callMethod<ReviewBulkChangeResult>(getReviewSession(getWasmApi), 'rejectAll');
        },

        async saveSession(path: string, pageIndex: number): Promise<unknown> {
            const { saveSession: save } = await import('./api');
            return save(path, pageIndex);
        },
    };
}

