// ─────────────────────────────────────────────────────────────────────────────
// TileLayer — DOM presentation of the Rust TileManager's tile stream.
//
// Responsibilities:
// - canvas pool with an LRU budget, reused across tiles
// - self-stopping RAF pump: drains TileManager render requests through the
//   vector worker (512px display-space regions) and presents each bitmap as a
//   canvas INSIDE the vector container
// - zoom-animation awareness: marks animation start/end on the TileManager
//   (eviction marking + settle scheduling) from the zoom state read per tick
// - gesture streaming (ADR-0009): feeds the quantized visual zoom to the
//   Rust incremental scheduler (update_animation → schedule_incremental_tiles
//   every 3rd frame) and pumps those requests DURING the gesture, so sharp
//   native-resolution tiles progressively cover the CSS-stretched canvas
//   instead of the user staring at a stretched bitmap until settle
//
// Geometry (tile_geometry.ts + ADR-0009 unified present formula): every tile
// is presented at its render zoom's display-space rect with
//   transform = scale(visualZoom / tile.renderZoom),  origin 0 0.
// A page point p lands at p × visualZoom on BOTH surfaces (tiles and the
// CSS-scaled main canvas) — old-zoom and current-zoom tiles coexist aligned,
// so there is no hide/clear switch anywhere in the gesture lifecycle.
// ─────────────────────────────────────────────────────────────────────────────

import { renderTileRegion } from './vector_host';
import { isAbortedRenderRequest } from './vector_page_bundle';
import { tileFacade, type TileRenderRequest } from './tile_bridge';
import {
    tileDisplayRect,
    tileKeyString,
} from './tile_geometry';
import { logPdfLayoutTrace } from './layout_trace';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import { getMainCanvasTransformOwner } from './canvas_transform_owner';
import { getViewportGeometry } from '../viewer/viewport_geometry';

/** Max tile canvases kept in the DOM at once (memory budget). */
const MAX_ACTIVE_TILES = 12;
/** Max spare canvases kept pooled for reuse. */
const MAX_POOL_SIZE = 12;
/** Zoom values closer than this are considered equal (tile key + settle). */
const ZOOM_EPS = 0.001;
/**
 * Zoom gap below which the tile layer starts rendering the target zoom,
 * instead of waiting for full settle. Tiles are rasterized at targetZoom, so
 * they are already sharp when the animation closes the remaining gap; starting
 * ~2% early lets the first tiles arrive before the wheel is released instead
 * of filling in one-by-one afterwards.
 */
const NEAR_SETTLE_EPS = 0.02;
/**
 * Gesture tile zoom quantization step (ADR-0009). The visual zoom fed to the
 * Rust incremental scheduler is quantized to multiples of this step, so tile
 * cache keys stay stable across the frames of one ~3% zoom band instead of
 * changing every frame (an unquantized f32 would produce a fresh key per
 * frame and the cache would never hit). It is also the tolerance used when
 * accepting in-flight requests against the current intent: a tile may be up
 * to one band behind the visual zoom and still be presented, compensated by
 * scale(visual/renderZoom) ≤ ~6% — inside the reknock blur budget.
 */
const GESTURE_TILE_ZOOM_STEP = 0.03;
/**
 * Scroll events throttled to at most one viewport reschedule per window.
 * Kept close to a single RAF frame (16ms): the tile pump itself is RAF-based
 * and cannot run faster than the display refresh, so a tighter throttle just
 * lets every scroll event wake the pump instead of dropping 80% of them.
 */
const SCROLL_THROTTLE_MS = 16;
/**
 * Viewport movement below this many display px does not reschedule.
 * Lowered from 24 to 4 so short scrolls (a few px of scroll-bounce, a
 * single-step trackpad flick) still trigger a tile refresh.
 */
const VIEWPORT_MOVE_EPS = 4;
/** Retry delay while the scroll container has not mounted yet. */
const BIND_RETRY_MS = 250;

export type TileZoomState = {
    targetZoom: number;
    visualZoom: number;
    lastRenderedZoom: number;
};

