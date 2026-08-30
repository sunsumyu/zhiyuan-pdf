/**
 * Zoom controller — simplified for Rust-driven RAF architecture.
 *
 * The Rust RAF loop (raf_loop.rs) handles:
 *   - Animation state machine (advance_zoom_animation_state)
 *   - CSS transform application (web-sys)
 *   - Committed frame queue polling
 *   - Drawing delay after settle
 *
 * TS only needs to:
 *   1. Bind wheel events → Rust onWheelEvent()
 *   2. Push committed frames from render pipeline → Rust commitRenderedFrameToQueue()
 */

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

export type ZoomControllerDeps = {
    getCurrentPath: () => string | null;
    getZoomState: () => { targetZoom: number; visualZoom: number; lastRenderedZoom: number };
    getCurrentPageWidth: () => number;
    getCurrentPageHeight: () => number;
    getWrapper: () => HTMLElement | null;
    getScrollContainer: () => HTMLElement | null;
    getVectorContainer: () => HTMLElement | null;
    syncZoomSelect: () => void;
    requestRender: (reason?: 'default' | 'zoom' | 'editorVisibility' | 'documentMutation') => void;
    getMaxZoom: () => number;
    // New Rust-driven APIs
    startZoomRafLoop: () => void;
    stopZoomRafLoop: () => void;
    onWheelEvent: (input: WheelEventInput) => WheelEventOutput | null;
    commitRenderedFrameToQueue: (frame: RustAnchorFramePlan) => void;
    isImmediateMutationFrame: (renderReason: string) => boolean;
    /** Optional side-channel for zoom listeners (e.g. the tile layer). */
    onZoomGesture?: () => void;
};

type WheelEventInput = {
    deltaY: number;
    viewportX: number;
    viewportY: number;
    viewportWidth: number;
    viewportHeight: number;
    pageWidth: number;
    pageHeight: number;
    scrollLeft: number;
    scrollTop: number;
    timestampMs: number;
};

type WheelEventOutput = {
    targetZoom: number;
    visualZoom: number;
    cssScale: number;
};

export type ZoomController = {
    bindWheelZoom: () => void;
    commitRenderedFrame: (frame: RustAnchorFramePlan) => void;
    prepareImmediateRenderFrame: (frame: RustAnchorFramePlan) => void;
    // Legacy methods — delegated to Rust RAF loop
    clearPendingAnchor: () => void;
    resetVisualZoomPreview: () => void;
};

function isImmediateMutationFrame(frame: { renderReason?: string }): boolean {
    return frame.renderReason === 'editorVisibility' || frame.renderReason === 'documentMutation';
}

export function createZoomController(deps: ZoomControllerDeps): ZoomController {
    let wheelZoomBound = false;

    function bindWheelZoom(): void {
        if (wheelZoomBound) return;

        const scrollContainer = deps.getScrollContainer();
        if (!scrollContainer) {
            window.setTimeout(bindWheelZoom, 250);
            return;
        }

        // NOTE: the RAF loop is NOT started here. It self-stops shortly after
        // settle, so a loop started at bind time would die before the first
        // wheel event. `onWheelEvent` (Rust) restarts it on every gesture.

        scrollContainer.addEventListener('wheel', (event: WheelEvent) => {
            if (!(event.ctrlKey || event.metaKey) || !deps.getCurrentPath()) return;

            event.preventDefault();
            event.stopPropagation();
            event.stopImmediatePropagation();

            const rect = scrollContainer.getBoundingClientRect();

            // Collect raw DOM values — Rust does all computation
            const input: WheelEventInput = {
                deltaY: event.deltaY,
                viewportX: event.clientX - rect.left,
                viewportY: event.clientY - rect.top,
                viewportWidth: scrollContainer.clientWidth || rect.width || 0,
                viewportHeight: scrollContainer.clientHeight || rect.height || 0,
                pageWidth: deps.getCurrentPageWidth(),
                pageHeight: deps.getCurrentPageHeight(),
                scrollLeft: scrollContainer.scrollLeft,
                scrollTop: scrollContainer.scrollTop,
                timestampMs: performance.now(),
            };

            // Single WASM call — replaces 4-5 old calls
            const result = deps.onWheelEvent(input);
            // Wake zoom listeners (tile layer marks animation start/end).
            try { deps.onZoomGesture?.(); } catch {}

            deps.syncZoomSelect();
        }, { passive: false });

        wheelZoomBound = true;
    }

    function commitRenderedFrame(frame: RustAnchorFramePlan): void {
        logPdfLayoutTrace('zoom.commit-rendered-frame.received', {
            frame,
            immediateMutation: isImmediateMutationFrame(frame),
        });

        if (isImmediateMutationFrame(frame)) {
            // Immediate mutations bypass the RAF queue — apply directly via Rust
            // For now, push to queue and let Rust handle it
            deps.commitRenderedFrameToQueue(frame);
            return;
        }

        // Push to Rust committed frame queue — RAF loop will apply it
        deps.commitRenderedFrameToQueue(frame);
    }

    function prepareImmediateRenderFrame(frame: RustAnchorFramePlan): void {
        if (!isImmediateMutationFrame(frame)) return;
        if (frame.prepareVisibleLayout === false) return;

        logPdfLayoutTrace('zoom.prepare-immediate.before', {
            frame,
            zoomState: deps.getZoomState(),
        });

        // For immediate mutations, push to queue — Rust RAF will apply
        deps.commitRenderedFrameToQueue(frame);

        logPdfLayoutTrace('zoom.prepare-immediate.after', {
            frame,
            zoomState: deps.getZoomState(),
        });
    }

    function clearPendingAnchor(): void {
        // In the new architecture, the RAF loop handles anchor state.
        // This is a no-op for backward compatibility with renderFlow deps.
    }

    function resetVisualZoomPreview(): void {
        // In the new architecture, the RAF loop manages preview state.
        // Stop the RAF loop and reset zoom state via Rust.
        deps.stopZoomRafLoop();
    }

    return {
        bindWheelZoom,
        commitRenderedFrame,
        prepareImmediateRenderFrame,
        clearPendingAnchor,
        resetVisualZoomPreview,
    };
}
