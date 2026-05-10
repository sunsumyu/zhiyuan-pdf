import { ensureWasmInitialized, getWasmApi, targetInvokeV3 } from '../shared/wasm_loader';
import { clearVectorHost, invalidateVectorRenderCache } from '../render/vector_host';
import { createZoomController } from '../zoom/zoom_controller';
import { createViewerSessionAdapter } from './viewer_session';
import { createRenderFlow } from '../render/render_flow';
import { createFramePlanAdapter, type RenderReason } from '../render/frame_plan';
import { createViewerGeometryProbe } from './viewer_geometry_probe';
import { createResumeAiController } from '../ai/resume_ai_controller';
import { createEditorHost } from '../editor';
import { createDocumentEditApi } from '../document/document_edit_api';
import { createPdfKeyboardShortcutHandler } from './pdf_keyboard';
import { createPdfFindController } from '../find/pdf_find_controller';
import { createPdfAnnotationController } from '../annotation/pdf_annotation_controller';
import { createPdfCommentController } from '../comment/pdf_comment_controller';
import { createPdfReviewController } from '../review/pdf_review_controller';
import { createPdfDocumentRuntime, type PdfDocumentRuntime } from '../document/pdf_document_runtime';
import { logPdfLayoutTrace } from '../render/layout_trace';
import { getPdfViewerAPI } from './pdf_viewer_api';
import { createLayoutSync } from './pdf_layout_sync';
import {
    clampZoom,
    DEFAULT_PAGE_HEIGHT,
    DEFAULT_PAGE_WIDTH,
    getDynamicMaxZoom,
    getEmptyState,
    getPageIndicator,
    getRasterTarget,
    getScrollContainer,
    getVectorContainer,
    getWrapper,
    MAX_CANVAS_DIM,
    showDocumentWrapper,
    showEmptyDocumentState,
    syncTextEditButton as syncTextEditButtonState,
    syncZoomSelect as syncZoomSelectState,
} from './pdf_viewer_dom';

type ZoomStateSnapshot = {
    currentZoom: number;
    targetZoom: number;
    visualZoom: number;
    lastRenderedZoom: number;
};

export type PdfViewerRuntime = {
    ensureWasmInitialized: typeof ensureWasmInitialized;
    getWasmApi: () => any;
    viewerSession: ReturnType<typeof createViewerSessionAdapter>;
    documentEditApi: ReturnType<typeof createDocumentEditApi>;
    editorHost: ReturnType<typeof createEditorHost>;
    resumeAiController: ReturnType<typeof createResumeAiController>;
    findController: ReturnType<typeof createPdfFindController>;
    annotationController: ReturnType<typeof createPdfAnnotationController>;
    commentController: ReturnType<typeof createPdfCommentController>;
    reviewController: ReturnType<typeof createPdfReviewController>;
    geometryProbe: ReturnType<typeof createViewerGeometryProbe>;
    renderCurrentPage: (reason?: RenderReason) => Promise<void>;
    openTextPdfFlow: (path: string) => Promise<void>;
    resetPdfViewerState: () => void;
    readTargetZoom: () => number;
    clampZoom: typeof clampZoom;
    syncZoomSelect: () => void;
    syncTextEditButton: () => void;
    bindTileRefreshOnScroll: () => void;
    bindWheelZoom: () => void;
    handlePdfViewerKeydown: (event: KeyboardEvent) => void;
    defaultPageWidth: number;
    defaultPageHeight: number;
};

