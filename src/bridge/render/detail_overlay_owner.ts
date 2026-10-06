// ─────────────────────────────────────────────────────────────────────────────
// DetailOverlayOwner — ADR-0024.
//
// SINGLE writer of the detail overlay's (backCanvas) visual-space mapping:
// left / top / transform. The third zoom-driven surface, finally under the
// same unified present formula as the main canvas (CanvasTransformOwner,
// ADR-0010) and the tiles (ADR-0009):
//
//     recorded:  patch rect (viewportLeft/Top/Width/Height) in the frame's
//                displayZoom space, + that displayZoom as the patch's zoom
//     per tick:  s = visualZoom / zoom
//                left = rect.left × s,  top = rect.top × s,
//                transform = scale(s)   (transform-origin 0 0)
//
// A page point q then lands at q × visualZoom on the patch — old-band patches
// stay aligned with the settled page instead of freezing at their commit-time
// box (the 2026-10-03 video: stale white patch sticking out beyond the page,
// persisting at rest until the next base present).
//
// Contract (see detail_overlay_owner.test.ts):
//   present(rect, zoom, visual) → record + write the mapping ATOMICALLY in
//                                 this call (ADR-0018 lesson: never let the
//                                 compositor see a re-boxed surface with a
//                                 stale transform)
//   sync(visual)                → per-tick re-write; no-op when untracked or
//                                 when the surface is hidden (visibility
//                                 guard — hidden surfaces must not be
//                                 resurrected by stale tracking)
//   reset()                     → drop tracking (hide paths / teardown)
//
// The pixel blit and the raw CSS box remain the presenter's duty
// (presentViewportCanvasFromSource); this owner re-derives the visual-space
// mapping in the same JS turn right after, and keeps it fresh every tick —
// mirroring how the main canvas presenter re-boxes and the transform owner
// re-applies (ADR-0010).
// ─────────────────────────────────────────────────────────────────────────────

import {
    createMemoizedStyle,
    presentScale,
    visualOffset,
    visualTransform,
    type BaseRect,
} from './present_math';

/** The patch rect in its frame's zoom space — the unified BaseRect shape. */
export type DetailOverlayRect = BaseRect;

export type DetailOverlayOwner = {
    /**
     * Record the patch just presented (rect in the frame's displayZoom space,
     * that zoom as the patch's space) and write the visual-space mapping for
     * the CURRENT visual zoom in the same call.
     */
    present(rect: DetailOverlayRect, zoom: number, visualZoom: number): void;
    /** Re-write left/top/transform for the latest visual zoom (per tick). */
    sync(visualZoom: number): void;
    /** Drop tracking — the surface was hidden or the host torn down. */
    reset(): void;
    /** Whether a patch is currently tracked. */
    readonly tracked: boolean;
    /** The zoom space the tracked patch was presented in (diagnostics). */
    readonly zoom: number;
};

export function createDetailOverlayOwner(canvas: HTMLCanvasElement): DetailOverlayOwner {
    let baseRect: DetailOverlayRect | null = null;
    let baseZoom = 0;
    const leftStyle = createMemoizedStyle(canvas, 'left');
    const topStyle = createMemoizedStyle(canvas, 'top');
    const transformStyle = createMemoizedStyle(canvas, 'transform');

    function isHidden(): boolean {
        return canvas.style.visibility === 'hidden' || canvas.style.opacity === '0';
    }

    function writeMapping(visualZoom: number): void {
        if (!baseRect) return;
        const s = presentScale(visualZoom, baseZoom);
        if (s === null) return;
        if (isHidden()) return;
        leftStyle.write(visualOffset(baseRect.left, s));
        topStyle.write(visualOffset(baseRect.top, s));
        transformStyle.write(visualTransform(s));
    }

    /** The base-space box: written once per present, never re-derived per tick. */
    function writeBaseBox(rect: DetailOverlayRect): void {
        canvas.style.width = `${rect.width}px`;
        canvas.style.height = `${rect.height}px`;
    }

    return {
        present(rect: DetailOverlayRect, zoom: number, visualZoom: number): void {
            if (
                !Number.isFinite(zoom) || zoom <= 0 ||
                !Number.isFinite(rect.left) || !Number.isFinite(rect.top) ||
                !Number.isFinite(rect.width) || rect.width <= 0 || !Number.isFinite(rect.height)
            ) {
                return;
            }
            baseRect = { left: rect.left, top: rect.top, width: rect.width, height: rect.height };
            baseZoom = zoom;
            writeBaseBox(baseRect);
            // Invalidate the memos so the writes land even if the strings
            // coincide with the previous patch's.
            leftStyle.invalidate();
            topStyle.invalidate();
            transformStyle.invalidate();
            if (Number.isFinite(visualZoom) && visualZoom > 0) {
                writeMapping(visualZoom);
            }
        },
        sync(visualZoom: number): void {
            if (!baseRect) return;
            writeMapping(visualZoom);
        },
        reset(): void {
            baseRect = null;
            baseZoom = 0;
            leftStyle.invalidate();
            topStyle.invalidate();
            transformStyle.invalidate();
        },
        get tracked(): boolean {
            return baseRect !== null;
        },
        get zoom(): number {
            return baseZoom;
        },
    };
}

// ─── Element-keyed singleton ─────────────────────────────────────────────────
// The owner is bound to the back canvas element. The commit path (vector_host),
// the hide paths (vector_canvas_host) and the tile tick (tile_layer) must share
// ONE instance per element — same pattern as the main canvas transform owner.

const detailOverlayOwners = new WeakMap<HTMLCanvasElement, DetailOverlayOwner>();

export function getDetailOverlayOwner(canvas: HTMLCanvasElement): DetailOverlayOwner {
    let owner = detailOverlayOwners.get(canvas);
    if (!owner) {
        owner = createDetailOverlayOwner(canvas);
        detailOverlayOwners.set(canvas, owner);
    }
    return owner;
}

/** Drop the owner (and its tracking) for this element — hide paths / teardown. */
export function resetDetailOverlayOwner(canvas: HTMLCanvasElement): void {
    detailOverlayOwners.get(canvas)?.reset();
    detailOverlayOwners.delete(canvas);
}
