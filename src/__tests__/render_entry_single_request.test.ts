/**
 * RED contract for ADR-0021 — the render entry builds its plan request ONCE
 * and shares it between peek and scheduleRender.
 *
 * CDP Performance.getMetrics (2026-10-02): a 14-step ctrl-wheel gesture costs
 * +210 layouts / +73.7ms, driven by the ~190 buildRequest CALLS (each reads
 * the scroll position after the presenter dirtied styles → forced reflow).
 * executeActualRender called peek(zoom, reason) and scheduleRender(zoom,
 * reason) back-to-back with identical inputs, so each render built the
 * request twice.
 *
 * Also: the `render-current-page.scheduled` log evaluated two JSON.stringify
 * calls unconditionally, before logRenderFlow's verbose-only gate.
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const renderFlow = readFileSync(
    resolve(__dirname, '../bridge/render/render_flow.ts'),
    'utf8',
);

function executeActualRenderBody(): string {
    const start = renderFlow.indexOf('async function executeActualRender');
    const end = renderFlow.indexOf('async function presentPagePreview');
    expect(start).toBeGreaterThan(-1);
    expect(end).toBeGreaterThan(start);
    return renderFlow.slice(start, end);
}

describe('render entry builds the plan request once (ADR-0021)', () => {
    it('executeActualRender calls buildRenderRequest exactly once', () => {
        const body = executeActualRenderBody();
        const builds = body.match(/buildRenderRequest\(/g) ?? [];
        expect(builds.length).toBe(1);
    });

    it('peek and scheduleRender share the prebuilt request (3rd argument)', () => {
        const body = executeActualRenderBody();
        const peekCall = body.match(/\.peek\([^)]*\)/);
        const scheduleCall = body.match(/\.scheduleRender\([^)]*\)/);
        expect(peekCall).not.toBeNull();
        expect(scheduleCall).not.toBeNull();
        // Both calls take the shared request as their third argument.
        expect(peekCall![0]).toMatch(/,\s*request\s*\)/);
        expect(scheduleCall![0]).toMatch(/,\s*request\s*\)/);
        // No direct second request build inside the schedule call.
        expect(scheduleCall![0]).not.toMatch(/buildRequest\(/);
    });

    it('scheduled log (with JSON.stringify) is behind the verbose gate', () => {
        const body = executeActualRenderBody();
        const logIdx = body.indexOf("'render-current-page.scheduled'");
        expect(logIdx).toBeGreaterThan(-1);
        // The nearest enclosing gate above the log must be the verbose check.
        const before = body.slice(0, logIdx);
        const gate = before.lastIndexOf('verbosePdfDiagnosticsEnabled()');
        expect(gate).toBeGreaterThan(-1);
        const between = body.slice(gate, logIdx);
        // No closing brace of an enclosing block between the gate and the log.
        expect(between).not.toMatch(/\n    \}/);
    });
});
