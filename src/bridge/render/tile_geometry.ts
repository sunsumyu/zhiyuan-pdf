// ─────────────────────────────────────────────────────────────────────────────
// Tile geometry — pure coordinate math for the zoom tile layer (ADR-0003/0004).
//
// Coordinate spaces:
// - Display space:  page × visualZoom  (tile grid lives here — 512px cells)
//
// Container dimensions are set directly via SetBox (no CSS transform).
// Tiles are positioned at display-space coordinates inside the container.
// ─────────────────────────────────────────────────────────────────────────────

/** Fixed tile size in display-space pixels (ADR-0003). */
export const TILE_SIZE = 512;

/** Zoom floor guard — keeps degenerate (zero/negative) zoom out of coordinate math. */
const MIN_ZOOM_GUARD = 0.0001;

export type TileElementBox = {
    left: number;
    top: number;
    width: number;
    height: number;
};

/**
 * Element box (CSS px) for a tile inside the vector container.
 * @param tileX tile column in the display-space grid
 * @param tileY tile row in the display-space grid
 */
export function tileElementBox(tileX: number, tileY: number): TileElementBox {
    const size = TILE_SIZE;
    return {
        left: tileX * size,
        top: tileY * size,
        width: size,
        height: size,
    };
}

/** Device-pixel bitmap edge for a tile canvas (square). */
export function tileBitmapSize(dpr: number): number {
    const ratio = Number.isFinite(dpr) && dpr > 0 ? dpr : 1;
    return Math.max(1, Math.round(TILE_SIZE * ratio));
}

export type TileDisplayRect = {
    left: number;
    top: number;
    width: number;
    height: number;
};

/**
 * Display-space rectangle a tile covers, clipped to the rendered page.
 * Tiles on the page's right/bottom edge shrink to the page boundary so the
 * region renderer never samples outside the page model.
 */
export function tileDisplayRect(
    tileX: number,
    tileY: number,
    visualZoom: number,
    pageWidth: number,
    pageHeight: number,
): TileDisplayRect {
    const zoom = Number.isFinite(visualZoom) && visualZoom > MIN_ZOOM_GUARD ? visualZoom : MIN_ZOOM_GUARD;
    const left = tileX * TILE_SIZE;
    const top = tileY * TILE_SIZE;
    const pageW = pageWidth * zoom;
    const pageH = pageHeight * zoom;
    const width = Math.min(TILE_SIZE, pageW - left);
    const height = Math.min(TILE_SIZE, pageH - top);
    return {
        left,
        top,
        width: Math.max(1, width),
        height: Math.max(1, height),
    };
}

/** Stable cache/identity key for a tile (mirrors Rust TileKey string form). */
export function tileKeyString(page: number, zoom: number, dpr: number, x: number, y: number): string {
    return `${page}|${zoom.toFixed(4)}|${dpr.toFixed(4)}|${x}|${y}`;
}
