// ─────────────────────────────────────────────────────────────────────────────
// CanvasTransformOwner contract — ADR-0010.
//
// The owner is the SINGLE writer of the main canvas transform and the SINGLE
// source of truth for which zoom space the canvas CSS box is currently in
// (boxZoom). Invariant under ANY interleaving of presents and ticks:
//
//     canvas visual width = pageWidth × visualZoom
//
// i.e. after presentFrame(d) and sync(v): transform scale = v / d, because
// the CSS box is page × d. Written TDD-red before the implementation existed;
// runs in the node environment against a minimal style stub (the owner's full
// DOM surface is style.transform read/write).
// ─────────────────────────────────────────────────────────────────────────────
import { describe, it, expect, beforeEach } from 'vitest';
import { createCanvasTransformOwner } from '../bridge/render/canvas_transform_owner';

/** Minimal canvas stub: the owner only touches style.transform. */
function makeStubCanvas() {
    const store: Record<string, string> = {};
    const el = {
        style: {
            get transform() {
                return store.transform ?? '';
            },
            set transform(v: string) {
                store.transform = v;
            },
        },
    } as unknown as HTMLCanvasElement;
    return { el, store };
}

function readScale(el: HTMLCanvasElement): number {
    const m = el.style.transform.match(/scale\(([\d.]+)\)/);
    return m ? parseFloat(m[1]) : NaN;
}

describe('CanvasTransformOwner (ADR-0010)', () => {
    let el: HTMLCanvasElement;

    beforeEach(() => {
        el = makeStubCanvas().el;
    });

    it('presents a frame: boxZoom becomes the displayZoom and transform compensates', () => {
        const owner = createCanvasTransformOwner(el);
        owner.presentFrame(1.0);
        owner.sync(1.0);
        expect(el.style.transform).toBe('scale(1)');
        expect(owner.boxZoom).toBe(1.0);
    });

    it('sync uses boxZoom — NOT lastRendered/displayZoom guesses — for the divisor', () => {
        const owner = createCanvasTransformOwner(el);
        // Present at 0.658 (a stale band), then the visual has moved to 0.700.
        owner.presentFrame(0.658);
        owner.sync(0.700);
        // The exact defect from the 2026-09-30 probe: canvas must show
        // page × 0.700 (scale = 0.700/0.658 ≈ 1.0638), NOT identity.
        expect(owner.boxZoom).toBe(0.658);
        expect(readScale(el)).toBeCloseTo(0.700 / 0.658, 5);
    });

    it('keeps the invariant under a chaotic present/tick interleaving', () => {
        const owner = createCanvasTransformOwner(el);
        const pageWidth = 595;
        // Time-ordered events from a real burst: presents land late (worker
        // latency), ticks run every frame. Unordered on purpose. The present
        // branch also simulates the presenter's own duty (applyCanvasCssBox
        // sets css width = page × displayZoom) — that part is NOT the owner's.
        const events: Array<{ kind: 'present'; d: number } | { kind: 'tick'; v: number }> = [
            { kind: 'present', d: 1.0 },
            { kind: 'tick', v: 0.95 },
            { kind: 'tick', v: 0.9 },
            { kind: 'present', d: 0.9 },
            { kind: 'tick', v: 0.82 },
            { kind: 'tick', v: 0.78 },
            { kind: 'present', d: 0.7 },
            { kind: 'tick', v: 0.71 }, // visual can wobble around a band
            { kind: 'present', d: 0.7 }, // duplicate present of the same band
            { kind: 'tick', v: 0.65 },
            { kind: 'present', d: 0.55 },
            { kind: 'tick', v: 0.5 },
            { kind: 'tick', v: 0.45 },
            { kind: 'present', d: 0.4 },
            { kind: 'tick', v: 0.4 },
        ];
        let lastVisual = 1.0;
        for (const ev of events) {
            if (ev.kind === 'present') {
                owner.presentFrame(ev.d);
                // presenter duty (applyCanvasCssBox): css box = page × displayZoom
                el.style.transform = el.style.transform; // no-op; owner re-syncs below
                owner.sync(lastVisual);
            } else {
                owner.sync(ev.v);
            }
            const visual = ev.kind === 'tick' ? ev.v : lastVisual;
            lastVisual = visual;

            const scale = readScale(el);
            // THE invariant: on-screen width = pageWidth × latest visual,
            // i.e. scale must equal visual / boxZoom at all times.
            expect(scale).toBeCloseTo(visual / owner.boxZoom, 4);
        }
    });

    it('presentFrame re-applies the last visual atomically (no stale frame)', () => {
        const owner = createCanvasTransformOwner(el);
        // Visual is already at 0.700 while the box is still in the 1.0 space.
        owner.presentFrame(1.0);
        owner.sync(0.700);
        expect(readScale(el)).toBeCloseTo(0.700, 5);
        // A present lands re-boxing to 0.658 — the transform must move to the
        // new space in the SAME call, not wait for the next tick, or the
        // compositor paints one frame at the wrong size (the "一闪" flicker).
        owner.presentFrame(0.658);
        expect(owner.boxZoom).toBe(0.658);
        expect(readScale(el)).toBeCloseTo(0.700 / 0.658, 5);
    });

    it('sync before any present defaults boxZoom to 1 and writes the raw visual', () => {
        const owner = createCanvasTransformOwner(el);
        owner.sync(0.8);
        expect(owner.boxZoom).toBe(1);
        expect(el.style.transform).toBe('scale(0.8)');
    });

    it('reset returns boxZoom to 1 (document switch)', () => {
        const owner = createCanvasTransformOwner(el);
        owner.presentFrame(0.5);
        owner.sync(0.5);
        owner.reset();
        expect(owner.boxZoom).toBe(1);
        owner.sync(1.0);
        expect(el.style.transform).toBe('scale(1)');
    });

    it('sync is stable when neither boxZoom nor visual changed', () => {
        const owner = createCanvasTransformOwner(el);
        owner.presentFrame(0.8);
        owner.sync(0.8);
        const first = el.style.transform;
        owner.sync(0.8);
        expect(el.style.transform).toBe(first);
    });
});