export function createPdfViewerRuntime(): PdfViewerRuntime {
    const viewerSession = createViewerSessionAdapter({
        getWasmApi: () => getWasmApi() as any,
        getFallbackPageWidth: () => DEFAULT_PAGE_WIDTH,
        getFallbackPageHeight: () => DEFAULT_PAGE_HEIGHT,
    });

    function getCurrentPageWidthValue(): number {
        return viewerSession.read().pageWidth || DEFAULT_PAGE_WIDTH;
    }

    function getCurrentPageHeightValue(): number {
        return viewerSession.read().pageHeight || DEFAULT_PAGE_HEIGHT;
    }

    function readZoomState(): ZoomStateSnapshot {
        try {
            const wasm = getWasmApi() as any;
            const state = wasm.get_zoom_state();
            const session = viewerSession.read();
            return {
                currentZoom: state?.currentZoom ?? session.currentZoom,
                targetZoom: state?.targetZoom ?? session.currentZoom,
                visualZoom: state?.visualZoom ?? session.currentZoom,
                lastRenderedZoom: state?.lastRenderedZoom ?? session.currentZoom,
            };
        } catch {
            const session = viewerSession.read();
            return {
                currentZoom: session.currentZoom,
                targetZoom: session.currentZoom,
                visualZoom: session.currentZoom,
                lastRenderedZoom: session.currentZoom,
            };
        }
    }

    const framePlanAdapter = createFramePlanAdapter({
        getWasmApi: () => getWasmApi() as any,
        getScrollContainer,
        getPageWidth: () => getCurrentPageWidthValue(),
        getPageHeight: () => getCurrentPageHeightValue(),
        getMaxZoom: getDynamicMaxZoom,
        getMaxCanvasDim: () => MAX_CANVAS_DIM,
    });

    const { syncLayoutBox } = createLayoutSync({
        getWasmApi: () => getWasmApi() as any,
        getPageWidth: () => getCurrentPageWidthValue(),
        getPageHeight: () => getCurrentPageHeightValue(),
        readZoomState,
    });

    function syncZoomSelect(): void {
        syncZoomSelectState(readZoomState());
    }

    let editorHost!: ReturnType<typeof createEditorHost>;
    let renderFlow!: ReturnType<typeof createRenderFlow>;
    let resumeAiController!: ReturnType<typeof createResumeAiController>;
    let annotationController: ReturnType<typeof createPdfAnnotationController> | null = null;
    let commentController: ReturnType<typeof createPdfCommentController> | null = null;
    let reviewController: ReturnType<typeof createPdfReviewController> | null = null;
    const documentEditApi = createDocumentEditApi({
        getWasmApi: () => getWasmApi() as any,
        getCurrentPath: () => viewerSession.read().path,
        getCurrentPage: () => viewerSession.read().currentPage,
        getCurrentZoom: () => readZoomState().targetZoom,
        buildRenderRequest: (reason) =>
            framePlanAdapter.buildRenderRequest(readZoomState().targetZoom, reason ?? 'documentMutation'),
        renderScheduledFrame: (frame) => renderFlow.renderScheduledFrame(frame),
        invalidateRenderCache: () => invalidateVectorRenderCache(),
        syncViewerState: () => {
            resumeAiController.syncViewerState();
            void annotationController?.refresh();
            void commentController?.refresh();
            void reviewController?.refresh();
        },
    });
    const findController = createPdfFindController({
        getViewerSession: () => viewerSession.read(),
        getWasmApi: () => getWasmApi() as any,
        getScrollContainer,
        documentEdits: documentEditApi,
        goToPage: async (pageIndex) => {
            viewerSession.setCurrentPage(pageIndex);
            await renderCurrentPage();
        },
        openRegionEditor: async (pageIndex, regionId, kind, originalText) => {
            if (viewerSession.read().currentPage !== pageIndex) {
                viewerSession.setCurrentPage(pageIndex);
                await renderCurrentPage();
            }
            await editorHost.openRegionEditor(pageIndex, regionId, kind, originalText);
        },
    });
    annotationController = createPdfAnnotationController({
        getViewerSession: () => viewerSession.read(),
        documentEdits: documentEditApi,
    });
    commentController = createPdfCommentController({
        getViewerSession: () => viewerSession.read(),
        getWasmApi: () => getWasmApi() as any,
        documentEdits: documentEditApi,
        goToPage: async (pageIndex) => {
            viewerSession.setCurrentPage(pageIndex);
            await renderCurrentPage();
        },
    });
    reviewController = createPdfReviewController({
        getViewerSession: () => viewerSession.read(),
        documentEdits: documentEditApi,
        goToPage: async (pageIndex) => {
            viewerSession.setCurrentPage(pageIndex);
            await renderCurrentPage();
        },
        openRegionEditor: async (pageIndex, regionId, kind, originalText) => {
            if (viewerSession.read().currentPage !== pageIndex) {
                viewerSession.setCurrentPage(pageIndex);
                await renderCurrentPage();
            }
            await editorHost.openRegionEditor(pageIndex, regionId, kind, originalText);
        },
    });
    let documentRuntime!: PdfDocumentRuntime;

    function syncTextEditButton(): void {
        syncTextEditButtonState(editorHost.isTextEditEnabled());
    }

    editorHost = createEditorHost({
        getWasmApi: () => getWasmApi() as any,
        getCurrentPath: () => viewerSession.read().path,
        getCurrentPage: () => viewerSession.read().currentPage,
        getCurrentZoom: () => readZoomState().targetZoom,
        getPageWidth: () => getCurrentPageWidthValue(),
        getPageHeight: () => getCurrentPageHeightValue(),
        getVectorContainer,
        buildRenderRequest: (reason) =>
            framePlanAdapter.buildRenderRequest(readZoomState().targetZoom, reason ?? 'editorVisibility'),
        renderScheduledFrame: (frame) => renderFlow.renderScheduledFrame(frame),
        saveEditorSession: () => documentEditApi.saveEdits('manual-save'),
        syncViewerState: () => resumeAiController.syncViewerState(),
    });

    const zoomController = createZoomController({
        getCurrentPath: () => viewerSession.read().path,
        getZoomState: readZoomState,
        resetZoomPreviewState: () => {
            try {
                const wasm = getWasmApi() as any;
                wasm.clear_zoom_preview_host_state?.(false);
            } catch {
            }
        },
        getCurrentPageWidth: () => getCurrentPageWidthValue(),
        getCurrentPageHeight: () => getCurrentPageHeightValue(),
        getWrapper,
        getScrollContainer,
        getVectorContainer,
        syncLayoutBox,
        syncZoomSelect,
        requestRender: (reason) => {
            void documentRuntime.renderCurrentPage(reason ?? 'zoom');
        },
        peekFramePlan: (displayZoom) => framePlanAdapter.peek(displayZoom),
        takeFramePlan: (displayZoom) => framePlanAdapter.take(displayZoom),
        getMaxZoom: getDynamicMaxZoom,
        clearPendingAnchor: () => {
            const wasm = getWasmApi() as any;
            wasm.clear_pending_anchor();
        },
        clearPreviewPresent: () => {
            const wasm = getWasmApi() as any;
            wasm.clear_preview_present?.();
        },
        resolveWheelRenderDecision: (request) => framePlanAdapter.resolveWheelRenderDecision(request),
        handleWheelZoomHost: (displayZoom, wheelRequest) =>
            framePlanAdapter.handleWheelZoomHost(displayZoom, wheelRequest),
        stepPreviewHost: (displayZoom, timestampMs) =>
            framePlanAdapter.stepPreviewHost(displayZoom, timestampMs),
        setWheelRenderPending: (pending) => framePlanAdapter.setWheelRenderPending(pending),
        getWheelRenderPending: () => framePlanAdapter.getWheelRenderPending(),
        queueCommittedFrame: (frame) => framePlanAdapter.queueCommittedFrame(frame),
        takeReadyCommittedFrame: () => framePlanAdapter.takeReadyCommittedFrame(),
    });

    renderFlow = createRenderFlow({
        targetInvokeV3,
        viewerSession,
        framePlanAdapter,
        clearPendingAnchor: () => zoomController.clearPendingAnchor(),
        commitRenderedFrame: (frame) => {
            logPdfLayoutTrace('render.commit-frame.to-zoom-controller', {
                frame,
                zoomState: readZoomState(),
            });
            zoomController.commitRenderedFrame(frame);
        },
        getWrapper,
        getRasterTarget,
        getEmptyState,
        getPageIndicator,
        showWrapper: showDocumentWrapper,
        onPageDimensionsResolved: (width, height) => {
            if (!(width > 0) || !(height > 0)) return;
            viewerSession.setPageDimensions(width, height);
        },
        syncEditorOverlay: (displayZoom) => {
            editorHost.syncTargets(displayZoom);
        },
        clearEditorOverlay: () => {
            editorHost.clear();
        },
        prepareRenderFrame: (frame) => {
            zoomController.prepareImmediateRenderFrame(frame.framePlan);
        },
        scheduleRenderFollowUp: (renderedDisplayZoom) =>
            framePlanAdapter.scheduleRenderFollowUp(renderedDisplayZoom),
        commitRenderResult: (frameToken, renderedZoom, pageWidth, pageHeight) => {
            logPdfLayoutTrace('render.commit-result.before', {
                frameToken,
                renderedZoom,
                pageWidth,
                pageHeight,
                zoomState: readZoomState(),
            });
            const result = framePlanAdapter.commitRenderResult(frameToken, renderedZoom, pageWidth, pageHeight);
            logPdfLayoutTrace('render.commit-result.after', {
                frameToken,
                renderedZoom,
                pageWidth,
                pageHeight,
                result,
                zoomState: readZoomState(),
            });
            return result;
        },
    });

    resumeAiController = createResumeAiController({
        getViewerSession: () => viewerSession.read(),
        documentEdits: documentEditApi,
        openPdfPath: (path) => documentRuntime.openTextPdfFlow(path),
        renderCurrentPage: (reason) => documentRuntime.renderCurrentPage(reason),
        setCurrentPage: (pageIndex) => viewerSession.setCurrentPage(pageIndex),
    });

    const geometryProbe = createViewerGeometryProbe({
        ensureWasmInitialized,
        getWasmApi: () => getWasmApi() as any,
        viewerSession,
        framePlanAdapter,
        getZoomState: readZoomState,
        getScrollContainer,
        getVectorContainer,
        syncLayoutBox,
        syncZoomSelect,
        showWrapper: showDocumentWrapper,
        setPageDimensions: (_pageWidth, _pageHeight) => {},
        getPageWidth: () => getCurrentPageWidthValue(),
        getPageHeight: () => getCurrentPageHeightValue(),
        clampZoom,
        getMaxZoom: getDynamicMaxZoom,
    });

    const handlePdfViewerKeydown = createPdfKeyboardShortcutHandler({
        isTextEditEnabled: () => editorHost.isTextEditEnabled(),
        getScrollContainer,
        openFind: () => findController.open(),
        undo: () => { const api = getPdfViewerAPI(); if (api) void api.undo(); },
        redo: () => { const api = getPdfViewerAPI(); if (api) void api.redo(); },
        toggleBold: () => void editorHost.applyFormatAction({ type: 'toggleBold' }),
        toggleItalic: () => void editorHost.applyFormatAction({ type: 'toggleItalic' }),
        toggleUnderline: () => void editorHost.applyFormatAction({ type: 'toggleUnderline' }),
    });

    documentRuntime = createPdfDocumentRuntime({
        ensureWasmInitialized,
        getWasmApi: () => getWasmApi() as any,
        getTargetZoom: () => readZoomState().targetZoom,
        resolveHostScrollRefresh: (displayZoom, timestampMs) =>
            framePlanAdapter.resolveHostScrollRefresh(displayZoom, timestampMs),
        getScrollContainer,
        renderCurrentFrame: (reason) => renderFlow.renderCurrentPage(reason),
        refreshMutatedDocument: () => documentEditApi.refreshDocument('document-mutation'),
        clearVectorHost,
        clearEditorHost: () => editorHost.clear(),
        syncZoomSelect,
        syncTextEditButton,
        syncViewerState: () => resumeAiController.syncViewerState(),
        resetZoomPreview: () => zoomController.resetVisualZoomPreview(),
        clearPendingAnchor: () => zoomController.clearPendingAnchor(),
        showEmptyDocumentState,
        defaultPageWidth: DEFAULT_PAGE_WIDTH,
        defaultPageHeight: DEFAULT_PAGE_HEIGHT,
    });

    async function renderCurrentPage(reason?: RenderReason): Promise<void> {
        await documentRuntime.renderCurrentPage(reason);
        await annotationController?.refresh();
        await commentController?.refresh();
        await reviewController?.refresh();
        await findController.refresh();
    }

    async function openTextPdfFlow(path: string): Promise<void> {
        await documentRuntime.openTextPdfFlow(path);
        await annotationController?.refresh();
        await commentController?.refresh();
        await reviewController?.refresh();
        await findController.refresh();
    }

    function resetPdfViewerState(): void {
        documentRuntime.resetPdfViewerState();
        annotationController?.clear();
        commentController?.clear();
        reviewController?.clear();
        findController.clear();
    }

    return {
        ensureWasmInitialized,
        getWasmApi: () => getWasmApi() as any,
        viewerSession,
        documentEditApi,
        editorHost,
        resumeAiController,
        findController,
        annotationController: annotationController!,
        commentController: commentController!,
        reviewController: reviewController!,
        geometryProbe,
        renderCurrentPage,
        openTextPdfFlow,
        resetPdfViewerState,
        readTargetZoom: () => readZoomState().targetZoom,
        clampZoom,
        syncZoomSelect,
        syncTextEditButton,
        bindTileRefreshOnScroll: documentRuntime.bindTileRefreshOnScroll,
        bindWheelZoom: () => zoomController.bindWheelZoom(),
        handlePdfViewerKeydown,
        defaultPageWidth: DEFAULT_PAGE_WIDTH,
        defaultPageHeight: DEFAULT_PAGE_HEIGHT,
    };
}



