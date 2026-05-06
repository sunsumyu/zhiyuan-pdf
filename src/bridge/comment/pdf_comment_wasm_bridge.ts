import type {
    CommentReviewScope,
    CommentReviewSession,
    PdfCommentOverlayDisplay,
    PdfCommentReviewDisplay,
    PdfCommentTargetOverlayDisplay,
    ViewerSessionSnapshot,
} from './pdf_comment_contracts';
import {
    normalizeOverlayDisplay,
    normalizeReviewDisplay,
    normalizeReviewSession,
    normalizeTargetOverlayDisplay,
} from './pdf_comment_contracts';

type CreatePdfCommentWasmBridgeDeps = {
    getViewerSession: () => ViewerSessionSnapshot;
    getWasmApi: () => any;
};

export type PdfCommentWasmBridge = {
    readReviewSession: () => CommentReviewSession;
    clearReviewSession: () => void;
    setReviewPanelOpenAndLoad: (panelOpen: boolean) => Promise<PdfCommentReviewDisplay | null>;
    toggleReviewPanelAndLoad: () => Promise<PdfCommentReviewDisplay | null>;
    setReviewScopeAndLoad: (scope: CommentReviewScope) => Promise<PdfCommentReviewDisplay | null>;
    setReviewQueryAndLoad: (query: string) => Promise<PdfCommentReviewDisplay | null>;
    selectReviewCommentAndLoad: (selectedCommentId: string | null) => Promise<PdfCommentReviewDisplay | null>;
    loadCommentOverlay: (path: string, currentPage: number) => Promise<PdfCommentOverlayDisplay>;
    loadCommentTargetOverlay: (path: string, currentPage: number) => Promise<PdfCommentTargetOverlayDisplay>;
    loadCommentReview: (path: string, currentPage: number) => Promise<PdfCommentReviewDisplay>;
    addRegionCommentRequest: (path: string, request: Record<string, unknown>) => Promise<void>;
    deletePageAnnotationRequest: (path: string, request: Record<string, unknown>) => Promise<void>;
    updatePageCommentRequest: (path: string, request: Record<string, unknown>) => Promise<void>;
};

export function createPdfCommentWasmBridge(
    deps: CreatePdfCommentWasmBridgeDeps,
): PdfCommentWasmBridge {
    function readReviewSession(): CommentReviewSession {
        const raw = deps.getWasmApi().reviewFacadeReadFeed() as CommentReviewSession | null | undefined;
        return normalizeReviewSession(raw);
    }

    async function withCurrentDocument<T>(
        loader: (path: string, currentPage: number) => Promise<T>,
    ): Promise<T | null> {
        const session = deps.getViewerSession();
        if (!session.path) {
            return null;
        }
        return await loader(session.path, session.currentPage);
    }

    return {
        readReviewSession,
        clearReviewSession: () => {
            deps.getWasmApi().clear_comment_review_session();
        },
        setReviewPanelOpenAndLoad: async (panelOpen) =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await deps.getWasmApi().set_comment_review_panel_open_and_load(
                        path,
                        currentPage,
                        panelOpen,
                    ),
                ),
            ),
        toggleReviewPanelAndLoad: async () =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await deps.getWasmApi().toggle_comment_review_panel_and_load(
                        path,
                        currentPage,
                    ),
                ),
            ),
        setReviewScopeAndLoad: async (scope) =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await deps.getWasmApi().set_comment_review_scope_and_load(
                        path,
                        currentPage,
                        scope,
                    ),
                ),
            ),
        setReviewQueryAndLoad: async (query) =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await deps.getWasmApi().set_comment_review_query_and_load(
                        path,
                        currentPage,
                        query,
                    ),
                ),
            ),
        selectReviewCommentAndLoad: async (selectedCommentId) =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await deps.getWasmApi().select_comment_review_and_load(
                        path,
                        currentPage,
                        selectedCommentId ?? null,
                    ),
                ),
            ),
        loadCommentOverlay: async (path, currentPage) =>
            normalizeOverlayDisplay(
                await deps.getWasmApi().load_comment_overlay(path, currentPage) as
                    | PdfCommentOverlayDisplay
                    | null
                    | undefined,
            ),
        loadCommentTargetOverlay: async (path, currentPage) =>
            normalizeTargetOverlayDisplay(
                await deps.getWasmApi().load_comment_target_overlay(path, currentPage) as
                    | PdfCommentTargetOverlayDisplay
                    | null
                    | undefined,
            ),
        loadCommentReview: async (path, currentPage) =>
            normalizeReviewDisplay(await deps.getWasmApi().load_comment_review(path, currentPage)),
        addRegionCommentRequest: async (path, request) => {
            await deps.getWasmApi().apply_comment(path, request);
        },
        deletePageAnnotationRequest: async (path, request) => {
            await deps.getWasmApi().delete_page_annotation(path, request);
        },
        updatePageCommentRequest: async (path, request) => {
            await deps.getWasmApi().apply_comment_update(path, request);
        },
    };
}





