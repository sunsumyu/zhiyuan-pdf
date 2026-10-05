/**
 * Console-sink rate-limit contract (postmortem 2026-10-05, ADR-0015 前置条件).
 *
 * Forensics (tests/e2e/specs/zoom_longtask_attribution_probe.spec.ts, 2026-10-05):
 * a 16-step ctrl-wheel gesture under verbose tracing emitted ~2900 diagnostics;
 * the CONSOLE SINK alone (plus per-event formatting) blocked the main thread in
 * 9 long tasks / ~1.0s (max 132ms) — the "pipeline longtask train" ADR-0026
 * recorded as its next fix target. With the sink muted the train halved but did
 * not vanish; with verbose OFF (production default) the same gesture produced
 * ZERO long tasks. So the train is a diagnostic-flood artifact, and the sink is
 * its largest single component.
 *
 * Contract: emitPdfDiagnostic must rate-limit the console sink at producer side
 * (ADR-0015 "若未来重试的前置条件": 限流在生产者侧) while keeping the in-page
 * history COMPLETE (it is the authoritative probe source) and never sampling
 * ERROR/WARN (ADR-0013: the error stream is reserved for real faults).
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('../bridge/shared/wasm_loader', () => ({
    targetInvokeV3: vi.fn(() => Promise.resolve()),
}));

import {
    emitPdfDiagnostic,
    CONSOLE_SINK_WINDOW_MS,
    CONSOLE_SINK_MAX_PER_WINDOW,
    resetConsoleSinkLimiter,
} from '../bridge/shared/diagnostics';

describe('console sink rate limit (postmortem 2026-10-05)', () => {
    let history: any[];

    beforeEach(() => {
        vi.useFakeTimers();
        vi.setSystemTime(new Date('2026-10-05T12:00:00Z'));
        history = [];
        vi.stubGlobal('window', {
            __PDF_DIAGNOSTICS_VERBOSE: true,
            __PDF_DIAGNOSTICS_HISTORY: history,
        });
        resetConsoleSinkLimiter();
        vi.spyOn(console, 'log').mockImplementation(() => {});
        vi.spyOn(console, 'error').mockImplementation(() => {});
        vi.spyOn(console, 'warn').mockImplementation(() => {});
    });

    afterEach(() => {
        vi.unstubAllGlobals();
        vi.useRealTimers();
        vi.restoreAllMocks();
    });

    it('caps the console sink under a verbose flood while history stays complete', () => {
        for (let i = 0; i < 200; i++) {
            emitPdfDiagnostic('render', 'frame_complete', { i });
        }
        const logCalls = (console.log as ReturnType<typeof vi.fn>).mock.calls.length;
        expect(logCalls).toBeLessThanOrEqual(CONSOLE_SINK_MAX_PER_WINDOW);
        expect(logCalls).toBeGreaterThan(0);
        // The in-page history is the authoritative probe source — every event
        // must be recorded even when its console line is dropped.
        expect(history.length).toBe(200);
    });

    it('summarizes the suppression when the window rolls', () => {
        for (let i = 0; i < 200; i++) {
            emitPdfDiagnostic('render', 'frame_complete', { i });
        }
        const before = (console.log as ReturnType<typeof vi.fn>).mock.calls.length;
        vi.setSystemTime(new Date(Date.now() + CONSOLE_SINK_WINDOW_MS + 5));
        emitPdfDiagnostic('render', 'frame_complete', { after: true });
        const calls = (console.log as ReturnType<typeof vi.fn>).mock.calls;
        expect(calls.length).toBe(before + 2); // summary + the new event
        const summary = calls[before][0] as string;
        expect(summary).toContain('suppressed');
        expect(summary).toContain('192');
    });

    it('never samples ERROR/WARN even mid-flood', () => {
        for (let i = 0; i < 200; i++) {
            emitPdfDiagnostic('render', 'frame_complete', { i });
        }
        expect((console.log as ReturnType<typeof vi.fn>).mock.calls.length)
            .toBeLessThanOrEqual(CONSOLE_SINK_MAX_PER_WINDOW);
        emitPdfDiagnostic('render', 'decode_failed', {}, { level: 'ERROR' });
        expect(console.error).toHaveBeenCalledTimes(1);
        emitPdfDiagnostic('render', 'render_rejected', {}, { level: 'WARN' });
        expect(console.warn).toHaveBeenCalledTimes(1);
    });

    it('limits the sink regardless of verbose mode (production floods stay impossible)', () => {
        (window as any).__PDF_DIAGNOSTICS_VERBOSE = false;
        for (let i = 0; i < 200; i++) {
            emitPdfDiagnostic('render', 'frame_complete', { i });
        }
        expect((console.log as ReturnType<typeof vi.fn>).mock.calls.length)
            .toBeLessThanOrEqual(CONSOLE_SINK_MAX_PER_WINDOW);
        expect(history.length).toBe(200);
    });
});
