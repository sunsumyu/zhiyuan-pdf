/**
 * Zoom cycle decomposition probe — "缩放时性能不行/更新 render 跟不上" forensics.
 * MEASUREMENT ONLY — no assertions, no product change.
 *
 * Known from earlier forensics (longtask attribution probe): in production
 * (verbose off) the gesture main thread is CLEAN (0 long tasks), yet the
 * render pipeline completes only ~7-11 cycles/s (~90-140ms/cycle) while the
 * direct wasm render itself is 0.9-2.5ms. This probe decomposes ONE cycle in
 * the production configuration and quantifies the reknock cadence drivers:
 *
 *   1. knock → strategy-start → page-render-duration (totalTimeMs) — where
 *      does the cycle wall time actually go?
 *   2. renders per wheel tick and the blur-suppression windows: the reknock
 *      gate requires |visual/lastRendered − 1| ≥ 2%, and the exponential
 *      ease (rate 9-18/s) decays below 2% in ~130ms — so reknocks STOP until
 *      the next wheel tick even when the pipeline is idle. Per-rAF sampling
 *      of (visual, target, lastRendered) makes those windows visible.
 *   3. the settle tail: last wheel → final render (the "zoom keeps easing
 *      after I stop scrolling" feel).
 *
 * Output: e2e_cycle_decomp.log (repo-root; *.log gitignored, delete after
 * use per AGENTS.md §四).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const cdPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const cdFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const cdHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = cdPath.resolve(__dirname, '..', '..', '..');
const fixture = cdPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

const BURST_STEPS = 16;
const BURST_GAP_MS = 120;
const GESTURE_END_MS = BURST_STEPS * BURST_GAP_MS;
const OBSERVE_MS = 3000;

describe('Zoom cycle decomposition probe (production config)', () => {
    before(async () => {
        await cdHelpers.waitForApp();
        await cdHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
        // production diagnostic configuration
        await browser.execute(() => {
            (window as any).__PDF_LAYOUT_TRACE_VERBOSE = false;
            (window as any).__PDF_DIAGNOSTICS_VERBOSE = false;
        });
    });

    it('decomposes the reknock render cycle and blur-suppression windows', async () => {
        await browser.execute((gestureEnd: number) => {
            const w = window as any;
            w.__cd = { knock: [], prof: [], frames: [], done: false, err: '' };
            w.__cdLT = [];
            const t0 = performance.now();

            try {
                new PerformanceObserver((list) => {
                    for (const e of list.getEntries()) {
                        w.__cdLT.push({ t: Math.round(e.startTime), d: Math.round((e as any).duration) });
                    }
                }).observe({ entryTypes: ['longtask'] });
            } catch { /* unsupported */ }

            // 1. wrap the reknock/settle knock entry
            const origDrain = w.__pdfDrainPendingRenderFrame;
            if (typeof origDrain === 'function') {
                w.__pdfDrainPendingRenderFrame = () => {
                    w.__cd.knock.push(Math.round((performance.now() - t0) * 10) / 10);
                    return origDrain();
                };
            }

            // 2. PROF timeline via console patch (emitPdfDiagnostic always logs)
            const profNames = new Set(['plan-build-timing', 'page-render-duration', 'reknock-phase-timing', 'canvas.visibility', 'wheel-event-timing']);
            const parse = (first: unknown): { ev: string; fields: string } | null => {
                if (typeof first !== 'string' || first.indexOf('%c') < 0) return null;
                const parts = first.split('%c');
                if (parts.length < 5) return null;
                const tail = parts[4].trim();
                const ev = tail.split(/\s+/)[0];
                return ev ? { ev, fields: tail.slice(ev.length).trim().slice(0, 120) } : null;
            };
            for (const name of ['log', 'error', 'warn'] as const) {
                const orig = (console as any)[name].bind(console);
                (console as any)[name] = (...args: any[]) => {
                    if (!w.__cd.done) {
                        const p = parse(args[0]);
                        if (p && profNames.has(p.ev) && w.__cd.prof.length < 8000) {
                            w.__cd.prof.push({ t: Math.round((performance.now() - t0) * 10) / 10, ev: p.ev, f: p.fields });
                        }
                    }
                    return orig(...args);
                };
            }

            // 3. per-rAF zoom-state sampling (blur windows)
            const sample = () => {
                try {
                    if (w.__cd.done || w.__cd.frames.length >= 3000) { w.__cd.done = true; return; }
                    const zs = w.wasmv3?.readZoomState?.();
                    if (zs) {
                        w.__cd.frames.push({
                            t: Math.round((performance.now() - t0) * 10) / 10,
                            vis: +zs.visualZoom.toFixed(4),
                            target: +zs.targetZoom.toFixed(4),
                            lastR: +zs.lastRenderedZoom.toFixed(4),
                        });
                    }
                    requestAnimationFrame(sample);
                } catch (e: any) {
                    w.__cd.err = String(e && e.message ? e.message : e);
                    w.__cd.done = true;
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
            void gestureEnd;
        }, GESTURE_END_MS);

        await browser.pause(GESTURE_END_MS + OBSERVE_MS);
        await browser.execute(() => { (window as any).__cd.done = true; });
        await browser.pause(200);

        const state: any = await browser.execute(() => ({
            knock: (window as any).__cd.knock,
            prof: (window as any).__cd.prof,
            frames: (window as any).__cd.frames,
            err: (window as any).__cd.err,
        }));
        const longTasks: Array<{ t: number; d: number }> =
            await browser.execute(() => (window as any).__cdLT || []);

        const knock: number[] = state.knock || [];
        const prof: Array<{ t: number; ev: string; f: string }> = state.prof || [];
        const frames: Array<{ t: number; vis: number; target: number; lastR: number }> = state.frames || [];
        if (frames.length < 50) throw new Error(`sampler too few frames: ${frames.length} err=${state.err}`);

        const dur = prof.filter((p) => p.ev === 'page-render-duration')
            .map((p) => ({ t: p.t, ms: parseFloat(/totalTimeMs=([\d.]+)/.exec(p.f)?.[1] ?? '-1') }))
            .filter((p) => p.ms >= 0);
        const planMs = prof.filter((p) => p.ev === 'plan-build-timing')
            .map((p) => ({ t: p.t, label: /label=(\w+)/.exec(p.f)?.[1] ?? '?', ms: parseFloat(/ms=([\d.]+)/.exec(p.f)?.[1] ?? '-1') }));
        const directMs = prof.filter((p) => p.ev === 'reknock-phase-timing')
            .map((p) => ({ t: p.t, wasmMs: parseFloat(/wasmMs=([\d.]+)/.exec(p.f)?.[1] ?? '-1') }));

        // cycle: knock → the NEXT page-render-duration completion
        const cycles: number[] = [];
        for (const k of knock) {
            const done = dur.find((d) => d.t >= k);
            if (done) cycles.push(+(done.t - k).toFixed(1));
        }
        const pct = (arr: number[], p: number) => {
            const s = [...arr].sort((a, b) => a - b);
            return s.length ? s[Math.min(s.length - 1, Math.floor(s.length * p))] : -1;
        };
        const dist = (arr: number[]) => `n=${arr.length} p50=${pct(arr, 0.5)} p90=${pct(arr, 0.9)} max=${pct(arr, 1)}`;

        // blur-suppression: fraction of gesture rAF frames where
        // |visual/lastRendered − 1| < 2% while |target − visual| > 0.1%
        const gesture = frames.filter((f) => f.t <= GESTURE_END_MS + 1500);
        let suppressed = 0;
        for (const f of gesture) {
            if (f.lastR > 0 && Math.abs(f.vis / f.lastR - 1) < 0.02 && Math.abs(f.target - f.vis) > 0.001) suppressed++;
        }
        // settle tail: last wheel (≈GESTURE_END_MS) → last render completion
        const lastDone = dur.length ? dur[dur.length - 1].t : -1;
        const lastKnock = knock.length ? knock[knock.length - 1] : -1;

        console.log(
            `[cycle] frames=${frames.length} knocks=${knock.length} renders=${dur.length} longtasks=${longTasks.length} err=${state.err || 'none'}`,
        );
        console.log(
            `[cycle] knock→complete ${dist(cycles)} ms | strategy totalTimeMs ${dist(dur.map((d) => d.ms))} ms | ` +
            `direct wasmMs ${dist(directMs.map((d) => d.wasmMs))} ms | plan-build>2ms ${dist(planMs.map((p) => p.ms))} ms (labels: ${[...new Set(planMs.map((p) => p.label))].join(',')})`,
        );
        console.log(
            `[cycle] blur-suppressed rAF frames (blur<2% while converging): ${suppressed}/${gesture.length} ` +
            `| last knock @${lastKnock} last render done @${lastDone} (tail=${(lastDone - GESTURE_END_MS).toFixed(0)}ms after last wheel)`,
        );

        const lines = [
            `# knocks=${knock.length} renders=${dur.length} longtasks=${longTasks.length}`,
            `# knock->complete ${dist(cycles)}`,
            `# strategy totalTimeMs ${dist(dur.map((d) => d.ms))}`,
            `# direct wasmMs ${dist(directMs.map((d) => d.wasmMs))}`,
            `# plan-build>2ms ${JSON.stringify(planMs)}`,
            '# knock times',
            ...knock.map((k) => `${k}`),
            '# renders (t, totalTimeMs)',
            ...dur.map((d) => `${d.t},${d.ms}`),
            '# frames (t,vis,target,lastR)',
            ...frames.map((f) => `${f.t},${f.vis},${f.target},${f.lastR}`),
        ];
        cdFs.writeFileSync(cdPath.join(repoRoot, 'e2e_cycle_decomp.log'), lines.join('\n') + '\n');
    });
});
