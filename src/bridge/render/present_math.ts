// ─────────────────────────────────────────────────────────────────────────────
// Present math — THE single implementation of the unified present formula
// (ADR-0009 tiles, ADR-0010 main canvas, ADR-0024 detail overlay):
//
//     a surface painted in zoom-space Z maps a base-space rect into visual
//     space with  s = visualZoom / Z:
//         left = rect.left × s,   top = rect.top × s,   transform = scale(s)
//     (transform-origin 0 0)
//
// A page point q then lands at q × visualZoom on every zoom-driven surface.
// Until 2026-10-06 this formula lived as four inline copies (both owners +
// tile_layer ×2); the 2026-09-30 / 2026-10-03 double-exposure bugs were
// exactly copies disagreeing. Contract: src/__tests__/present_math.test.ts.
// ─────────────────────────────────────────────────────────────────────────────

export type BaseRect = {
    left: number;
    top: number;
    width: number;
    height: number;
};

/**
 * Validate the zoom pair and compute the base→visual scale. Returns null when
 * the mapping is undefined (non-finite or non-positive zoom) — callers skip
 * the style writes, matching the validation the inline copies performed.
 */
export function presentScale(visualZoom: number, surfaceZoom: number): number | null {
    if (!Number.isFinite(visualZoom) || visualZoom <= 0) return null;
    if (!Number.isFinite(surfaceZoom) || surfaceZoom <= 0) return null;
    const s = visualZoom / surfaceZoom;
    if (!Number.isFinite(s) || s <= 0) return null;
    return s;
}

/** Base-space rect corner → visual-space CSS length. */
export function visualOffset(base: number, s: number): string {
    return `${base * s}px`;
}

/** Compositor transform for the scale. */
export function visualTransform(s: number): string {
    // Identity is written as scale(1) — mathematically equal to 'none' and
    // keeps the writers single-valued (no transform-flapping between
    // representations).
    return `scale(${s})`;
}

export type MemoizedStyle = {
    /** Write unless the value string equals the last written one. */
    write(value: string): void;
    /** Force the next write to land even if the string coincides. */
    invalidate(): void;
};

/**
 * Style writer with last-value memo — the shared write-suppression mechanism
 * of CanvasTransformOwner (transform) and DetailOverlayOwner (left/top/
 * transform). Call `invalidate()` when something else touched the property in
 * the same turn (e.g. a presenter re-boxed the surface) so the next write
 * cannot be swallowed by a coincidental string equality.
 *
 * Writes go through direct property assignment — the same mechanism as the
 * original inline writers (the contract-test style stubs implement exactly
 * the property set trap, not setProperty).
 */
export function createMemoizedStyle(el: HTMLElement, property: string): MemoizedStyle {
    let lastWritten = '';
    const style = el.style as unknown as Record<string, string>;
    return {
        write(value: string): void {
            if (value === lastWritten) return;
            style[property] = value;
            lastWritten = value;
        },
        invalidate(): void {
            lastWritten = '';
        },
    };
}
