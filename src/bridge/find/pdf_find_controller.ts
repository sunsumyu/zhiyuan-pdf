import type { DocumentEditApi, PdfRegionTextReplace } from '../document/document_edit_api';
import {
    findInPageAsync,
    findInDocumentAsync,
    replaceOne,
    replaceAll as replaceAllFacade,
    setFindSession,
    clearFindSession,
    moveFindMatch,
    getFindSession,
    type SearchResult,
    type SearchMatch,
    type ReplaceResult,
    type BatchReplaceResult,
    type FindSession,
} from './find_facade';
import { setToolbarButtonActive } from '../viewer/pdf_viewer_dom';

type ViewerSessionSnapshot = {
    path: string | null;
    currentPage: number;
    pageCount: number;
};

// Types imported from find_facade

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

function emptyResult(query = ''): SearchResult {
    return {
        query,
        totalMatches: 0,
        matches: [],
    };
}

function isEditableRegionKind(kind: string): boolean {
    return kind === 'paragraph-region' || kind === 'list-item-region';
}

export function createPdfFindController(deps: CreatePdfFindControllerDeps): PdfFindController {
    let initialized = false;
    let isOpen = false;
    let lastResult: SearchResult = emptyResult();
    let searchTimerId: number | null = null;

    function readActiveIndex(): number {
        const session = getFindSession();
        return session?.activeIndex ?? 0;
    }

    function readScope(): FindScope {
        return getNodes().scope?.value === 'document' ? 'document' : 'page';
    }

    function getCurrentPageMatches(): Array<{ match: SearchMatch; globalIndex: number }> {
        const currentPage = deps.getViewerSession().currentPage;
        return lastResult.matches
            .map((match: SearchMatch, globalIndex: number) => ({ match, globalIndex }))
            .filter((entry: { match: SearchMatch; globalIndex: number }) => entry.match.pageIndex === currentPage);
    }

    function renderToolbarState(): void {
        const nodes = getNodes();
        const scope = readScope();
        if (nodes.bar) {
            nodes.bar.style.display = isOpen ? 'flex' : 'none';
        }
        if (nodes.toggle) {
            setToolbarButtonActive(nodes.toggle, isOpen);
        }
        const activeIndex = readActiveIndex();
        if (nodes.count) {
            nodes.count.textContent = lastResult.totalMatches > 0 ? `${activeIndex + 1} / ${lastResult.totalMatches}` : '0 / 0';
        }
        const hasMatches = lastResult.totalMatches > 0;
        const editablePageMatches = getCurrentPageMatches().filter(({ match }: { match: SearchMatch }) =>
            isEditableRegionKind(match.kind),
        );
        const editableDocumentMatches = lastResult.matches.filter((match: SearchMatch) =>
            isEditableRegionKind(match.kind),
        );
        const activeMatch = lastResult.matches[readActiveIndex()] ?? null;
        const canReplaceCurrent = scope === 'page'
            && !!activeMatch
            && activeMatch.pageIndex === deps.getViewerSession().currentPage
            && isEditableRegionKind(activeMatch.kind);
        const canReplaceAll = scope === 'page'
            ? editablePageMatches.length > 0
            : editableDocumentMatches.length > 0;
        if (nodes.prev) nodes.prev.disabled = !hasMatches;
        if (nodes.next) nodes.next.disabled = !hasMatches;
        if (nodes.replaceCurrent) nodes.replaceCurrent.disabled = !canReplaceCurrent;
        if (nodes.replaceAll) nodes.replaceAll.disabled = !canReplaceAll;
        if (nodes.replaceInput) {
            nodes.replaceInput.disabled = false;
            nodes.replaceInput.placeholder = scope === 'page'
                ? 'Replace on current page...'
                : 'Replace across whole PDF...';
        }
    }

    function clearOverlay(): void {
        const nodes = getNodes();
        if (nodes.overlay) {
            nodes.overlay.innerHTML = '';
        }
    }

    function renderOverlay(): void {
        const nodes = getNodes();
        if (!nodes.overlay) return;
        nodes.overlay.innerHTML = '';

        const pageMatches = getCurrentPageMatches();
        if (pageMatches.length === 0) {
            return;
        }

        pageMatches.forEach(({ match, globalIndex }: { match: SearchMatch; globalIndex: number }) => {
            const activeIndex = readActiveIndex();
            const editable = isEditableRegionKind(match.kind);
            const pageWidth = Math.max(1, match.pageWidth || 1);
            const pageHeight = Math.max(1, match.pageHeight || 1);
            const highlight = document.createElement('div');
            highlight.dataset.searchMatch = '1';
            highlight.dataset.matchIndex = String(globalIndex);
            highlight.style.position = 'absolute';
            highlight.style.pointerEvents = 'auto';
            highlight.style.cursor = editable ? 'text' : 'default';
            highlight.style.left = `${(match.boxRect.left / pageWidth) * 100}%`;
            highlight.style.top = `${(match.boxRect.top / pageHeight) * 100}%`;
            highlight.style.width = `${(match.boxRect.width / pageWidth) * 100}%`;
            highlight.style.height = `${(match.boxRect.height / pageHeight) * 100}%`;
            highlight.style.borderRadius = '6px';
            highlight.style.border = globalIndex === activeIndex
                ? '2px solid rgba(249, 226, 175, 0.98)'
                : '1px solid rgba(137, 180, 250, 0.95)';
            highlight.style.background = globalIndex === activeIndex
                ? 'rgba(249, 226, 175, 0.24)'
                : 'rgba(137, 180, 250, 0.18)';
            highlight.style.boxSizing = 'border-box';
            highlight.style.boxShadow = globalIndex === activeIndex
                ? '0 0 0 1px rgba(249, 226, 175, 0.25), 0 0 14px rgba(249, 226, 175, 0.35)'
                : 'none';
            highlight.title = match.previewText || match.matchedText;
            highlight.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                if (!editable) {
                    return;
                }
                void deps.openRegionEditor(
                    match.pageIndex,
                    match.id,
                    match.kind,
                    match.sourceText,
                );
            });
            nodes.overlay!.appendChild(highlight);
        });
    }

    function scrollActiveMatchIntoView(): void {
        const nodes = getNodes();
        const activeIndex = readActiveIndex();
        const active = nodes.overlay?.querySelector(`[data-match-index="${activeIndex}"]`) as HTMLElement | null;
        if (!active) return;
        active.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'smooth' });
    }

    function updateRenderedState(): void {
        renderToolbarState();
        renderOverlay();
    }

    async function executeSearch(): Promise<void> {
        const nodes = getNodes();
        const query = nodes.input?.value?.trim() ?? '';
        const session = deps.getViewerSession();
        if (!isOpen || !session.path || !query) {
            lastResult = emptyResult(query);
            clearFindSession();
            updateRenderedState();
            return;
        }

        const scope = readScope();
        const result = scope === 'document'
            ? await findInDocumentAsync({
                path: session.path,
                pageCount: session.pageCount,
                query,
                caseSensitive: false,
            })
            : await findInPageAsync({
                path: session.path,
                pageIndex: session.currentPage,
                query,
                caseSensitive: false,
            });

        if (!result) {
            lastResult = emptyResult(query);
            clearFindSession();
            updateRenderedState();
            return;
        }

        lastResult = result;
        setFindSession({
            query: result.query,
            scope,
            pageIndices: result.matches.map((match) => match.pageIndex),
            currentPage: session.currentPage,
            activeIndex: 0,
        });
        updateRenderedState();
        scrollActiveMatchIntoView();
    }

    function scheduleSearch(): void {
        if (searchTimerId !== null) {
            window.clearTimeout(searchTimerId);
        }
        searchTimerId = window.setTimeout(() => {
            searchTimerId = null;
            void executeSearch();
        }, 120);
    }

    function focusInput(): void {
        const nodes = getNodes();
        if (!nodes.input) return;
        nodes.input.focus();
        nodes.input.select();
    }

    function open(): void {
        isOpen = true;
        renderToolbarState();
        focusInput();
        if (getNodes().input?.value?.trim()) {
            scheduleSearch();
        }
    }

    function close(): void {
        isOpen = false;
        lastResult = emptyResult();
        clearFindSession();
        renderToolbarState();
        clearOverlay();
    }

    function toggle(): void {
        if (isOpen) {
            close();
        } else {
            open();
        }
    }

    async function moveActive(step: number): Promise<void> {
        if (lastResult.totalMatches === 0) return;
        const navigation = moveFindMatch(step);
        const activeIndex = navigation?.navigation?.activeIndex ?? 0;
        const targetMatch = lastResult.matches[activeIndex];
        if (targetMatch && targetMatch.pageIndex !== deps.getViewerSession().currentPage) {
            await deps.goToPage(targetMatch.pageIndex);
            return;
        }
        updateRenderedState();
        scrollActiveMatchIntoView();
    }

    async function next(): Promise<void> {
        await moveActive(1);
    }

    async function prev(): Promise<void> {
        await moveActive(-1);
    }

    function buildReplaceRequest(
        match: SearchMatch,
        replacement: string,
        replaceAllOccurrences: boolean,
    ): PdfRegionTextReplace | null {
        if (!isEditableRegionKind(match.kind)) {
            return null;
        }
        return {
            pageIndex: match.pageIndex,
            regionId: match.id,
            kind: match.kind,
            originalText: match.sourceText,
            query: lastResult.query,
            replacement,
            replaceAllOccurrences,
        };
    }

    async function replaceCurrent(): Promise<void> {
        const scope = readScope();
        const replacement = getNodes().replaceInput?.value ?? '';
        const activeIndex = readActiveIndex();
        const activeMatch = lastResult.matches[activeIndex];
        if (!activeMatch) {
            return;
        }
        if (scope === 'document') {
            const session = deps.getViewerSession();
            if (!session.path || !isEditableRegionKind(activeMatch.kind)) {
                return;
            }
            const result = replaceOne({
                path: session.path,
                pageIndex: activeMatch.pageIndex,
                regionId: activeMatch.id,
                kind: activeMatch.kind,
                originalText: activeMatch.sourceText,
                query: lastResult.query,
                replacement,
                caseSensitive: false,
            });
            if (!result?.applied) {
                return;
            }
            await deps.goToPage(activeMatch.pageIndex);
            await executeSearch();
            return;
        }
        if (activeMatch.pageIndex !== deps.getViewerSession().currentPage) {
            return;
        }
        const request = buildReplaceRequest(activeMatch, replacement, false);
        if (!request) {
            return;
        }
        await deps.documentEdits.replaceRegionTexts(
            [request],
            'find-replace',
        );
        await executeSearch();
    }

    async function replaceAll(): Promise<void> {
        const scope = readScope();
        const replacement = getNodes().replaceInput?.value ?? '';
        if (scope === 'document') {
            const session = deps.getViewerSession();
            if (!session.path || !lastResult.query.trim()) {
                return;
            }
            const result = replaceAllFacade({
                path: session.path,
                pageCount: session.pageCount,
                query: lastResult.query,
                replacement,
                caseSensitive: false,
            });
            await deps.goToPage(session.currentPage);
            await executeSearch();
            return;
        }

        const requests = getCurrentPageMatches()
            .map(({ match }) => buildReplaceRequest(match, replacement, true))
            .filter((request): request is PdfRegionTextReplace => request != null);
        if (requests.length === 0) return;
        await deps.documentEdits.replaceRegionTexts(requests, 'find-replace');
        await executeSearch();
    }

    async function refresh(): Promise<void> {
        if (!isOpen) {
            clearOverlay();
            return;
        }
        if (readScope() === 'document' && lastResult.totalMatches > 0 && getNodes().input?.value?.trim()) {
            updateRenderedState();
            scrollActiveMatchIntoView();
            return;
        }
        await executeSearch();
    }

    function clear(): void {
        const nodes = getNodes();
        if (nodes.input) {
            nodes.input.value = '';
        }
        if (nodes.scope) {
            nodes.scope.value = 'page';
        }
        lastResult = emptyResult();
        clearFindSession();
        renderToolbarState();
        clearOverlay();
    }

    function initialize(): void {
        if (initialized) return;
        initialized = true;
        const nodes = getNodes();
        nodes.input?.addEventListener('input', () => {
            scheduleSearch();
        });
        nodes.scope?.addEventListener('change', () => {
            scheduleSearch();
        });
        nodes.replaceCurrent?.addEventListener('click', () => {
            void replaceCurrent();
        });
        nodes.replaceAll?.addEventListener('click', () => {
            void replaceAll();
        });
        nodes.input?.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                event.preventDefault();
                void (event.shiftKey ? prev() : next());
            } else if (event.key === 'Escape') {
                event.preventDefault();
                close();
            }
        });
        nodes.replaceInput?.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                event.preventDefault();
                void (event.shiftKey ? replaceAll() : replaceCurrent());
            }
        });
        renderToolbarState();
    }

    return {
        initialize,
        toggle,
        open,
        close,
        refresh,
        clear,
        focusInput,
        next,
        prev,
        replaceCurrent,
        replaceAll,
    };
}





