import { ensureWasmInitialized, getWasmApi, targetInvokeV3 } from '../shared/wasm_loader';
import type { WasmModule } from '../shared/wasm_loader';
import { clearVectorHost, invalidateVectorRenderCache } from '../render/vector_host';
import { configureVectorPageBundleRuntime, prefetchAdjacentPages, findCachedBundle } from '../render/vector_page_bundle';
import { updateTextLayer } from '../render/text_layer';
import { clearRasterImageCache, warmRasterImage } from '../render/raster_image_cache';
import { createZoomController } from '../zoom/zoom_controller';
import { createViewerSessionAdapter } from './viewer_session';
import { createPagePresentationRuntimeAdapter } from './page_presentation_runtime';
import { createRenderFlow, type VisibleSurface } from '../render/render_flow';
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
import { createRenderScheduler, type RenderRequest, type RenderScheduler } from '../render/render_scheduler';
import { logPdfLayoutTrace } from '../render/layout_trace';
import { emitPdfDiagnostic } from '../shared/diagnostics';
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
    getWasmApi: () => WasmModule;
    viewerSession: ReturnType<typeof createViewerSessionAdapter>;
    pagePresentationRuntime: ReturnType<typeof createPagePresentationRuntimeAdapter>;
    documentEditApi: ReturnType<typeof createDocumentEditApi>;
    editorHost: ReturnType<typeof createEditorHost>;
    resumeAiController: ReturnType<typeof createResumeAiController>;
    findController: ReturnType<typeof createPdfFindController>;
    annotationController: ReturnType<typeof createPdfAnnotationController>;
    commentController: ReturnType<typeof createPdfCommentController>;
    reviewController: ReturnType<typeof createPdfReviewController>;
    geometryProbe: ReturnType<typeof createViewerGeometryProbe>;
    renderScheduler: RenderScheduler;
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
    prefetchAdjacentPreviews: (path: string, currentPage: number, pageCount: number) => void;
};

