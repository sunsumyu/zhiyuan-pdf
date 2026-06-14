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

function getDocumentSession(getWasmApi: () => WasmModule): DocumentSession | null {
    if (!_documentSession) {
        const api = getWasmApi();
        if (typeof api?.DocumentSession === 'function') {
            _documentSession = new api.DocumentSession();
        }
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
        await deps.ensureWasmInitialized();
        try {
            const session = getDocumentSession(deps.getWasmApi);
            emitPdfDiagnostic('DOC', 'openTextPdfFlow', { path, session: session ? 'OK' : 'NULL' });
            const openResult = session
                ? await session.open({
                    path,
                    initialZoom: 1.0,
                    defaultPageWidth: 595,
                    defaultPageHeight: 842,
                })
                : null;
            emitPdfDiagnostic('DOC', 'openResult', { openResult: openResult ? JSON.stringify(openResult) : 'null' });
            const pageCount: number = Number(openResult?.pageCount || 0);
            if (!openResult?.opened || pageCount <= 0) {
                emitPdfDiagnostic('DOC', 'openFailed', { path, openResult: openResult ? JSON.stringify(openResult) : 'null' }, { level: 'ERROR' });
                resetPdfViewerState();
                return;
            }
            emitPdfDiagnostic('DOC', 'openSuccess', { path, pageCount });
            deps.clearVectorHost();
            deps.clearEditorHost();
            deps.syncZoomSelect();
            deps.syncTextEditButton();
            await renderCurrentPage();
            emitPdfDiagnostic('DOC', 'renderCompleted', { path });
        } catch (err) {
            emitPdfDiagnostic('DOC', 'openException', { path, error: String(err) }, { level: 'ERROR' });
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




