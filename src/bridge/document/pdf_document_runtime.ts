import type { RenderReason } from '../render/frame_plan';
import type { RenderScheduler } from '../render/render_scheduler';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import type { WasmModule } from '../shared/wasm_loader';
import type { DocumentSession } from '../../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui';

// ── DocumentSession bridge (P1 of session-API plan) ─────────────────────────
//
// Replaces the prior raw `wasm.open_document_pipeline()` /
// `wasm.close_document_pipeline()` calls with the struct-based
// `DocumentSession` API exported from `crates/pdf-viewer-ui/src/document/
// document_api.rs`. Singleton handle — all state lives in wasm thread_locals.

let _documentSession: DocumentSession | null = null;

function resolveDocumentSession(getWasmApi: () => WasmModule): DocumentSession {
    if (!_documentSession) {
        const api = getWasmApi();
        if (typeof api?.DocumentSession === 'function') {
            _documentSession = new api.DocumentSession();
        }
    }
    if (!_documentSession) {
        throw new Error('DocumentSession WASM API is unavailable');
    }
    return _documentSession;
}

type CreatePdfDocumentRuntimeDeps = {
    ensureWasmInitialized: () => Promise<void>;
    getWasmApi: () => WasmModule;
    getTargetZoom: () => number;
    resolveHostScrollRefresh: (displayZoom: number, timestampMs?: number) => { shouldRefresh?: boolean; delayMs?: number } | null;
    getScrollContainer: () => HTMLElement | null;
    renderScheduler: RenderScheduler;
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
            void deps.renderScheduler.requestRender('scroll');
        }, { passive: true });

        viewportTileScrollBound = true;
    }

    async function openTextPdfFlow(path: string): Promise<void> {
        emitPdfDiagnostic('DOC', 'openTextPdfFlow.start', { path });
        await deps.ensureWasmInitialized();
        emitPdfDiagnostic('DOC', 'openTextPdfFlow.wasmReady', { path });
        try {
            const session = resolveDocumentSession(deps.getWasmApi);
            emitPdfDiagnostic('DOC', 'openTextPdfFlow.sessionResolved', { path, session: 'OK' });
            // Eagerly clear the vector host BEFORE awaiting session.open().
            // This cancels any in-flight Rust render (cancelProgressiveRender + resetFrameCache)
            // so the old document's Worker render cannot complete and flash old pixels
            // during the async IPC gap of session.open().
            deps.clearVectorHost();
            deps.clearEditorHost();
            emitPdfDiagnostic('DOC', 'openTextPdfFlow.callingSessionOpen', { path });
            const openResult = await session.open({
                path,
                initialZoom: 1.0,
                defaultPageWidth: 595,
                defaultPageHeight: 842,
            });
            emitPdfDiagnostic('DOC', 'openResult', { 
                opened: openResult?.opened,
                pageCount: openResult?.pageCount,
                path,
                fullResult: openResult ? JSON.stringify(openResult) : 'null' 
            });
            const pageCount: number = Number(openResult?.pageCount || 0);
            if (!openResult?.opened || pageCount <= 0) {
                const reason = pageCount <= 0
                    ? 'open_pdf returned 0 pages'
                    : 'document open pipeline returned opened=false';
                emitPdfDiagnostic('DOC', 'openFailed', { path, reason, openResult: openResult ? JSON.stringify(openResult) : 'null' }, { level: 'ERROR' });
                throw new Error(`Failed to open PDF: ${reason}`);
            }
            emitPdfDiagnostic('DOC', 'openSuccess', { path, pageCount });
            deps.syncZoomSelect();
            deps.syncTextEditButton();
            emitPdfDiagnostic('DOC', 'openTextPdfFlow.callingRender', { path });
            await renderCurrentPage();
            emitPdfDiagnostic('DOC', 'renderCompleted', { path });
        } catch (err) {
            const errorMsg = err instanceof Error ? err.message : String(err);
            const errorStack = err instanceof Error ? err.stack : undefined;
            emitPdfDiagnostic('DOC', 'openException', { 
                path, 
                error: errorMsg,
                stack: errorStack,
            }, { level: 'ERROR' });
            resetPdfViewerState();
            throw err instanceof Error ? err : new Error(String(err));
        }
    }

    function resetPdfViewerState(): void {
        try {
            const session = resolveDocumentSession(deps.getWasmApi);
            session.close(deps.defaultPageWidth, deps.defaultPageHeight);
        } catch (err) {
            emitPdfDiagnostic('DOC', 'resetCloseFailed', { error: String(err) }, { level: 'ERROR' });
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




