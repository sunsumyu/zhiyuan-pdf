import type { DocumentEditApi } from '../document/document_edit_api';
import type {
    PdfCommentReviewCardAction,
    PdfPageAnnotationTarget,
    PdfPageCommentItem,
    ViewerSessionSnapshot,
} from './pdf_comment_contracts';
import type { PdfCommentWasmBridge } from './pdf_comment_wasm_bridge';

type CreatePdfCommentHostActionsDeps = {
    getViewerSession: () => ViewerSessionSnapshot;
    documentEdits: DocumentEditApi;
    bridge: PdfCommentWasmBridge;
    goToPage: (pageIndex: number) => Promise<void>;
    readBusy: () => boolean;
    setBusy: (busy: boolean) => void;
    markNeedsReload: () => void;
    refreshController: () => Promise<void>;
    renderOverlayFromDisplay: (displayOverlay: { comments: any[] } | null | undefined) => void;
    scrollToCommentMarker: (commentId: string) => Promise<void>;
};

export type PdfCommentHostActions = {
    deleteComment: (comment: PdfPageCommentItem) => Promise<void>;
    editComment: (comment: PdfPageCommentItem) => Promise<void>;
    addRegionComment: (target: PdfPageAnnotationTarget) => Promise<void>;
    focusComment: (comment: PdfPageCommentItem) => Promise<void>;
    handleReviewCardAction: (action: PdfCommentReviewCardAction, comment: PdfPageCommentItem) => void;
};

const DEFAULT_COMMENT_COLOR: [number, number, number] = [0.42, 0.73, 0.98];

export function createPdfCommentHostActions(
    deps: CreatePdfCommentHostActionsDeps,
): PdfCommentHostActions {
    async function deleteComment(comment: PdfPageCommentItem): Promise<void> {
        if (deps.readBusy()) return;
        const session = deps.getViewerSession();
        if (!session.path) return;
        deps.setBusy(true);
        try {
            deps.markNeedsReload();
            if (deps.bridge.readReviewSession().selectedCommentId === comment.id) {
                await deps.bridge.selectReviewCommentAndLoad(null);
            }
            await deps.bridge.deletePageAnnotationRequest(session.path, {
                pageIndex: comment.pageIndex,
                annotationId: comment.id,
            });
            await deps.documentEdits.refreshDocument('comment');
            await deps.refreshController();
        } finally {
            deps.setBusy(false);
        }
    }

    async function editComment(comment: PdfPageCommentItem): Promise<void> {
        if (deps.readBusy()) return;
        const session = deps.getViewerSession();
        if (!session.path) return;
                const nextContents = window.prompt('编辑批注内容：', comment.contents);
        if (nextContents == null) return;
        if (!nextContents.trim()) {
            window.alert('批注内容不能为空');
            return;
        }
        deps.setBusy(true);
        try {
            deps.markNeedsReload();
            await deps.bridge.updatePageCommentRequest(session.path, {
                pageIndex: comment.pageIndex,
                annotationId: comment.id,
                contents: nextContents.trim(),
            });
            await deps.documentEdits.refreshDocument('comment');
            await deps.refreshController();
        } finally {
            deps.setBusy(false);
        }
    }

    async function addRegionComment(target: PdfPageAnnotationTarget): Promise<void> {
        if (deps.readBusy()) return;
        const session = deps.getViewerSession();
        if (!session.path) return;
        const contents = window.prompt(`为“${target.label}”添加批注：`, '');
        if (contents == null || !contents.trim()) return;
        deps.setBusy(true);
        try {
            deps.markNeedsReload();
            await deps.bridge.addRegionCommentRequest(session.path, {
                pageIndex: target.pageIndex,
                regionId: target.id,
                kind: target.kind,
                contents: contents.trim(),
                color: DEFAULT_COMMENT_COLOR,
            });
            await deps.documentEdits.refreshDocument('comment');
            await deps.refreshController();
        } finally {
            deps.setBusy(false);
        }
    }

    async function focusComment(comment: PdfPageCommentItem): Promise<void> {
        const display = await deps.bridge.selectReviewCommentAndLoad(comment.id);
        if (deps.getViewerSession().currentPage !== comment.pageIndex) {
            await deps.goToPage(comment.pageIndex);
        } else {
            deps.renderOverlayFromDisplay(display?.overlay);
        }
        await deps.scrollToCommentMarker(comment.id);
    }

    function handleReviewCardAction(action: PdfCommentReviewCardAction, comment: PdfPageCommentItem): void {
        if (action.id === 'edit') {
            void editComment(comment);
            return;
        }
        if (action.id === 'delete') {
            const shouldDelete = window.confirm('删除这条批注');
            if (shouldDelete) {
                void deleteComment(comment);
            }
            return;
        }
        void focusComment(comment);
    }

    return {
        deleteComment,
        editComment,
        addRegionComment,
        focusComment,
        handleReviewCardAction,
    };
}