export type TileLayerDeps = {
    getZoomState: () => TileZoomState;
    getCurrentPath: () => string | null;
    getCurrentPage: () => number;
    getDocumentRevision: () => number;
    getPageWidth: () => number;
    getPageHeight: () => number;
    getScrollContainer: () => HTMLElement | null;
    getVectorContainer: () => HTMLElement | null;
    /** Main vector canvas — the element CanvasTransformOwner drives. */
    getMainCanvas: () => HTMLCanvasElement | null;
};

export type TileLayer = {
    /** Wheel gesture seen — wake the loop so animation state is marked. */
    notifyZoomGesture: () => void;
    /** Viewport/commit/scroll changed — wake the loop to reschedule tiles. */
    notifyViewportChanged: () => void;
    /**
     * ADR-0018: the zoom animation clock advanced `visualZoom` — re-sync the
     * canvas transform and tile present scales NOW, in the same JS turn.
     * Must stay O(active tiles): it runs once per animation frame.
     */
    onZoomAnimationFrame: () => void;
    /** Drop all presented tile canvases (document reset). */
    clear: () => void;
    /** Bind the throttled scroll listener (retries until container mounts). */
    bindScrollRefresh: () => void;
};

type ActiveTile = {
    canvas: HTMLCanvasElement;
    /** Zoom the bitmap was rasterized at (drives the present scale formula). */
    renderZoom: number;
    /**
     * Tile rect origin in the RENDER zoom's display space (512-grid coords).
     * The on-screen box is baseLeft × scale — the visual-space projection of
     * the render-space rect (ADR-0009 present formula: a page point p must
     * land at p × visualZoom on every surface).
     */
    baseLeft: number;
    baseTop: number;
};

/**
 * Single source of truth for "which zoom should tiles be rendered at right
 * now" (ADR-0009). All request-validity checks compare against this value
 * instead of inlining gesture branches at every check site: extending gesture
 * behavior means changing this one function, not every caller.
 *
 - Animating: quantized visual zoom — tiles stream at the zoom the user sees.
 - Settled: target zoom — the classic settle path, unchanged.
 */
function tileZoomIntent(zs: TileZoomState, animating: boolean): number {
    if (!animating) return zs.targetZoom;
    const raw = zs.visualZoom > 0 ? zs.visualZoom : zs.targetZoom;
    return Math.round(raw / GESTURE_TILE_ZOOM_STEP) * GESTURE_TILE_ZOOM_STEP;
}

