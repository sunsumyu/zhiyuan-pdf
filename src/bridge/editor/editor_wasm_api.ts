// ─────────────────────────────────────────────────────────────────────────────
// editor_wasm_api.ts — thin adapter over the canonical facades.
//
// Phase 2D migration: this file no longer calls raw `wasm_api/editor.rs`
// snake_case bindings. All operations are forwarded to the frozen facades:
//   • document.* (`@/.../document/facade.rs`)
//   • editor.*   (`@/.../editor/facade.rs`)
//   • review.*   (`@/.../review/facade.rs`)
//
// Only the methods actually consumed by `document_edit_api.ts` are exported.
// Type aliases (`EditorFormatAction`, `RegionTextReplace*`, `Review*`) are kept
// here because they are also imported by `editor_host.ts` / `pdf_viewer_api.ts`.
// ─────────────────────────────────────────────────────────────────────────────

import type { RustRenderFrame } from '../render/frame_plan';

type GetWasmApi = () => any;

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

export type ActiveEditorFormatState = {
    bold: boolean;
    italic: boolean;
    underline: boolean;
    color: string;
    fontFamily?: string;
    fontSize?: number;
    charSpacing?: number;
    lineHeight?: number;
    paragraphMode?: string;
    alignment?: string;
    listKind?: string;
    changed?: boolean;
};

export type EditorFormatAction =
    | { type: 'toggleBold' }
    | { type: 'toggleItalic' }
    | { type: 'toggleUnderline' }
    | { type: 'increaseFontSize' }
    | { type: 'decreaseFontSize' }
    | { type: 'setParagraphMode'; mode: string }
    | { type: 'setColor'; color: string }
    | { type: 'setFontFamily'; fontFamily: string }
    | { type: 'setFontSize'; fontSize: number }
    | { type: 'setCharSpacing'; charSpacing: number }
    | { type: 'setLineHeight'; lineHeight: number }
    | { type: 'setAlignment'; alignment: string }
    | { type: 'setListKind'; listKind: string };

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
    ) => RegionTextReplaceResult | null;
    /** document.requestRefresh */
    requestDocumentRefresh: (
        source: string,
        frameRequest: Record<string, unknown>,
    ) => DocumentRefreshResult | null;
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

function safeCall<T>(fn: ((...args: any[]) => T) | undefined, ...args: unknown[]): T | null {
    if (typeof fn !== 'function') return null;
    try {
        return fn(...args);
    } catch {
        return null;
    }
}

export function createEditorWasmApi(getWasmApi: GetWasmApi): EditorWasmApi {
    return {
        applyDocumentPatch(patch: unknown): void {
            getWasmApi().documentFacadeApplyPatch?.(patch);
        },

        buildRegionTextPatch(pageIndex, regionId, kind, originalText, newText) {
            return safeCall<unknown>(
                getWasmApi().documentFacadeBuildRegionPatch,
                pageIndex,
                regionId,
                kind,
                originalText,
                newText,
            );
        },

        applyRegionTextReplacements(replacements, frameRequest) {
            return safeCall<RegionTextReplaceResult>(
                getWasmApi().documentFacadeApplyRegionReplacements,
                replacements,
                frameRequest,
            );
        },

        requestDocumentRefresh(source, frameRequest) {
            return safeCall<DocumentRefreshResult>(
                getWasmApi().documentFacadeRequestRefresh,
                source,
                frameRequest,
            );
        },

        getReviewFeed() {
            return safeCall<ReviewFeedResult>(getWasmApi().reviewFacadeReadFeed);
        },

        acceptReviewChange(patchKey) {
            return safeCall<AcceptReviewChangeResult>(getWasmApi().reviewFacadeAccept, patchKey);
        },

        rejectReviewChange(patchKey) {
            return safeCall<RejectReviewChangeResult>(getWasmApi().reviewFacadeReject, patchKey);
        },

        acceptAllReviewChanges() {
            return safeCall<ReviewBulkChangeResult>(getWasmApi().reviewFacadeAcceptAll);
        },

        rejectAllReviewChanges() {
            return safeCall<ReviewBulkChangeResult>(getWasmApi().reviewFacadeRejectAll);
        },

        async saveSession(path: string, pageIndex: number): Promise<unknown> {
            const fn = getWasmApi().editorFacadeSaveSession;
            if (typeof fn !== 'function') return null;
            try {
                return await fn(path, pageIndex);
            } catch {
                return null;
            }
        },
    };
}

