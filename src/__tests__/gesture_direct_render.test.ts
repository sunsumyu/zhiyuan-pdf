// ─────────────────────────────────────────────────────────────────────────────
// Gesture direct render contract — ADR-0026.
//
// The gesture reknock viewport patch used to render through the vector
// worker: round trip p50 70.9–90.6ms while the wasm render itself costs
// 3.1–3.5ms (~95% of the cycle is worker queueing + message delay). The patch
// content lagged 15–27 quantized bands mid-gesture (13–14% CSS stretch).
//
// ADR-0026 moves that render onto the main thread, synchronously, straight
// into the existing detail stage buffer. Red lines pinned here:
//   - Eligibility is decided by the frame plan (core stays the decision
//     authority): zoom reason + use_viewport_tile + preview NOT settled, and
//     only for the detail layer. A runtime flag is the escape hatch back to
//     the worker path (ADR-0016-style rollback).
//   - No per-frame allocation: the render target is the reused stage buffer
//     itself, never a fresh OffscreenCanvas (60Hz × ~6MB would be a GC storm).
//   - Budget guard: one over-budget render skips exactly the NEXT frame
//     (natural backpressure; geometry stays aligned via DetailOverlayOwner).
//   - Hot-path cleanliness (ADR-0013): PROF emission is sampled, not per-frame.
// Written TDD-red before implementation.
// ─────────────────────────────────────────────────────────────────────────────
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
    GESTURE_DIRECT_RENDER_BUDGET_MS,
    GestureDirectRenderBudget,
    isGestureDirectRenderEligible,
    renderGestureViewportPatchDirect,
} from '../bridge/render/gesture_direct_render';

function makeStubCanvas(w = 100, h = 80) {
    const calls: string[] = [];
    const ctx = {
        clearRect: () => { calls.push('clearRect'); },
    } as unknown as CanvasRenderingContext2D;
    const el = {
        width: w,
        height: h,
        getContext: (type: string) => (type === '2d' ? ctx : null),
    } as unknown as HTMLCanvasElement;
    return { el, calls };
}

const ZOOM_PREVIEW_PLAN = {
    renderReason: 'zoom',
    useViewportTile: true,
    previewSettled: false,
};

describe('gesture direct render eligibility (ADR-0026)', () => {
    beforeEach(() => {
        delete (globalThis as any).__pdfGestureDirectRenderDisabled;
    });

    it('eligible for the zoom-reason detail layer while preview is active', () => {
        expect(isGestureDirectRenderEligible(ZOOM_PREVIEW_PLAN, true)).toBe(true);
    });

    it('not eligible for non-zoom render reasons (editor overlays keep their own path)', () => {
        expect(isGestureDirectRenderEligible({ ...ZOOM_PREVIEW_PLAN, renderReason: 'editorVisibility' }, true)).toBe(false);
        expect(isGestureDirectRenderEligible({ ...ZOOM_PREVIEW_PLAN, renderReason: 'documentMutation' }, true)).toBe(false);
        expect(isGestureDirectRenderEligible({ ...ZOOM_PREVIEW_PLAN, renderReason: 'default' }, true)).toBe(false);
    });

    it('not eligible once preview settled (settle path keeps its worker semantics)', () => {
        expect(isGestureDirectRenderEligible({ ...ZOOM_PREVIEW_PLAN, previewSettled: true }, true)).toBe(false);
    });

    it('not eligible when the frame plan stays on the full-page path', () => {
        expect(isGestureDirectRenderEligible({ ...ZOOM_PREVIEW_PLAN, useViewportTile: false }, true)).toBe(false);
    });

    it('not eligible for the base layer render', () => {
        expect(isGestureDirectRenderEligible(ZOOM_PREVIEW_PLAN, false)).toBe(false);
    });

    it('escape hatch flag forces the worker path back (ADR-0016-style rollback)', () => {
        (globalThis as any).__pdfGestureDirectRenderDisabled = true;
        expect(isGestureDirectRenderEligible(ZOOM_PREVIEW_PLAN, true)).toBe(false);
    });
});

describe('gesture direct render budget guard (ADR-0026 §4)', () => {
    it('budget constant is the frame-budget guard value', () => {
        expect(GESTURE_DIRECT_RENDER_BUDGET_MS).toBe(10);
    });

    it('a cheap render never arms the skip', () => {
        const budget = new GestureDirectRenderBudget();
        budget.record(5);
        expect(budget.shouldSkip()).toBe(false);
    });

    it('an over-budget render skips exactly ONE following frame, then recovers', () => {
        const budget = new GestureDirectRenderBudget();
        budget.record(12);
        expect(budget.shouldSkip()).toBe(true);
        expect(budget.shouldSkip()).toBe(false);
        expect(budget.shouldSkip()).toBe(false);
    });

    it('reset clears the armed skip and the recorded cost', () => {
        const budget = new GestureDirectRenderBudget();
        budget.record(12);
        budget.reset();
        expect(budget.shouldSkip()).toBe(false);
        expect(budget.lastCost).toBe(-1);
    });
});

describe('renderGestureViewportPatchDirect (ADR-0026 §1/§5/§6)', () => {
    let budget: GestureDirectRenderBudget;

    beforeEach(() => {
        budget = new GestureDirectRenderBudget();
    });

    it('renders straight into the stage buffer (no new OffscreenCanvas), clearing first', () => {
        const { el: renderTarget, calls } = makeStubCanvas(320, 240);
        const renderPageOffscreen = vi.fn();
        let fakeNow = 1000;
        const result = renderGestureViewportPatchDirect({
            renderTarget,
            imageCacheMap: new Map(),
            dpr: 1.25,
            renderPageOffscreen,
            budget,
            seq: 1,
            now: () => fakeNow,
        });
        // The wasm renderer receives the reused stage buffer itself — the
        // single-rendering-chain output lands in the existing pipeline buffer.
        expect(renderPageOffscreen).toHaveBeenCalledTimes(1);
        expect(renderPageOffscreen).toHaveBeenCalledWith(renderTarget, expect.any(Map), 1.25);
        expect(calls).toEqual(['clearRect']);
        expect(result.skipped).toBe(false);
        fakeNow += 3.5;
        expect(result.costMs).toBeGreaterThanOrEqual(0);
    });

    it('over-budget budget guard skips the render entirely (worker path untouched)', () => {
        const { el: renderTarget } = makeStubCanvas();
        const renderPageOffscreen = vi.fn();
        budget.record(GESTURE_DIRECT_RENDER_BUDGET_MS + 1);
        const result = renderGestureViewportPatchDirect({
            renderTarget,
            imageCacheMap: new Map(),
            dpr: 1,
            renderPageOffscreen,
            budget,
            seq: 1,
            now: () => 0,
        });
        expect(result.skipped).toBe(true);
        expect(renderPageOffscreen).not.toHaveBeenCalled();
    });

    it('PROF emission is sampled (one in N frames), never per-frame', () => {
        const { el: renderTarget } = makeStubCanvas();
        const onProf = vi.fn();
        for (let seq = 1; seq <= 32; seq++) {
            renderGestureViewportPatchDirect({
                renderTarget,
                imageCacheMap: new Map(),
                dpr: 1,
                renderPageOffscreen: vi.fn(),
                budget,
                seq,
                now: () => 0,
                onProf,
            });
        }
        expect(onProf).toHaveBeenCalledTimes(2); // seq 16 and 32
        const fields = onProf.mock.calls[0][0] as Record<string, unknown>;
        expect(fields.workerMs).toBe(0);
        expect(fields.tile).toBe(true);
        expect(fields.direct).toBe(true);
    });
});
