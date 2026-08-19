import { logPdfLayoutTrace } from '../render/layout_trace';

type AnchorViewportLayout = {
    hostWidth: number;
    hostHeight: number;
    contentLeft: number;
    contentTop: number;
    scrollLeft: number;
    scrollTop: number;
};

type RustAnchorFramePlan = AnchorViewportLayout & {
    renderReason?: string;
    prepareVisibleLayout?: boolean;
    displayZoom: number;
    renderZoom: number;
    allowRenderDuringPreview?: boolean;
};

type RustWheelRenderDecision = {
    requestRenderNow: boolean;
    deferUntilSettled: boolean;
    skipRender: boolean;
};

type RustPreviewTickDecision = {
    continuePreview: boolean;
    flushCommittedFrame: boolean;
    requestRenderNow: boolean;
    keepWheelRenderPending: boolean;
};

type RustWheelZoomHostResult = {
    renderDecision: RustWheelRenderDecision;
};

type RustPreviewHostStepResult = {
    preview: {
        settled: boolean;
        visualZoom: number;
        renderedBaseZoom: number;
        cssScale: number;
        previewPresent?: {
            translateX: number;
            translateY: number;
            cssScale: number;
        };
        framePlan: RustAnchorFramePlan;
    };
    decision: RustPreviewTickDecision;
};

export type ZoomControllerDeps = {
    getCurrentPath: () => string | null;
    getZoomState: () => { targetZoom: number; visualZoom: number; lastRenderedZoom: number; };
    resetZoomPreviewState: () => void;
    getCurrentPageWidth: () => number;
    getCurrentPageHeight: () => number;
    getWrapper: () => HTMLElement | null;
    getScrollContainer: () => HTMLElement | null;
    getVectorContainer: () => HTMLElement | null;
    syncLayoutBox: (displayZoom: number, renderedZoom: number, layout?: AnchorViewportLayout | null) => void;
    syncZoomSelect: () => void;
    requestRender: (reason?: 'default' | 'zoom' | 'editorVisibility' | 'documentMutation') => void;
    peekFramePlan: (displayZoom: number) => RustAnchorFramePlan | null;
    takeFramePlan: (displayZoom: number) => RustAnchorFramePlan | null;
    getMaxZoom: () => number;
    clearPendingAnchor: () => void;
    clearPreviewPresent: () => void;
    resolveWheelRenderDecision: (request: Record<string, boolean | number>) => RustWheelRenderDecision | null;
    handleWheelZoomHost: (displayZoom: number, wheelRequest: Record<string, number>) => RustWheelZoomHostResult | null;
    stepPreviewHost: (displayZoom: number, timestampMs?: number) => RustPreviewHostStepResult | null;
    setWheelRenderPending: (pending: boolean) => void;
    getWheelRenderPending: () => boolean;
    queueCommittedFrame: (frame: RustAnchorFramePlan) => void;
    takeReadyCommittedFrame: () => RustAnchorFramePlan | null;
};

export type ZoomController = {
    bindWheelZoom: () => void;
    resetVisualZoomPreview: () => void;
    applyVisualZoomPreview: (previewZoom: number) => void;
    prepareImmediateRenderFrame: (frame: RustAnchorFramePlan) => void;
    commitRenderedFrame: (frame: RustAnchorFramePlan) => void;
    restorePendingAnchor: (targetZoom: number) => void;
    clearPendingAnchor: () => void;
};

