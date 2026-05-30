import { clearVectorHost, commitVectorRenderResult, ensureVectorHost, renderVectorPageWithPlan } from './vector_host';
import type { FramePlanAdapter, RenderReason, RustRenderCommitResult, RustRenderFrame } from './frame_plan';
import type { ViewerSessionAdapter } from '../viewer/viewer_session';
import { logPdfLayoutTrace } from './layout_trace';
import { emitPdfDiagnostic } from '../shared/diagnostics';

type RenderFlowDeps = {
    targetInvokeV3: (cmd: string, args: any) => Promise<any>;
    viewerSession: ViewerSessionAdapter;
    framePlanAdapter: FramePlanAdapter;
    clearPendingAnchor: () => void;
    commitRenderedFrame: (frame: { displayZoom: number; renderZoom: number; hostWidth: number; hostHeight: number; contentLeft: number; contentTop: number; scrollLeft: number; scrollTop: number; }) => void;
    getWrapper: () => HTMLElement | null;
    getRasterTarget: () => HTMLImageElement | null;
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

export function createRenderFlow(deps: RenderFlowDeps) {
    function logRenderFlow(node: string, details: Record<string, unknown>): void {
        emitPdfDiagnostic('render-flow', node, details, { verboseOnly: true });
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

            const indicator = deps.getPageIndicator();
            if (indicator) {
                indicator.textContent = `Page ${session.currentPage + 1} / ${session.pageCount}`;
            }
            // Update the actual UI elements present in index.html
            const currentPageInput = document.getElementById('pdf-current-page-input') as HTMLInputElement | null;
            if (currentPageInput) {
                currentPageInput.value = String(session.currentPage + 1);
            }
            const totalPagesSpan = document.getElementById('pdf-total-pages');
            if (totalPagesSpan) {
                totalPagesSpan.textContent = String(session.pageCount);
            }

            try {
                logPdfLayoutTrace('render-loop.frame.begin', {
                    frameToken: currentFrame.frameToken,
                    framePlan: currentFrame.framePlan,
                    session,
                });
                ensureVectorHost();
                console.log('[PDF-DIAG] render-loop: calling showWrapper, page=', session.currentPage, 'pageCount=', session.pageCount, 'path=', session.path?.substring(0, 30));
                deps.showWrapper();

                const renderPlan = currentFrame.framePlan;
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
                    session.currentPage,
                    renderPlan,
                    currentFrame.frameToken,
                );
                if (result.aborted) {
                    nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                    renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                    continue;
                }
                if (!deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken)) {
                    logRenderFlow('stale.before-commit', {
                        frameToken: currentFrame.frameToken,
                        reason: renderPlan.renderReason,
                        displayZoom: renderPlan.displayZoom,
                        renderZoom: renderPlan.renderZoom,
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
                        deps.commitRenderedFrame(renderPlan);
                        logPdfLayoutTrace('render-loop.deferred.commit-layout.after', {
                            frameToken: currentFrame.frameToken,
                            renderPlan,
                        });
                        logPdfLayoutTrace('render-loop.deferred.present.before', {
                            frameToken: currentFrame.frameToken,
                            pendingPresentCount: result.pendingPresents?.length ?? 0,
                        });
                        commitVectorRenderResult(result);
                        logPdfLayoutTrace('render-loop.deferred.present.after', {
                            frameToken: currentFrame.frameToken,
                            pendingPresentCount: result.pendingPresents?.length ?? 0,
                        });
                    } else {
                        logPdfLayoutTrace('render-loop.commit-layout.before', {
                            frameToken: currentFrame.frameToken,
                            renderPlan,
                        });
                        deps.commitRenderedFrame(renderPlan);
                        logPdfLayoutTrace('render-loop.commit-layout.after', {
                            frameToken: currentFrame.frameToken,
                            renderPlan,
                        });
                        logPdfLayoutTrace('render-loop.present.before', {
                            frameToken: currentFrame.frameToken,
                            pendingPresentCount: result.pendingPresents?.length ?? 0,
                        });
                        commitVectorRenderResult(result);
                        logPdfLayoutTrace('render-loop.present.after', {
                            frameToken: currentFrame.frameToken,
                            pendingPresentCount: result.pendingPresents?.length ?? 0,
                        });
                    }
                    deps.syncEditorOverlay(renderPlan.displayZoom);
                    deps.onRenderCommitted();
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
                const preview: any = await deps.targetInvokeV3('read_preview', {
                    path: session.path,
                    pageIndex: session.currentPage,
                });
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
                        deps.onPageDimensionsResolved(width, height);
                        await updateRasterFallback(preview.imageUrl, width, height, renderPlan.displayZoom);
                        deps.clearPendingAnchor();
                        deps.clearEditorOverlay();
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
            } catch (error) {
                console.error('[PDF-VECTOR] preview fallback failed', error);
            }

            nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
            renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
        }
    }

    async function updateRasterFallback(src: string, pageWidth?: number, pageHeight?: number, displayZoom?: number): Promise<void> {
        const img = deps.getRasterTarget();
        const wrapper = deps.getWrapper();
        const emptyState = deps.getEmptyState();
        if (!img || !wrapper) return;

        clearVectorHost();
        deps.clearEditorOverlay();
        img.style.display = 'block';
        img.src = src;

        if (pageWidth && pageWidth > 0 && pageHeight && pageHeight > 0) {
            const zoom = displayZoom && displayZoom > 0 ? displayZoom : 1;
            const cssW = Math.round(pageWidth * zoom);
            const cssH = Math.round(pageHeight * zoom);
            img.style.width = cssW + 'px';
            img.style.height = cssH + 'px';
            wrapper.style.width = cssW + 'px';
            wrapper.style.height = cssH + 'px';
        }

        wrapper.style.display = 'block';
        if (emptyState) emptyState.style.display = 'none';
    }

    async function renderCurrentPage(renderReason: RenderReason = 'default'): Promise<void> {
        const session = deps.viewerSession.read();
        console.log('[PDF-DIAG] renderFlow.renderCurrentPage: path=', session.path?.substring(0, 30) ?? 'NULL', 'pageCount=', session.pageCount, 'zoom=', session.currentZoom);
        const scheduled = session.path
            ? deps.framePlanAdapter.scheduleRender(session.currentZoom, renderReason)
            : null;
        console.log('[PDF-DIAG] renderFlow scheduled=', scheduled ? 'FRAME' : 'NULL');
        await runRenderLoop(scheduled);
    }

    return {
        renderCurrentPage,
        renderScheduledFrame: runRenderLoop,
    };
}

