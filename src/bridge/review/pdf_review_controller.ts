import type { DocumentEditApi } from '../document/document_edit_api';
import {
    getReviewFeed,
    acceptChange as acceptChangeFacade,
    rejectChange as rejectChangeFacade,
    acceptAllChanges as acceptAllChangesFacade,
    rejectAllChanges as rejectAllChangesFacade,
    locateChange as locateChangeFacade,
    type ReviewChangeEntry,
    type ReviewFeedResult,
    type ReviewLocateResult,
} from '../find/find_facade';
import { setToolbarButtonActive } from '../viewer/pdf_viewer_dom';

type ViewerSessionSnapshot = {
    path: string | null;
    currentPage: number;
};

type CreatePdfReviewControllerDeps = {
    getViewerSession: () => ViewerSessionSnapshot;
    documentEdits: DocumentEditApi;
    goToPage: (pageIndex: number) => Promise<void>;
    openRegionEditor: (
        pageIndex: number,
        regionId: string,
        kind: string,
        originalText: string,
    ) => Promise<void>;
};

type ReviewScope = 'page' | 'document';

type ReviewNodes = {
    toggle: HTMLButtonElement | null;
    panel: HTMLElement | null;
    meta: HTMLElement | null;
    scope: HTMLSelectElement | null;
    search: HTMLInputElement | null;
    bulkActions: HTMLElement | null;
    list: HTMLElement | null;
    empty: HTMLElement | null;
    close: HTMLButtonElement | null;
    acceptAll: HTMLButtonElement | null;
    rejectAll: HTMLButtonElement | null;
};

type ReviewUiState = {
    panelOpen: boolean;
    scope: ReviewScope;
    query: string;
    selectedPatchKey: string | null;
};

export type PdfReviewController = {
    initialize: () => void;
    togglePanel: () => Promise<void>;
    refresh: () => Promise<void>;
    clear: () => void;
};

const DEFAULT_STATE: ReviewUiState = {
    panelOpen: false,
    scope: 'page',
    query: '',
    selectedPatchKey: null,
};

function getNodes(): ReviewNodes {
    return {
        toggle: document.getElementById('pdf-review-btn') as HTMLButtonElement | null,
        panel: document.getElementById('pdf-review-panel') as HTMLElement | null,
        meta: document.getElementById('pdf-review-meta') as HTMLElement | null,
        scope: document.getElementById('pdf-review-scope') as HTMLSelectElement | null,
        search: document.getElementById('pdf-review-search') as HTMLInputElement | null,
        bulkActions: document.getElementById('pdf-review-bulk-actions') as HTMLElement | null,
        list: document.getElementById('pdf-review-list') as HTMLElement | null,
        empty: document.getElementById('pdf-review-empty') as HTMLElement | null,
        close: document.getElementById('pdf-review-close-btn') as HTMLButtonElement | null,
        acceptAll: document.getElementById('pdf-review-accept-all-btn') as HTMLButtonElement | null,
        rejectAll: document.getElementById('pdf-review-reject-all-btn') as HTMLButtonElement | null,
    };
}

function normalizeText(value: string | null | undefined): string {
    return (value ?? '').trim();
}

function matchesReviewQuery(entry: ReviewChangeEntry, query: string): boolean {
    if (!query) return true;
    const haystack = [
        entry.currentText,
        entry.originalText,
        entry.regionId,
        entry.source,
        entry.kind ?? '',
    ]
        .join('\n')
        .toLowerCase();
    return haystack.includes(query.toLowerCase());
}

function summarizeText(value: string, maxLength = 120): string {
    const compact = value.replace(/\s+/g, ' ').trim();
    if (compact.length <= maxLength) return compact;
    return `${compact.slice(0, Math.max(0, maxLength - 1))}…`;
}

function createBadge(label: string, accent: string): HTMLSpanElement {
    const badge = document.createElement('span');
    badge.textContent = label;
    badge.style.background = accent;
    badge.style.color = '#cdd6f4';
    badge.style.border = '1px solid rgba(137, 180, 250, 0.22)';
    badge.style.borderRadius = '999px';
    badge.style.padding = '3px 8px';
    badge.style.fontSize = '11px';
    return badge;
}

function computeVisibleChanges(
    feed: ReviewFeedResult | null,
    state: ReviewUiState,
    currentPage: number,
): ReviewChangeEntry[] {
    const all = feed?.changes ?? [];
    return all.filter((entry) => {
        if (state.scope === 'page' && entry.pageIndex !== currentPage) {
            return false;
        }
        return matchesReviewQuery(entry, state.query);
    });
}

