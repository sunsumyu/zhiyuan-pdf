import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createViewportGeometry } from '../bridge/viewer/viewport_geometry';

/**
 * RED contract for ADR-0014 — the single owner of viewport geometry.
 *
 * CPU profile (2026-10-01): `getBoundingClientRect` cost ~100ms per gesture,
 * ~48ms of it from `frame_plan.buildRequest`, which runs ~6x per render and
 * re-measured the scroll container every time. The value only changes on
 * window resize / sidebar toggle, so within a frame it must be measured once.
 */

function makeScroller(opts: { w?: number; h?: number; rectW?: number; rectH?: number } = {}) {
    let rectReads = 0;
    const el = {
        clientWidth: opts.w ?? 1200,
        clientHeight: opts.h ?? 800,
        getBoundingClientRect() {
            rectReads += 1;
            return { left: 0, top: 0, width: opts.rectW ?? 1200, height: opts.rectH ?? 800 } as DOMRect;
        },
    } as unknown as HTMLElement;
    return { el, reads: () => rectReads };
}

describe('viewport geometry owner (ADR-0014)', () => {
    it('measures once and serves the same frame from cache', () => {
        const { el, reads } = makeScroller({ w: 1200, h: 800 });
        const geo = createViewportGeometry({ getScrollContainer: () => el });

        const a = geo.read();
        const b = geo.read();
        const c = geo.read();

        expect(a.width).toBe(1200);
        expect(a.height).toBe(800);
        expect(b).toEqual(a);
        expect(c).toEqual(a);
        // Three reads in one frame must not re-measure.
        expect(reads()).toBeLessThanOrEqual(1);
    });

    it('re-measures after invalidate()', () => {
        const state = { w: 1200, h: 800 };
        const el = {
            get clientWidth() { return state.w; },
            get clientHeight() { return state.h; },
            getBoundingClientRect() {
                return { left: 0, top: 0, width: state.w, height: state.h } as DOMRect;
            },
        } as unknown as HTMLElement;
        const geo = createViewportGeometry({ getScrollContainer: () => el });

        expect(geo.read().width).toBe(1200);
        state.w = 900;
        // Stale until invalidated (frame boundary).
        expect(geo.read().width).toBe(1200);
        geo.invalidate();
        expect(geo.read().width).toBe(900);
    });

    it('falls back to the bounding rect when clientWidth is 0', () => {
        const { el } = makeScroller({ w: 0, h: 0, rectW: 640, rectH: 480 });
        const geo = createViewportGeometry({ getScrollContainer: () => el });
        const g = geo.read();
        expect(g.width).toBe(640);
        expect(g.height).toBe(480);
    });

    it('reports dpr and degrades safely without a scroller', () => {
        const geo = createViewportGeometry({ getScrollContainer: () => null });
        const g = geo.read();
        expect(g.dpr).toBeGreaterThan(0);
        expect(g.width).toBe(0);
        expect(g.height).toBe(0);
    });

    it('hot paths read geometry through the owner, not getBoundingClientRect', () => {
        // frame_plan.buildRequest runs ~6x per render; a direct rect read there
        // forced a reflow each time (~48ms/gesture in the CDP profile).
        const framePlan = readFileSync(
            resolve(__dirname, '../bridge/render/frame_plan.ts'),
            'utf8',
        );
        const buildRequest = framePlan.slice(
            framePlan.indexOf('function buildRequest'),
            framePlan.indexOf('function buildRenderRequest'),
        );
        expect(buildRequest).toMatch(/getViewportGeometry\(\)\.read\(\)/);
        expect(buildRequest).not.toMatch(/getBoundingClientRect\(\)/);
        expect(buildRequest).not.toMatch(/\.clientWidth/);
        expect(buildRequest).not.toMatch(/\.clientHeight/);
    });
});
