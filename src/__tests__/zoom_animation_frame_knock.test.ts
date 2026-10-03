/**
 * ADR-0018 contract — the canvas transform must be causally ordered AFTER the
 * zoom animation advance within the same JS turn.
 *
 * Measured root cause (E2E frame dump, 2026-10-02): the Rust animation loop
 * (advances visual_zoom in its own rAF) and the TS tile tick (writes the
 * canvas transform in another rAF) have no intra-frame ordering. The tick ran
 * first every frame, so the transform always showed the PREVIOUS frame's
 * visual zoom; at a gesture reversal/convergence one animation frame spans a
 * full wheel band (~4.6-5.3%) and the contract catches 1-3 violating frames.
 *
 * Fix shape: Rust knocks a fixed TS global (`__pdfZoomAnimationFrame`, the
 * ADR-0001 knock pattern) immediately after advancing the animation state;
 * TS syncs the canvas transform + tile present scales from the same live
 * zoom state in that same turn.
 *
 * These assertions follow the zoom_settle_envelope structural-contract style:
 * the wiring must exist, and the sync must not depend on tile-queue activity.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const RAF_LOOP = readFileSync(
    resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_loop.rs'),
    'utf8',
);
const RAF_DISPATCH = readFileSync(
    resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_dispatch.rs'),
    'utf8',
);
const TILE_LAYER = readFileSync(
    resolve(__dirname, '../bridge/render/tile_layer.ts'),
    'utf8',
);
const PDF_RUNTIME = readFileSync(
    resolve(__dirname, '../bridge/viewer/pdf_runtime.ts'),
    'utf8',
);

describe('ADR-0018: canvas transform follows the animation clock', () => {
    it('defines the fixed animation-frame knock global in raf_dispatch', () => {
        expect(RAF_DISPATCH).toMatch(/__pdfZoomAnimationFrame/);
        expect(RAF_DISPATCH).toMatch(/pub fn dispatch_animation_frame/);
    });

    it('raf_loop knocks the animation frame AFTER the state advance, before settle dispatch', () => {
        expect(RAF_LOOP).toMatch(/dispatch_animation_frame\(\)/);

        const advance = RAF_LOOP.indexOf('advance_zoom_animation_state');
        const knock = RAF_LOOP.indexOf('dispatch_animation_frame()');
        const settle = RAF_LOOP.indexOf('dispatch_settle_envelope()');
        expect(advance).toBeGreaterThan(-1);
        expect(knock).toBeGreaterThan(advance);
        expect(settle).toBeGreaterThan(knock);
    });

    it('tile layer exposes onZoomAnimationFrame syncing canvas AND tiles from one live read', () => {
        expect(TILE_LAYER).toMatch(/onZoomAnimationFrame/);
        // The sync function must drive both surfaces from the same zoom state.
        const syncFn = TILE_LAYER.match(/function syncVisualTransforms[\s\S]*?\n\}/);
        expect(syncFn).not.toBeNull();
        expect(syncFn![0]).toMatch(/getMainCanvasTransformOwner/);
        expect(syncFn![0]).toMatch(/refreshTileTransforms/);
        // The tick must use the same function (no divergent second path).
        expect(TILE_LAYER).toMatch(/syncVisualTransforms\(zs\)/);
    });

    it('animation-frame entry must not touch the tile pump or tick scheduling', () => {
        // The knock fires every animation frame; it must stay O(active tiles)
        // and must not schedule tile work — that stays the tile tick's job.
        const entry = TILE_LAYER.match(/onZoomAnimationFrame:\s*\(\)\s*=>[^,]*,?/);
        expect(entry).not.toBeNull();
        expect(entry![0]).not.toMatch(/scheduleTick|pumpNext|startAnimation/);
    });

    it('pdf_runtime assigns the fixed global next to the settle knock', () => {
        expect(PDF_RUNTIME).toMatch(/__pdfZoomAnimationFrame\s*=/);
        expect(PDF_RUNTIME).toMatch(/__pdfZoomAnimationFrame[^;]*onZoomAnimationFrame/);
    });
});