export function createPdfViewerRuntime(): PdfViewerRuntime {
    let isNewDocument = false;

    const viewerSession = createViewerSessionAdapter({
        getWasmApi: () => getWasmApi(),
        getFallbackPageWidth: () => DEFAULT_PAGE_WIDTH,
        getFallbackPageHeight: () => DEFAULT_PAGE_HEIGHT,
    });
    const pagePresentationRuntime = createPagePresentationRuntimeAdapter({
        getWasmApi: () => getWasmApi(),
    });
    configureVectorPageBundleRuntime({
        pagePresentationRuntime,
        viewerSession,
    });

    function getCurrentPageWidthValue(): number {
        return viewerSession.read().pageWidth || DEFAULT_PAGE_WIDTH;
    }

    function getCurrentPageHeightValue(): number {
        return viewerSession.read().pageHeight || DEFAULT_PAGE_HEIGHT;
    }

    function readZoomState(): ZoomStateSnapshot {
        try {
            const wasm = getWasmApi();
            const state = wasm.getZoomState?.();
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
        getWasmApi: () => getWasmApi(),
        getScrollContainer,
        getPageWidth: () => getCurrentPageWidthValue(),
        getPageHeight: () => getCurrentPageHeightValue(),
        getMaxZoom: getDynamicMaxZoom,
        getMaxCanvasDim: () => MAX_CANVAS_DIM,
    });

    const { syncLayoutBox } = createLayoutSync({
        getWasmApi: () => getWasmApi(),
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
        getWasmApi: () => getWasmApi(),
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
        getWasmApi: () => getWasmApi(),
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
        getWasmApi: () => getWasmApi(),
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
        getWasmApi: () => getWasmApi(),
        getCurrentPath: () => viewerSession.read().path,
        getCurrentPage: () => viewerSession.read().currentPage,
        getCurrentZoom: () => readZoomState().targetZoom,
        getPageWidth: () => getCurrentPageWidthValue(),
        getPageHeight: () => getCurrentPageHeightValue(),
        getVectorContainer,
        buildRenderRequest: (reason) =>
            framePlanAdapter.buildRenderRequest(readZoomState().targetZoom, reason ?? 'editorVisibility'),
        renderScheduledFrame: (frame) => renderFlow.renderScheduledFrame(frame),
        renderCurrentPage: (reason) => documentRuntime.renderCurrentPage(reason ?? 'editorVisibility'),
        saveEditorSession: () => documentEditApi.saveEdits('manual-save'),
        syncViewerState: () => resumeAiController.syncViewerState(),
    });

    const zoomController = createZoomController({
        getCurrentPath: () => viewerSession.read().path,
        getZoomState: readZoomState,
        resetZoomPreviewState: () => {
            try {
                const wasm = getWasmApi();
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
            const wasm = getWasmApi();
            wasm.clearPendingAnchor?.();
        },
        clearPreviewPresent: () => {
            const wasm = getWasmApi();
            wasm.clearPreviewPresent?.();
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
        pagePresentationRuntime,
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
            const prevWidth = viewerSession.read().pageWidth;
            logPdfLayoutTrace('viewer.page-dimensions.resolved', {
                width,
                height,
                prevWidth,
                isNewDocument,
            });
            viewerSession.setPageDimensions(width, height);

            // Auto fit-to-width on first dimension resolve (new document)
            if (isNewDocument) {
                isNewDocument = false; // reset the flag so it only triggers once per document
                const scrollContainer = getScrollContainer();
                const vpWidth = scrollContainer?.clientWidth || 0;
                if (vpWidth > 0 && width > vpWidth) {
                    const fitZoom = clampZoom(vpWidth / width);
                    const wasm = getWasmApi();
                    const res = wasm.applyZoomSelection?.(fitZoom);
                    logPdfLayoutTrace('viewer.auto-fit.applied', {
                        viewportWidth: vpWidth,
                        pageWidth: width,
                        fitZoom,
                        changed: !!res?.changed,
                    });
                    syncZoomSelectState(readZoomState());
                    void renderScheduler.requestRender('default');
                } else {
                    logPdfLayoutTrace('viewer.auto-fit.skipped', {
                        reason: 'pageFitsViewport',
                        viewportWidth: vpWidth,
                        pageWidth: width,
                    });
                }
            } else {
                logPdfLayoutTrace('viewer.auto-fit.skipped', {
                    reason: 'notNewDocument',
                });
            }
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
        onRenderCommitted: () => { renderScheduler.notifyCommit(); },
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
        getWasmApi: () => getWasmApi(),
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

    function prefetchAdjacentPreviews(path: string, currentPage: number, pageCount: number): void {
        const decision = pagePresentationRuntime.decideAdjacentPrefetch(currentPage, pageCount);
        if (!decision.allowed) {
            logPdfLayoutTrace('page-preview.prefetch.rejected', {
                path,
                currentPage,
                pageCount,
                rejectReason: decision.rejectReason,
                snapshot: decision.snapshot,
            });
            return;
        }

        for (const target of decision.targets) {
            void targetInvokeV3('read_preview', {
                path,
                pageIndex: target.pageIndex,
                requestRole: 'prefetch',
            }).then((preview: any) => {
                if (preview?.imageUrl) {
                    return warmRasterImage(preview.imageUrl, {
                        role: 'prefetch',
                        pageIndex: target.pageIndex,
                    });
                }
                return null;
            }).catch((error) => {
                logPdfLayoutTrace('page-preview.prefetch.failed', {
                    path,
                    pageIndex: target.pageIndex,
                    error: String(error),
                });
            });
        }
    }

    function prefetchAdjacentAssets(
        path: string,
        currentPage: number,
        pageCount: number,
        surface: VisibleSurface,
    ): void {
        if (surface === 'raster' || surface === 'preview') {
            prefetchAdjacentPreviews(path, currentPage, pageCount);
            return;
        }
        prefetchAdjacentPages(path, currentPage, pageCount);
    }

    const handlePdfViewerKeydown = createPdfKeyboardShortcutHandler({
        isTextEditEnabled: () => editorHost.isTextEditEnabled(),
        getScrollContainer,
        openFind: () => findController.open(),
        undo: () => { const api = getPdfViewerAPI(); if (api) void api.undo(); },
        redo: () => { const api = getPdfViewerAPI(); if (api) void api.redo(); },
        toggleBold: () => void editorHost.applyFormatAction({ type: 'toggleBold' }),
        toggleItalic: () => void editorHost.applyFormatAction({ type: 'toggleItalic' }),
        toggleUnderline: () => void editorHost.applyFormatAction({ type: 'toggleUnderline' }),
        prevPage: () => { const api = getPdfViewerAPI(); if (api) void api.prevPage(); },
        nextPage: () => { const api = getPdfViewerAPI(); if (api) void api.nextPage(); },
    });

    documentRuntime = createPdfDocumentRuntime({
        ensureWasmInitialized,
        getWasmApi: () => getWasmApi(),
        getTargetZoom: () => readZoomState().targetZoom,
        resolveHostScrollRefresh: (displayZoom, timestampMs) =>
            framePlanAdapter.resolveHostScrollRefresh(displayZoom, timestampMs),
        getScrollContainer,
        get renderScheduler() { return renderScheduler; },
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

    const renderScheduler = createRenderScheduler({
        pagePresentationRuntime,
        presentPagePreview: (pageIndex: number) => renderFlow.presentPagePreview(pageIndex),
        executeRender: async (request: RenderRequest) => {
            if (
                request.source === 'navigation' &&
                Number.isFinite(request.targetPage as number) &&
                viewerSession.read().currentPage !== request.targetPage
            ) {
                logPdfLayoutTrace('page-turn.session-realign', {
                    pageTurnId: request.pageTurnId,
                    targetPage: request.targetPage,
                    currentPage: viewerSession.read().currentPage,
                    reason: request.reason,
                });
                viewerSession.setCurrentPage(request.targetPage as number);
            }

            await documentRuntime.renderCurrentPage(request.reason);
            const renderedSession = viewerSession.read();
            const visiblePage = renderFlow.getLastRenderedPageIndex() ?? renderedSession.currentPage;
            const visibleSurface = renderFlow.getLastVisibleSurface() ?? 'vector';
            if (
                request.source === 'navigation' &&
                Number.isFinite(request.pageTurnId as number) &&
                Number.isFinite(request.targetPage as number) &&
                !pagePresentationRuntime.isLatestPageTurn(
                    request.pageTurnId as number,
                    request.targetPage as number,
                )
            ) {
                emitPdfDiagnostic('PROF', 'page-turn.stale-render-skipped', {
                    pageTurnId: request.pageTurnId,
                    targetPage: request.targetPage,
                    visiblePage,
                    visibleSurface,
                    elapsedMs: performance.now() - request.issuedAt,
                    accepted: false,
                    rejectReason: 'stalePageTurn',
                });
                return;
            }
            if (
                request.source === 'navigation' &&
                Number.isFinite(request.targetPage as number) &&
                visiblePage !== request.targetPage
            ) {
                logPdfLayoutTrace('page-turn.visible-mismatch', {
                    pageTurnId: request.pageTurnId,
                    targetPage: request.targetPage,
                    visiblePage,
                    visibleSurface,
                    reason: request.reason,
                });
            }
            const visible = pagePresentationRuntime.markPageVisible(
                visiblePage,
                visibleSurface,
            );
            if (request.source === 'navigation') {
                emitPdfDiagnostic('PROF', 'page-turn.visible-ready', {
                    pageTurnId: request.pageTurnId,
                    targetPage: request.targetPage,
                    visiblePage,
                    visibleSurface,
                    elapsedMs: performance.now() - request.issuedAt,
                    accepted: visible.accepted,
                    rejectReason: visible.rejectReason,
                });
            }
            await annotationController?.refresh();
            await commentController?.refresh();
            await reviewController?.refresh();
            await findController.refresh();

            // Prefetch adjacent pages in background after render
            const session = viewerSession.read();
            if (
                visible.canPrefetch &&
                pagePresentationRuntime.canPrefetch(visiblePage) &&
                session.path &&
                session.pageCount > 0
            ) {
                prefetchAdjacentAssets(
                    session.path,
                    visiblePage,
                    session.pageCount,
                    visibleSurface,
                );
            }
        },
    });

    async function renderCurrentPage(reason?: RenderReason): Promise<void> {
        await renderScheduler.requestRender('default', reason);
    }

    async function openTextPdfFlow(path: string): Promise<void> {
        isNewDocument = true; // Mark as a new document so we run auto fit-to-width on first render
        clearRasterImageCache();
        await documentRuntime.openTextPdfFlow(path);
        await annotationController?.refresh();
        await commentController?.refresh();
        await reviewController?.refresh();
        await findController.refresh();
    }

    function resetPdfViewerState(): void {
        isNewDocument = false;
        pagePresentationRuntime.reset();
        clearRasterImageCache();
        documentRuntime.resetPdfViewerState();
        annotationController?.clear();
        commentController?.clear();
        reviewController?.clear();
        findController.clear();
    }

    if (typeof window !== 'undefined') {
        window.addEventListener('pdf-text-layer-ready', ((e: CustomEvent) => {
            const { path, pageIndex } = e.detail;
            const session = viewerSession.read();
            if (path === session.path && pageIndex === session.currentPage) {
                const currentRevision = session.documentRevision;
                const cached = findCachedBundle(path, pageIndex, currentRevision);
                if (cached) {
                    updateTextLayer(path, pageIndex, cached.model, session.currentZoom);
                }
            }
        }) as any);
    }

    return {
        ensureWasmInitialized,
        getWasmApi: () => getWasmApi(),
        viewerSession,
        pagePresentationRuntime,
        documentEditApi,
        editorHost,
        resumeAiController,
        findController,
        annotationController: annotationController!,
        commentController: commentController!,
        reviewController: reviewController!,
        geometryProbe,
        renderScheduler,
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
        prefetchAdjacentPreviews,
    };
}



