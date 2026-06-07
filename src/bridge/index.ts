import { registerPdfViewerAPI } from './viewer/pdf_viewer_api';
import { clearVectorHost } from './render/vector_host';
import { bindSaveFocusGuard } from './viewer/pdf_viewer_dom';
import { createPdfViewerRuntime } from './viewer/pdf_runtime';

const runtime = createPdfViewerRuntime();

export const plugin = {
    id: 'pdf-viewer',
    name: 'pdf-viewer',
    version: '1.0.0',
    initialize: async () => {
        performance.mark('plugin-runtime-init-start');
        await runtime.ensureWasmInitialized();
        performance.mark('plugin-runtime-init-end');
        performance.measure('runtime.ensureWasmInitialized', 'plugin-runtime-init-start', 'plugin-runtime-init-end');

        // Register global open-pdf action on window
        (window as any)['open-pdf'] = (args: any) => runtime.openTextPdfFlow(args.path);

        runtime.bindWheelZoom();
        runtime.bindTileRefreshOnScroll();
        runtime.syncZoomSelect();
        runtime.syncTextEditButton();
        performance.mark('plugin-controller-init-start');
        await Promise.all([
            runtime.findController.initialize(),
            runtime.commentController.initialize(),
            runtime.reviewController.initialize(),
            runtime.resumeAiController.initialize(),
        ]);
        performance.mark('plugin-controller-init-end');
        performance.measure('plugin.controllers.initialize', 'plugin-controller-init-start', 'plugin-controller-init-end');
        window.removeEventListener('keydown', runtime.handlePdfViewerKeydown, true);
        window.addEventListener('keydown', runtime.handlePdfViewerKeydown, true);
        performance.measure('plugin.initialize.total', 'plugin-runtime-init-start', 'plugin-controller-init-end');
    },
    destroy: async () => {
        window.removeEventListener('keydown', runtime.handlePdfViewerKeydown, true);
    },
};

registerPdfViewerAPI({
    ensureWasmInitialized: runtime.ensureWasmInitialized,
    getWasmApi: runtime.getWasmApi,
    readPath: () => runtime.viewerSession.read().path,
    readCurrentPage: () => runtime.viewerSession.read().currentPage,
    readPageCount: () => runtime.viewerSession.read().pageCount,
    requestPageTurn: (targetPage, reason, nowMs) =>
        runtime.pagePresentationRuntime.requestPageTurn(targetPage, reason, nowMs),
    setCurrentPage: (pageIndex: number) => runtime.viewerSession.setCurrentPage(pageIndex),
    refreshDocument: (reason) => runtime.documentEditApi.refreshDocument(reason),
    resetPdfViewerState: runtime.resetPdfViewerState,
    renderScheduler: runtime.renderScheduler,
    renderCurrentPage: runtime.renderCurrentPage,
    clampZoom: runtime.clampZoom,
    syncZoomSelect: runtime.syncZoomSelect,
    syncTextEditButton: runtime.syncTextEditButton,
    readTargetZoom: runtime.readTargetZoom,
    editorHost: runtime.editorHost,
    annotationController: runtime.annotationController,
    commentController: runtime.commentController,
    reviewController: runtime.reviewController,
    findController: runtime.findController,
    resumeAiController: runtime.resumeAiController,
    defaultPageWidth: runtime.defaultPageWidth,
    defaultPageHeight: runtime.defaultPageHeight,
    openTextPdfFlow: runtime.openTextPdfFlow,
    clearVectorHost,
    geometryProbe: runtime.geometryProbe,
});

bindSaveFocusGuard();

