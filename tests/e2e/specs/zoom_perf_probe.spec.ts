/**
 * Perf probe for the 2026-09-28 follow-up: mid-gesture main-thread starvation
 * (~7fps rAF in zoom_frame_probe while settled runs at ~51fps).
 *
 * Measures, with NO screenshots or driver traffic inside the measured window:
 *   - per-wheel synchronous handler cost (dispatchEvent round-trip)
 *   - longtask (>50ms) durations via PerformanceObserver
 *   - rAF frame-gap distribution during gesture vs settled
 *   - PROF / render-chain diagnostics from __PDF_DIAGNOSTICS_HISTORY sliced
 *     to the gesture window (verbose tracing enabled in-page)
 *
 * No assertions — measurement instrument. Output: scratch/perf_probe.json.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const perfPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const perfFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const perfHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = perfPath.resolve(__dirname, '..', '..', '..');
const fixture = perfPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');
const outFile = perfPath.join(repoRoot, 'scratch', 'perf_probe.json');

const WHEEL_EVENTS = 12;
const WHEEL_INTERVAL_MS = 150;
const SETTLE_MS = 3000;

describe('Zoom perf probe', () => {
    before(async () => {
        await perfHelpers.waitForApp();
        await perfHelpers.loadFixturePdf(fixture);
        await browser.pause(1200);
    });

    it('samples main-thread cost across a wheel gesture', async () => {
        await browser.execute(() => {
            const w = window as any;
            w.__perf = {
                wheelSync: [] as number[],
                longtasks: [] as { t: number; d: number }[],
                raf: [] as number[],
                beat: [] as number[],
                t0: 0,
                done: false,
            };
            // NOTE: __PDF_DIAGNOSTICS_VERBOSE deliberately left OFF — with it
            // on, the diagnostic flood (console + IPC per event) becomes part
            // of the measured workload and masks the production path.
            if (w.__PDF_DIAGNOSTICS_HISTORY) w.__PDF_DIAGNOSTICS_HISTORY.length = 0;

            const po = new PerformanceObserver((list) => {
                for (const e of list.getEntries()) {
                    w.__perf.longtasks.push({ t: Math.round(e.startTime), d: Math.round(e.duration) });
                }
            });
            try { po.observe({ entryTypes: ['longtask'] }); } catch { /* not supported */ }

            const rafTick = () => {
                if (w.__perf.done) return;
                w.__perf.raf.push(Math.round(performance.now() - w.__perf.t0));
                requestAnimationFrame(rafTick);
            };
            // Macrotask heartbeat: fires whenever the main thread gets a spare
            // slot — tight while only the compositor/GPU is busy, gapped when
            // the main thread itself is blocked.
            const beat = () => {
                if (w.__perf.done) return;
                w.__perf.beat.push(Math.round(performance.now() - w.__perf.t0));
            };

            const scroller = document.getElementById('pdf-scroll-container') as HTMLElement | null;
            if (!scroller) return;
            const r = scroller.getBoundingClientRect();
            const fire = () => {
                const t0 = performance.now();
                scroller.dispatchEvent(new WheelEvent('wheel', {
                    bubbles: true, cancelable: true, ctrlKey: true,
                    deltaY: 120, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2,
                }));
                return performance.now() - t0;
            };

            setTimeout(() => {
                w.__perf.t0 = performance.now();
                requestAnimationFrame(rafTick);
                setInterval(beat, 5);
                let i = 0;
                const timer = setInterval(() => {
                    w.__perf.wheelSync.push(Math.round(fire() * 100) / 100);
                    i += 1;
                    if (i >= 12) clearInterval(timer);
                }, 150);
            }, 200);
        });

        // gesture 200+12*150 ≈ 2000ms + settle window; no screenshots inside
        await browser.pause(2000 + SETTLE_MS);

        const data = await browser.execute(() => {
            const w = window as any;
            w.__perf.done = true;
            const hist = (w.__PDF_DIAGNOSTICS_HISTORY || []) as any[];
            return {
                t0Perf: w.__perf.t0,
                wheelSync: w.__perf.wheelSync,
                longtasks: w.__perf.longtasks,
                raf: w.__perf.raf,
                beat: w.__perf.beat,
                diag: hist.map((h) => ({ m: h.message })),
            };
        });

        perfFs.mkdirSync(perfPath.join(repoRoot, 'scratch'), { recursive: true });
        perfFs.writeFileSync(outFile, JSON.stringify(data));
        const gaps: number[] = [];
        for (let i = 1; i < data.raf.length; i++) gaps.push(data.raf[i] - data.raf[i - 1]);
        const gestureGaps = gaps.filter((g, idx) => data.raf[idx] < 2200);
        const settledGaps = gaps.filter((g, idx) => data.raf[idx] > 4200);
        const med = (a: number[]) => (a.length ? a.slice().sort((x, y) => x - y)[Math.floor(a.length / 2)] : -1);
        const p90 = (a: number[]) => (a.length ? a.slice().sort((x, y) => x - y)[Math.floor(a.length * 0.9)] : -1);
        console.log(`[perf-probe] rAF gaps gesture: median=${med(gestureGaps)} p90=${p90(gestureGaps)} n=${gestureGaps.length}`);
        console.log(`[perf-probe] rAF gaps settled: median=${med(settledGaps)} p90=${p90(settledGaps)} n=${settledGaps.length}`);
        console.log(`[perf-probe] wheel sync ms: ${JSON.stringify(data.wheelSync)}`);
        const lt = data.longtasks.filter((l: any) => l.t < 2600);
        console.log(`[perf-probe] longtasks in gesture window: n=${lt.length} max=${lt.reduce((m: number, l: any) => Math.max(m, l.d), 0)}`);
        console.log(`[perf-probe] wrote ${outFile}`);
    });
});
