import type { DocumentEditApi } from '../document/document_edit_api';
import type {
    CommentReviewScope,
    PdfCommentOverlayDisplay,
    PdfCommentReviewDisplay,
    PdfCommentTargetOverlayDisplay,
    ViewerSessionSnapshot,
} from './pdf_comment_contracts';
import { EMPTY_OVERLAY_DISPLAY } from './pdf_comment_contracts';
import {
    clearPdfCommentLayerContainers,
    getPdfCommentDomNodes,
    syncPdfCommentDomState,
} from './pdf_comment_dom';
import { createPdfCommentHostActions } from './pdf_comment_host_actions';
import { renderCommentOverlay, renderCommentTargetOverlay } from './pdf_comment_overlay_view';
import { clearCommentReviewView, renderCommentReviewView } from './pdf_comment_review_view';
import { createPdfCommentWasmBridge } from './pdf_comment_wasm_bridge';

type CreatePdfCommentControllerDeps = {
    getViewerSession: () => ViewerSessionSnapshot;
    getWasmApi: () => any;
    documentEdits: DocumentEditApi;
    goToPage: (pageIndex: number) => Promise<void>;
};

export type PdfCommentController = {
    initialize: () => void;
    toggle: () => Promise<void>;
    refresh: () => Promise<void>;
    clear: () => void;
    togglePanel: () => Promise<void>;
};

function readReviewScope(): CommentReviewScope {
    return getPdfCommentDomNodes().reviewScope?.value === 'document' ? 'document' : 'page';
}

function escapeCssSelector(value: string): string {
    try {
        if (typeof CSS !== 'undefined' && typeof CSS.escape === 'function') {
            return CSS.escape(value);
        }
    } catch {
    }
    return value.replace(/["\\]/g, '\\$&');
}

function waitForAnimationFrame(): Promise<void> {
    return new Promise((resolve) => {
        window.requestAnimationFrame(() => resolve());
    });
}

async function scrollToCommentMarker(commentId: string): Promise<void> {
    await waitForAnimationFrame();
    await waitForAnimationFrame();
    const selector = `[data-comment-id="${escapeCssSelector(commentId)}"]`;
    const target = document.querySelector(selector) as HTMLElement | null;
    if (target) {
        target.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'smooth' });
    }
}