export function createPdfReviewController(
    deps: CreatePdfReviewControllerDeps,
): PdfReviewController {
    let initialized = false;
    let busy = false;
    let state: ReviewUiState = { ...DEFAULT_STATE };

    function syncPanelVisibility(): void {
        const nodes = getNodes();
        if (nodes.toggle) {
            setToolbarButtonActive(nodes.toggle, state.panelOpen);
        }
        if (nodes.panel) {
            nodes.panel.style.display = state.panelOpen ? 'block' : 'none';
        }
        if (nodes.scope && nodes.scope.value !== state.scope) {
            nodes.scope.value = state.scope;
        }
        if (nodes.search && nodes.search.value !== state.query) {
            nodes.search.value = state.query;
        }
    }

    function clearView(emptyLabel = 'No pending review changes.'): void {
        const nodes = getNodes();
        if (nodes.meta) {
            nodes.meta.textContent = 'No review data loaded';
        }
        if (nodes.list) {
            nodes.list.innerHTML = '';
        }
        if (nodes.empty) {
            nodes.empty.textContent = emptyLabel;
            nodes.empty.style.display = state.panelOpen ? 'block' : 'none';
        }
    }

    async function locateChange(entry: ReviewChangeEntry): Promise<void> {
        if (busy) return;
        const locateResult = locateChangeFacade(entry.patchKey);
        if (!locateResult) return;
        
        if (deps.getViewerSession().currentPage !== locateResult.pageIndex) {
            await deps.goToPage(locateResult.pageIndex);
        }
        if (!locateResult.kind) return;
        await deps.openRegionEditor(
            locateResult.pageIndex,
            locateResult.regionId,
            locateResult.kind,
            locateResult.originalText,
        );
    }

    async function rejectChange(entry: ReviewChangeEntry): Promise<void> {
        if (busy) return;
        busy = true;
        try {
            const result = rejectChangeFacade(entry.patchKey);
            if (result?.changed) {
                if (state.selectedPatchKey === entry.patchKey) {
                    state = {
                        ...state,
                        selectedPatchKey: null,
                    };
                }
                await refresh();
            }
        } finally {
            busy = false;
        }
    }

    async function acceptChange(entry: ReviewChangeEntry): Promise<void> {
        if (busy) return;
        busy = true;
        try {
            const result = acceptChangeFacade(entry.patchKey);
            if (result?.changed) {
                if (state.selectedPatchKey === entry.patchKey) {
                    state = {
                        ...state,
                        selectedPatchKey: null,
                    };
                }
                await refresh();
            }
        } finally {
            busy = false;
        }
    }

    async function acceptAllChanges(): Promise<void> {
        if (busy) return;
        busy = true;
        try {
            const result = acceptAllChangesFacade();
            if (result?.changed) {
                state = {
                    ...state,
                    selectedPatchKey: null,
                };
                await refresh();
            }
        } finally {
            busy = false;
        }
    }

    async function rejectAllChanges(): Promise<void> {
        if (busy) return;
        busy = true;
        try {
            const result = rejectAllChangesFacade();
            if (result?.changed) {
                state = {
                    ...state,
                    selectedPatchKey: null,
                };
                await refresh();
            }
        } finally {
            busy = false;
        }
    }

    function renderFeed(feed: ReviewFeedResult | null): void {
        const nodes = getNodes();
        const session = deps.getViewerSession();
        const visibleChanges = computeVisibleChanges(feed, state, session.currentPage);
        const scopeLabel = state.scope === 'page'
            ? `Current Page · ${session.currentPage + 1}`
            : 'Whole PDF';

        if (nodes.meta) {
            nodes.meta.textContent = `${scopeLabel} · ${visibleChanges.length} shown / ${feed?.pendingCount ?? 0} pending · rev ${feed?.revision ?? 0}`;
        }
        if (nodes.bulkActions) {
            nodes.bulkActions.style.display = (feed?.pendingCount ?? 0) > 0 ? 'flex' : 'none';
        }
        if (nodes.acceptAll) {
            nodes.acceptAll.disabled = busy || (feed?.pendingCount ?? 0) === 0;
            nodes.acceptAll.style.cursor = nodes.acceptAll.disabled ? 'not-allowed' : 'pointer';
        }
        if (nodes.rejectAll) {
            nodes.rejectAll.disabled = busy || (feed?.pendingCount ?? 0) === 0;
            nodes.rejectAll.style.cursor = nodes.rejectAll.disabled ? 'not-allowed' : 'pointer';
        }
        if (nodes.list) {
            nodes.list.innerHTML = '';
        }
        if (nodes.empty) {
            nodes.empty.style.display = visibleChanges.length === 0 && state.panelOpen ? 'block' : 'none';
            nodes.empty.textContent = state.query
                ? 'No review changes match the current filter.'
                : 'No pending review changes.';
        }
        if (!nodes.list || visibleChanges.length === 0) {
            return;
        }

        for (const entry of visibleChanges) {
            const selected = state.selectedPatchKey === entry.patchKey;
            const card = document.createElement('article');
            card.style.border = selected
                ? '1px solid rgba(249, 226, 175, 0.55)'
                : '1px solid #313244';
            card.style.background = selected
                ? 'rgba(249, 226, 175, 0.08)'
                : 'rgba(24, 24, 37, 0.9)';
            card.style.borderRadius = '12px';
            card.style.padding = '12px';
            card.style.display = 'flex';
            card.style.flexDirection = 'column';
            card.style.gap = '10px';

            const header = document.createElement('div');
            header.style.display = 'flex';
            header.style.justifyContent = 'space-between';
            header.style.alignItems = 'flex-start';
            header.style.gap = '10px';

            const title = document.createElement('div');
            title.style.display = 'flex';
            title.style.flexDirection = 'column';
            title.style.gap = '6px';

            const heading = document.createElement('div');
            heading.textContent = summarizeText(entry.currentText || entry.originalText, 72);
            heading.style.fontSize = '13px';
            heading.style.fontWeight = '700';
            heading.style.color = '#f9e2af';

            const badges = document.createElement('div');
            badges.style.display = 'flex';
            badges.style.flexWrap = 'wrap';
            badges.style.gap = '6px';
            badges.appendChild(createBadge(`P${entry.pageIndex + 1}`, 'rgba(137, 180, 250, 0.16)'));
            badges.appendChild(createBadge(entry.source, 'rgba(166, 227, 161, 0.14)'));
            if (entry.kind) {
                badges.appendChild(createBadge(entry.kind, 'rgba(203, 166, 247, 0.14)'));
            }

            title.appendChild(heading);
            title.appendChild(badges);

            const patchKey = document.createElement('div');
            patchKey.textContent = entry.patchKey;
            patchKey.style.fontSize = '11px';
            patchKey.style.color = '#6c7086';
            patchKey.style.fontFamily = 'monospace';

            header.appendChild(title);
            header.appendChild(patchKey);

            const before = document.createElement('div');
            before.style.display = 'flex';
            before.style.flexDirection = 'column';
            before.style.gap = '4px';
            before.innerHTML = `<div style="font-size:11px;color:#9399b2;">Original</div>`;
            const beforeText = document.createElement('div');
            beforeText.textContent = summarizeText(entry.originalText, 180);
            beforeText.style.fontSize = '12px';
            beforeText.style.color = '#bac2de';
            beforeText.style.lineHeight = '1.6';
            beforeText.style.padding = '8px 10px';
            beforeText.style.borderRadius = '8px';
            beforeText.style.background = 'rgba(17, 17, 27, 0.72)';
            beforeText.style.border = '1px solid #313244';
            before.appendChild(beforeText);

            const current = document.createElement('div');
            current.style.display = 'flex';
            current.style.flexDirection = 'column';
            current.style.gap = '4px';
            current.innerHTML = `<div style="font-size:11px;color:#9399b2;">Current</div>`;
            const currentText = document.createElement('div');
            currentText.textContent = summarizeText(entry.currentText, 180);
            currentText.style.fontSize = '12px';
            currentText.style.color = '#cdd6f4';
            currentText.style.lineHeight = '1.6';
            currentText.style.padding = '8px 10px';
            currentText.style.borderRadius = '8px';
            currentText.style.background = 'rgba(30, 30, 46, 0.82)';
            currentText.style.border = '1px solid #313244';
            current.appendChild(currentText);

            const actions = document.createElement('div');
            actions.style.display = 'flex';
            actions.style.justifyContent = 'space-between';
            actions.style.alignItems = 'center';
            actions.style.gap = '8px';

            const helper = document.createElement('div');
            helper.textContent = entry.kind
                ? `Region ${entry.regionId}`
                : `Region ${entry.regionId} · locate unavailable`;
            helper.style.fontSize = '11px';
            helper.style.color = '#9399b2';

            const buttons = document.createElement('div');
            buttons.style.display = 'flex';
            buttons.style.gap = '8px';

            const locateButton = document.createElement('button');
            locateButton.type = 'button';
            locateButton.textContent = 'Locate';
            locateButton.disabled = !entry.kind || busy;
            locateButton.style.background = 'rgba(137, 180, 250, 0.14)';
            locateButton.style.color = '#89b4fa';
            locateButton.style.border = '1px solid rgba(137, 180, 250, 0.28)';
            locateButton.style.borderRadius = '8px';
            locateButton.style.padding = '6px 10px';
            locateButton.style.cursor = locateButton.disabled ? 'not-allowed' : 'pointer';
            locateButton.addEventListener('click', (event) => {
                event.stopPropagation();
                state = { ...state, selectedPatchKey: entry.patchKey };
                void locateChange(entry);
            });

            const acceptButton = document.createElement('button');
            acceptButton.type = 'button';
            acceptButton.textContent = 'Accept';
            acceptButton.disabled = busy;
            acceptButton.style.background = 'rgba(166, 227, 161, 0.14)';
            acceptButton.style.color = '#a6e3a1';
            acceptButton.style.border = '1px solid rgba(166, 227, 161, 0.26)';
            acceptButton.style.borderRadius = '8px';
            acceptButton.style.padding = '6px 10px';
            acceptButton.style.cursor = busy ? 'not-allowed' : 'pointer';
            acceptButton.addEventListener('click', (event) => {
                event.stopPropagation();
                void acceptChange(entry);
            });

            const rejectButton = document.createElement('button');
            rejectButton.type = 'button';
            rejectButton.textContent = 'Reject';
            rejectButton.disabled = busy;
            rejectButton.style.background = 'rgba(243, 139, 168, 0.14)';
            rejectButton.style.color = '#f38ba8';
            rejectButton.style.border = '1px solid rgba(243, 139, 168, 0.26)';
            rejectButton.style.borderRadius = '8px';
            rejectButton.style.padding = '6px 10px';
            rejectButton.style.cursor = busy ? 'not-allowed' : 'pointer';
            rejectButton.addEventListener('click', (event) => {
                event.stopPropagation();
                const shouldReject = window.confirm('拒绝这条待审变更？');
                if (shouldReject) {
                    void rejectChange(entry);
                }
            });

            buttons.appendChild(locateButton);
            buttons.appendChild(acceptButton);
            buttons.appendChild(rejectButton);

            actions.appendChild(helper);
            actions.appendChild(buttons);

            card.appendChild(header);
            card.appendChild(before);
            card.appendChild(current);
            card.appendChild(actions);
            card.addEventListener('click', () => {
                state = { ...state, selectedPatchKey: entry.patchKey };
                renderFeed(feed);
            });

            nodes.list.appendChild(card);
        }
    }

    async function refresh(): Promise<void> {
        syncPanelVisibility();
        if (!state.panelOpen) {
            return;
        }
        const session = deps.getViewerSession();
        if (!session.path) {
            clearView('Open a PDF to review pending changes.');
            return;
        }
        const feed = getReviewFeed();
        renderFeed(feed);
    }

    async function togglePanel(): Promise<void> {
        state = {
            ...state,
            panelOpen: !state.panelOpen,
        };
        syncPanelVisibility();
        await refresh();
    }

    function clear(): void {
        state = { ...DEFAULT_STATE };
        clearView();
        syncPanelVisibility();
    }

    function initialize(): void {
        if (initialized) return;
        initialized = true;
        const nodes = getNodes();
        nodes.scope?.addEventListener('change', () => {
            state = {
                ...state,
                scope: nodes.scope?.value === 'document' ? 'document' : 'page',
            };
            void refresh();
        });
        nodes.search?.addEventListener('input', () => {
            state = {
                ...state,
                query: normalizeText(nodes.search?.value),
            };
            void refresh();
        });
        nodes.search?.addEventListener('keydown', (event) => {
            if (event.key === 'Escape') {
                event.preventDefault();
                state = {
                    ...state,
                    panelOpen: false,
                };
                syncPanelVisibility();
            }
        });
        nodes.close?.addEventListener('click', () => {
            state = {
                ...state,
                panelOpen: false,
            };
            syncPanelVisibility();
        });
        nodes.acceptAll?.addEventListener('click', () => {
            const shouldAcceptAll = window.confirm('接受所有待审变更？');
            if (shouldAcceptAll) {
                void acceptAllChanges();
            }
        });
        nodes.rejectAll?.addEventListener('click', () => {
            const shouldRejectAll = window.confirm('拒绝所有待审变更？');
            if (shouldRejectAll) {
                void rejectAllChanges();
            }
        });
        syncPanelVisibility();
    }

    return {
        initialize,
        togglePanel,
        refresh,
        clear,
    };
}
