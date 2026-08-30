// ─────────────────────────────────────────────────────────────────────────────
// Tile geometry — pure coordinate math for the zoom tile layer (ADR-0003/0004).
//
// Coordinate spaces:
// - Display space:  page × visualZoom  (tile grid lives here — 512px cells)
// - Layout space:   page × layoutZoom  (container box inside the DOM)
// - Visual space:   what the user sees; container transform scales layout
//                   space by cssScale s = visualZoom / layoutZoom.
//
// A tile element positioned INSIDE the container at layout-space coordinates
// appears at display-space coordinates after the container's scale(s):
//     layout_pos = display_pos / s
// so a 512px display-space tile must be a 512/s CSS-px element. At settle
// s → 1 (render tracks visual zoom) and elements sit at exactly 512 CSS px.
// ─────────────────────────────────────────────────────────────────────────────

/** Fixed tile size in display-space pixels (ADR-0003). */
export const TILE_SIZE = 512;

/** Minimum cssScale guard — avoids divide-by-zero on transient state. */
const MIN_CSS_SCALE = 0.0001;

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
 * @param cssScale visualZoom / layoutZoom (container transform scale)
 */
export function tileElementBox(tileX: number, tileY: number, cssScale: number): TileElementBox {
    const s = Number.isFinite(cssScale) && cssScale > MIN_CSS_SCALE ? cssScale : MIN_CSS_SCALE;
    const size = TILE_SIZE / s;
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
    const zoom = Number.isFinite(visualZoom) && visualZoom > MIN_CSS_SCALE ? visualZoom : MIN_CSS_SCALE;
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
