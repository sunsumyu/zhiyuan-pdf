/**
 * Gesture clarity contract — ADR-0026 (HANDOFF 未决 #0 "手势过程中也要清晰").
 *
 * The gesture reknock viewport patch renders at the EXACT visual zoom via
 * main-thread direct render (ADR-0026) — no worker round trip (p50 was
 * 70.9–90.6ms, 95% queueing), straight into the reused detail stage buffer.
 *
 * Metric (per rAF frame, zero instrumentation — same pairing as
 * zoom_gesture_tilegrid_probe): the patch's content band = the newest
 * completed detail-layer render (`ts.layer.rendered` useViewportTile=true)
 * paired with the newest `ts.layer-plan` at-or-before it (plan.displayZoom).
 * Stretch = |visualZoom / patchBand − 1|.
 *
 * Calibrated bounds (2026-10-05 forensics → ADR-0026 §Tests):
 *   - Virtual zoom applies each wheel tick's target IMMEDIATELY
 *     (on_wheel_event), so the on-screen patch can lag by up to ONE tick's
 *     jump — 16×ctrl+120 deltaY spans 1→5.28, i.e. ~10.8% per tick —
 *     regardless of render speed. max ≤ 15% pins "lag ≤ 1 tick"; the worker
 *     world (pre-0026) lagged 4–8 ticks (15–27 bands = 45–80%).
 *   - p50 = 0 pins the structural guarantee: renders land at the exact
 *     visual, so most frames are pixel-exact.
 *   - A pre-existing ~100ms per-cycle pipeline longtask (present on the
 *     worker path too) caps the reknock cadence at ~10fps in this
 *     environment; the finer per-frame bars are blocked on fixing it
 *     (HANDOFF 未决 #0 → next loop). Mutation drill: inject
 *     `__pdfGestureDirectRenderDisabled = true` (the ADR-0016-style rollback
 *     flag) → `direct ≥ 3` flips red (recorded 2× in the ADR-0026 A/B).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const ccHelpers = require('../helpers/app') as typeof import('../helpers/app');

// eslint-disable-next-line @typescript-eslint/no-var-requires
const ccPath = require('node:path') as typeof import('node:path');
const repoRoot = ccPath.resolve(__dirname, '..', '..', '..');
const fixture = ccPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

const BURST_STEPS = 16;
const BURST_GAP_MS = 120;
const GESTURE_END_MS = BURST_STEPS * BURST_GAP_MS;
const OBSERVE_MS = 2500;

const DIRECT_EVENTS_MIN = 3;
const MAX_STRETCH_MAX = 0.15; // 1 wheel tick (~10.8% on this fixture) + margin
const P50_STRETCH_MAX = 0.01; // renders land at the exact visual

async function runGestureAndSample() {
    await browser.execute((burstSteps: number, burstGapMs: number) => {
        const w = window as any;
        w.__gc = { frames: [], done: false, err: '' };
        const t0 = performance.now();
        const sample = () => {
            try {
                if (w.__gc.done || w.__gc.frames.length >= 3000) { w.__gc.done = true; return; }
                const zs = w.wasmv3?.readZoomState?.();
                const vis = zs ? +zs.visualZoom.toFixed(4) : -1;
                // Patch content band: newest completed detail-layer render
                // paired with the newest layer-plan at-or-before it. Backward
                // scan over the full history — the diagnostics flood can push
                // the newest render past a small window and corrupt the stats.
                let patchBand = -1;
                const hist: any[] = w.__PDF_DIAGNOSTICS_HISTORY || [];
                for (let i = hist.length - 1; i >= 0; i--) {
                    const e = hist[i];
                    if (e && e.event === 'ts.layer.rendered' &&
                        e.fields && e.fields.useViewportTile === true) {
                        for (let j = i; j >= 0; j--) {
                            const p = hist[j];
                            if (p && p.event === 'ts.layer-plan' &&
                                p.fields && Number.isFinite(p.fields.displayZoom)) {
                                patchBand = p.fields.displayZoom;
                                break;
                            }
                        }
                        break;
                    }
                }
                w.__gc.frames.push({
                    ms: Math.round(performance.now() - t0),
                    vis,
                    pb: patchBand > 0 ? +patchBand.toFixed(4) : -1,
                });
                requestAnimationFrame(sample);
            } catch (e: any) {
                w.__gc.err = String(e && e.message ? e.message : e);
                w.__gc.done = true;
            }
        };
        requestAnimationFrame(sample);

        const scroller = document.getElementById('pdf-scroll-container') as HTMLElement | null;
        if (!scroller) return;
        const rr = scroller.getBoundingClientRect();
        const fire = (deltaY: number) => {
            scroller.dispatchEvent(new WheelEvent('wheel', {
                bubbles: true, cancelable: true, ctrlKey: true,
                deltaY, clientX: rr.left + rr.width / 2, clientY: rr.top + rr.height / 2,
            }));
        };
        let i = 0;
        const timer = setInterval(() => {
            fire(-120); i += 1; if (i >= burstSteps) clearInterval(timer);
        }, burstGapMs);
    }, BURST_STEPS, BURST_GAP_MS);

    await browser.pause(GESTURE_END_MS + OBSERVE_MS);
    await browser.execute(() => { (window as any).__gc.done = true; });
    await browser.pause(150);

    const state: any = await browser.execute(() => ({
        frames: (window as any).__gc.frames,
        err: (window as any).__gc.err,
    }));
    const frames: Array<{ ms: number; vis: number; pb: number }> = state.frames || [];
    if (frames.length < 15) {
        throw new Error(`sampler captured too few frames: ${frames.length} err=${state.err}`);
    }
    return frames;
}

async function countDirectRenderEvents(): Promise<{ direct: number; skips: number }> {
    return browser.execute(() => {
        const hist: any[] = (window as any).__PDF_DIAGNOSTICS_HISTORY || [];
        let direct = 0;
        let skips = 0;
        for (const e of hist) {
            if (e && e.event === 'ts.layer.gesture-direct-render') direct += 1;
            if (e && e.event === 'ts.layer.gesture-direct-render.skip') skips += 1;
        }
        return { direct, skips };
    });
}

describe('Gesture clarity contract (ADR-0026)', () => {
    before(async () => {
        await ccHelpers.waitForApp();
        await ccHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('direct render pins the mid-gesture patch to the exact visual within 1 tick', async () => {
        const frames = await runGestureAndSample();

        const direct = await countDirectRenderEvents();
        expect(direct.direct).toBeGreaterThanOrEqual(DIRECT_EVENTS_MIN);

        const gestureFrames = frames.filter((f) => f.ms <= GESTURE_END_MS + 1200);
        const measurable = gestureFrames.filter((f) => f.pb > 0 && f.vis > 0);
        expect(measurable.length).toBeGreaterThanOrEqual(10);

        const visSpan = gestureFrames.length
            ? Math.max(...gestureFrames.map((f) => f.vis)) / Math.min(...gestureFrames.map((f) => f.vis))
            : 0;
        expect(visSpan).toBeGreaterThan(1.25);

        const stretches = measurable.map((f) => Math.abs(f.vis / f.pb - 1)).sort((a, b) => a - b);
        const p50 = stretches[Math.floor(stretches.length * 0.5)];
        const max = stretches[stretches.length - 1];
        console.log(
            `[clarity] direct=${direct.direct} skips=${direct.skips} n=${measurable.length} ` +
            `p50=${p50.toFixed(4)} max=${max.toFixed(4)} visSpan=${visSpan.toFixed(2)}x`,
        );

        expect(p50).toBeLessThanOrEqual(P50_STRETCH_MAX);
        expect(max).toBeLessThanOrEqual(MAX_STRETCH_MAX);
    });
});
