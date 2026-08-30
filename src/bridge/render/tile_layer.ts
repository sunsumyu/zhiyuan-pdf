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
//
// Geometry (tile_geometry.ts): the tile grid lives in display space
// (page × visualZoom); the container is laid out at layoutZoom and scaled by
// the single CSS transform s = visualZoom / layoutZoom. A tile element is
// positioned at display_pos / s inside the container so the container
// transform lands it exactly on its display-space rect (ADR-0002: tiles are
// children of the primary surface — they never write transforms themselves).
//
// Mid-gesture there is deliberately NO tile rendering: ADR-0004 (revised)
// handles animation via render-tracks-visual + reknock; tiles render at
// settle, where they are sharp at native resolution and never CSS-stretched.
// ─────────────────────────────────────────────────────────────────────────────

import { renderTileRegion } from './vector_host';
import { tileFacade, type TileRenderRequest } from './tile_bridge';
import {
    tileDisplayRect,
    tileKeyString,
} from './tile_geometry';
import { logPdfLayoutTrace } from './layout_trace';
import { emitPdfDiagnostic } from '../shared/diagnostics';

/** Max tile canvases kept in the DOM at once (memory budget). */
const MAX_ACTIVE_TILES = 12;
/** Max spare canvases kept pooled for reuse. */
const MAX_POOL_SIZE = 12;
/** Zoom values closer than this are considered equal (tile key + settle). */
const ZOOM_EPS = 0.001;
/** Scroll events throttled to at most one viewport reschedule per window. */
const SCROLL_THROTTLE_MS = 120;
/** Viewport movement below this many display px does not reschedule. */
const VIEWPORT_MOVE_EPS = 24;
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
};

export type TileLayer = {
    /** Wheel gesture seen — wake the loop so animation state is marked. */
    notifyZoomGesture: () => void;
    /** Viewport/commit/scroll changed — wake the loop to reschedule tiles. */
    notifyViewportChanged: () => void;
    /** Drop all presented tile canvases (document reset). */
    clear: () => void;
    /** Bind the throttled scroll listener (retries until container mounts). */
    bindScrollRefresh: () => void;
};

type ActiveTile = {
    canvas: HTMLCanvasElement;
};

function cssScaleOf(zs: TileZoomState): number {
    const visual = Math.max(zs.visualZoom, 0.0001);
    const layout = Math.max(zs.lastRenderedZoom, 0.0001);
    return visual / layout;
}

export function createTileLayer(deps: TileLayerDeps): TileLayer {
    let host: HTMLElement | null = null;
    const pool: HTMLCanvasElement[] = [];
    const active = new Map<string, ActiveTile>(); // insertion order = LRU order

    let tileEpoch = 0;
    let rafHandle: number | null = null;
    let inFlight: Promise<unknown> | null = null;
    let animStarted = false;

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
        active.set(key, { canvas });
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

    function isAnimating(zs: TileZoomState): boolean {
        return Math.abs(zs.visualZoom - zs.targetZoom) > ZOOM_EPS;
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

        // Page, zoom intent, or document moved on while the render was in
        // flight — the bitmap is stale, do not present it.
        if (
            page !== scheduledPage ||
            revision !== scheduledRevision ||
            scheduledZoom === null ||
            Math.abs(zs.targetZoom - scheduledZoom) > ZOOM_EPS
        ) {
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
        ctx.drawImage(bitmap, 0, 0, bitmapWidth, bitmapHeight);

        // Position in layout space (display / s); the container transform
        // scales it back to the display-space rect.
        const s = cssScaleOf(zs);
        canvas.style.left = `${rect.left / s}px`;
        canvas.style.top = `${rect.top / s}px`;
        canvas.style.width = `${rect.width / s}px`;
        canvas.style.height = `${rect.height / s}px`;
        canvas.style.display = 'block';

        presentedPage = page;
        presentedZoom = scheduledZoom;
        presentedRevision = revision;
        return true;
    }

    function pumpRequest(req: TileRenderRequest, zs: TileZoomState): void {
        const path = deps.getCurrentPath();
        const page = deps.getCurrentPage();
        if (!path) return;
        const key = req.tile_key;
        if (
            key.page !== page ||
            Math.abs(key.zoom - zs.targetZoom) > ZOOM_EPS ||
            Math.abs(key.dpr - dpr()) > 0.001
        ) {
            // Stale request (Rust filters by epoch; belt and braces).
            scheduleTick();
            return;
        }
        const pageW = deps.getPageWidth() * key.zoom;
        const pageH = deps.getPageHeight() * key.zoom;
        const rect = tileDisplayRect(key.x, key.y, key.zoom, deps.getPageWidth(), deps.getPageHeight());
        if (rect.left >= pageW || rect.top >= pageH) {
            // Entirely outside the page — nothing to render.
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
                emitPdfDiagnostic('tile-layer', 'tile-render-failed', {
                    page: key.page,
                    x: key.x,
                    y: key.y,
                    error: String(err),
                });
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

    function tick(): void {
        const path = deps.getCurrentPath();
        if (!path) {
            clearDom();
            return;
        }
        const zs = deps.getZoomState();
        const page = deps.getCurrentPage();
        const revision = deps.getDocumentRevision();

        // Stale presentation — a commit landed at a different zoom, the page
        // turned, or the document mutated: drop DOM tiles before scheduling.
        if (presentedPage !== null && presentedPage !== page) {
            tileFacade.clearPage(presentedPage);
            clearDom();
        } else if (presentedRevision !== null && presentedRevision !== revision) {
            tileFacade.clearPage(page);
            clearDom();
        } else if (
            presentedZoom !== null &&
            Math.abs(zs.lastRenderedZoom - presentedZoom) > ZOOM_EPS
        ) {
            clearDom();
        }

        let animJustEnded = false;
        if (isAnimating(zs)) {
            // Mid-gesture: mark state only; rendering waits for settle.
            if (!animStarted) {
                animStarted = true;
                tileFacade.startAnimation(zs.targetZoom);
            }
            scheduleTick();
            return;
        }
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
        scrollBound = true;
    }

    return {
        notifyZoomGesture: wake,
        notifyViewportChanged: wake,
        clear,
        bindScrollRefresh,
    };
}
