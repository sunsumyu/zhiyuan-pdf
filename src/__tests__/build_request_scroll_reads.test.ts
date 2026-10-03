/**
 * RED contract for ADR-0020 — `frame_plan.buildRequest` must read the scroll
 * position ONCE per call.
 *
 * CDP profile (2026-10-02): a 12-step ctrl-wheel gesture issued 416 scroll
 * DOM property reads (`get scrollLeft` was the top non-idle CPU entry, ~1.3%).
 * buildRequest ran ~6x per render and read `scrollContainer.scrollLeft` and
 * `.scrollTop` twice each — once for the layout-trace log object, once for the
 * returned request. The log object is evaluated before the function body's
 * fast-path return, so the reads happen even when tracing is disabled.
 *
 * The scroll position is written by Rust during zoom (raf_loop anchor scroll,
 * raf_committed), so caching it would need a cross-language invalidation
 * protocol; this contract only removes the unambiguous duplicate read.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const framePlan = readFileSync(
    resolve(__dirname, '../bridge/render/frame_plan.ts'),
    'utf8',
);

function buildRequestBody(): string {
    const start = framePlan.indexOf('function buildRequest');
    const end = framePlan.indexOf('function buildRenderRequest');
    expect(start).toBeGreaterThan(-1);
    expect(end).toBeGreaterThan(start);
    return framePlan.slice(start, end);
}

describe('buildRequest single scroll read (ADR-0020)', () => {
    it('reads scrollLeft / scrollTop exactly once each', () => {
        const body = buildRequestBody();
        const leftReads = body.match(/\.scrollLeft\b/g) ?? [];
        const topReads = body.match(/\.scrollTop\b/g) ?? [];
        expect(leftReads.length).toBe(1);
        expect(topReads.length).toBe(1);
    });

    it('shares the single read between the log and the returned request', () => {
        const body = buildRequestBody();
        // The locals must feed both the log object and the return value.
        expect(body).toMatch(/const\s+scrollLeft\s*=/);
        expect(body).toMatch(/const\s+scrollTop\s*=/);
        const logIdx = body.indexOf('logPdfLayoutTrace');
        const returnIdx = body.indexOf('return {');
        expect(logIdx).toBeGreaterThan(-1);
        expect(returnIdx).toBeGreaterThan(logIdx);
        // Both the log slice and the return slice reference the locals.
        expect(body.slice(logIdx, returnIdx)).toMatch(/\bscrollLeft\b/);
        expect(body.slice(returnIdx)).toMatch(/\bscrollLeft\b/);
    });
});
