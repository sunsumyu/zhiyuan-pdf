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

// ─── Strategy result ──────────────────────────────────────────────

type StrategyResult = {
    /** Whether this strategy handled the frame (true = loop continues with nextFrame). */
    handled: boolean;
    /** Next frame to render, or null to stop the loop. */
    nextFrame: RustRenderFrame | null;
};

// ─── Shared render context ────────────────────────────────────────

type RenderFlowContext = {
    deps: RenderFlowDeps;
    currentFrame: RustRenderFrame;
    session: { path: string | null; currentPage: number; pageCount: number; pageWidth: number; pageHeight: number; currentZoom: number };
    targetPageIndex: number;
    preview: any;
    pagePresenter: ReturnType<typeof createPagePresenter>;
    isPageProgress: (pageIndex: number) => boolean;
    logRenderFlow: (node: string, details: Record<string, unknown>) => void;
    /** Mutable state shared across strategies. */
    state: {
        lastVisibleSurface: VisibleSurface | null;
        lastRenderedPageIndex: number | null;
    };
};

// ─── Helper: compute next frame from transition ───────────────────

function nextFrameFromTransition(
    transition: RustRenderCommitResult | null,
    renderPlan: { renderReason?: string; displayZoom: number },
    deps: RenderFlowDeps,
): RustRenderFrame | null {
    return (
        transition?.nextFrame ??
        (renderPlan.renderReason === 'zoom'
            ? deps.scheduleRenderFollowUp(renderPlan.displayZoom)
            : null) ??
        null
    );
}

// ─── Strategy 1: Scanned PDF fast path ────────────────────────────
// If the page is classified as scanned and has a ready preview image,
// render it directly via raster and bypass Vello entirely.

async function strategyScannedFastPath(ctx: RenderFlowContext): Promise<StrategyResult> {
    const { deps, currentFrame, session, targetPageIndex, preview, pagePresenter, isPageProgress, logRenderFlow, state } = ctx;

    if (!preview?.imageUrl || preview?.kind !== 'scanned') {
        return { handled: false, nextFrame: null };
    }

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
            const presented = await pagePresenter.presentRaster(
                preview.imageUrl,
                width,
                height,
                renderPlan.displayZoom,
                { role: 'current', pageIndex: targetPageIndex },
            );
            if (presented) {
                state.lastVisibleSurface = 'raster';
                state.lastRenderedPageIndex = targetPageIndex;
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

    const nextFrame = nextFrameFromTransition(transition, renderPlan, deps);
    return { handled: true, nextFrame };
}

// ─── Strategy 2: Preview-first presentation ───────────────────────
// For default/navigation reasons, present the preview image immediately
// while Vello renders in the background.

async function strategyPreviewFirst(ctx: RenderFlowContext): Promise<StrategyResult> {
    const { deps, currentFrame, session, targetPageIndex, preview, pagePresenter, isPageProgress, logRenderFlow, state } = ctx;

    const renderPlan = currentFrame.framePlan;
    if (
        !preview?.imageUrl ||
        !(renderPlan.renderReason === 'default' || renderPlan.renderReason === 'navigation') ||
        !deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken)
    ) {
        return { handled: false, nextFrame: null };
    }

    if (!isPageProgress(targetPageIndex)) {
        return { handled: false, nextFrame: null };
    }

    const admission = deps.pagePresentationRuntime.admitPageAsset(
        targetPageIndex,
        'current',
        'preview',
    );
    if (!admission.accepted) {
        return { handled: false, nextFrame: null };
    }

    const width = preview.width || session.pageWidth;
    const height = preview.height || session.pageHeight;
    deps.onPageDimensionsResolved(width, height);

    // Ready-only commit: present immediately if preview is already decoded.
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
        state.lastVisibleSurface = 'preview';
        state.lastRenderedPageIndex = targetPageIndex;
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
        // Async decode — present when ready
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
                    state.lastVisibleSurface = 'preview';
                    state.lastRenderedPageIndex = targetPageIndex;
                    deps.clearPendingAnchor();
                    deps.clearEditorOverlay();
                    deps.pagePresentationRuntime.markPageVisible(targetPageIndex, 'preview');
                    deps.commitRenderedFrame(renderPlan);
                }
            }
        });
    }

    // Preview-first doesn't consume the frame — vector rendering continues below.
    return { handled: false, nextFrame: null };
}

// ─── Strategy 3: Vector (Vello) rendering ─────────────────────────
// Full WASM vector render with dual-layer support.

