import type { CommentReviewSession } from './pdf_comment_contracts';
import { setToolbarButtonActive } from '../viewer/pdf_viewer_dom';

export type PdfCommentDomNodes = {
    toggle: HTMLButtonElement | null;
    overlay: HTMLElement | null;
    targets: HTMLElement | null;
    reviewToggle: HTMLButtonElement | null;
    reviewPanel: HTMLElement | null;
    reviewMeta: HTMLElement | null;
    reviewScope: HTMLSelectElement | null;
    reviewSearch: HTMLInputElement | null;
    reviewSummary: HTMLElement | null;
    reviewList: HTMLElement | null;
    reviewEmpty: HTMLElement | null;
    reviewClose: HTMLButtonElement | null;
};

export function getPdfCommentDomNodes(): PdfCommentDomNodes {
    return {
        toggle: document.getElementById('pdf-comment-btn') as HTMLButtonElement | null,
        overlay: document.getElementById('pdf-comment-overlay') as HTMLElement | null,
        targets: document.getElementById('pdf-comment-target-overlay') as HTMLElement | null,
        reviewToggle: document.getElementById('pdf-comment-review-btn') as HTMLButtonElement | null,
        reviewPanel: document.getElementById('pdf-comment-review-panel') as HTMLElement | null,
        reviewMeta: document.getElementById('pdf-comment-review-meta') as HTMLElement | null,
        reviewScope: document.getElementById('pdf-comment-review-scope') as HTMLSelectElement | null,
        reviewSearch: document.getElementById('pdf-comment-review-search') as HTMLInputElement | null,
        reviewSummary: document.getElementById('pdf-comment-review-summary') as HTMLElement | null,
        reviewList: document.getElementById('pdf-comment-review-list') as HTMLElement | null,
        reviewEmpty: document.getElementById('pdf-comment-review-empty') as HTMLElement | null,
        reviewClose: document.getElementById('pdf-comment-review-close-btn') as HTMLButtonElement | null,
    };
}

export function syncPdfCommentDomState(
    nodes: PdfCommentDomNodes,
    enabled: boolean,
    session: CommentReviewSession,
): void {
    if (nodes.toggle) {
        setToolbarButtonActive(nodes.toggle, enabled);
    }
    if (nodes.reviewToggle) {
        setToolbarButtonActive(nodes.reviewToggle, session.panelOpen);
    }
    if (nodes.reviewPanel) {
        nodes.reviewPanel.style.display = session.panelOpen ? 'block' : 'none';
    }
    if (nodes.reviewScope) {
        nodes.reviewScope.value = session.scope;
    }
    if (nodes.reviewSearch && nodes.reviewSearch.value !== session.query) {
        nodes.reviewSearch.value = session.query;
    }
}

export function clearPdfCommentLayerContainers(nodes: PdfCommentDomNodes): void {
    if (nodes.overlay) {
        nodes.overlay.innerHTML = '';
    }
    if (nodes.targets) {
        nodes.targets.innerHTML = '';
        nodes.targets.style.pointerEvents = 'none';
    }
}

