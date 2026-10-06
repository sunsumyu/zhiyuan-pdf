// ─────────────────────────────────────────────────────────────────────────────
// Present math contract — the unified present formula (ADR-0009 tiles /
// ADR-0010 main canvas / ADR-0024 detail overlay).
//
// The 2026-09-30 / 2026-10-03 double-exposure bugs were inline copies of this
// formula disagreeing; since 2026-10-06 the formula has exactly one
// implementation (present_math.ts) and all zoom-driven surfaces go through
// it. These tests pin the shared core:
//
//     s = visualZoom / surfaceZoom
//     left = rect.left × s,   top = rect.top × s,   transform = scale(s)
//
// Node test environment (no jsdom) — style stubs mirror the Owner contract
// tests' Proxy stubs and record every property write.
// ─────────────────────────────────────────────────────────────────────────────
import { describe, it, expect } from 'vitest';
import {
    presentScale,
    visualOffset,
    visualTransform,
    createMemoizedStyle,
} from '../bridge/render/present_math';

function makeStubElement() {
    const store: Record<string, string> = {};
    const writes: Array<{ prop: string; value: string }> = [];
    const el = {
        style: new Proxy(store, {
            get(target, prop: string) {
                return target[prop] ?? '';
            },
            set(target, prop: string, value: string) {
                target[prop] = value;
                writes.push({ prop, value });
                return true;
            },
        }),
    } as unknown as HTMLElement;
    return { el, store, writes };
}

describe('present math — unified present formula (ADR-0009/0010/0024)', () => {
    it('computes s = visualZoom / surfaceZoom', () => {
        expect(presentScale(2, 1)).toBe(2);
        expect(presentScale(3, 1.5)).toBe(2);
        expect(presentScale(0.5, 2)).toBe(0.25);
    });

    it('rejects non-finite or non-positive zooms (mapping undefined → skip writes)', () => {
        expect(presentScale(0, 1)).toBeNull();
        expect(presentScale(-1, 1)).toBeNull();
        expect(presentScale(1, 0)).toBeNull();
        expect(presentScale(1, -2)).toBeNull();
        expect(presentScale(Number.NaN, 1)).toBeNull();
        expect(presentScale(Number.POSITIVE_INFINITY, 1)).toBeNull();
        expect(presentScale(1, Number.NaN)).toBeNull();
    });

    it('maps a base-space rect so a page point lands at q × visualZoom', () => {
        // The invariant every zoom-driven surface shares. With the patch
        // recorded at zoom 1.2 and visual now 2.4: s = 2, so the base corner
        // 300 lands at 600 = 300 × visualZoom / 1.2 — identical strings must
        // come out of the tile layer, the main canvas owner and the detail
        // overlay owner because they all call these two functions.
        const s = presentScale(2.4, 1.2);
        expect(s).not.toBeNull();
        expect(visualOffset(300, s as number)).toBe('600px');
        expect(visualTransform(s as number)).toBe('scale(2)');
    });

    it('visualOffset formats base × s as a px length', () => {
        expect(visualOffset(512, 2)).toBe('1024px');
        expect(visualOffset(0, 3)).toBe('0px');
        expect(visualOffset(-40, 0.5)).toBe('-20px');
    });

    it('writes identity as scale(1) — single-valued representation', () => {
        expect(visualTransform(1)).toBe('scale(1)');
        expect(visualTransform(0.75)).toBe('scale(0.75)');
    });
});

describe('createMemoizedStyle — shared write suppression', () => {
    it('skips writes equal to the last written value', () => {
        const { el, writes } = makeStubElement();
        const style = createMemoizedStyle(el, 'transform');
        style.write('scale(2)');
        style.write('scale(3)');
        style.write('scale(3)');
        style.write('scale(3)');
        expect(writes).toEqual([
            { prop: 'transform', value: 'scale(2)' },
            { prop: 'transform', value: 'scale(3)' },
        ]);
    });

    it('invalidate() forces the next write to land even if the string coincides', () => {
        const { el, writes } = makeStubElement();
        const style = createMemoizedStyle(el, 'left');
        style.write('100px');
        style.invalidate();
        style.write('100px');
        expect(writes).toEqual([
            { prop: 'left', value: '100px' },
            { prop: 'left', value: '100px' },
        ]);
    });

    it('memoizes per property — two writers on one element do not interfere', () => {
        const { el, store } = makeStubElement();
        const left = createMemoizedStyle(el, 'left');
        const top = createMemoizedStyle(el, 'top');
        left.write('1px');
        top.write('2px');
        left.write('1px');
        top.write('3px');
        expect(store.left).toBe('1px');
        expect(store.top).toBe('3px');
    });
});
