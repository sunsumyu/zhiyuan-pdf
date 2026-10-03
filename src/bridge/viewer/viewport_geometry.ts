// ─────────────────────────────────────────────────────────────────────────────
// ViewportGeometry — the single owner of viewport geometry reads (ADR-0014).
//
// Why: `getBoundingClientRect` / `clientWidth` force a synchronous layout when
// styles are dirty. `frame_plan.buildRequest` runs ~6x per render and used to
// re-measure the scroll container every time — the CDP profile attributed ~48ms
// per gesture to that path alone (~100ms total for the gesture). The scroll
// container's size only changes on window resize or a sidebar toggle, so within
// a frame it must be measured once and shared.
//
// Owner contract:
//   - read()       — cached; measures only when invalidated. The ONLY geometry
//                    read on hot paths.
//   - invalidate() — mark dirty; the next read() re-measures.
//
// Frame boundary: the tile layer's rAF tick calls invalidate() at the top of
// every frame, and a window resize listener invalidates immediately, so a size
// change is absorbed within the same frame it happens.
// ─────────────────────────────────────────────────────────────────────────────

export type ViewportGeometry = {
    width: number;
    height: number;
    dpr: number;
};

export type ViewportGeometryOwner = {
    /** Current viewport geometry (cached within a frame). */
    read(): ViewportGeometry;
    /** Mark the cache dirty; the next read() re-measures. */
    invalidate(): void;
};

type Deps = {
    getScrollContainer: () => HTMLElement | null;
};

export function createViewportGeometry(deps: Deps): ViewportGeometryOwner {
    let cached: ViewportGeometry | null = null;

    function measure(): ViewportGeometry {
        const el = deps.getScrollContainer();
        const dpr = (typeof window !== 'undefined' && window.devicePixelRatio) || 1;
        if (!el) {
            cached = { width: 0, height: 0, dpr };
            return cached;
        }
        // clientWidth/Height are the CSS layout size. They are themselves a
        // forced layout read, but the cache means we pay it at most once per
        // frame. The rect is only a 0-fallback (e.g. display:none).
        let width = el.clientWidth;
        let height = el.clientHeight;
        if (!width || !height) {
            const rect = el.getBoundingClientRect();
            width = width || rect.width;
            height = height || rect.height;
        }
        cached = { width, height, dpr };
        return cached;
    }

    return {
        read(): ViewportGeometry {
            return cached ?? measure();
        },
        invalidate(): void {
            cached = null;
        },
    };
}

// ─── DOM-resolved singleton ────────────────────────────────────────────────
// One owner per scroll-container element; a rebuilt host gets a fresh owner.

import { getScrollContainer as getScrollContainerEl } from './pdf_viewer_dom';

let singleton: { el: HTMLElement; owner: ViewportGeometryOwner } | null = null;

/**
 * Get the shared viewport-geometry owner. Falls back to a detached instance
 * (zero geometry) only when the shell is not mounted yet, so callers never
 * need a null check.
 */
export function getViewportGeometry(): ViewportGeometryOwner {
    const el = getScrollContainerEl();
    if (singleton && el && singleton.el === el) {
        return singleton.owner;
    }
    if (el) {
        singleton = { el, owner: createViewportGeometry({ getScrollContainer: () => el }) };
        return singleton.owner;
    }
    return createViewportGeometry({ getScrollContainer: () => null });
}

/** Drop the singleton (test/reset hook). */
export function resetViewportGeometry(): void {
    singleton = null;
}

/**
 * Invalidate the shared owner's cache (layout-affecting UI events outside the
 * render pipeline, e.g. a sidebar toggle). No-op before the shell mounts.
 */
export function invalidateViewportGeometry(): void {
    singleton?.owner.invalidate();
}
