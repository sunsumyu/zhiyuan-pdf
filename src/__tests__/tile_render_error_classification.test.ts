/**
 * RED contract for ADR-0022 — design-internal render aborts must not be
 * reported as ERROR failures.
 *
 * Observed (E2E, packaged binary, 2026-10-02): `ERROR [TILELAYE]
 * tile-render-failed ... error=Error: stale frame`. `stale frame` is NOT a
 * failure — vector_host resolves it as `{ aborted: true }` and the tile is
 * re-requested at the new band. But tile_layer's catch sent every error to
 * `tile-render-failed`, whose name infers ERROR → console.error PLUS one
 * `terminal_log` IPC per abort (DEBUG is the only IPC-free level).
 *
 * Fix shape: one owner for "what counts as an aborted render request"
 * (vector_page_bundle, where the error strings originate), reused by
 * vector_host and tile_layer; tile_layer routes aborts to a DEBUG-level
 * `tile-render-aborted` event and keeps `tile-render-failed` for real faults.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const bundle = readFileSync(
    resolve(__dirname, '../bridge/render/vector_page_bundle.ts'),
    'utf8',
);
const vectorHost = readFileSync(
    resolve(__dirname, '../bridge/render/vector_host.ts'),
    'utf8',
);
const tileLayer = readFileSync(
    resolve(__dirname, '../bridge/render/tile_layer.ts'),
    'utf8',
);

describe('render abort classification (ADR-0022)', () => {
    it('vector_page_bundle owns the abort predicate covering both abort strings', () => {
        expect(bundle).toMatch(/export function isAbortedRenderRequest/);
        const fn = bundle.match(/export function isAbortedRenderRequest[\s\S]*?\n\}/);
        expect(fn).not.toBeNull();
        expect(fn![0]).toMatch(/stale frame/);
        expect(fn![0]).toMatch(/stale page asset request/);
    });

    it('vector_host uses the shared predicate for its message check', () => {
        expect(vectorHost).toMatch(/isAbortedRenderRequest\(/);
        // The inline string comparisons must be gone from the host.
        expect(vectorHost).not.toMatch(/errMsg === 'stale frame'/);
    });

    it('tile_layer routes aborts to a DEBUG event and keeps ERROR for real failures', () => {
        expect(tileLayer).toMatch(/tile-render-aborted/);
        expect(tileLayer).toMatch(/isAbortedRenderRequest\(/);
        const aborted = tileLayer.match(/tile-render-aborted[\s\S]{0,200}/);
        expect(aborted).not.toBeNull();
        expect(aborted![0]).toMatch(/level:\s*'DEBUG'/);
        expect(tileLayer).toMatch(/tile-render-failed/);
    });
});
