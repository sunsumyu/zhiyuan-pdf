import { commitVectorRenderResult, ensureVectorHost, renderVectorPageWithPlan } from './vector_host';
import { createPagePresenter } from '../presentation/page_presenter';
import type { FramePlanAdapter, RenderReason, RustRenderCommitResult, RustRenderFrame } from './frame_plan';
import type { ViewerSessionAdapter } from '../viewer/viewer_session';
import type { PagePresentationRuntimeAdapter } from '../viewer/page_presentation_runtime';
import { logPdfLayoutTrace } from './layout_trace';
import { emitPdfDiagnostic } from '../shared/diagnostics';

type RenderFlowDeps = {
    targetInvokeV3: (cmd: string, args: any) => Promise<any>;
    viewerSession: ViewerSessionAdapter;
    pagePresentationRuntime: PagePresentationRuntimeAdapter;
    framePlanAdapter: FramePlanAdapter;
    clearPendingAnchor: () => void;
    commitRenderedFrame: (frame: { displayZoom: number; renderZoom: number; hostWidth: number; hostHeight: number; contentLeft: number; contentTop: number; scrollLeft: number; scrollTop: number; }) => void;
    getWrapper: () => HTMLElement | null;
    getRasterTarget: () => HTMLCanvasElement | null;
    getEmptyState: () => HTMLElement | null;
    getPageIndicator: () => HTMLElement | null;
    showWrapper: () => void;
    onPageDimensionsResolved: (width: number, height: number) => void;
    syncEditorOverlay: (displayZoom: number) => void;
    clearEditorOverlay: () => void;
    prepareRenderFrame: (frame: RustRenderFrame) => void;
    scheduleRenderFollowUp: (renderedDisplayZoom: number) => RustRenderFrame | null;
    commitRenderResult: (frameToken: number, renderedZoom: number, pageWidth: number, pageHeight: number) => RustRenderCommitResult | null;
    onRenderCommitted: () => void;
};

export type VisibleSurface = 'preview' | 'vector' | 'detail' | 'raster';