async function strategyVectorRender(ctx: RenderFlowContext): Promise<StrategyResult> {
    const { deps, currentFrame, session, targetPageIndex, isPageProgress, logRenderFlow, state } = ctx;

    const renderPlan = currentFrame.framePlan;
    const frameStartTs = performance.now();

    try {
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
            session.path!,
            targetPageIndex,
            renderPlan,
            currentFrame.frameToken,
        );

        if (result.aborted) {
            const nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
            return { handled: true, nextFrame };
        }

        // Staleness check after render
        if (!deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken) || !isPageProgress(targetPageIndex)) {
            logRenderFlow('stale.before-commit', {
                frameToken: currentFrame.frameToken,
                reason: renderPlan.renderReason,
                displayZoom: renderPlan.displayZoom,
                renderZoom: renderPlan.renderZoom,
                isCurrent: deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken),
                isProgress: isPageProgress(targetPageIndex),
            });
            const nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
            return { handled: true, nextFrame };
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

                // Commit layout + present vector result
                const beforePresentCb = () => {
                    deps.commitRenderedFrame(renderPlan);
                    const traceNode = renderPlan.prepareVisibleLayout === false
                        ? 'render-loop.deferred.commit-layout.after'
                        : 'render-loop.commit-layout.after';
                    logPdfLayoutTrace(traceNode, { frameToken: currentFrame.frameToken, renderPlan });
                };

                logPdfLayoutTrace(
                    renderPlan.prepareVisibleLayout === false
                        ? 'render-loop.deferred.present.before'
                        : 'render-loop.present.before',
                    { frameToken: currentFrame.frameToken, pendingPresentCount: result.pendingPresents?.length ?? 0 },
                );

                commitVectorRenderResult(result, { beforePresent: beforePresentCb });

                logPdfLayoutTrace(
                    renderPlan.prepareVisibleLayout === false
                        ? 'render-loop.deferred.present.after'
                        : 'render-loop.present.after',
                    { frameToken: currentFrame.frameToken, pendingPresentCount: result.pendingPresents?.length ?? 0 },
                );

                state.lastVisibleSurface = 'vector';
                state.lastRenderedPageIndex = targetPageIndex;
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

        const nextFrame = nextFrameFromTransition(transition, renderPlan, deps);
        return { handled: true, nextFrame };
    } catch (error) {
        console.warn('[PDF-VECTOR] vector render failed, falling back to preview', error);
        return { handled: false, nextFrame: null };
    }
}

// ─── Strategy 4: Raster fallback ──────────────────────────────────
// If vector rendering fails, fall back to preview raster image.

async function strategyRasterFallback(ctx: RenderFlowContext): Promise<StrategyResult> {
    const { deps, currentFrame, session, targetPageIndex, preview: initialPreview, isPageProgress, state } = ctx;

    if (!isPageProgress(targetPageIndex)) {
        return { handled: false, nextFrame: null };
    }

    try {
        const preview = session.path ? initialPreview : null;
        if (!preview?.imageUrl) {
            return { handled: false, nextFrame: null };
        }

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
                const presented = await ctx.pagePresenter.presentRaster(
                    preview.imageUrl,
                    width,
                    height,
                    renderPlan.displayZoom,
                    { role: 'current', pageIndex: targetPageIndex },
                );
                if (presented) {
                    state.lastVisibleSurface = 'raster';
                    state.lastRenderedPageIndex = targetPageIndex;
                    deps.clearPendingAnchor();
                    deps.clearEditorOverlay();
                    deps.commitRenderedFrame(renderPlan);
                }
            }
        }

        const nextFrame = nextFrameFromTransition(transition, renderPlan, deps);
        return { handled: true, nextFrame };
    } catch (error) {
        console.error('[PDF-VECTOR] preview fallback failed', error);
        return { handled: false, nextFrame: null };
    }
}

