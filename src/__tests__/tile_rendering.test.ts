// ─────────────────────────────────────────────────────────────────────────────
// Tile rendering tests
//
// The tile pipeline spans Rust (TileManager scheduling + LRU cache — covered
// by cargo tests in tile_manager.rs / tile_v2.rs) and TS (pure tile geometry +
// the wasm bridge + the DOM TileLayer). This file covers the TS side:
// - tile_geometry coordinate math (display/layout spaces, 512px grid)
// - tile_bridge no-throw safety before WASM initialization
//
// See docs/adr/0003-tile-based-rendering.md and docs/adr/0004-always-vector-rendering.md
// ─────────────────────────────────────────────────────────────────────────────

import { describe, it, expect } from 'vitest';
import {
    TILE_SIZE,
    tileElementBox,
    tileBitmapSize,
    tileDisplayRect,
    tileKeyString,
} from '../bridge/render/tile_geometry';
import { tileFacade } from '../bridge/render/tile_bridge';

describe('Tile geometry', () => {
    describe('tile size constant', () => {
        it('uses the fixed 512px display-space grid (ADR-0003)', () => {
            expect(TILE_SIZE).toBe(512);
        });
    });

    describe('tileElementBox', () => {
        it('yields exact 512px CSS boxes at settle (cssScale = 1)', () => {
            const box = tileElementBox(2, 3, 1.0);
            expect(box).toEqual({ left: 1024, top: 1536, width: 512, height: 512 });
        });

        it('divides by cssScale so the container transform lands the tile on its display rect', () => {
            // During interpolation s = visual / layout; a display-space 512px
            // tile must be a 512/s CSS-px element inside the scaled container.
            const box = tileElementBox(1, 0, 2.0);
            expect(box.width).toBeCloseTo(256, 5);
            expect(box.left).toBeCloseTo(256, 5);
            expect(box.top).toBe(0);
        });

        it('guards against degenerate cssScale', () => {
            const boxZero = tileElementBox(0, 0, 0);
            expect(Number.isFinite(boxZero.width)).toBe(true);
            expect(boxZero.width).toBeGreaterThan(0);

            const boxNeg = tileElementBox(0, 0, -3);
            expect(Number.isFinite(boxNeg.width)).toBe(true);
            expect(boxNeg.width).toBeGreaterThan(0);
        });
    });

    describe('tileBitmapSize', () => {
        it('scales the tile by device pixel ratio', () => {
            expect(tileBitmapSize(1)).toBe(512);
            expect(tileBitmapSize(2)).toBe(1024);
            expect(tileBitmapSize(1.5)).toBe(768);
        });

        it('falls back to dpr 1 for invalid input', () => {
            expect(tileBitmapSize(0)).toBe(512);
            expect(tileBitmapSize(Number.NaN)).toBe(512);
        });
    });

    describe('tileDisplayRect', () => {
        it('covers a full 512px cell for interior tiles', () => {
            const rect = tileDisplayRect(1, 1, 2.0, 1000, 1000);
            expect(rect.left).toBe(512);
            expect(rect.top).toBe(512);
            expect(rect.width).toBe(512);
            expect(rect.height).toBe(512);
        });

        it('clips edge tiles to the page boundary', () => {
            // Page is 600×900 display px at this zoom → tile (1,0) is clipped
            // to 88px wide; tile (0,2) would start beyond the page.
            const right = tileDisplayRect(1, 0, 1.0, 600, 900);
            expect(right.left).toBe(512);
            expect(right.width).toBe(88);
            expect(right.height).toBe(512);

            const bottom = tileDisplayRect(0, 1, 1.0, 600, 900);
            expect(bottom.top).toBe(512);
            expect(bottom.height).toBe(388);
        });

        it('never returns non-positive sizes', () => {
            const outside = tileDisplayRect(50, 50, 1.0, 600, 900);
            expect(outside.width).toBeGreaterThanOrEqual(1);
            expect(outside.height).toBeGreaterThanOrEqual(1);
        });
    });

    describe('tileKeyString', () => {
        it('mirrors the Rust TileKey string form `{page}|{zoom}|{dpr}|{x}|{y}`', () => {
            expect(tileKeyString(1, 1.5, 2, 0, 3)).toBe('1|1.5000|2.0000|0|3');
        });
    });
});

describe('Tile bridge (wasm facade)', () => {
    it('is a no-throw no-op before WASM initialization', () => {
        // getWasmApi() throws before init; every facade entry must absorb it.
        expect(() => {
            tileFacade.updateViewport(0, 1, 1, 0, 0, 100, 100, 1);
            tileFacade.startAnimation(2);
            tileFacade.updateAnimation(1.5, 1);
            tileFacade.endAnimation(1);
            tileFacade.markRendering(0, 1, 1, 0, 0);
            tileFacade.markReady(0, 1, 1, 0, 0);
            tileFacade.resetTile(0, 1, 1, 0, 0);
            tileFacade.clearPage(0);
        }).not.toThrow();

        expect(tileFacade.nextRequest()).toBeNull();
        expect(tileFacade.stats()).toBeNull();
        expect(tileFacade.isReady(0, 1, 1, 0, 0)).toBe(false);
    });
});
