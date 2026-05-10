// ─────────────────────────────────────────────────────────────────────────────
// PDF Find Controller — thin DOM shell.
// All state management and search orchestration lives in WASM (find::controller).
// This file only handles: DOM element access, event binding, overlay rendering.
// ─────────────────────────────────────────────────────────────────────────────

import type { DocumentEditApi, PdfRegionTextReplace } from '../document/document_edit_api';
import {
    findInPageAsync,
    findInDocumentAsync,
    replaceOne,
    replaceAll as replaceAllFacade,
    type SearchResult,
    type SearchMatch,
} from './find_facade';
import { setToolbarButtonActive } from '../viewer/pdf_viewer_dom';

type ViewerSessionSnapshot = {
    path: string | null;
    currentPage: number;
    pageCount: number;
};

type FindScope = 'page' | 'document';

type CreatePdfFindControllerDeps = {
    getViewerSession: () => ViewerSessionSnapshot;
    getWasmApi: () => any;
    getScrollContainer: () => HTMLElement | null;
    goToPage: (pageIndex: number) => Promise<void>;
    documentEdits: DocumentEditApi;
    openRegionEditor: (
        pageIndex: number,
        regionId: string,
        kind: string,
        originalText: string,
    ) => Promise<void>;
};

export type PdfFindController = {
    initialize: () => void;
    toggle: () => void;
    open: () => void;
    close: () => void;
    refresh: () => Promise<void>;
    clear: () => void;
    focusInput: () => void;
    next: () => Promise<void>;
    prev: () => Promise<void>;
    replaceCurrent: () => Promise<void>;
    replaceAll: () => Promise<void>;
};

// ─── FindSession bridge (P2 of session-API plan) ─────────────────────────────
//
// Migrated 2025: the legacy `findController*` flat WASM exports are now thin
// wrappers around `FindSession` (see crates/pdf-viewer-ui/src/find/find_api.rs).
// Constructing a single `FindSession` per page is fine — it is a zero-sized
// handle, all state lives in the wasm `HOST_FIND_SESSION` thread_local.

type FindStateUpdate = {
    state: {
        isOpen: boolean;
        query: string;
        scope: string;
        activeIndex: number;
        totalMatches: number;
        matches: SearchMatch[];
        currentPage: number;
    };
    currentPageMatches: CurrentPageMatch[];
    navigateToPage: number | null;
};

type CurrentPageMatch = {
    globalIndex: number;
    isActive: boolean;
    isEditable: boolean;
    boxRect: { left: number; top: number; width: number; height: number };
    pageWidth: number;
    pageHeight: number;
    previewText: string;
    id: string;
    kind: string;
    sourceText: string;
};

type FindToolbarState = {
    isOpen: boolean;
    countText: string;
    hasMatches: boolean;
    canReplaceCurrent: boolean;
    canReplaceAll: boolean;
};

let _findSession: any = null;

function getFindSession(getWasmApi: () => any): any {
    if (!_findSession) {
        const api = getWasmApi() as any;
        if (typeof api?.FindSession === 'function') {
            _findSession = new api.FindSession();
        }
    }
    return _findSession;
}

function callSession<T>(target: any, method: string, ...args: unknown[]): T | null {
    const fn = target?.[method];
    if (typeof fn !== 'function') return null;
    try {
        return fn.apply(target, args) as T;
    } catch {
        return null;
    }
}

// ─── DOM Nodes ───────────────────────────────────────────────────────────────

type FindNodes = {
    bar: HTMLElement | null;
    input: HTMLInputElement | null;
    scope: HTMLSelectElement | null;
    count: HTMLElement | null;
    prev: HTMLButtonElement | null;
    next: HTMLButtonElement | null;
    close: HTMLButtonElement | null;
    toggle: HTMLButtonElement | null;
    replaceInput: HTMLInputElement | null;
    replaceCurrent: HTMLButtonElement | null;
    replaceAll: HTMLButtonElement | null;
    overlay: HTMLElement | null;
};

