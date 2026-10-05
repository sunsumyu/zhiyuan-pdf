// ─────────────────────────────────────────────────────────────────────────────
// Gesture direct render — ADR-0026.
//
// The gesture reknock viewport patch previously rendered through the vector
// worker (round trip p50 70.9–90.6ms; the wasm render itself is 3.1–3.5ms) —
// the patch content lagged 15–27 quantized bands mid-gesture. ADR-0026 moves
// the render onto the main thread, synchronously, straight into the existing
// detail stage buffer: same wasm CanvasRenderer, same pixels, different
// execution context (the execution context is NOT part of the rendering
// chain — core → paint plan → Rust canvas → DOM canvas is unchanged).
//
// Eligibility (decision authority stays in core): the frame plan must say
// zoom reason + use_viewport_tile + preview active, and the layer must be
// the detail layer. The runtime flag __pdfGestureDirectRenderDisabled is the
// ADR-0016-style escape hatch back to the worker path.
//
// Budget guard: one over-budget render skips exactly the NEXT frame —
// natural backpressure with no mode machinery. While skipped the patch stays
// on screen and DetailOverlayOwner keeps it geometrically aligned, so the
// degradation is exactly the old P2 behavior (known-acceptable).
// ─────────────────────────────────────────────────────────────────────────────

/** Frame-budget guard for one direct render (probe-calibrated initial value). */
export const GESTURE_DIRECT_RENDER_BUDGET_MS = 10;

/** Hot-path cleanliness (ADR-0013): PROF emission is sampled, not per-frame. */
export const GESTURE_DIRECT_RENDER_PROF_SAMPLE = 16;

export type GestureDirectRenderPlan = {
    renderReason?: string;
    useViewportTile?: boolean;
    previewSettled?: boolean;
};

export function isGestureDirectRenderEligible(
    plan: GestureDirectRenderPlan,
    layerIsDetailLayer: boolean,
): boolean {
    if ((globalThis as any).__pdfGestureDirectRenderDisabled === true) {
        return false;
    }
    return plan.renderReason === 'zoom'
        && plan.useViewportTile === true
        && plan.previewSettled === false
        && layerIsDetailLayer;
}

export class GestureDirectRenderBudget {
    private skipNext = false;
    private lastCostMs = -1;

    /** Consume the armed skip: true exactly once after an over-budget render. */
    shouldSkip(): boolean {
        if (!this.skipNext) {
            return false;
        }
        this.skipNext = false;
        return true;
    }

    record(costMs: number): void {
        this.lastCostMs = costMs;
        this.skipNext = costMs > GESTURE_DIRECT_RENDER_BUDGET_MS;
    }

    reset(): void {
        this.skipNext = false;
        this.lastCostMs = -1;
    }

    get lastCost(): number {
        return this.lastCostMs;
    }
}

/** Module-singleton budget — the guard is deliberately stateless across frames
 *  beyond the one-armed skip, so a single instance serves every gesture. */
export const gestureDirectRenderBudget = new GestureDirectRenderBudget();

export type GestureDirectRenderParams = {
    /** The reused detail stage buffer (getRenderBufferCanvas) — never a fresh
     *  canvas: 60Hz × ~6MB of OffscreenCanvas allocations would be a GC storm. */
    renderTarget: HTMLCanvasElement;
    imageCacheMap: Map<string, ImageBitmap>;
    dpr: number;
    renderPageOffscreen: (
        canvas: HTMLCanvasElement | OffscreenCanvas,
        imageCacheMap: Map<string, ImageBitmap>,
        dpr: number,
    ) => void;
    budget: GestureDirectRenderBudget;
    /** Monotonic counter of direct-render attempts — drives PROF sampling. */
    seq: number;
    now?: () => number;
    onProf?: (fields: Record<string, unknown>) => void;
};

export type GestureDirectRenderResult = {
    skipped: boolean;
    costMs: number;
};

/**
 * Render the gesture viewport patch synchronously on the main thread. The
 * caller's store/present flow continues unchanged after this returns — the
 * patch pixels land in the stage buffer, then flow through the same
 * frame-cache store and presenter path as a worker render.
 */
export function renderGestureViewportPatchDirect(params: GestureDirectRenderParams): GestureDirectRenderResult {
    const { renderTarget, imageCacheMap, dpr, renderPageOffscreen, budget, seq, onProf } = params;
    const now = params.now ?? (() => performance.now());

    if (budget.shouldSkip()) {
        return { skipped: true, costMs: 0 };
    }

    const t0 = now();
    const ctx = renderTarget.getContext('2d');
    if (ctx) {
        ctx.clearRect(0, 0, renderTarget.width, renderTarget.height);
    }
    renderPageOffscreen(renderTarget, imageCacheMap, dpr);
    const costMs = Math.round((now() - t0) * 10) / 10;
    budget.record(costMs);

    if (onProf && seq % GESTURE_DIRECT_RENDER_PROF_SAMPLE === 0) {
        onProf({
            wasmMs: costMs,
            roundMs: costMs,
            workerMs: 0,
            recvDelayMs: 0,
            blitMs: 0,
            w: renderTarget.width,
            h: renderTarget.height,
            prog: false,
            tile: true,
            direct: true,
        });
    }

    return { skipped: false, costMs };
}
