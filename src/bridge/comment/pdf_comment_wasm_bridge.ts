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
import type { WasmModule } from '../shared/wasm_loader';
import type { CommentManager, ReviewSession } from '../../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui';

type CreatePdfCommentWasmBridgeDeps = {
    getViewerSession: () => ViewerSessionSnapshot;
    getWasmApi: () => WasmModule;
};

// ── Session singletons ─────────────────────────────────────────

let _commentManager: CommentManager | null = null;
let _reviewSession: ReviewSession | null = null;

function getCommentManager(getWasmApi: () => WasmModule): CommentManager | null {
    if (!_commentManager) {
        const api = getWasmApi();
        if (typeof api?.CommentManager === 'function') {
            _commentManager = new api.CommentManager();
        }
    }
    return _commentManager;
}

function getReviewSession(getWasmApi: () => WasmModule): ReviewSession | null {
    if (!_reviewSession) {
        const api = getWasmApi();
        if (typeof api?.ReviewSession === 'function') {
            _reviewSession = new api.ReviewSession();
        }
    }
    return _reviewSession;
}

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
        const raw = getReviewSession(deps.getWasmApi)?.readFeed() as
            | CommentReviewSession
            | null
            | undefined;
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

    const cm = () => getCommentManager(deps.getWasmApi);

    return {
        readReviewSession,
        clearReviewSession: () => {
            cm()?.clearReviewSession();
        },
        setReviewPanelOpenAndLoad: async (panelOpen) =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await cm()?.setPanelOpenAndLoad(path, currentPage, panelOpen),
                ),
            ),
        toggleReviewPanelAndLoad: async () =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await cm()?.togglePanelAndLoad(path, currentPage),
                ),
            ),
        setReviewScopeAndLoad: async (scope) =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await cm()?.setScopeAndLoad(path, currentPage, scope),
                ),
            ),
        setReviewQueryAndLoad: async (query) =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await cm()?.setQueryAndLoad(path, currentPage, query),
                ),
            ),
        selectReviewCommentAndLoad: async (selectedCommentId) =>
            await withCurrentDocument(async (path, currentPage) =>
                normalizeReviewDisplay(
                    await cm()?.selectAndLoad(path, currentPage, selectedCommentId ?? null),
                ),
            ),
        loadCommentOverlay: async (path, currentPage) =>
            normalizeOverlayDisplay(
                (await cm()?.loadOverlay(path, currentPage)) as
                    | PdfCommentOverlayDisplay
                    | null
                    | undefined,
            ),
        loadCommentTargetOverlay: async (path, currentPage) =>
            normalizeTargetOverlayDisplay(
                (await cm()?.loadTargetOverlay(path, currentPage)) as
                    | PdfCommentTargetOverlayDisplay
                    | null
                    | undefined,
            ),
        loadCommentReview: async (path, currentPage) =>
            normalizeReviewDisplay(await cm()?.loadReview(path, currentPage)),
        addRegionCommentRequest: async (path, request) => {
            await cm()?.addRegionComment(path, request);
        },
        deletePageAnnotationRequest: async (path, request) => {
            await cm()?.deleteAnnotation(path, request);
        },
        updatePageCommentRequest: async (path, request) => {
            await cm()?.updateComment(path, request);
        },
    };
}