export function createRenderFlow(deps: RenderFlowDeps) {
    let lastVisibleSurface: VisibleSurface | null = null;
    let lastRenderedPageIndex: number | null = null;
    let lastPreviewPath: string | null = null;
    let lastPreviewPageIndex: number | null = null;
    let lastPreview: any = null;

    async function getPagePreview(path: string, pageIndex: number): Promise<any> {
        if (lastPreviewPath === path && lastPreviewPageIndex === pageIndex) {
            return lastPreview;
        }
        try {
            const preview = await deps.targetInvokeV3('read_preview', {
                path,
                pageIndex,
            });
            lastPreviewPath = path;
            lastPreviewPageIndex = pageIndex;
            lastPreview = preview;
            return preview;
        } catch (e) {
            console.warn('[PDF-FLOW] read_preview failed:', e);
            return null;
        }
    }

    function isPageProgress(pageIndex: number): boolean {
        const currentSession = deps.viewerSession.read();
        const currentPage = currentSession.currentPage;
        if (pageIndex === currentPage) {
            return true;
        }
        if (lastRenderedPageIndex === null) {
            return true;
        }
        const currentDist = Math.abs(currentPage - pageIndex);
        const renderedDist = Math.abs(currentPage - lastRenderedPageIndex);
        return currentDist < renderedDist;
    }

    const pagePresenter = createPagePresenter({
        getWrapper: deps.getWrapper,
        getRasterTarget: deps.getRasterTarget,
        getEmptyState: deps.getEmptyState,
        clearEditorOverlay: deps.clearEditorOverlay,
        isPageProgress,
    });

    function logRenderFlow(node: string, details: Record<string, unknown>): void {
        emitPdfDiagnostic('render-flow', node, details, { verboseOnly: true });
    }

    function shouldPresentPreviewFirst(renderReason: string | undefined): boolean {
        return renderReason === 'default' || renderReason === 'navigation';
    }

    async function runRenderLoop(initialFrame: RustRenderFrame | null): Promise<void> {
        let renderFrame = deps.framePlanAdapter.queueRenderLoopFrame(initialFrame);
        if (!renderFrame) {
            return;
        }

        while (renderFrame) {
            const currentFrame = renderFrame;
            const session = deps.viewerSession.read();
            let nextFrame: RustRenderFrame | null = null;
            if (!session.path) {
                nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            if (!deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken)) {
                logRenderFlow('stale.before-prepare', {
                    frameToken: currentFrame.frameToken,
                    reason: currentFrame.framePlan.renderReason,
                    displayZoom: currentFrame.framePlan.displayZoom,
                    renderZoom: currentFrame.framePlan.renderZoom,
                });
                nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            const targetPageIndex = session.currentPage;

            const indicator = deps.getPageIndicator();
            if (indicator) {
                indicator.textContent = `Page ${targetPageIndex + 1} / ${session.pageCount}`;
            }
            // Update the actual UI elements present in index.html
            const currentPageInput = document.getElementById('pdf-current-page-input') as HTMLInputElement | null;
            if (currentPageInput) {
                currentPageInput.value = String(targetPageIndex + 1);
            }
            const totalPagesSpan = document.getElementById('pdf-total-pages');
            if (totalPagesSpan) {
                totalPagesSpan.textContent = String(session.pageCount);
            }

            // Scanned PDF fast path: if the page is classified as scanned and has a ready preview image,
            // render it directly via updateRasterFallback and bypass Vello entirely.
            let preview: any = null;
            if (session.path) {
                preview = await getPagePreview(session.path, targetPageIndex);
            }

            // Check if page has progressed after fetching the preview
            if (!isPageProgress(targetPageIndex)) {
                logRenderFlow('stale.after-preview-fetch', {
                    frameToken: currentFrame.frameToken,
                    targetPageIndex,
                });
                nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            if (preview?.imageUrl && preview?.kind === 'scanned') {
                logRenderFlow('scanned-preview-fast-path', {
                    page: targetPageIndex,
                    hasImage: true,
                });
                const frameStartTs = performance.now();
                const width = preview.width || session.pageWidth;
                const height = preview.height || session.pageHeight;
                const renderPlan = currentFrame.framePlan;
                const transition = deps.commitRenderResult(
                    currentFrame.frameToken,
                    renderPlan.renderZoom,
                    width,
                    height,
                );

                if (transition?.accepted) {
                    if (isPageProgress(targetPageIndex)) {
                        deps.onPageDimensionsResolved(width, height);
                        const presented = await updateRasterFallback(preview.imageUrl, width, height, renderPlan.displayZoom, {
                            role: 'current',
                            pageIndex: targetPageIndex,
                        });
                        if (presented) {
                            lastVisibleSurface = 'raster';
                            lastRenderedPageIndex = targetPageIndex;
                            deps.clearPendingAnchor();
                            deps.clearEditorOverlay();
                            deps.showWrapper();
                            deps.commitRenderedFrame(renderPlan);
                            deps.onRenderCommitted();

                            const totalTime = performance.now() - frameStartTs;
                            emitPdfDiagnostic('PROF', 'page-render-duration-raster-visible', {
                                page: targetPageIndex,
                                totalTimeMs: totalTime,
                            });
                            emitPdfDiagnostic('PROF', 'page-render-duration-fast', {
                                page: targetPageIndex,
                                totalTimeMs: totalTime,
                            });
                        }
                    }
                }

                nextFrame =
                    transition?.nextFrame ??
                    (renderPlan.renderReason === 'zoom'
                        ? deps.scheduleRenderFollowUp(renderPlan.displayZoom)
                        : null) ??
                    null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            const renderPlan = currentFrame.framePlan;
            if (
                preview?.imageUrl &&
                shouldPresentPreviewFirst(renderPlan.renderReason) &&
                deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken)
            ) {
                if (isPageProgress(targetPageIndex)) {
                    const admission = deps.pagePresentationRuntime.admitPageAsset(
                        targetPageIndex,
                        'current',
                        'preview',
                    );
                    if (admission.accepted) {
                        const width = preview.width || session.pageWidth;
                        const height = preview.height || session.pageHeight;
                        deps.onPageDimensionsResolved(width, height);
                        // ready-only commit: 仅当 preview 已解码时立即提交，不等待 decode 阻塞 current 可见路径。
                        // miss 时打性能违规日志，vector 渲染继续在后台执行。
                        const readyPresented = pagePresenter.commitReadySurfaceOrFallback(
                            preview.imageUrl,
                            width,
                            height,
                            renderPlan.displayZoom,
                            {
                                hideVectorOnly: true,
                                role: 'preview',
                                pageIndex: targetPageIndex,
                            },
                        );
                        if (readyPresented) {
                            lastVisibleSurface = 'preview';
                            lastRenderedPageIndex = targetPageIndex;
                            deps.clearPendingAnchor();
                            deps.clearEditorOverlay();
                            deps.pagePresentationRuntime.markPageVisible(targetPageIndex, 'preview');
                            deps.commitRenderedFrame(renderPlan);
                            logRenderFlow('preview-first-frame.presented', {
                                page: targetPageIndex,
                                width,
                                height,
                                frameToken: currentFrame.frameToken,
                                renderReason: renderPlan.renderReason,
                                readyOnly: true,
                            });
                        } else {
                            void pagePresenter.presentRaster(
                                preview.imageUrl,
                                width,
                                height,
                                renderPlan.displayZoom,
                                {
                                    hideVectorOnly: true,
                                    role: 'preview',
                                    pageIndex: targetPageIndex,
                                }
                            ).then((presented) => {
                                if (presented) {
                                    if (isPageProgress(targetPageIndex)) {
                                        lastVisibleSurface = 'preview';
                                        lastRenderedPageIndex = targetPageIndex;
                                        deps.clearPendingAnchor();
                                        deps.clearEditorOverlay();
                                        deps.pagePresentationRuntime.markPageVisible(targetPageIndex, 'preview');
                                        deps.commitRenderedFrame(renderPlan);
                                    }
                                }
                            });
                        }
                    }
                }
            }

            // Check if page has progressed before starting Vello rendering
            if (!isPageProgress(targetPageIndex)) {
                logRenderFlow('stale.before-vector-render', {
                    frameToken: currentFrame.frameToken,
                    targetPageIndex,
                });
                nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            try {
                const frameStartTs = performance.now();
                logPdfLayoutTrace('render-loop.frame.begin', {
                    frameToken: currentFrame.frameToken,
                    framePlan: currentFrame.framePlan,
                    session,
                });
                ensureVectorHost();
                deps.showWrapper();

                if (renderPlan.prepareVisibleLayout === false) {
                    logRenderFlow('defer-visible-layout', {
                        frameToken: currentFrame.frameToken,
                        reason: renderPlan.renderReason,
                        displayZoom: renderPlan.displayZoom,
                        renderZoom: renderPlan.renderZoom,
                    });
                } else {
                    deps.prepareRenderFrame(currentFrame);
                }
                const result = await renderVectorPageWithPlan(
                    session.path,
                    targetPageIndex,
                    renderPlan,
                    currentFrame.frameToken,
                );
                if (result.aborted) {
                    nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                    renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                    continue;
                }
                // Check if page has progressed or if frame is stale after Vello rendering completes
                if (!deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken) || !isPageProgress(targetPageIndex)) {
                    logRenderFlow('stale.before-commit', {
                        frameToken: currentFrame.frameToken,
                        reason: renderPlan.renderReason,
                        displayZoom: renderPlan.displayZoom,
                        renderZoom: renderPlan.renderZoom,
                        isCurrent: deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken),
                        isProgress: isPageProgress(targetPageIndex),
                    });
                    nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                    renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                    continue;
                }
                const transition = deps.commitRenderResult(
                    currentFrame.frameToken,
                    renderPlan.renderZoom,
                    result.width,
                    result.height,
                );
                logPdfLayoutTrace('render-loop.commit-result.accepted', {
                    frameToken: currentFrame.frameToken,
                    renderPlan,
                    resultWidth: result.width,
                    resultHeight: result.height,
                    transition,
                });

                if (transition?.accepted) {
                    if (isPageProgress(targetPageIndex)) {
                        deps.onPageDimensionsResolved(result.width, result.height);
                        if (renderPlan.prepareVisibleLayout === false) {
                            logRenderFlow('commit.deferred-layout-first', {
                                frameToken: currentFrame.frameToken,
                                reason: renderPlan.renderReason,
                                displayZoom: renderPlan.displayZoom,
                                renderZoom: renderPlan.renderZoom,
                            });
                            logPdfLayoutTrace('render-loop.deferred.commit-layout.before', {
                                frameToken: currentFrame.frameToken,
                                renderPlan,
                            });
                            logPdfLayoutTrace('render-loop.deferred.present.before', {
                                frameToken: currentFrame.frameToken,
                                pendingPresentCount: result.pendingPresents?.length ?? 0,
                            });
                            commitVectorRenderResult(result, {
                                beforePresent: () => {
                                    deps.commitRenderedFrame(renderPlan);
                                    logPdfLayoutTrace('render-loop.deferred.commit-layout.after', {
                                        frameToken: currentFrame.frameToken,
                                        renderPlan,
                                    });
                                },
                            });
                            lastVisibleSurface = 'vector';
                            lastRenderedPageIndex = targetPageIndex;
                            logPdfLayoutTrace('render-loop.deferred.present.after', {
                                frameToken: currentFrame.frameToken,
                                pendingPresentCount: result.pendingPresents?.length ?? 0,
                            });
                        } else {
                            logPdfLayoutTrace('render-loop.commit-layout.before', {
                                frameToken: currentFrame.frameToken,
                                renderPlan,
                            });
                            logPdfLayoutTrace('render-loop.present.before', {
                                frameToken: currentFrame.frameToken,
                                pendingPresentCount: result.pendingPresents?.length ?? 0,
                            });
                            commitVectorRenderResult(result, {
                                beforePresent: () => {
                                    deps.commitRenderedFrame(renderPlan);
                                    logPdfLayoutTrace('render-loop.commit-layout.after', {
                                        frameToken: currentFrame.frameToken,
                                        renderPlan,
                                    });
                                },
                            });
                            lastVisibleSurface = 'vector';
                            lastRenderedPageIndex = targetPageIndex;
                            logPdfLayoutTrace('render-loop.present.after', {
                                frameToken: currentFrame.frameToken,
                                pendingPresentCount: result.pendingPresents?.length ?? 0,
                            });
                        }
                        deps.syncEditorOverlay(renderPlan.displayZoom);
                        deps.onRenderCommitted();

                        const totalTime = performance.now() - frameStartTs;
                        emitPdfDiagnostic('PROF', 'page-render-duration', {
                            page: targetPageIndex,
                            reason: renderPlan.renderReason,
                            totalTimeMs: totalTime,
                        });
                    }
                }

                nextFrame =
                    transition?.nextFrame ??
                    (renderPlan.renderReason === 'zoom'
                        ? deps.scheduleRenderFollowUp(renderPlan.displayZoom)
                        : null) ??
                    null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            } catch (error) {
                console.warn('[PDF-VECTOR] vector render failed, falling back to preview', error);
            }

            try {
                if (isPageProgress(targetPageIndex)) {
                    const preview = session.path ? await getPagePreview(session.path, targetPageIndex) : null;
                    if (preview?.imageUrl) {
                        const width = preview.width || session.pageWidth;
                        const height = preview.height || session.pageHeight;
                        const renderPlan = currentFrame.framePlan;
                        const transition = deps.commitRenderResult(
                            currentFrame.frameToken,
                            renderPlan.renderZoom,
                            width,
                            height,
                        );

                        if (transition?.accepted) {
                            if (isPageProgress(targetPageIndex)) {
                                deps.onPageDimensionsResolved(width, height);
                                const presented = await updateRasterFallback(preview.imageUrl, width, height, renderPlan.displayZoom, {
                                    role: 'current',
                                    pageIndex: targetPageIndex,
                                });
                                if (presented) {
                                    lastVisibleSurface = 'raster';
                                    lastRenderedPageIndex = targetPageIndex;
                                    deps.clearPendingAnchor();
                                    deps.clearEditorOverlay();
                                    deps.commitRenderedFrame(renderPlan);
                                }
                            }
                        }

                        nextFrame =
                            transition?.nextFrame ??
                            (renderPlan.renderReason === 'zoom'
                                ? deps.scheduleRenderFollowUp(renderPlan.displayZoom)
                                : null) ??
                            null;
                        renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                        continue;
                    }
                }
            } catch (error) {
                console.error('[PDF-VECTOR] preview fallback failed', error);
            }

            nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
            renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
        }
    }

    async function updateRasterFallback(
        src: string,
        pageWidth?: number,
        pageHeight?: number,
        displayZoom?: number,
        options: {
            hideVectorOnly?: boolean;
            role?: 'current' | 'preview' | 'prefetch' | 'unknown';
            pageIndex?: number;
        } = {},
    ): Promise<boolean> {
        const presented = await pagePresenter.presentRaster(
            src,
            pageWidth,
            pageHeight,
            displayZoom,
            options,
        );
        logRenderFlow('raster-present.result', {
            role: options.role ?? 'current',
            pageIndex: options.pageIndex,
            presented,
        });
        return presented;
    }

    async function renderCurrentPage(renderReason: RenderReason = 'default'): Promise<void> {
        await executeActualRender(renderReason);
    }

    async function executeActualRender(renderReason: RenderReason): Promise<void> {
        let session = deps.viewerSession.read();
        if (session.path) {
            const preview = await getPagePreview(session.path, session.currentPage);
            if (preview?.width && preview?.height) {
                deps.onPageDimensionsResolved(preview.width, preview.height);
                session = deps.viewerSession.read();
            }
        }
        const plan = session.path
            ? deps.framePlanAdapter.peek(session.currentZoom, renderReason)
            : null;
        const scheduled = session.path
            ? deps.framePlanAdapter.scheduleRender(session.currentZoom, renderReason)
            : null;
        logRenderFlow('render-current-page.scheduled', {
            hasPath: !!session.path,
            page: session.currentPage,
            pageCount: session.pageCount,
            zoom: session.currentZoom,
            scheduled: !!scheduled,
            reason: renderReason,
            planJson: JSON.stringify(plan),
            scheduledJson: JSON.stringify(scheduled),
        });
        if (plan) {
            logRenderFlow('render-current-page.plan_layers', {
                renderBaseLayer: plan.renderBaseLayer,
                renderDetailLayer: plan.renderDetailLayer,
                previewSettled: plan.previewSettled,
                useViewportTile: plan.useViewportTile,
                reuseActiveBaseLayer: plan.reuseActiveBaseLayer,
                reuseActiveDetailTile: plan.reuseActiveDetailTile,
            });
        }
        lastVisibleSurface = null;
        lastRenderedPageIndex = null;
        await runRenderLoop(scheduled);
    }

    async function presentPagePreview(pageIndex: number): Promise<boolean> {
        const session = deps.viewerSession.read();
        if (!session.path) return false;

        const preview = await getPagePreview(session.path, pageIndex);
        if (!isPageProgress(pageIndex)) return false;
        if (!preview?.imageUrl) return false;

        const width = preview.width || session.pageWidth;
        const height = preview.height || session.pageHeight;
        deps.onPageDimensionsResolved(width, height);

        const presented = await pagePresenter.presentRaster(
            preview.imageUrl,
            width,
            height,
            session.currentZoom,
            {
                hideVectorOnly: true,
                role: 'preview',
                pageIndex: pageIndex,
            }
        );

        if (presented && isPageProgress(pageIndex)) {
            lastVisibleSurface = 'preview';
            lastRenderedPageIndex = pageIndex;
            deps.clearPendingAnchor();
            deps.clearEditorOverlay();
            deps.pagePresentationRuntime.markPageVisible(pageIndex, 'preview');
            const plan = deps.framePlanAdapter.peek(session.currentZoom, 'navigation');
            if (plan) {
                deps.commitRenderedFrame(plan);
            }
        }
        return presented;
    }

    return {
        renderCurrentPage,
        renderScheduledFrame: runRenderLoop,
        getLastVisibleSurface: () => lastVisibleSurface,
        getLastRenderedPageIndex: () => lastRenderedPageIndex,
        presentPagePreview,
    };
}

