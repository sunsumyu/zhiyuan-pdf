// ─────────────────────────────────────────────────────────────────────────────
// CanvasTransformOwner — ADR-0010.
//
// SINGLE writer of the main canvas transform and SINGLE source of truth for
// which zoom space the canvas CSS box is currently in (boxZoom).
//
// Why this exists: the box is re-set by the presenter at render-commit time to
// `page × displayZoom`, while commits can land WITHOUT a present (bitmap-reuse
// / skipped-render frames update lastRendered but leave the box in the older
// band). Any writer that guesses the box's space from lastRendered or
// renderZoom is intermittently wrong — the 2026-09-30 recordings (6-12%
// visual drift, settle-window offsets) are exactly that guessing failing.
//
// Contract (see canvas_transform_owner.test.ts):
//   presentFrame(displayZoom) → boxZoom = displayZoom; re-apply the last visual
//                               so the box swap and the transform land in the
//                               same JS turn (atomic, flicker-free)
//   sync(visualZoom)          → transform = scale(visualZoom / boxZoom)
//   reset()                   → boxZoom = 1
// Invariant under any interleaving: canvas visual width = page × visualZoom.
//
// The CSS width itself is set by the presenter (applyCanvasCssBox) — NOT here.
// ─────────────────────────────────────────────────────────────────────────────

import { createMemoizedStyle, presentScale, visualTransform } from './present_math';

export type CanvasTransformOwner = {
    /** Record the zoom space of the box the presenter just installed. */
    presentFrame(displayZoom: number): void;
    /** Re-compensate the transform to the latest visual zoom (per tick). */
    sync(visualZoom: number): void;
    /** Document/page switch: box returns to the unzoomed space. */
    reset(): void;
    /** Current box space (diagnostics + tile tick residency decisions). */
    readonly boxZoom: number;
};

export function createCanvasTransformOwner(canvas: HTMLCanvasElement): CanvasTransformOwner {
    let boxZoom = 1;
    let lastVisual = 0;
    const transformStyle = createMemoizedStyle(canvas, 'transform');

    function write(visualZoom: number): void {
        const s = presentScale(visualZoom, boxZoom);
        if (s === null) return;
        transformStyle.write(visualTransform(s));
    }

    return {
        presentFrame(displayZoom: number): void {
            boxZoom = Number.isFinite(displayZoom) && displayZoom > 0 ? displayZoom : 1;
            // Invalidate the memo: the box changed, the next write must land
            // even if the computed scale string coincides.
            transformStyle.invalidate();
            // Re-apply the last visual immediately: the presenter re-boxed the
            // canvas in the SAME JS turn, so recomputing the transform here
            // (rather than waiting for the next tick) keeps the on-screen size
            // continuous — no one-frame jump between box swap and transform.
            if (lastVisual > 0) write(lastVisual);
        },
        sync(visualZoom: number): void {
            if (!Number.isFinite(visualZoom) || visualZoom <= 0) return;
            lastVisual = visualZoom;
            write(visualZoom);
        },
        reset(): void {
            boxZoom = 1;
            transformStyle.invalidate();
        },
        get boxZoom(): number {
            return boxZoom;
        },
    };
}

// ─── Main-canvas singleton ────────────────────────────────────────────────────
// The owner is bound to the main canvas element, which is recreated whenever
// the vector host is torn down (document switch). A singleton keyed to the
// element id keeps the presenter and the tile tick on the SAME instance.

let mainCanvasOwner: { canvas: HTMLCanvasElement; owner: CanvasTransformOwner } | null = null;

export function getMainCanvasTransformOwner(canvas: HTMLCanvasElement): CanvasTransformOwner {
    if (mainCanvasOwner && mainCanvasOwner.canvas === canvas && canvas.isConnected) {
        return mainCanvasOwner.owner;
    }
    mainCanvasOwner = { canvas, owner: createCanvasTransformOwner(canvas) };
    return mainCanvasOwner.owner;
}

