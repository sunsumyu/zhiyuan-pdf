// ─────────────────────────────────────────────────────────────────────────────
// DetailOverlayOwner contract — ADR-0024.
//
// The detail overlay (backCanvas) is the third zoom-driven surface. Before
// this owner it was the ONLY presented surface without per-frame geometry
// compensation: a viewport-tile patch presented at a mid-gesture zoom band
// froze at that band's CSS box while visual zoom kept moving (the 2026-10-03
// video: stale white patch sticking out beyond the settled page, persisting
// at rest until the next base present).
//
// Invariant under ANY interleaving of presents and ticks (identical to the
// tile present formula, ADR-0009): a page point q lands at q × visualZoom on
// the patch, i.e. with the patch recorded in the frame's displayZoom space:
//
//     left = rect.left × (visual / zoom),  transform = scale(visual / zoom)
//
// The CSS width/height stay in the recorded (base) space — the scale does the
// mapping, exactly like tile canvases. Written TDD-red before implementation.
// ─────────────────────────────────────────────────────────────────────────────
import { describe, it, expect, beforeEach } from 'vitest';
import {
    createDetailOverlayOwner,
    getDetailOverlayOwner,
    resetDetailOverlayOwner,
} from '../bridge/render/detail_overlay_owner';

/** Minimal canvas stub: the owner touches style.left/top/width/height/transform/visibility. */
function makeStubCanvas() {
    const store: Record<string, string> = {};
    const el = {
        style: new Proxy(store, {
            get(target, prop: string) {
                return target[prop] ?? '';
            },
            set(target, prop: string, value: string) {
                target[prop] = value;
                return true;
            },
        }),
    } as unknown as HTMLCanvasElement;
    return { el, store };
}

function readScale(el: HTMLCanvasElement): number {
    const m = el.style.transform.match(/scale\(([\d.]+)\)/);
    return m ? parseFloat(m[1]) : NaN;
}

const RECT = { left: 130, top: 80, width: 800, height: 1100 };

describe('DetailOverlayOwner (ADR-0024)', () => {
    let el: HTMLCanvasElement;

    beforeEach(() => {
        el = makeStubCanvas().el;
    });

    it('present writes the visual-space mapping ATOMICALLY (no one-frame stale box)', () => {
        const owner = createDetailOverlayOwner(el);
        // The patch was rendered for the 0.65 band; visual already moved to
        // 0.42 (fast zoom-out, worker latency). The present call itself must
        // land left/top/transform in visual space — the exact "ghost sticking
        // out of the page" frame from the video must not exist.
        owner.present(RECT, 0.65, 0.42);
        const s = 0.42 / 0.65;
        expect(parseFloat(el.style.left)).toBeCloseTo(RECT.left * s, 4);
        expect(parseFloat(el.style.top)).toBeCloseTo(RECT.top * s, 4);
        expect(readScale(el)).toBeCloseTo(s, 5);
        // width/height stay in the recorded base space (scale does the mapping)
        expect(parseFloat(el.style.width)).toBe(RECT.width);
        expect(parseFloat(el.style.height)).toBe(RECT.height);
    });

    it('keeps the page-point invariant under a chaotic present/tick interleaving', () => {
        const owner = createDetailOverlayOwner(el);
        const pagePoint = 300; // a page-space coordinate (PDF units)
        // Time-ordered events from a real burst: reknock presents land late
        // (serial worker), ticks run every frame.
        const events: Array<{ kind: 'present'; d: number } | { kind: 'tick'; v: number }> = [
            { kind: 'present', d: 1.0 },
            { kind: 'tick', v: 0.95 },
            { kind: 'present', d: 0.9 },
            { kind: 'tick', v: 0.82 },
            { kind: 'present', d: 0.75 },
            { kind: 'tick', v: 0.65 },
            { kind: 'present', d: 0.65 }, // duplicate band present
            { kind: 'tick', v: 0.55 },
            { kind: 'tick', v: 0.45 },
            { kind: 'present', d: 0.45 },
            { kind: 'tick', v: 0.42 },
            { kind: 'tick', v: 0.42 }, // rest at target
        ];
        let lastVisual = 1.0;
        for (const ev of events) {
            if (ev.kind === 'present') {
                owner.present(RECT, ev.d, lastVisual);
            } else {
                owner.sync(ev.v);
                lastVisual = ev.v;
            }
            const visual = ev.kind === 'tick' ? ev.v : lastVisual;
            const s = visual / owner.zoom;
            // THE invariant: page point q lands at q × visual on the patch —
            // left must equal rect.left × s and transform scale(visual/zoom).
            expect(parseFloat(el.style.left)).toBeCloseTo(RECT.left * s, 3);
            expect(parseFloat(el.style.top)).toBeCloseTo(RECT.top * s, 3);
            expect(readScale(el)).toBeCloseTo(s, 4);
            // end-to-end page-point mapping through the composed styles
            const cssX =
                parseFloat(el.style.left) +
                (pagePoint * owner.zoom - RECT.left) * readScale(el);
            expect(cssX).toBeCloseTo(pagePoint * visual, 2);
        }
    });

    it('rest state at target ≠ record zoom stays aligned (the permanent-variant defusal)', () => {
        const owner = createDetailOverlayOwner(el);
        // Gesture ended at 0.42; the last patch is from the 0.65 band and the
        // settle frame was SKIPPED (legit base reuse) — nothing will ever
        // re-present or hide this patch. It must still show page × 0.42.
        owner.present(RECT, 0.65, 0.5);
        owner.sync(0.42);
        owner.sync(0.42);
        owner.sync(0.42);
        const s = 0.42 / 0.65;
        expect(readScale(el)).toBeCloseTo(s, 5);
        expect(parseFloat(el.style.left)).toBeCloseTo(RECT.left * s, 4);
    });

    it('sync before any present is a no-op (never writes an unbacked transform)', () => {
        const owner = createDetailOverlayOwner(el);
        owner.sync(0.8);
        expect(el.style.transform).toBe('');
        expect(owner.tracked).toBe(false);
    });

    it('reset drops tracking; sync afterwards writes nothing', () => {
        const owner = createDetailOverlayOwner(el);
        owner.present(RECT, 0.65, 0.65);
        owner.reset();
        expect(owner.tracked).toBe(false);
        el.style.transform = 'scale(9)'; // sentinel: nothing may overwrite it
        owner.sync(0.42);
        expect(el.style.transform).toBe('scale(9)');
    });

    it('skips writes while the surface is hidden (visibility guard)', () => {
        const owner = createDetailOverlayOwner(el);
        owner.present(RECT, 0.65, 0.65);
        el.style.visibility = 'hidden';
        el.style.transform = 'scale(9)'; // sentinel
        owner.sync(0.42);
        expect(el.style.transform).toBe('scale(9)');
        // re-shown without a fresh present (retain path): last mapping still valid
        el.style.visibility = 'visible';
        owner.sync(0.42);
        expect(readScale(el)).toBeCloseTo(0.42 / 0.65, 5);
    });

    it('element-keyed singleton: same canvas → same owner; resetDetailOverlayOwner drops it', () => {
        const a1 = getDetailOverlayOwner(el);
        const a2 = getDetailOverlayOwner(el);
        expect(a1).toBe(a2);
        const other = makeStubCanvas().el;
        expect(getDetailOverlayOwner(other)).not.toBe(a1);
        a1.present(RECT, 0.65, 0.65);
        resetDetailOverlayOwner(el);
        expect(getDetailOverlayOwner(el).tracked).toBe(false);
        expect(getDetailOverlayOwner(el)).not.toBe(a1);
    });
});
