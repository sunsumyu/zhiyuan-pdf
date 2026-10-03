// ─────────────────────────────────────────────────────────────────────────────
// PresentationSurfaceOwner — single owner of page-surface VISIBILITY (ADR-0011).
//
// The 2026-09-30 recording (video i5eOPYh5IH) showed the whole page surface
// vanish for ~2 frames during a big zoom-in: wrapper + container + canvases all
// unpainted, the application background (#1a1a1a) showing through. Root cause
// class: visibility had >=6 writers (clearVectorCanvasHost,
// hideVectorCanvasHostForPreview, presentViewportCanvas, syncLayoutBox,
// showDocumentWrapper/showEmptyDocumentState, commitRasterSurface) with no
// single owner, so a "both surfaces hidden" intermediate state could be
// composited.
//
// This owner is the ONE writer of `display`/`visibility`/`opacity` on the page
// surface chain and the single source of truth for which surface is active.
// Every swap is atomic in the "show target BEFORE hide source" order, so at
// least one surface is painted at every instant — the blank state is not
// representable.
//
// Scope: the page SURFACE visibility. Layout/dimensions stay with
// syncLayoutBox; pixel contents stay with the presenters. This module only
// decides what is on screen.
// ─────────────────────────────────────────────────────────────────────────────

/** Minimal structural element shape — real HTMLElement or a test stub. */
export type StyledElement = { style: Record<string, any> };

export type SurfaceElements = {
    wrapper: StyledElement | null;
    container: StyledElement | null;
    mainCanvas: StyledElement | null;
    raster: StyledElement | null;
    backCanvas: StyledElement | null;
};

export type SurfaceActive = 'vector' | 'raster' | 'none';

export type PresentationSurfaceOwner = {
    /** Paint the vector chain (container + main canvas); then hide raster. */
    showVector(): void;
    /** Paint the raster surface (wrapper + raster); then hide the vector chain. */
    showRaster(): void;
    /** Document closed/reset — the ONLY path to the `none` state. */
    hideAll(): void;
    /** Reveal the active surface (re-asserts "a document is open"). */
    showDocument(): void;
    /** Show the detail overlay canvas (does not affect the base surface). */
    showDetail(): void;
    /** Hide the detail overlay canvas (visibility only; pixel clear stays with the host). */
    hideDetail(): void;
    /** Which surface currently owns the page area. */
    readonly active: SurfaceActive;
};

function setStyle(el: StyledElement | null | undefined, prop: string, value: string): void {
    if (el && el.style) {
        el.style[prop] = value;
    }
}

export function createPresentationSurfaceOwner(
    getElements: () => SurfaceElements | null,
): PresentationSurfaceOwner {
    let active: SurfaceActive = 'none';

    function showVector(): void {
        const els = getElements();
        if (!els) return;
        // 1. Paint the vector chain FIRST (target visible).
        setStyle(els.wrapper, 'display', 'block');
        setStyle(els.container, 'display', 'block');
        setStyle(els.container, 'visibility', 'visible');
        setStyle(els.container, 'pointerEvents', '');
        setStyle(els.mainCanvas, 'visibility', 'visible');
        setStyle(els.mainCanvas, 'opacity', '1');
        // 2. Only then hide the other surface (source hidden).
        //    Atomic swap: never both hidden.
        setStyle(els.raster, 'display', 'none');
        active = 'vector';
    }

    function showRaster(): void {
        const els = getElements();
        if (!els) return;
        // 1. Paint the raster surface FIRST (target visible).
        setStyle(els.wrapper, 'display', 'block');
        setStyle(els.raster, 'display', 'block');
        // 2. Only then hide the vector chain (source hidden).
        setStyle(els.container, 'display', 'none');
        setStyle(els.container, 'visibility', 'hidden');
        setStyle(els.container, 'pointerEvents', 'none');
        active = 'raster';
    }

    function hideAll(): void {
        const els = getElements();
        if (!els) return;
        setStyle(els.container, 'display', 'none');
        setStyle(els.container, 'visibility', 'hidden');
        setStyle(els.container, 'pointerEvents', 'none');
        setStyle(els.raster, 'display', 'none');
        // No document: the wrapper (outer page box) is hidden too.
        setStyle(els.wrapper, 'display', 'none');
        active = 'none';
    }

    function showDocument(): void {
        // Reveal whatever surface is (or should be) active. Defaults to the
        // vector surface, which is what a freshly opened document presents.
        if (active === 'raster') showRaster();
        else showVector();
    }

    function showDetail(): void {
        const els = getElements();
        if (!els) return;
        setStyle(els.backCanvas, 'visibility', 'visible');
        setStyle(els.backCanvas, 'opacity', '1');
    }

    function hideDetail(): void {
        const els = getElements();
        if (!els) return;
        setStyle(els.backCanvas, 'visibility', 'hidden');
        setStyle(els.backCanvas, 'opacity', '0');
    }

    return {
        showVector,
        showRaster,
        hideAll,
        showDocument,
        showDetail,
        hideDetail,
        get active(): SurfaceActive {
            return active;
        },
    };
}

// ─── DOM-resolved singleton ────────────────────────────────────────────────
// All call sites resolve the same elements; one owner instance keeps the
// `active` fact consistent. Keyed to the container element identity so a
// rebuilt host (element replaced) gets a fresh owner.

export const SURFACE_WRAPPER_ID = 'pdf-content-wrapper';
export const SURFACE_CONTAINER_ID = 'pdf-page-container';
export const SURFACE_MAIN_CANVAS_ID = 'pdf-vector-main-canvas';
export const SURFACE_BACK_CANVAS_ID = 'pdf-vector-detail-canvas';
export const SURFACE_RASTER_ID = 'pdf-render-target';

function resolveSurfaceElements(): SurfaceElements | null {
    const wrapper = document.getElementById(SURFACE_WRAPPER_ID) as StyledElement | null;
    const container = document.getElementById(SURFACE_CONTAINER_ID) as StyledElement | null;
    if (!wrapper || !container) return null;
    return {
        wrapper,
        container,
        // Main/detail canvases are created lazily by ensureVectorCanvasHost;
        // resolve them per call so the owner works from first paint onward.
        mainCanvas: document.getElementById(SURFACE_MAIN_CANVAS_ID) as StyledElement | null,
        raster: document.getElementById(SURFACE_RASTER_ID) as StyledElement | null,
        backCanvas: document.getElementById(SURFACE_BACK_CANVAS_ID) as StyledElement | null,
    };
}

// Keyed on the outer wrapper (`#pdf-content-wrapper`), which exists in the
// static markup from first paint. The inner page container is created lazily,
// so keying on it would give the empty state a detached no-op owner.
let singleton: { wrapper: StyledElement; owner: PresentationSurfaceOwner } | null = null;

/**
 * Get the shared owner for the current page-surface DOM. Falls back to a
 * detached instance (no-op writes) only when the app shell is not mounted yet,
 * so callers never need a null check.
 */
export function getPresentationSurfaceOwner(): PresentationSurfaceOwner {
    const wrapper = document.getElementById(SURFACE_WRAPPER_ID) as StyledElement | null;
    if (singleton && wrapper && singleton.wrapper === wrapper) {
        return singleton.owner;
    }
    if (wrapper) {
        singleton = { wrapper, owner: createPresentationSurfaceOwner(resolveSurfaceElements) };
        return singleton.owner;
    }
    return createPresentationSurfaceOwner(() => null);
}

/** Drop the singleton (test/reset hook). */
export function resetPresentationSurfaceOwner(): void {
    singleton = null;
}
