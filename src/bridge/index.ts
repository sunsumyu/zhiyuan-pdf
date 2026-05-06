import { Plugin } from '../core/types';
import { pluginLoader } from '../core/plugin-loader';
import { registerWindowActions } from '../core/window-actions';
import { registerPdfViewerAPI } from './viewer/pdf_viewer_api';
import { clearVectorHost } from './render/vector_host';
import { bindSaveFocusGuard } from './viewer/pdf_viewer_dom';
import { createPdfViewerRuntime } from './viewer/pdf_runtime';

const runtime = createPdfViewerRuntime();

export const plugin: Plugin = {
    id: 'pdf-viewer',
    name: 'pdf-viewer',
    version: '1.0.0',
    initialize: async () => {
        await runtime.ensureWasmInitialized();

        registerWindowActions({
            'open-pdf': (args: any) => runtime.openTextPdfFlow(args.path),
        });

        runtime.bindWheelZoom();
        runtime.bindTileRefreshOnScroll();
        runtime.syncZoomSelect();
        runtime.syncTextEditButton();
        runtime.findController.initialize();
        runtime.commentController.initialize();
        runtime.reviewController.initialize();
        runtime.resumeAiController.initialize();
        window.removeEventListener('keydown', runtime.handlePdfViewerKeydown, true);
        window.addEventListener('keydown', runtime.handlePdfViewerKeydown, true);
    },
    destroy: async () => {
        window.removeEventListener('keydown', runtime.handlePdfViewerKeydown, true);
    },
};

registerPdfViewerAPI({
    ensureWasmInitialized: runtime.ensureWasmInitialized,
    getWasmApi: runtime.getWasmApi,
    readPath: () => runtime.viewerSession.read().path,
    refreshDocument: (reason) => runtime.documentEditApi.refreshDocument(reason),
    resetPdfViewerState: runtime.resetPdfViewerState,
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

pluginLoader.register(plugin);