export function createZoomController(deps: ZoomControllerDeps): ZoomController {
    let wheelZoomBound = false;
    let wheelZoomRafId: number | null = null;
    let wheelZoomRenderTimerId: number | null = null;

    function isImmediateMutationFrame(frame: { renderReason?: string }): boolean {
        return frame.renderReason === 'editorVisibility' || frame.renderReason === 'documentMutation';
    }

    function stopSmoothZoomPreview(): void {
        if (wheelZoomRafId !== null) {
            window.cancelAnimationFrame(wheelZoomRafId);
            wheelZoomRafId = null;
        }
        // Safety: clear any leftover preview CSS scale when the tick loop
        // is cancelled externally (e.g. by commitRenderedFrame).
        const container = deps.getVectorContainer();
        if (container) {
            container.style.transform = '';
        }
    }

    function applyCommittedFrame(frame: AnchorViewportLayout & { displayZoom: number; renderZoom: number; }): void {
        const container = deps.getVectorContainer();
        const scrollContainer = deps.getScrollContainer();
        if (!container || !scrollContainer) return;

        const zs = deps.getZoomState();
        logPdfLayoutTrace('zoom.apply-committed-frame.before', {
            frame,
            zoomState: zs,
        });
        container.style.transformOrigin = '0 0';
        container.style.transition = '';
        deps.clearPreviewPresent();
        deps.syncLayoutBox(frame.displayZoom, frame.renderZoom, frame);

        // Always clear any leftover preview CSS scale.  When the preview
        // loop is still active, re-apply the visual offset so the user
        // sees no jump; otherwise leave the transform at identity.
        container.style.transform = '';
        if (wheelZoomRafId !== null) {
            const baseZoom = frame.renderZoom > 0 ? frame.renderZoom : 1.0;
            const cssScale = zs.visualZoom / baseZoom;
            if (Math.abs(cssScale - 1.0) >= 0.001) {
                container.style.transform = `scale(${cssScale})`;
            }
        }

        scrollContainer.scrollLeft = frame.scrollLeft;
        scrollContainer.scrollTop = frame.scrollTop;
        deps.clearPendingAnchor();

        logPdfLayoutTrace('zoom.apply-committed-frame.after', {
            frame,
            zoomState: deps.getZoomState(),
        });
    }

    function flushCommittedFrameIfSettled(): boolean {
        const pendingCommittedFrame = deps.takeReadyCommittedFrame();
        if (!pendingCommittedFrame) return false;

        // When the committed frame's renderZoom is far from the settled
        // targetZoom, applying it would strand a blurry bitmap under a
        // stale CSS scale (e.g. bitmap at zoom 0.63 displayed at zoom 1.0
        // with scale(1.6)).  Skip the flush and force a fresh settle
        // render at the correct zoom instead.  Using 'documentMutation'
        // bypasses the wasm reuse check (stable_document_frame=true).
        const zoomState = deps.getZoomState();
        const frameZoom = pendingCommittedFrame.renderZoom;
        const settledZoom = zoomState.targetZoom;
        if (Math.abs(frameZoom - settledZoom) / Math.max(settledZoom, 0.01) > 0.10) {
            logPdfLayoutTrace('zoom.flush-settled.skip-stale', {
                frameZoom,
                settledZoom,
                zoomState,
            });
            deps.setWheelRenderPending(false);
            window.requestAnimationFrame(() => {
                deps.requestRender('documentMutation');
            });
            return false;
        }

        applyCommittedFrame(pendingCommittedFrame);
        return true;
    }

    function applyVisualZoomPreview(previewZoom: number, options?: { skipTransform?: boolean }): void {
        const container = deps.getVectorContainer();
        const scrollContainer = deps.getScrollContainer();
        if (!container) return;

        logPdfLayoutTrace('zoom.preview.apply.before', {
            previewZoom,
            zoomState: deps.getZoomState(),
        });
        const baseZoom = deps.getZoomState().lastRenderedZoom > 0 ? deps.getZoomState().lastRenderedZoom : 1.0;
        const cssScale = previewZoom / baseZoom;
        const anchorLayout = deps.peekFramePlan(previewZoom);
        deps.syncLayoutBox(previewZoom, baseZoom, anchorLayout);

        // When the preview loop is active, skipTransform should be true —
        // the preview loop (applyPreviewFrame) manages the CSS transform.
        // Only set the transform when the preview is stopping (error path)
        // or there is no active preview.
        if (!options?.skipTransform) {
            if (Math.abs(cssScale - 1.0) < 0.001) {
                container.style.transform = '';
            } else {
                container.style.transform = `scale(${cssScale})`;
            }
        }

        if (scrollContainer) {
            if (anchorLayout) {
                scrollContainer.scrollLeft = anchorLayout.scrollLeft;
                scrollContainer.scrollTop = anchorLayout.scrollTop;
            }
        }
        logPdfLayoutTrace('zoom.preview.apply.after', {
            previewZoom,
            baseZoom,
            cssScale,
            anchorLayout,
            zoomState: deps.getZoomState(),
        });
    }

    function applyPreviewFrame(preview: { visualZoom: number; renderedBaseZoom: number; cssScale: number; framePlan: AnchorViewportLayout & { displayZoom: number; renderZoom: number; }; }): void {
        const container = deps.getVectorContainer();
        if (!container) return;
        logPdfLayoutTrace('zoom.preview-frame.apply.before', {
            preview,
            zoomState: deps.getZoomState(),
        });
        const previewPresent = (preview as any).previewPresent || {
            translateX: 0,
            translateY: 0,
            cssScale: preview.cssScale,
        };
        const translateX = Number.isFinite(previewPresent.translateX) ? previewPresent.translateX : 0;
        const translateY = Number.isFinite(previewPresent.translateY) ? previewPresent.translateY : 0;
        const cssScale = Number.isFinite(previewPresent.cssScale) ? previewPresent.cssScale : preview.cssScale;

        if (
            Math.abs(cssScale - 1.0) < 0.001 &&
            Math.abs(translateX) < 0.01 &&
            Math.abs(translateY) < 0.01
        ) {
            container.style.transform = '';
        } else {
            container.style.transform = `translate3d(${translateX}px, ${translateY}px, 0) scale(${cssScale})`;
        }

        logPdfLayoutTrace('zoom.preview-frame.apply.after', {
            preview,
            translateX,
            translateY,
            cssScale,
            zoomState: deps.getZoomState(),
        });
    }

    function resetVisualZoomPreview(): void {
        const container = deps.getVectorContainer();
        logPdfLayoutTrace('zoom.preview.reset.before', {
            zoomState: deps.getZoomState(),
        });
        if (container) {
            container.style.transform = '';
            container.style.transformOrigin = '0 0';
        }
        deps.clearPreviewPresent();
        deps.resetZoomPreviewState();
        stopSmoothZoomPreview();
        if (wheelZoomRenderTimerId !== null) {
            window.clearTimeout(wheelZoomRenderTimerId);
            wheelZoomRenderTimerId = null;
        }
        logPdfLayoutTrace('zoom.preview.reset.after', {
            zoomState: deps.getZoomState(),
        });
    }

    function startSmoothZoomPreview(): void {
        if (wheelZoomRafId !== null) return;

        const tick = (timestampMs: number) => {
            try {
                const container = deps.getVectorContainer();
                if (!container) {
                    wheelZoomRafId = null;
                    return;
                }

                const zoomState = deps.getZoomState();
                let previewHostResult: RustPreviewHostStepResult | null = null;
                try {
                    previewHostResult = deps.stepPreviewHost(
                        zoomState.targetZoom,
                        timestampMs,
                    );
                } catch (error) {
                    console.error('[PDF-ZOOM] Rust preview frame failed, using host preview fallback', {
                        error,
                        targetZoom: zoomState.targetZoom,
                    });
                }
                if (!previewHostResult?.preview) {
                    // The wasm preview host failed or returned no preview data.
                    // Stop the preview loop and clear any CSS transform so the
                    // container shows the bitmap at its native size.
                    logPdfLayoutTrace('zoom.tick.error-path', {
                        reason: previewHostResult ? 'no-preview' : 'host-threw',
                        zoomState: deps.getZoomState(),
                    });
                    wheelZoomRafId = null;
                    deps.clearPreviewPresent();
                    deps.resetZoomPreviewState();
                    applyVisualZoomPreview(zoomState.targetZoom);
                    container.style.transform = '';
                    flushCommittedFrameIfSettled();
                    return;
                }

                const preview = previewHostResult.preview;
                applyPreviewFrame(preview);
                const tickDecision = previewHostResult.decision;
                if (tickDecision?.flushCommittedFrame) {
                    flushCommittedFrameIfSettled();
                }
                if (tickDecision?.requestRenderNow) {
                    window.requestAnimationFrame(() => {
                        deps.requestRender('zoom');
                    });
                }
                if (!tickDecision?.continuePreview) {
                    // The preview has settled.  Clear any leftover CSS scale
                    // from the preview loop so the container shows the bitmap
                    // at its native size.  Then schedule a settle render at the
                    // correct zoom so the bitmap matches the display.
                    container.style.transform = '';
                    container.style.transformOrigin = '0 0';
                    container.dataset.settledClear = String(Date.now());
                    if (!tickDecision?.requestRenderNow) {
                        window.requestAnimationFrame(() => {
                            deps.requestRender('default');
                        });
                    }
                    wheelZoomRafId = null;
                    return;
                }

                wheelZoomRafId = window.requestAnimationFrame((nextTimestampMs) => tick(nextTimestampMs));
            } catch (error) {
                // Safety net: if anything in the tick body throws (e.g.
                // applyPreviewFrame or flushCommittedFrameIfSettled), clear
                // the CSS transform so the container doesn't show a stale
                // preview scale, and stop the RAF loop.
                console.error('[PDF-ZOOM] tick error, clearing preview CSS', error);
                const container = deps.getVectorContainer();
                if (container) {
                    container.style.transform = '';
                }
                wheelZoomRafId = null;
            }
        };

        tick(performance.now());
    }

    function restorePendingAnchor(targetZoom: number): void {
        const scrollContainer = deps.getScrollContainer();
        if (!scrollContainer) return;
        const container = deps.getVectorContainer();
        const renderedZoom = deps.getZoomState().lastRenderedZoom > 0 ? deps.getZoomState().lastRenderedZoom : targetZoom;

        const nextLayout = deps.takeFramePlan(targetZoom);
        if (nextLayout) {
            deps.syncLayoutBox(targetZoom, renderedZoom, nextLayout);
            // syncLayoutBox no longer sets the CSS transform — apply it here.
            if (container) {
                const cssScale = targetZoom / renderedZoom;
                container.style.transform = Math.abs(cssScale - 1.0) < 0.001 ? '' : `scale(${cssScale})`;
            }
            scrollContainer.scrollLeft = nextLayout.scrollLeft;
            scrollContainer.scrollTop = nextLayout.scrollTop;
            return;
        }
    }

    function commitRenderedFrame(frame: AnchorViewportLayout & { displayZoom: number; renderZoom: number; }): void {
        const container = deps.getVectorContainer();
        if (!container) return;

        const zs = deps.getZoomState();
        logPdfLayoutTrace('zoom.commit-rendered-frame.received', {
            frame,
            immediateMutation: isImmediateMutationFrame(frame as any),
            zoomState: zs,
        });
        if (isImmediateMutationFrame(frame as any)) {
            stopSmoothZoomPreview();
            deps.resetZoomPreviewState();
            applyCommittedFrame(frame);
            return;
        }

        deps.queueCommittedFrame(frame);

        const zoomState = deps.getZoomState();
        if (Math.abs(zoomState.targetZoom - zoomState.visualZoom) < 0.001) {
            // Preview is settled.  If the committed frame was rendered at a
            // zoom far from the settled target (e.g. throttle rendered at
            // visualZoom=0.63 while settling at 1.0), applying it would
            // strand a blurry bitmap under a stale CSS scale.  Skip the
            // apply and force a fresh settle render at the correct zoom.
            const frameZoom = frame.renderZoom;
            const settledZoom = zoomState.targetZoom;
            if (Math.abs(frameZoom - settledZoom) / Math.max(settledZoom, 0.01) > 0.10) {
                logPdfLayoutTrace('zoom.commit-rendered-frame.skip-stale', {
                    frameZoom,
                    settledZoom,
                    zoomState,
                });
                deps.setWheelRenderPending(false);
                window.requestAnimationFrame(() => {
                    deps.requestRender('documentMutation');
                });
                return;
            }
            stopSmoothZoomPreview();
            deps.resetZoomPreviewState();
            applyCommittedFrame(frame);
            return;
        }

        // Preview is active: update container DOM to match committed zoom
        // so the preview loop's CSS scale calculation has correct base
        // dimensions. Without this, the container stays at the old
        // lastRenderedZoom size, causing a visible jump when the preview
        // loop computes cssScale = visualZoom / newLastRenderedZoom.
        deps.syncLayoutBox(frame.displayZoom, frame.renderZoom, frame);

        // Immediately resync the CSS scale to match the new renderZoom.
        // Before this fix, syncLayoutBox updated container dimensions to
        // renderZoom while the CSS transform still used the old
        // lastRenderedZoom as base: visual size = renderZoom * cssScale
        // = renderZoom * (visualZoom / oldRenderedZoom), which overshoots
        // when renderZoom > oldRenderedZoom (flash big) then corrects on
        // the next tick (flash small).  Applying the correct scale here
        // eliminates the one-frame gap.
        const baseZoom = frame.renderZoom > 0 ? frame.renderZoom : 1.0;
        const cssScale = zs.visualZoom / baseZoom;
        if (Math.abs(cssScale - 1.0) < 0.001) {
            container.style.transform = '';
        } else {
            container.style.transform = `scale(${cssScale})`;
        }

        startSmoothZoomPreview();
    }

    function prepareImmediateRenderFrame(frame: RustAnchorFramePlan): void {
        if (!isImmediateMutationFrame(frame)) return;
        if (frame.prepareVisibleLayout === false) return;
        const container = deps.getVectorContainer();
        const scrollContainer = deps.getScrollContainer();
        if (!container || !scrollContainer) return;

        logPdfLayoutTrace('zoom.prepare-immediate.before', {
            frame,
            zoomState: deps.getZoomState(),
        });
        stopSmoothZoomPreview();
        deps.resetZoomPreviewState();
        container.style.transformOrigin = '0 0';
        container.style.transition = '';
        container.style.transform = '';
        deps.syncLayoutBox(frame.displayZoom, frame.renderZoom, frame);
        scrollContainer.scrollLeft = frame.scrollLeft;
        scrollContainer.scrollTop = frame.scrollTop;
        deps.clearPendingAnchor();
        logPdfLayoutTrace('zoom.prepare-immediate.after', {
            frame,
            zoomState: deps.getZoomState(),
        });
    }

    function scheduleWheelZoomRender(): void {
        // Throttle, not debounce: once a render timer is scheduled, let it
        // fire on its own cadence instead of resetting on every wheel tick.
        // The old debounce cleared the timer on every wheel event, which
        // meant continuous scrolling never triggered a render — the bitmap
        // stayed at the old zoom while CSS preview scaled it further and
        // further (cssScale 1.0→1.6, density 0.62), producing severe
        // alternating blur/clear.  With throttle, a render fires at most
        // every delayMs (≈72 ms during preview), keeping lastRenderedZoom
        // close to visualZoom and cssScale near 1.0.
        if (wheelZoomRenderTimerId !== null) {
            return;
        }

        const zoomState = deps.getZoomState();
        const framePlan = deps.peekFramePlan(zoomState.targetZoom);
        const decision = deps.resolveWheelRenderDecision({
            targetZoom: zoomState.targetZoom,
            visualZoom: zoomState.visualZoom,
            lastRenderedZoom: zoomState.lastRenderedZoom,
            previewActive: wheelZoomRafId !== null,
            allowRenderDuringPreview: !!framePlan?.allowRenderDuringPreview,
        });
        const delayMs = Number.isFinite((decision as any)?.delayMs)
            ? Math.max(0, Number((decision as any).delayMs))
            : 96;

        wheelZoomRenderTimerId = window.setTimeout(() => {
            wheelZoomRenderTimerId = null;
            const zoomState = deps.getZoomState();
            const framePlan = deps.peekFramePlan(zoomState.targetZoom);
            const decision = deps.resolveWheelRenderDecision({
                targetZoom: zoomState.targetZoom,
                visualZoom: zoomState.visualZoom,
                lastRenderedZoom: zoomState.lastRenderedZoom,
                previewActive: wheelZoomRafId !== null,
                allowRenderDuringPreview: !!framePlan?.allowRenderDuringPreview,
            });
            if (decision?.skipRender) {
                // skipTransform: the preview loop manages the CSS transform.
                applyVisualZoomPreview(zoomState.targetZoom, { skipTransform: true });
                deps.setWheelRenderPending(false);
                return;
            }
            if (decision?.requestRenderNow) {
                deps.setWheelRenderPending(false);
                window.requestAnimationFrame(() => {
                    deps.requestRender('zoom');
                });
                return;
            }
            if (decision?.deferUntilSettled) {
                deps.setWheelRenderPending(true);
                return;
            }
            deps.setWheelRenderPending(false);
            window.requestAnimationFrame(() => {
                deps.requestRender('zoom');
            });
        }, delayMs);
    }

    function bindWheelZoom(): void {
        if (wheelZoomBound) return;

        const scrollContainer = deps.getScrollContainer();
        if (!scrollContainer) {
            window.setTimeout(bindWheelZoom, 250);
            return;
        }

        scrollContainer.addEventListener('wheel', (event: WheelEvent) => {
            if (!(event.ctrlKey || event.metaKey) || !deps.getCurrentPath()) return;

            const container = deps.getVectorContainer();
            if (!container) return;

            event.preventDefault();
            event.stopPropagation();
            event.stopImmediatePropagation();

            const rect = scrollContainer.getBoundingClientRect();
            const viewportX = event.clientX - rect.left;
            const viewportY = event.clientY - rect.top;
            const zoomState = deps.getZoomState();
            const currentDisplayZoom = zoomState.visualZoom > 0 ? zoomState.visualZoom : zoomState.targetZoom;
            const displayWidth = deps.getCurrentPageWidth() * currentDisplayZoom;
            const displayHeight = deps.getCurrentPageHeight() * currentDisplayZoom;
            const request = {
                deltaY: event.deltaY,
                viewportX,
                viewportY,
                viewportWidth: scrollContainer.clientWidth || rect.width || 0,
                viewportHeight: scrollContainer.clientHeight || rect.height || 0,
                pageWidth: deps.getCurrentPageWidth(),
                pageHeight: deps.getCurrentPageHeight(),
                scrollLeft: scrollContainer.scrollLeft,
                scrollTop: scrollContainer.scrollTop,
                contentWidth: displayWidth,
                contentHeight: displayHeight,
                targetZoom: zoomState.targetZoom,
                minZoom: 0.1,
                maxZoom: deps.getMaxZoom(),
            };
            const wheelHostResult = deps.handleWheelZoomHost(
                zoomState.targetZoom,
                request,
            );
            if (!wheelHostResult) {
                console.error('[PDF-ZOOM] Rust wheel host workflow failed', { request });
                return;
            }
            try {
                const decision = wheelHostResult.renderDecision;
                if (decision?.skipRender) {
                    deps.setWheelRenderPending(false);
                }
            } catch (error) {
                console.error('[PDF-ZOOM] Rust wheel host workflow failed', {
                    error,
                    request,
                });
                return;
            }

            container.style.transformOrigin = '0 0';
            deps.syncZoomSelect();
            startSmoothZoomPreview();
            scheduleWheelZoomRender();
        }, { passive: false });

        wheelZoomBound = true;
    }

    function clearPendingAnchor(): void {
        deps.clearPendingAnchor();
    }

    return {
        bindWheelZoom,
        resetVisualZoomPreview,
        applyVisualZoomPreview,
        prepareImmediateRenderFrame,
        commitRenderedFrame,
        restorePendingAnchor,
        clearPendingAnchor,
    };
}