function getNodes(): FindNodes {
    return {
        bar: document.getElementById('pdf-find-bar'),
        input: document.getElementById('pdf-find-input') as HTMLInputElement | null,
        scope: document.getElementById('pdf-find-scope') as HTMLSelectElement | null,
        count: document.getElementById('pdf-find-count'),
        prev: document.getElementById('pdf-find-prev-btn') as HTMLButtonElement | null,
        next: document.getElementById('pdf-find-next-btn') as HTMLButtonElement | null,
        close: document.getElementById('pdf-find-close-btn') as HTMLButtonElement | null,
        toggle: document.getElementById('pdf-find-toggle-btn') as HTMLButtonElement | null,
        replaceInput: document.getElementById('pdf-find-replace-input') as HTMLInputElement | null,
        replaceCurrent: document.getElementById('pdf-find-replace-btn') as HTMLButtonElement | null,
        replaceAll: document.getElementById('pdf-find-replace-all-btn') as HTMLButtonElement | null,
        overlay: document.getElementById('pdf-search-overlay'),
    };
}

// ─── Controller ──────────────────────────────────────────────────────────────

export function createPdfFindController(deps: CreatePdfFindControllerDeps): PdfFindController {
    let initialized = false;
    let searchTimerId: number | null = null;

    function findSession(): any { return getFindSession(deps.getWasmApi); }
    function readScope(): FindScope {
        return getNodes().scope?.value === 'document' ? 'document' : 'page';
    }

    // ─── Render (DOM only) ───────────────────────────────────────────────────

    function renderToolbarFromWasm(): void {
        const toolbar = callSession<FindToolbarState>(findSession(), 'getToolbarState');
        if (!toolbar) return;
        const nodes = getNodes();
        if (nodes.bar) nodes.bar.style.display = toolbar.isOpen ? 'flex' : 'none';
        if (nodes.toggle) setToolbarButtonActive(nodes.toggle, toolbar.isOpen);
        if (nodes.count) nodes.count.textContent = toolbar.countText;
        if (nodes.prev) nodes.prev.disabled = !toolbar.hasMatches;
        if (nodes.next) nodes.next.disabled = !toolbar.hasMatches;
        if (nodes.replaceCurrent) nodes.replaceCurrent.disabled = !toolbar.canReplaceCurrent;
        if (nodes.replaceAll) nodes.replaceAll.disabled = !toolbar.canReplaceAll;
        if (nodes.replaceInput) {
            nodes.replaceInput.disabled = false;
            nodes.replaceInput.placeholder = readScope() === 'page'
                ? 'Replace on current page...'
                : 'Replace across whole PDF...';
        }
    }

    function renderOverlayFromUpdate(update: FindStateUpdate): void {
        const nodes = getNodes();
        if (!nodes.overlay) return;
        nodes.overlay.innerHTML = '';

        for (const m of update.currentPageMatches) {
            const pageWidth = Math.max(1, m.pageWidth || 1);
            const pageHeight = Math.max(1, m.pageHeight || 1);
            const el = document.createElement('div');
            el.dataset.searchMatch = '1';
            el.dataset.matchIndex = String(m.globalIndex);
            el.style.position = 'absolute';
            el.style.pointerEvents = 'auto';
            el.style.cursor = m.isEditable ? 'text' : 'default';
            el.style.left = `${(m.boxRect.left / pageWidth) * 100}%`;
            el.style.top = `${(m.boxRect.top / pageHeight) * 100}%`;
            el.style.width = `${(m.boxRect.width / pageWidth) * 100}%`;
            el.style.height = `${(m.boxRect.height / pageHeight) * 100}%`;
            el.style.borderRadius = '6px';
            el.style.boxSizing = 'border-box';
            if (m.isActive) {
                el.style.border = '2px solid rgba(249, 226, 175, 0.98)';
                el.style.background = 'rgba(249, 226, 175, 0.24)';
                el.style.boxShadow = '0 0 0 1px rgba(249, 226, 175, 0.25), 0 0 14px rgba(249, 226, 175, 0.35)';
            } else {
                el.style.border = '1px solid rgba(137, 180, 250, 0.95)';
                el.style.background = 'rgba(137, 180, 250, 0.18)';
                el.style.boxShadow = 'none';
            }
            el.title = m.previewText;
            if (m.isEditable) {
                el.addEventListener('click', (event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    void deps.openRegionEditor(m.globalIndex >= 0 ? update.state.matches[m.globalIndex]?.pageIndex ?? 0 : 0, m.id, m.kind, m.sourceText);
                });
            }
            nodes.overlay.appendChild(el);
        }
    }

    function scrollActiveIntoView(activeIndex: number): void {
        const nodes = getNodes();
        const el = nodes.overlay?.querySelector(`[data-match-index="${activeIndex}"]`) as HTMLElement | null;
        el?.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'smooth' });
    }

    // Re-render the find UI from a post-mutation snapshot.
    //
    // `snapshot` is what `FindSession.{open,close,toggle,clear,setResult,...}`
    // return after they have already mutated the wasm-side state. It is *not*
    // a delta to be "applied" — every state-mutating call is fire-and-forget
    // on the wasm side; this function is purely a DOM re-render driven by the
    // returned data plus a fresh `getToolbarState()` read.
    function renderFindUi(snapshot: FindStateUpdate | null): void {
        if (!snapshot) return;
        renderToolbarFromWasm();
        renderOverlayFromUpdate(snapshot);
        scrollActiveIntoView(snapshot.state.activeIndex);
        if (snapshot.navigateToPage != null) {
            void deps.goToPage(snapshot.navigateToPage).then(() => refresh());
        }
    }

    // ─── Search Execution ────────────────────────────────────────────────────

    async function executeSearch(): Promise<void> {
        const nodes = getNodes();
        const query = nodes.input?.value?.trim() ?? '';
        const session = deps.getViewerSession();
        if (!session.path || !query) {
            renderFindUi(callSession<FindStateUpdate>(findSession(), 'clear'));
            return;
        }

        const scope = readScope();
        const result: SearchResult | null = scope === 'document'
            ? await findInDocumentAsync({ path: session.path, pageCount: session.pageCount, query, caseSensitive: false })
            : await findInPageAsync({ path: session.path, pageIndex: session.currentPage, query, caseSensitive: false });

        if (!result) {
            renderFindUi(callSession<FindStateUpdate>(findSession(), 'clear'));
            return;
        }

        const update = callSession<FindStateUpdate>(findSession(), 'setResult', result, scope, session.currentPage);
        renderFindUi(update);
    }

    function scheduleSearch(): void {
        if (searchTimerId !== null) window.clearTimeout(searchTimerId);
        searchTimerId = window.setTimeout(() => { searchTimerId = null; void executeSearch(); }, 120);
    }

    // ─── Public API ──────────────────────────────────────────────────────────

    function focusInput(): void {
        const nodes = getNodes();
        nodes.input?.focus();
        nodes.input?.select();
    }

    function open(): void {
        const s = deps.getViewerSession();
        renderFindUi(callSession<FindStateUpdate>(findSession(), 'open', s.currentPage, s.pageCount, s.path ?? ''));
        focusInput();
        if (getNodes().input?.value?.trim()) scheduleSearch();
    }

    function close(): void {
        renderFindUi(callSession<FindStateUpdate>(findSession(), 'close'));
    }

    function toggle(): void {
        const s = deps.getViewerSession();
        renderFindUi(callSession<FindStateUpdate>(findSession(), 'toggle', s.currentPage, s.pageCount, s.path ?? ''));
        const toolbar = callSession<FindToolbarState>(findSession(), 'getToolbarState');
        if (toolbar?.isOpen) {
            focusInput();
            if (getNodes().input?.value?.trim()) scheduleSearch();
        }
    }

    async function next(): Promise<void> {
        const update = callSession<FindStateUpdate>(findSession(), 'moveActive', 1);
        renderFindUi(update);
    }

    async function prev(): Promise<void> {
        const update = callSession<FindStateUpdate>(findSession(), 'moveActive', -1);
        renderFindUi(update);
    }

    async function replaceCurrent(): Promise<void> {
        const scope = readScope();
        const replacement = getNodes().replaceInput?.value ?? '';
        const session = deps.getViewerSession();

        if (scope === 'document') {
            if (!session.path) return;
            const requests = callSession<any[]>(findSession(), 'getReplaceRequests', replacement, false, scope) ?? []
            if (requests.length === 0) return;
            const req = requests[0];
            const result = replaceOne({
                path: session.path,
                pageIndex: req.pageIndex,
                regionId: req.regionId,
                kind: req.kind,
                originalText: req.originalText,
                query: req.query,
                replacement: req.replacement,
                caseSensitive: false,
            });
            if (!result?.applied) return;
            await deps.goToPage(req.pageIndex);
            await executeSearch();
            return;
        }

        const requests = callSession<PdfRegionTextReplace[]>(findSession(), 'getReplaceRequests', replacement, false, scope) ?? [];
        if (requests.length === 0) return;
        await deps.documentEdits.replaceRegionTexts(requests, 'find-replace');
        await executeSearch();
    }

    async function replaceAll(): Promise<void> {
        const scope = readScope();
        const replacement = getNodes().replaceInput?.value ?? '';
        const session = deps.getViewerSession();

        if (scope === 'document') {
            if (!session.path) return;
            const toolbar = callSession<FindToolbarState>(findSession(), 'getToolbarState');
            if (!toolbar?.hasMatches) return;
            replaceAllFacade({
                path: session.path,
                pageCount: session.pageCount,
                query: getNodes().input?.value?.trim() ?? '',
                replacement,
                caseSensitive: false,
            });
            await deps.goToPage(session.currentPage);
            await executeSearch();
            return;
        }

        const requests = callSession<PdfRegionTextReplace[]>(findSession(), 'getReplaceRequests', replacement, true, scope) ?? []
        if (requests.length === 0) return;
        await deps.documentEdits.replaceRegionTexts(requests, 'find-replace');
        await executeSearch();
    }

    async function refresh(): Promise<void> {
        const toolbar = callSession<FindToolbarState>(findSession(), 'getToolbarState');
        if (!toolbar?.isOpen) {
            const nodes = getNodes();
            if (nodes.overlay) nodes.overlay.innerHTML = '';
            return;
        }
        // Update current page in WASM
        const s = deps.getViewerSession();
        callSession(findSession(), 'setCurrentPage', s.currentPage);

        if (readScope() === 'document' && toolbar.hasMatches && getNodes().input?.value?.trim()) {
            renderToolbarFromWasm();
            const update = callSession<FindStateUpdate>(findSession(), 'setCurrentPage', s.currentPage);
            if (update) renderOverlayFromUpdate(update);
            return;
        }
        await executeSearch();
    }

    function clear(): void {
        const nodes = getNodes();
        if (nodes.input) nodes.input.value = '';
        if (nodes.scope) nodes.scope.value = 'page';
        renderFindUi(callSession<FindStateUpdate>(findSession(), 'clear'));
    }

    function initialize(): void {
        if (initialized) return;
        initialized = true;
        const nodes = getNodes();
        nodes.input?.addEventListener('input', () => scheduleSearch());
        nodes.scope?.addEventListener('change', () => scheduleSearch());
        nodes.replaceCurrent?.addEventListener('click', () => { void replaceCurrent(); });
        nodes.replaceAll?.addEventListener('click', () => { void replaceAll(); });
        nodes.input?.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') { event.preventDefault(); void (event.shiftKey ? prev() : next()); }
            else if (event.key === 'Escape') { event.preventDefault(); close(); }
        });
        nodes.replaceInput?.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') { event.preventDefault(); void (event.shiftKey ? replaceAll() : replaceCurrent()); }
        });
        renderToolbarFromWasm();
    }

    return { initialize, toggle, open, close, refresh, clear, focusInput, next, prev, replaceCurrent, replaceAll };
}