export function createPdfCommentController(
    deps: CreatePdfCommentControllerDeps,
): PdfCommentController {
    let initialized = false;
    let enabled = false;
    let busy = false;
    let lastLoadedKey: string | null = null;
    let needsReload = true;
    const bridge = createPdfCommentWasmBridge({
        getViewerSession: deps.getViewerSession,
        getWasmApi: deps.getWasmApi,
    });
    const hostActions = createPdfCommentHostActions({
        getViewerSession: deps.getViewerSession,
        documentEdits: deps.documentEdits,
        bridge,
        goToPage: deps.goToPage,
        readBusy: () => busy,
        setBusy: (nextBusy) => {
            busy = nextBusy;
        },
        markNeedsReload: () => {
            needsReload = true;
        },
        refreshController: async () => {
            await refresh();
        },
        renderOverlayFromDisplay: (overlay) => {
            renderPersistedComments(overlay ?? EMPTY_OVERLAY_DISPLAY);
        },
        scrollToCommentMarker,
    });

    function syncButtonState(): void {
        syncPdfCommentDomState(getPdfCommentDomNodes(), enabled, bridge.readReviewSession());
    }

    function clearReviewView(): void {
        const nodes = getPdfCommentDomNodes();
        clearCommentReviewView(
            {
                meta: nodes.reviewMeta,
                summary: nodes.reviewSummary,
                list: nodes.reviewList,
                empty: nodes.reviewEmpty,
            },
            bridge.readReviewSession().panelOpen,
        );
    }

    function clear(): void {
        clearPdfCommentLayerContainers(getPdfCommentDomNodes());
        enabled = false;
        busy = false;
        lastLoadedKey = null;
        needsReload = false;
        bridge.clearReviewSession();
        clearReviewView();
        syncButtonState();
    }

    function renderPersistedComments(overlay: PdfCommentOverlayDisplay): void {
        const nodes = getPdfCommentDomNodes();
        if (!nodes.overlay) return;
        renderCommentOverlay(nodes.overlay, overlay, (commentId) => {
            void (async () => {
                await bridge.selectReviewCommentAndLoad(commentId);
                const display = await bridge.setReviewPanelOpenAndLoad(true);
                syncButtonState();
                renderPersistedComments(display?.overlay ?? EMPTY_OVERLAY_DISPLAY);
                if (display) {
                    renderReviewList(display);
                }
            })();
        });
    }

    function renderCommentTargets(targetDisplay: PdfCommentTargetOverlayDisplay): void {
        const nodes = getPdfCommentDomNodes();
        if (!nodes.targets) return;
        renderCommentTargetOverlay(nodes.targets, enabled, targetDisplay, (targetId) => {
            const target = targetDisplay.targets.find((entry) => entry.id === targetId);
            if (!target) {
                return;
            }
            void hostActions.addRegionComment({
                id: target.id,
                kind: target.kind,
                pageIndex: target.pageIndex,
                label: target.label,
            });
        });
    }

    async function fetchReview(): Promise<PdfCommentReviewDisplay | null> {
        const session = deps.getViewerSession();
        if (!session.path) {
            return null;
        }
        return await bridge.loadCommentReview(session.path, session.currentPage);
    }

    function renderReviewList(display: PdfCommentReviewDisplay): void {
        const nodes = getPdfCommentDomNodes();
        renderCommentReviewView(
            {
                meta: nodes.reviewMeta,
                summary: nodes.reviewSummary,
                list: nodes.reviewList,
                empty: nodes.reviewEmpty,
            },
            display,
            {
                onSummaryChipClick: (pageIndex) => {
                    void (async () => {
                        await bridge.setReviewScopeAndLoad('page');
                        syncButtonState();
                        await deps.goToPage(pageIndex);
                    })();
                },
                onCardClick: (commentId) => {
                    const comment = display.review.comments.find((item) => item.id === commentId);
                    if (!comment) {
                        return;
                    }
                    void (async () => {
                        const nextDisplay = await bridge.selectReviewCommentAndLoad(comment.id);
                        renderPersistedComments(nextDisplay?.overlay ?? EMPTY_OVERLAY_DISPLAY);
                        if (nextDisplay) {
                            renderReviewList(nextDisplay);
                        }
                    })();
                },
                onActionClick: (action, commentId) => {
                    const comment = display.review.comments.find((item) => item.id === commentId);
                    if (!comment) {
                        return;
                    }
                    hostActions.handleReviewCardAction(action, comment);
                },
            },
        );
    }

    async function refreshReviewPanel(): Promise<void> {
        syncButtonState();
        if (!bridge.readReviewSession().panelOpen) {
            return;
        }
        const display = await fetchReview();
        if (!display) {
            clearReviewView();
            return;
        }
        renderReviewList(display);
    }

    async function refresh(): Promise<void> {
        const session = deps.getViewerSession();
        const nodes = getPdfCommentDomNodes();
        if (!session.path) {
            if (nodes.overlay) nodes.overlay.innerHTML = '';
            if (nodes.targets) nodes.targets.innerHTML = '';
            clearReviewView();
            syncButtonState();
            return;
        }

        const nextKey = `${session.path}::${session.currentPage}::${enabled ? 'on' : 'off'}`;
        if (!needsReload && lastLoadedKey === nextKey) {
            await refreshReviewPanel();
            syncButtonState();
            return;
        }

        const overlay = await bridge.loadCommentOverlay(session.path, session.currentPage);
        renderPersistedComments(overlay);

        if (enabled) {
            const targetDisplay = await bridge.loadCommentTargetOverlay(session.path, session.currentPage);
            renderCommentTargets(targetDisplay);
        } else if (nodes.targets) {
            nodes.targets.innerHTML = '';
            nodes.targets.style.pointerEvents = 'none';
        }

        lastLoadedKey = nextKey;
        needsReload = false;
        await refreshReviewPanel();
        syncButtonState();
    }

    async function toggle(): Promise<void> {
        enabled = !enabled;
        needsReload = true;
        await refresh();
    }

    async function togglePanel(): Promise<void> {
        const nextDisplay = await bridge.toggleReviewPanelAndLoad();
        const next = nextDisplay?.session ?? bridge.readReviewSession();
        syncButtonState();
        if (next.panelOpen) {
            if (nextDisplay) {
                renderReviewList(nextDisplay);
            } else {
                await refreshReviewPanel();
            }
        }
    }

    function initialize(): void {
        if (initialized) return;
        initialized = true;
        const nodes = getPdfCommentDomNodes();
        nodes.reviewScope?.addEventListener('change', () => {
            void (async () => {
                const display = await bridge.setReviewScopeAndLoad(readReviewScope());
                syncButtonState();
                if (display) {
                    renderReviewList(display);
                } else {
                    await refreshReviewPanel();
                }
            })();
        });
        nodes.reviewSearch?.addEventListener('input', () => {
            void (async () => {
                const display = await bridge.setReviewQueryAndLoad(nodes.reviewSearch?.value ?? '');
                syncButtonState();
                if (display) {
                    renderReviewList(display);
                } else {
                    await refreshReviewPanel();
                }
            })();
        });
        nodes.reviewSearch?.addEventListener('keydown', (event) => {
            if (event.key === 'Escape') {
                event.preventDefault();
                void (async () => {
                    await bridge.setReviewPanelOpenAndLoad(false);
                    syncButtonState();
                })();
            }
        });
        nodes.reviewClose?.addEventListener('click', () => {
            void (async () => {
                await bridge.setReviewPanelOpenAndLoad(false);
                syncButtonState();
            })();
        });
        syncButtonState();
    }

    return {
        initialize,
        toggle,
        refresh,
        clear,
        togglePanel,
    };
}

