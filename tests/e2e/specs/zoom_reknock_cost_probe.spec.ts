/**
 * Reknock cost probe — candidate-A feasibility (HANDOFF 未决 #0). MEASUREMENT
 * ONLY — no assertions, no product change.
 *
 * Candidate A = re-render the detail (viewport) patch EVERY FRAME during the
 * gesture instead of on the 60ms reknock throttle, so the visible viewport is
 * native-sharp mid-gesture (user requirement: "手势过程中也清晰").
 *
 * Arithmetic this probe feeds: per-frame cadence needs one full render cycle
 * (main-thread wasm segment + worker round trip + main-thread blit + present)
 * to fit in ~16ms. The existing `reknock-phase-timing` PROF event already
 * measures exactly those phases per completed layer render, with
 * `tile=true` marking the VIEWPORT (detail-layer) renders — the same cost
 * profile candidate A would need at 60Hz.
 *
 * Reports, over a fast zoom-in gesture:
 *   - per-phase cost distributions for tile=true (viewport) vs tile=false
 *     (full-page) render cycles: wasmMs / roundMs / workerMs / blitMs;
 *   - sustained reknock completion cadence (events/s, inter-completion ms)
 *     — the rate the pipeline ALREADY sustains under the 60ms throttle;
 *   - main-thread long tasks, to show what the throttle may be protecting.
 *
 * Verdict rule for the grilling session: if viewport roundMs + blitMs is
 * well under ~16ms and the pipeline already sustains a higher rate than
 * 60/s worth of headroom, candidate A is feasible without a GPU; if the
 * cycle is long or longtasks track reknock completions, the throttle is
 * load-bearing and candidate A needs candidate B (GPU) or a redesign.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const rcPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const rcFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const rcHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = rcPath.resolve(__dirname, '..', '..', '..');
const fixture = rcPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

const BURST_STEPS = 16;
const BURST_GAP_MS = 120; // matches zoom_p2_probe
const OBSERVE_MS = 2500; // post-gesture convergence window to keep sampling

describe('Reknock cost probe (candidate-A feasibility)', () => {
    before(async () => {
        await rcHelpers.waitForApp();
        await rcHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('measures per-phase cost + cadence of viewport vs full-page reknock renders', async () => {
        await browser.execute(() => {
            const w = window as any;
            w.__rc = { frames: [], cycles: [], done: false, scanPtr: 0, err: '' };
            w.__rcLT = [];
            try {
                const obs = new PerformanceObserver((list) => {
                    for (const e of list.getEntries()) {
                        w.__rcLT.push({ t: Math.round(e.startTime), d: Math.round((e as any).duration) });
                    }
                });
                obs.observe({ entryTypes: ['longtask'] });
            } catch { /* longtask unsupported */ }

            const t0 = performance.now();
            const sample = () => {
                try {
                    if (w.__rc.done || w.__rc.frames.length >= 3000) { w.__rc.done = true; return; }
                    const zs = w.wasmv3?.readZoomState?.();
                    const vis = zs ? +zs.visualZoom.toFixed(4) : -1;
                    // Drain new diagnostics entries since last frame — count
                    // completed reknock render cycles and grab their phases.
                    const hist: any[] = w.__PDF_DIAGNOSTICS_HISTORY || [];
                    let i = Math.min(w.__rc.scanPtr, hist.length);
                    let newCycles = 0;
                    for (; i < hist.length; i++) {
                        const e = hist[i];
                        if (e && e.event === 'reknock-phase-timing' && e.fields) {
                            newCycles += 1;
                            w.__rc.cycles.push({
                                ms: Math.round(performance.now() - t0),
                                tile: e.fields.tile ? 1 : 0,
                                wasmMs: e.fields.wasmMs, roundMs: e.fields.roundMs,
                                workerMs: e.fields.workerMs, blitMs: e.fields.blitMs,
                                w: e.fields.w, h: e.fields.h,
                            });
                        }
                    }
                    w.__rc.scanPtr = i;
                    w.__rc.frames.push({
                        ms: Math.round(performance.now() - t0),
                        vis, cycles: w.__rc.cycles.length, newCycles,
                    });
                    requestAnimationFrame(sample);
                } catch (e: any) {
                    w.__rc.err = String(e && e.message ? e.message : e);
                    w.__rc.done = true;
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
                fire(-120); i += 1; if (i >= 16) clearInterval(timer);
            }, 120);
        });

        await browser.pause(16 * 120 + OBSERVE_MS);
        await browser.execute(() => { (window as any).__rc.done = true; });
        await browser.pause(150);

        const state: any = await browser.execute(() => ({
            frames: (window as any).__rc.frames,
            cycles: (window as any).__rc.cycles,
            err: (window as any).__rc.err,
            ticks: (window as any).__rc.frames.length,
        }));
        const longTasks: Array<{ t: number; d: number }> = await browser.execute(() => (window as any).__rcLT || []);
        const frames = state.frames || [];
        const cycles = state.cycles || [];
        if (frames.length < 20) {
            throw new Error(`sampler captured too few frames: ${frames.length} err=${state.err}`);
        }

        const pct = (arr: number[], p: number) => {
            const s = [...arr].filter((v) => v >= 0).sort((a, b) => a - b);
            return s.length ? s[Math.min(s.length - 1, Math.floor(s.length * p))] : -1;
        };
        const dist = (arr: number[]) =>
            `n=${arr.length} p50=${pct(arr, 0.5)} p90=${pct(arr, 0.9)} max=${pct(arr, 1)}`;

        const vp = cycles.filter((c) => c.tile === 1);
        const fp = cycles.filter((c) => c.tile === 0);
        const vpCycle = vp.map((c) => (c.roundMs || 0) + (c.blitMs || 0));
        // Sustained completion cadence while the gesture was running.
        const gestureEndMs = 16 * 120;
        const gestureCycles = vp.filter((c) => c.ms <= gestureEndMs + 300);
        let cadence = -1;
        if (gestureCycles.length >= 2) {
            const span = gestureCycles[gestureCycles.length - 1].ms - gestureCycles[0].ms;
            cadence = span > 0 ? Math.round(gestureCycles.length / (span / 1000) * 10) / 10 : -1;
        }

        console.log(
            `[reknock-cost] frames=${frames.length} err=${state.err || 'none'} ` +
            `VIEWPORT(tile=true) ${dist(vp.map((c) => c.roundMs))} roundMs | ${dist(vp.map((c) => c.workerMs))} workerMs | ${dist(vp.map((c) => c.blitMs))} blitMs | ${dist(vp.map((c) => c.wasmMs))} wasmMs | ` +
            `cycle(round+blit) ${dist(vpCycle)} ms | ` +
            `FULLPAGE(tile=false) ${dist(fp.map((c) => c.roundMs))} roundMs | ` +
            `gesture viewport completions=${gestureCycles.length} sustainedRate=${cadence}/s ` +
            `longtasks=${longTasks.length} ltMax=${longTasks.length ? Math.max(...longTasks.map((e) => e.d)) : 0}ms`,
        );

        const logFile = rcPath.join(repoRoot, 'e2e_reknock_cost.log');
        const lines = [
            `# viewport roundMs ${dist(vp.map((c) => c.roundMs))} | workerMs ${dist(vp.map((c) => c.workerMs))} | blitMs ${dist(vp.map((c) => c.blitMs))} | wasmMs ${dist(vp.map((c) => c.wasmMs))}`,
            `# fullpage roundMs ${dist(fp.map((c) => c.roundMs))}`,
            `# gesture viewport completions=${gestureCycles.length} sustainedRate=${cadence}/s longtasks=${longTasks.length}`,
            'ms,tile,wasmMs,roundMs,workerMs,blitMs,w,h',
            ...cycles.map((c) => `${c.ms},${c.tile},${c.wasmMs},${c.roundMs},${c.workerMs},${c.blitMs},${c.w},${c.h}`),
        ];
        rcFs.writeFileSync(logFile, lines.join('\n') + '\n');
    });
});