// ─── Main render loop ─────────────────────────────────────────────

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

    async function runRenderLoop(initialFrame: RustRenderFrame | null): Promise<void> {
        let renderFrame = deps.framePlanAdapter.queueRenderLoopFrame(initialFrame);
        if (!renderFrame) {
            return;
        }

        // Shared mutable state across strategies
        const sharedState = {
            lastVisibleSurface: null as VisibleSurface | null,
            lastRenderedPageIndex: null as number | null,
        };

        while (renderFrame) {
            const currentFrame = renderFrame;
            const session = deps.viewerSession.read();

            // ── Guard: no path ──
            if (!session.path) {
                const nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            // ── Guard: frame stale ──
            if (!deps.framePlanAdapter.isRenderFrameCurrent(currentFrame.frameToken)) {
                logRenderFlow('stale.before-prepare', {
                    frameToken: currentFrame.frameToken,
                    reason: currentFrame.framePlan.renderReason,
                    displayZoom: currentFrame.framePlan.displayZoom,
                    renderZoom: currentFrame.framePlan.renderZoom,
                });
                const nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            const targetPageIndex = session.currentPage;

            // ── Update page indicator ──
            const indicator = deps.getPageIndicator();
            if (indicator) {
                indicator.textContent = `Page ${targetPageIndex + 1} / ${session.pageCount}`;
            }
            const currentPageInput = document.getElementById('pdf-current-page-input') as HTMLInputElement | null;
            if (currentPageInput) {
                currentPageInput.value = String(targetPageIndex + 1);
            }
            const totalPagesSpan = document.getElementById('pdf-total-pages');
            if (totalPagesSpan) {
                totalPagesSpan.textContent = String(session.pageCount);
            }

            // ── Fetch preview ──
            const preview = await getPagePreview(session.path, targetPageIndex);

            // ── Guard: page progressed during preview fetch ──
            if (!isPageProgress(targetPageIndex)) {
                logRenderFlow('stale.after-preview-fetch', {
                    frameToken: currentFrame.frameToken,
                    targetPageIndex,
                });
                const nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            // ── Build render context ──
            const ctx: RenderFlowContext = {
                deps,
                currentFrame,
                session,
                targetPageIndex,
                preview,
                pagePresenter,
                isPageProgress,
                logRenderFlow,
                state: sharedState,
            };

            // ── Strategy 1: Scanned fast path ──
            const scanned = await strategyScannedFastPath(ctx);
            if (scanned.handled) {
                lastVisibleSurface = sharedState.lastVisibleSurface;
                lastRenderedPageIndex = sharedState.lastRenderedPageIndex;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(scanned.nextFrame);
                continue;
            }

            // ── Strategy 2: Preview-first (non-consuming) ──
            await strategyPreviewFirst(ctx);
            lastVisibleSurface = sharedState.lastVisibleSurface;
            lastRenderedPageIndex = sharedState.lastRenderedPageIndex;

            // ── Guard: page progressed before vector render ──
            if (!isPageProgress(targetPageIndex)) {
                logRenderFlow('stale.before-vector-render', {
                    frameToken: currentFrame.frameToken,
                    targetPageIndex,
                });
                const nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(nextFrame);
                continue;
            }

            // ── Strategy 3: Vector render ──
            const vector = await strategyVectorRender(ctx);
            if (vector.handled) {
                lastVisibleSurface = sharedState.lastVisibleSurface;
                lastRenderedPageIndex = sharedState.lastRenderedPageIndex;
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(vector.nextFrame);
                continue;
            }

            // ── Strategy 4: Raster fallback ──
            const raster = await strategyRasterFallback(ctx);
            lastVisibleSurface = sharedState.lastVisibleSurface;
            lastRenderedPageIndex = sharedState.lastRenderedPageIndex;
            if (raster.handled) {
                renderFrame = deps.framePlanAdapter.advanceRenderLoopFrame(raster.nextFrame);
                continue;
            }

            // ── Final fallback: abort ──
            const nextFrame = deps.framePlanAdapter.abortRender(currentFrame.frameToken)?.nextFrame ?? null;
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

    async function renderCurrentPage(renderReason: RenderReason = 'default', zoomOverride?: number): Promise<void> {
        await executeActualRender(renderReason, zoomOverride);
    }

    async function executeActualRender(renderReason: RenderReason, zoomOverride?: number): Promise<void> {
        let session = deps.viewerSession.read();
        if (session.path) {
            const preview = await getPagePreview(session.path, session.currentPage);
            if (preview?.width && preview?.height) {
                deps.onPageDimensionsResolved(preview.width, preview.height);
                session = deps.viewerSession.read();
            }
        }
        const effectiveZoom = Number.isFinite(zoomOverride)
            ? (zoomOverride as number)
            : session.currentZoom;
        const plan = session.path
            ? deps.framePlanAdapter.peek(effectiveZoom, renderReason)
            : null;
        const scheduled = session.path
            ? deps.framePlanAdapter.scheduleRender(effectiveZoom, renderReason)
            : null;
        logRenderFlow('render-current-page.scheduled', {
            hasPath: !!session.path,
            page: session.currentPage,
            pageCount: session.pageCount,
            zoom: effectiveZoom,
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
