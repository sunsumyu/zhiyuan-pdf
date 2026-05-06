import type { RenderReason } from '../render/frame_plan';

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
            const openResult = await deps.getWasmApi().open_document_pipeline?.({
                path,
                initialZoom: 1.0,
                defaultPageWidth: 595,
                defaultPageHeight: 842,
            });
            const pageCount: number = Number(openResult?.pageCount || 0);
            if (!openResult?.opened || pageCount <= 0) {
                console.warn('[PDF] open_document_pipeline returned no pages for:', path, openResult);
                resetPdfViewerState();
                return;
            }
            deps.clearVectorHost();
            deps.clearEditorHost();
            deps.syncZoomSelect();
            deps.syncTextEditButton();
            await renderCurrentPage();
        } catch (err) {
            console.error('[PDF] Failed to open document pipeline:', err);
            resetPdfViewerState();
        }
    }

    function resetPdfViewerState(): void {
        try {
            deps.getWasmApi().close_document_pipeline?.(
                deps.defaultPageWidth,
                deps.defaultPageHeight,
            );
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