export function createTileLayer(deps: TileLayerDeps): TileLayer {
    let host: HTMLElement | null = null;
    const pool: HTMLCanvasElement[] = [];
    const active = new Map<string, ActiveTile>(); // insertion order = LRU order

    let tileEpoch = 0;
    let rafHandle: number | null = null;
    let inFlight: Promise<unknown> | null = null;
    let animStarted = false;
    // Gesture streaming state (ADR-0009): the epoch bands the incremental
    // requests — Rust drops requests whose frame token no longer matches, so
    // the token may only advance when the quantized zoom band advances, never
    // per frame (a per-frame bump would invalidate every queued tile render).
    let gestureEpoch = 0;
    let lastGestureIntent: number | null = null;

    // What is currently presented / scheduled — staleness keys for the DOM.
    let presentedPage: number | null = null;
    let presentedZoom: number | null = null;
    let presentedRevision: number | null = null;
    let scheduledPage: number | null = null;
    let scheduledZoom: number | null = null;
    let scheduledRevision: number | null = null;
    let scheduledViewport: { x: number; y: number; w: number; h: number } | null = null;

    let scrollBound = false;
    let lastScrollKick = 0;

    function dpr(): number {
        return window.devicePixelRatio || 1;
    }

    function ensureHost(): HTMLElement | null {
        if (host && host.isConnected) return host;
        const container = deps.getVectorContainer();
        if (!container) return null;
        let el = document.getElementById('pdf-tile-layer');
        if (!el) {
            el = document.createElement('div');
            el.id = 'pdf-tile-layer';
            el.style.cssText = [
                'position: absolute',
                'inset: 0',
                'overflow: visible',
                'pointer-events: none',
                'z-index: 3',
            ].join(';');
            container.appendChild(el);
        }
        host = el;
        return host;
    }

    function createCanvas(): HTMLCanvasElement {
        const canvas = document.createElement('canvas');
        canvas.style.cssText = [
            'position: absolute',
            'display: none',
            'transform-origin: 0 0',
            'pointer-events: none',
        ].join(';');
        return canvas;
    }

    function acquireCanvas(key: string): HTMLCanvasElement | null {
        const layerHost = ensureHost();
        if (!layerHost) return null;
        const existing = active.get(key);
        if (existing) {
            // Refresh LRU position.
            active.delete(key);
            active.set(key, existing);
            return existing.canvas;
        }
        let canvas = pool.pop();
        if (!canvas) {
            canvas = createCanvas();
            layerHost.appendChild(canvas);
        }
        // Enforce the active budget — retire the least-recently-drawn tile.
        while (active.size >= MAX_ACTIVE_TILES) {
            const oldestKey = active.keys().next().value as string | undefined;
            if (oldestKey === undefined) break;
            const oldest = active.get(oldestKey)!;
            active.delete(oldestKey);
            if (pool.length < MAX_POOL_SIZE) {
                oldest.canvas.style.display = 'none';
                pool.push(oldest.canvas);
            } else {
                oldest.canvas.remove();
            }
        }
        active.set(key, { canvas, renderZoom: 1, baseLeft: 0, baseTop: 0 });
        return canvas;
    }

    /** @internal exported for tests */
    function clearDom(): void {
        for (const { canvas } of active.values()) {
            canvas.style.display = 'none';
            if (pool.length < MAX_POOL_SIZE) {
                pool.push(canvas);
            } else {
                canvas.remove();
            }
        }
        active.clear();
        presentedPage = null;
        presentedZoom = null;
        presentedRevision = null;
    }

    function clear(): void {
        clearDom();
        scheduledPage = null;
        scheduledZoom = null;
        scheduledRevision = null;
        scheduledViewport = null;
    }

    function dropActiveTile(key: string): void {
        const entry = active.get(key);
        if (!entry) return;
        active.delete(key);
        if (pool.length < MAX_POOL_SIZE) {
            entry.canvas.style.display = 'none';
            pool.push(entry.canvas);
        } else {
            entry.canvas.remove();
        }
    }

    function scheduleViewportTiles(zs: TileZoomState, page: number): void {
        const container = deps.getVectorContainer();
        const scroller = deps.getScrollContainer();
        if (!container || !scroller) return;
        const cRect = container.getBoundingClientRect();
        const sRect = scroller.getBoundingClientRect();
        if (cRect.width <= 0 || cRect.height <= 0) return;

        // Visible window of the page in display space (s ≈ 1 at settle, where
        // scheduling happens; container rect is post-transform = display).
        const vx = sRect.left - cRect.left;
        const vy = sRect.top - cRect.top;
        const vw = Math.max(1, sRect.width);
        const vh = Math.max(1, sRect.height);

        scheduledPage = page;
        scheduledZoom = zs.targetZoom;
        scheduledRevision = deps.getDocumentRevision();
        scheduledViewport = { x: vx, y: vy, w: vw, h: vh };
        tileEpoch += 1;
        tileFacade.updateViewport(page, zs.targetZoom, dpr(), vx, vy, vw, vh, tileEpoch);
        logPdfLayoutTrace('tile-layer.schedule-viewport', {
            page,
            zoom: zs.targetZoom,
            dpr: dpr(),
            vx,
            vy,
            vw,
            vh,
            epoch: tileEpoch,
        });
    }

    function drawTile(
        req: TileRenderRequest,
        bitmap: ImageBitmap,
        rect: { left: number; top: number; width: number; height: number },
        bitmapWidth: number,
        bitmapHeight: number,
    ): boolean {
        const zs = deps.getZoomState();
        const page = deps.getCurrentPage();
        const revision = deps.getDocumentRevision();
        const key = tileKeyString(
            req.tile_key.page,
            req.tile_key.zoom,
            req.tile_key.dpr,
            req.tile_key.x,
            req.tile_key.y,
        );
        const animating = animStarted;
        const intent = tileZoomIntent(zs, animating);

        // Page or document moved on while the render was in flight, the tile
        // schedule was never established, or the tile's zoom has fallen more
        // than one quantized band behind the current intent (ADR-0009) — the
        // bitmap is stale, do not present it.
        if (
            page !== scheduledPage ||
            revision !== scheduledRevision ||
            scheduledZoom === null ||
            Math.abs(req.tile_key.zoom - intent) > GESTURE_TILE_ZOOM_STEP + ZOOM_EPS
        ) {
            emitPdfDiagnostic('tile-layer', 'gesture-stream.draw-discard', {
                keyZoom: req.tile_key.zoom,
                intent,
                animating,
                scheduledZoom,
                scheduledPage,
                page,
                revision,
                scheduledRevision,
            });
            tileFacade.resetTile(
                req.tile_key.page,
                req.tile_key.zoom,
                req.tile_key.dpr,
                req.tile_key.x,
                req.tile_key.y,
            );
            return false;
        }

        const canvas = acquireCanvas(key);
        if (!canvas) {
            tileFacade.resetTile(
                req.tile_key.page,
                req.tile_key.zoom,
                req.tile_key.dpr,
                req.tile_key.x,
                req.tile_key.y,
            );
            return false;
        }
        canvas.width = bitmapWidth;
        canvas.height = bitmapHeight;
        const ctx = canvas.getContext('2d', { alpha: false });
        if (!ctx) return false;
        // Use high-quality smoothing for better visual quality
        ctx.imageSmoothingEnabled = true;
        ctx.imageSmoothingQuality = 'high';
        ctx.drawImage(bitmap, 0, 0, bitmapWidth, bitmapHeight);

        // ADR-0009 unified present formula — BOTH the position and the scale
        // must map the tile's render-space rect into visual space:
        //   left = rect.left × s,  top = rect.top × s,  transform = scale(s)
        // with s = visual / renderZoom. A page point p inside the tile lands
        // at left + (p×Zr − rect.left) × s = p × visual ONLY when the
        // translation term is included — without it every tile at x>0/y>0
        // drifts by 512 × (s − 1) px (the 2026-09-30 double-exposure video:
        // second-column content sheared over the first, "(Nacos, S|entineI)").
        const s = zs.visualZoom / req.tile_key.zoom;
        canvas.style.left = `${rect.left * s}px`;
        canvas.style.top = `${rect.top * s}px`;
        canvas.style.width = `${rect.width}px`;
        canvas.style.height = `${rect.height}px`;
        canvas.style.transform = `scale(${s})`;
        canvas.style.display = 'block';

        const entry = active.get(key);
        if (entry) {
            entry.renderZoom = req.tile_key.zoom;
            entry.baseLeft = rect.left;
            entry.baseTop = rect.top;
        }
        presentedPage = page;
        presentedZoom = req.tile_key.zoom;
        presentedRevision = revision;
        return true;
    }

    function pumpRequest(req: TileRenderRequest, zs: TileZoomState): void {
        const path = deps.getCurrentPath();
        const page = deps.getCurrentPage();
        if (!path) return;
        const key = req.tile_key;
        const animating = animStarted;
        const intent = tileZoomIntent(zs, animating);
        if (
            key.page !== page ||
            // Accept requests up to one quantized band behind the intent
            // (ADR-0009): a tile that started rendering one band ago is still
            // presentable with a ≤ ~6% scale compensation. Settled, the
            // intent IS the target zoom, restoring the old strict behavior.
            Math.abs(key.zoom - intent) > GESTURE_TILE_ZOOM_STEP + ZOOM_EPS ||
            Math.abs(key.dpr - dpr()) > 0.001
        ) {
            // Stale request (Rust filters by epoch; belt and braces).
            emitPdfDiagnostic('tile-layer', 'gesture-stream.pump-discard', {
                keyZoom: key.zoom,
                intent,
                animating,
                keyPage: key.page,
                page,
            });
            scheduleTick();
            return;
        }
        const pageW = deps.getPageWidth() * key.zoom;
        const pageH = deps.getPageHeight() * key.zoom;
        const rect = tileDisplayRect(key.x, key.y, key.zoom, deps.getPageWidth(), deps.getPageHeight());
        // Rust's near-viewport margin schedules tiles at negative indices and
        // beyond the right/bottom edges — skip every tile whose rect does not
        // actually intersect the page's display rect (they render as blank
        // white boxes floating outside the page otherwise).
        const intersectsPage =
            rect.left < pageW &&
            rect.top < pageH &&
            rect.left + rect.width > 0 &&
            rect.top + rect.height > 0;
        if (!intersectsPage) {
            scheduleTick();
            return;
        }
        const ratio = dpr();
        const bitmapWidth = Math.max(1, Math.round(rect.width * ratio));
        const bitmapHeight = Math.max(1, Math.round(rect.height * ratio));

        tileFacade.markRendering(key.page, key.zoom, key.dpr, key.x, key.y);
        inFlight = renderTileRegion({
            path,
            pageIndex: key.page,
            zoom: key.zoom,
            dpr: ratio,
            regionLeft: rect.left,
            regionTop: rect.top,
            regionWidth: rect.width,
            regionHeight: rect.height,
            bitmapWidth,
            bitmapHeight,
        })
            .then((bitmap) => {
                const drawn = drawTile(req, bitmap, rect, bitmapWidth, bitmapHeight);
                bitmap.close();
                if (drawn) {
                    tileFacade.markReady(key.page, key.zoom, key.dpr, key.x, key.y);
                }
            })
            .catch((err) => {
                const message = err instanceof Error ? err.message : String(err);
                if (isAbortedRenderRequest(message)) {
                    // ADR-0022: band/epoch advancement intentionally orphaned
                    // this in-flight render; vector_host resolves it as
                    // `aborted` and the tile is re-requested at the new band.
                    // Recovery, not failure — DEBUG keeps the error stream
                    // reserved for actionable faults (and skips the IPC).
                    emitPdfDiagnostic('tile-layer', 'tile-render-aborted', {
                        page: key.page,
                        x: key.x,
                        y: key.y,
                        reason: message,
                    }, { level: 'DEBUG' });
                } else {
                    emitPdfDiagnostic('tile-layer', 'tile-render-failed', {
                        page: key.page,
                        x: key.x,
                        y: key.y,
                        error: message,
                    });
                }
                tileFacade.resetTile(key.page, key.zoom, key.dpr, key.x, key.y);
            })
            .finally(() => {
                inFlight = null;
                scheduleTick();
            });
    }

    function scheduleTick(): void {
        if (rafHandle !== null) return;
        rafHandle = requestAnimationFrame(() => {
            rafHandle = null;
            tick();
        });
    }

    /**
     * Per-frame present refresh for already-presented tiles (ADR-0009). The
     * visual zoom changes every animation frame; a tile's on-screen box was
     * frozen at drawTile time, so position AND scale must be recomputed
     * together each tick or tiles drift out of alignment with the
     * per-frame-scaled main canvas (position drift = 512 × (s − 1) px for
     * tiles off the origin row/column). Compositor-only style writes, ≤ 12
     * elements.
     */
    function refreshTileTransforms(zs: TileZoomState): void {
        for (const { canvas, renderZoom, baseLeft, baseTop } of active.values()) {
            if (canvas.style.display === 'none') continue;
            const s = zs.visualZoom / renderZoom;
            canvas.style.left = `${baseLeft * s}px`;
            canvas.style.top = `${baseTop * s}px`;
            canvas.style.transform = `scale(${s})`;
        }
    }

    /**
     * ADR-0018: one per-frame visual-space sync for BOTH zoom-driven DOM
     * surfaces — the main canvas transform (CanvasTransformOwner, ADR-0010)
     * and the presented tiles' present scales (ADR-0009). Reading the zoom
     * state once per call and driving both surfaces from it keeps them on the
     * SAME visual zoom within a frame; splitting them (canvas here, tiles
     * there) would let the two surfaces disagree by one animation step at
     * gesture reversal.
     *
     * Called from two drivers: the zoom animation clock (Rust RAF knock →
     * `onZoomAnimationFrame`, AFTER the state advance — zero lag) and the tile
     * tick (scroll/commit-driven frames while the animation loop is parked).
     * The owner's memo makes same-turn duplicate calls no-ops.
     */
    function syncVisualTransforms(zs: TileZoomState): void {
        const mainCanvas = deps.getMainCanvas();
        if (mainCanvas) {
            getMainCanvasTransformOwner(mainCanvas).sync(zs.visualZoom);
        }
        refreshTileTransforms(zs);
    }

    /** Drain one render request from the TileManager queue (shared pump). */
    function pumpNext(zs: TileZoomState): void {
        if (!inFlight) {
            const req = tileFacade.nextRequest();
            if (req) {
                pumpRequest(req, zs);
                return;
            }
        }
        const stats = tileFacade.stats();
        if (inFlight || (stats && stats.queue_size > 0)) {
            scheduleTick();
        }
    }

    function tick(): void {
        const path = deps.getCurrentPath();
        if (!path) {
            clearDom();
            return;
        }
        const zs = deps.getZoomState();
        const page = deps.getCurrentPage();
        const revision = deps.getDocumentRevision();
        const zoomGap = Math.abs(zs.visualZoom - zs.targetZoom);
        const settled = zoomGap <= ZOOM_EPS;

        // ADR-0010/0018: per-frame visual-space sync of the main canvas
        // transform and the tile present scales. The animation clock knocks
        // `onZoomAnimationFrame` right after advancing visual zoom; this tick
        // covers the frames the animation loop is parked (scroll, commits).
        // Both drivers call the same sync so canvas and tiles always share
        // one visual zoom.
        syncVisualTransforms(zs);

        // Stale presentation — a commit landed at a different zoom, the page
        // turned, or the document mutated: drop DOM tiles before scheduling.
        // Only clear when the SCHEDULED zoom changed (tiles were rendered at
        // scheduledZoom), not when lastRenderedZoom changed (that's just
        // 簿记 from the render pipeline and doesn't invalidate existing tiles).
        if (presentedPage !== null && presentedPage !== page) {
            tileFacade.clearPage(presentedPage);
            clearDom();
        } else if (presentedRevision !== null && presentedRevision !== revision) {
            tileFacade.clearPage(page);
            clearDom();
        } else if (
            scheduledZoom !== null &&
            presentedZoom !== null &&
            Math.abs(scheduledZoom - presentedZoom) > ZOOM_EPS &&
            // ADR-0009: tiles presented within one quantized band of the new
            // target stay on screen through settle (their present scale is
            // ≈ 1 — sharp and aligned). Only a genuinely stale band clears.
            Math.abs(zs.targetZoom - presentedZoom) > GESTURE_TILE_ZOOM_STEP + ZOOM_EPS &&
            settled
        ) {
            clearDom();
        }

        let animJustEnded = false;
        if (zoomGap > NEAR_SETTLE_EPS) {
            // Gesture streaming (ADR-0009): the Rust incremental scheduler
            // (update_animation → schedule_incremental_tiles every 3rd frame)
            // produces viewport tiles at the quantized visual zoom; the pump
            // presents them progressively over the stretched canvas.
            if (!animStarted) {
                animStarted = true;
                tileFacade.startAnimation(zs.targetZoom);
                gestureEpoch = nextEpoch();
                lastGestureIntent = null;
            }
            const intent = tileZoomIntent(zs, true);
            if (intent !== lastGestureIntent) {
                // New zoom band → new frame token: queued requests for the
                // previous band are dropped by Rust, the fresh band renders.
                lastGestureIntent = intent;
                gestureEpoch = nextEpoch();
            }
            tileFacade.updateAnimation(intent, gestureEpoch);
            pumpNext(zs);
            // The streaming loop must stay resident for the whole gesture —
            // Rust only enqueues a band's tiles on every 3rd updateAnimation.
            scheduleTick();
            return;
        }
        // Near-settle: the animation is within ~2% of target, so targetZoom
        // tiles will still be valid when they present. Begin scheduling and
        // bumping the epoch here so the first tiles arrive BEFORE the user
        // sees settle, not after.
        if (animStarted) {
            animStarted = false;
            animJustEnded = true;
            tileFacade.endAnimation(nextEpoch());
        }

        // (Re)schedule when the schedule no longer matches intent: first open
        // (nothing scheduled yet), a settled zoom change, a page turn, a
        // document mutation, or a scroll/resize that moved the viewport.
        const scheduleStale =
            animJustEnded ||
            scheduledPage !== page ||
            scheduledZoom === null ||
            Math.abs(zs.targetZoom - scheduledZoom) > ZOOM_EPS ||
            scheduledRevision !== revision ||
            !scheduledViewport;
        if (scheduleStale) {
            scheduleViewportTiles(zs, page);
        } else {
            const container = deps.getVectorContainer();
            const scroller = deps.getScrollContainer();
            if (container && scroller) {
                const cRect = container.getBoundingClientRect();
                const sRect = scroller.getBoundingClientRect();
                if (cRect.width > 0 && cRect.height > 0) {
                    const vx = sRect.left - cRect.left;
                    const vy = sRect.top - cRect.top;
                    const vw = Math.max(1, sRect.width);
                    const vh = Math.max(1, sRect.height);
                    const sv = scheduledViewport;
                    const moved =
                        !!sv && (
                            Math.abs(vx - sv.x) > VIEWPORT_MOVE_EPS ||
                            Math.abs(vy - sv.y) > VIEWPORT_MOVE_EPS ||
                            Math.abs(vw - sv.w) > VIEWPORT_MOVE_EPS ||
                            Math.abs(vh - sv.h) > VIEWPORT_MOVE_EPS
                        );
                    if (moved) {
                        scheduleViewportTiles(zs, page);
                    }
                }
            }
        }

        pumpNext(zs);
        // ADR-0009: keep the tick resident while the animation is still
        // converging (visual → target inside the near-settle band). Tiles
        // presented at an older band ride on scale(visual / renderZoom), and
        // the visual zoom keeps changing until full settle — without this
        // residency their transforms freeze mid-convergence and the old-band
        // tiles drift out of alignment with the canvas for ~300ms.
        if (zoomGap > ZOOM_EPS) {
            scheduleTick();
        }
        // else: nothing to do — loop sleeps until the next wake event.
    }

    function nextEpoch(): number {
        tileEpoch += 1;
        return tileEpoch;
    }

    function wake(): void {
        scheduleTick();
    }

    function bindScrollRefresh(): void {
        if (scrollBound) return;
        const scroller = deps.getScrollContainer();
        if (!scroller) {
            window.setTimeout(bindScrollRefresh, BIND_RETRY_MS);
            return;
        }
        scroller.addEventListener(
            'scroll',
            () => {
                const now = performance.now();
                if (now - lastScrollKick < SCROLL_THROTTLE_MS) return;
                lastScrollKick = now;
                wake();
            },
            { passive: true },
        );
        // ADR-0014: a window resize changes the viewport geometry immediately
        // (the per-frame tick invalidate would also catch it one frame later).
        window.addEventListener(
            'resize',
            () => {
                getViewportGeometry().invalidate();
                wake();
            },
            { passive: true },
        );
        scrollBound = true;
    }

    return {
        notifyZoomGesture: wake,
        notifyViewportChanged: wake,
        onZoomAnimationFrame: () => syncVisualTransforms(deps.getZoomState()),
        clear,
        bindScrollRefresh,
    };
}
