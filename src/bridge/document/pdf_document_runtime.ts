import type { RenderReason } from '../render/frame_plan';

// ── DocumentSession bridge (P1 of session-API plan) ─────────────────────────
//
// Replaces the prior raw `wasm.open_document_pipeline()` /
// `wasm.close_document_pipeline()` calls with the struct-based
// `DocumentSession` API exported from `crates/pdf-viewer-ui/src/document/
// document_api.rs`. Singleton handle — all state lives in wasm thread_locals.

let _documentSession: any = null;

function getDocumentSession(getWasmApi: () => any): any {
    if (!_documentSession) {
        const api = getWasmApi() as any;
        if (typeof api?.DocumentSession === 'function') {
            _documentSession = new api.DocumentSession();
        }
    }
    return _documentSession;
}

type CreatePdfDocumentRuntimeDeps = {
    ensureWasmInitialized: () => Promise<void>;
    getWasmApi: () => any;
    getTargetZoom: () => number;
    resolveHostScrollRefresh: (displayZoom: number, timestampMs?: number) => { shouldRefresh?: boolean; delayMs?: number } | null;
    getScrollContainer: () => HTMLElement | null;
    renderCurrentFrame: (reason?: RenderReason) => Promise<void>;
    refreshMutatedDocument: () => Promise<void>;
    clearVectorHost: () => void;
    clearEditorHost: () => void;
    syncZoomSelect: () => void;
    syncTextEditButton: () => void;
    syncViewerState: () => void;
    resetZoomPreview: () => void;
    clearPendingAnchor: () => void;
    showEmptyDocumentState: () => void;
    defaultPageWidth: number;
    defaultPageHeight: number;
};

export type PdfDocumentRuntime = {
    renderCurrentPage: (reason?: RenderReason) => Promise<void>;
    bindTileRefreshOnScroll: () => void;
    openTextPdfFlow: (path: string) => Promise<void>;
    resetPdfViewerState: () => void;
};

export function createPdfDocumentRuntime(deps: CreatePdfDocumentRuntimeDeps): PdfDocumentRuntime {
    let viewportTileScrollBound = false;
    let viewportTileScrollRafId: number | null = null;
    let viewportTileScrollTimerId: number | null = null;

    async function renderCurrentPage(reason: RenderReason = 'default'): Promise<void> {
        if (reason === 'documentMutation') {
            await deps.refreshMutatedDocument();
            deps.syncTextEditButton();
            return;
        }
        await deps.renderCurrentFrame(reason);
        deps.syncTextEditButton();
        deps.syncViewerState();
    }

    function bindTileRefreshOnScroll(): void {
        if (viewportTileScrollBound) return;
        const scrollContainer = deps.getScrollContainer();
        if (!scrollContainer) {
            window.setTimeout(bindTileRefreshOnScroll, 250);
            return;
        }

        scrollContainer.addEventListener('scroll', () => {
            const decision = deps.resolveHostScrollRefresh(deps.getTargetZoom(), performance.now());
            if (!decision?.shouldRefresh) return;
            if (viewportTileScrollTimerId !== null) {
                window.clearTimeout(viewportTileScrollTimerId);
            }
            viewportTileScrollTimerId = window.setTimeout(() => {
                viewportTileScrollTimerId = null;
                if (viewportTileScrollRafId !== null) return;
                viewportTileScrollRafId = window.requestAnimationFrame(() => {
                    viewportTileScrollRafId = null;
                    void renderCurrentPage();
                });
            }, decision.delayMs);
        }, { passive: true });

        viewportTileScrollBound = true;
    }

    async function openTextPdfFlow(path: string): Promise<void> {
        await deps.ensureWasmInitialized();
        try {
            const session = getDocumentSession(deps.getWasmApi);
            console.log('[PDF-DIAG] openTextPdfFlow: session=', session ? 'OK' : 'NULL');
            const openResult = session
                ? await session.open({
                    path,
                    initialZoom: 1.0,
                    defaultPageWidth: 595,
                    defaultPageHeight: 842,
                })
                : null;
            console.log('[PDF-DIAG] openResult=', JSON.stringify(openResult));
            const pageCount: number = Number(openResult?.pageCount || 0);
            if (!openResult?.opened || pageCount <= 0) {
                console.warn('[PDF] DocumentSession.open returned no pages for:', path, openResult);
                resetPdfViewerState();
                return;
            }
            console.log('[PDF-DIAG] open succeeded, pageCount=', pageCount, '— entering render');
            deps.clearVectorHost();
            deps.clearEditorHost();
            deps.syncZoomSelect();
            deps.syncTextEditButton();
            await renderCurrentPage();
            console.log('[PDF-DIAG] renderCurrentPage completed');
        } catch (err) {
            console.error('[PDF] Failed to open document pipeline:', err);
            resetPdfViewerState();
        }
    }

    function resetPdfViewerState(): void {
        try {
            const session = getDocumentSession(deps.getWasmApi);
            session?.close?.(deps.defaultPageWidth, deps.defaultPageHeight);
        } catch {
        }
        deps.clearPendingAnchor();
        deps.resetZoomPreview();
        deps.clearVectorHost();
        deps.clearEditorHost();
        deps.showEmptyDocumentState();
        deps.syncZoomSelect();
        deps.syncTextEditButton();
        deps.syncViewerState();
    }

    return {
        renderCurrentPage,
        bindTileRefreshOnScroll,
        openTextPdfFlow,
        resetPdfViewerState,
    };
}




