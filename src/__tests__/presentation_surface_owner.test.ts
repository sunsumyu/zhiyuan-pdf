import { describe, it, expect } from 'vitest';
import {
    createPresentationSurfaceOwner,
    type SurfaceElements,
} from '../bridge/render/presentation_surface_owner';

/**
 * RED contract for ADR-0011 — the single owner of page-surface visibility.
 *
 * The 2026-09-30 video (i5eOPYh5IH) showed the whole page surface vanish for
 * ~2 frames during a big zoom-in. Root cause class: visibility had >=6 writers
 * with no single owner, so a "both surfaces hidden" intermediate state could be
 * composited. These tests pin the owner contract that makes that unrepresentable.
 *
 * Uses a stub surface that records the ORDER of style writes, so the
 * "show-target-before-hide-source" (atomic swap) requirement is testable
 * without a DOM.
 */

type Rec = { el: string; prop: string; value: string };

function makeSurface(): { els: SurfaceElements; rec: Rec[] } {
    const rec: Rec[] = [];
    const style = (el: string, initial: Record<string, string>) => {
        const store: Record<string, string> = { ...initial };
        return new Proxy(store, {
            set(t, p: string, v: string) {
                rec.push({ el, prop: p, value: v });
                t[p] = v;
                return true;
            },
            get(t, p: string) {
                return t[p];
            },
        });
    };
    const els: SurfaceElements = {
        wrapper: { style: style('wrapper', { display: 'none' }) },
        container: { style: style('container', { display: 'none', visibility: 'hidden' }) },
        mainCanvas: { style: style('mainCanvas', { visibility: 'hidden', opacity: '0' }) },
        raster: { style: style('raster', { display: 'none' }) },
        backCanvas: { style: style('backCanvas', { visibility: 'hidden', opacity: '0' }) },
    };
    return { els, rec };
}

const isPainted = (els: SurfaceElements): boolean => {
    const wrapperOk = els.wrapper?.style.display !== 'none';
    const containerOk =
        els.container?.style.display !== 'none' && els.container?.style.visibility !== 'hidden';
    const mainOk =
        els.mainCanvas?.style.visibility !== 'hidden' && parseFloat(els.mainCanvas?.style.opacity || '0') > 0.01;
    const rasterOk = els.raster?.style.display !== 'none';
    return (wrapperOk && containerOk && mainOk) || rasterOk;
};

describe('PresentationSurfaceOwner (ADR-0011)', () => {
    it('showVector paints the vector chain and hides the raster surface', () => {
        const { els } = makeSurface();
        const owner = createPresentationSurfaceOwner(() => els);
        owner.showVector();
        expect(els.container!.style.display).toBe('block');
        expect(els.container!.style.visibility).toBe('visible');
        expect(els.mainCanvas!.style.visibility).toBe('visible');
        expect(els.mainCanvas!.style.opacity).toBe('1');
        expect(els.raster!.style.display).toBe('none');
        expect(owner.active).toBe('vector');
    });

    it('showRaster paints the raster surface and hides the vector chain', () => {
        const { els } = makeSurface();
        const owner = createPresentationSurfaceOwner(() => els);
        owner.showRaster();
        expect(els.wrapper!.style.display).toBe('block');
        expect(els.raster!.style.display).toBe('block');
        expect(els.container!.style.display).toBe('none');
        expect(owner.active).toBe('raster');
    });

    it('swaps atomically: the target surface is shown BEFORE the source is hidden', () => {
        const { els, rec } = makeSurface();
        const owner = createPresentationSurfaceOwner(() => els);

        owner.showVector();
        rec.length = 0;
        owner.showRaster(); // vector -> raster swap
        const rasterShown = rec.findIndex((r) => r.el === 'raster' && r.value === 'block');
        const containerHidden = rec.findIndex(
            (r) => r.el === 'container' && r.prop === 'display' && r.value === 'none',
        );
        expect(rasterShown).toBeGreaterThanOrEqual(0);
        expect(containerHidden).toBeGreaterThanOrEqual(0);
        expect(rasterShown).toBeLessThan(containerHidden);

        rec.length = 0;
        owner.showVector(); // raster -> vector swap
        const containerShown = rec.findIndex(
            (r) => r.el === 'container' && r.prop === 'display' && r.value === 'block',
        );
        const rasterHidden = rec.findIndex(
            (r) => r.el === 'raster' && r.prop === 'display' && r.value === 'none',
        );
        expect(containerShown).toBeGreaterThanOrEqual(0);
        expect(rasterHidden).toBeGreaterThanOrEqual(0);
        expect(containerShown).toBeLessThan(rasterHidden);
    });

    it('never leaves the page surface unpainted across any swap sequence', () => {
        const { els } = makeSurface();
        const owner = createPresentationSurfaceOwner(() => els);
        const ops: Array<() => void> = [
            () => owner.showVector(),
            () => owner.showRaster(),
            () => owner.showVector(),
            () => owner.showVector(),
            () => owner.showRaster(),
            () => owner.showRaster(),
        ];
        for (const op of ops) {
            op();
            expect(isPainted(els)).toBe(true);
        }
    });

    it('hideAll is the only path to the none state (document reset)', () => {
        const { els } = makeSurface();
        const owner = createPresentationSurfaceOwner(() => els);
        owner.showVector();
        owner.hideAll();
        expect(owner.active).toBe('none');
        expect(isPainted(els)).toBe(false);
    });

    it('showDetail/hideDetail toggle only the detail canvas', () => {
        const { els } = makeSurface();
        const owner = createPresentationSurfaceOwner(() => els);
        owner.showVector();
        owner.showDetail();
        expect(els.backCanvas!.style.visibility).toBe('visible');
        expect(els.backCanvas!.style.opacity).toBe('1');
        owner.hideDetail();
        expect(els.backCanvas!.style.visibility).toBe('hidden');
        expect(els.backCanvas!.style.opacity).toBe('0');
        // main surface untouched by detail toggles
        expect(els.container!.style.display).toBe('block');
    });

    it('showDocument reveals a painted surface and never leaves none', () => {
        const { els } = makeSurface();
        const owner = createPresentationSurfaceOwner(() => els);
        // From the none state (fresh document) it paints the vector surface.
        owner.showDocument();
        expect(isPainted(els)).toBe(true);
        expect(owner.active).toBe('vector');
        // After a raster present it re-asserts the raster surface, not vector.
        owner.showRaster();
        owner.showDocument();
        expect(isPainted(els)).toBe(true);
        expect(owner.active).toBe('raster');
    });
});
