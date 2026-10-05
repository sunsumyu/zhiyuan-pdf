/**
 * Gesture long-task attribution probe (ADR-0026 follow-up). MEASUREMENT ONLY —
 * no assertions, no product change.
 *
 * ADR-0026's implementation loop surfaced a "pre-existing ~100ms/cycle
 * main-thread long-task train" during ctrl-wheel gestures and recorded it as
 * the next fix target (HANDOFF #0). This probe falsifies that premise and
 * isolates the true cause with three ISOLATED phases — each starts from a
 * freshly reloaded app at zoom 1, so the only variable between phases is the
 * diagnostic configuration:
 *
 *   A  verbose tracing ON, console sink live   (what tests/e2e/helpers/app.js
 *      forces in every E2E run — NOT the production default)
 *   C  verbose tracing ON, console sink muted  (isolates the console.* sink
 *      from the rest of verbose tracing: readSnap stringify + the
 *      logPdfLayoutTrace 5-element snapshot still run)
 *   B  verbose tracing OFF                     (production default)
 *
 * Findings (2026-10-05, pre-fix):
 *   - A: long-task train present (up to 14 tasks / ~1.2s total / max ~150ms),
 *     windows packed with `readSnap` (a verboseOnly diagnostic that
 *     JSON.stringify's the session on every viewerSession.read()) and ~170
 *     getBoundingClientRect + getComputedStyle (the logPdfLayoutTrace
 *     snapshot path).
 *   - C: ZERO long tasks — muting only the console sink removes the entire
 *     train while stringify + snapshots still execute. The sink, not the
 *     instrumentation payload, is the cost (~260 console lines / 100ms).
 *   - B: ZERO long tasks — the production gesture main thread is clean.
 *
 * Post-fix expectation (console-sink rate limit, ADR-0015 precondition):
 * phase A joins C and B at ~0 long tasks; this probe's console-derived event
 * timeline becomes SAMPLED (the in-page history stays complete).
 *
 * Instruments (installed per phase, additive, no product edit):
 *   1. PerformanceObserver('longtask').
 *   2. console.* patch → emitPdfDiagnostic() always console-logs its
 *      formatted line, so wrapping console.log/error/warn gives every
 *      diagnostic event a performance.now() timestamp (the in-page history
 *      only has a wall-clock stamp).
 *   3. requestAnimationFrame patch → self-time of every rAF callback.
 *   4. Element.getBoundingClientRect / getComputedStyle / ctx.drawImage /
 *      clearRect patches → call count + cumulative self-time per frame.
 *
 * Output: e2e_longtask_attr.log (repo-root; *.log gitignored, delete after use
 * per AGENTS.md §四).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const laPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const laFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const laHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = laPath.resolve(__dirname, '..', '..', '..');
const fixture = laPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

const BURST_STEPS = 16;
const BURST_GAP_MS = 120;
const GESTURE_END_MS = BURST_STEPS * BURST_GAP_MS;
const OBSERVE_MS = 2500;

type PhaseResult = {
    label: string;
    verbose: boolean;
    mute: boolean;
    frames: any[];
    events: Array<{ t: number; ev: string }>;
    raf: Array<{ t: number; d: number }>;
    longTasks: Array<{ t: number; d: number }>;
    err: string;
    consoleMs: number;
};

function summarize(p: PhaseResult): string {
    const lt = p.longTasks.filter((e) => e.t <= GESTURE_END_MS + OBSERVE_MS);
    const ltTotal = lt.reduce((s, e) => s + e.d, 0);
    const ltMax = lt.length ? Math.max(...lt.map((e) => e.d)) : 0;
    const heavy = p.raf.filter((e) => e.d >= 8);
    const byEvent: Record<string, number> = {};
    for (const e of p.events) byEvent[e.ev] = (byEvent[e.ev] || 0) + 1;
    const top = Object.entries(byEvent).sort((a, b) => b[1] - a[1]).slice(0, 6)
        .map(([k, v]) => `${k}×${v}`).join(',');
    return `${p.label} verbose=${p.verbose} mute=${p.mute} frames=${p.frames.length} events=${p.events.length} ` +
        `longtasks=${lt.length} ltTotal=${ltTotal}ms ltMax=${ltMax}ms heavyRaf(>=8ms)=${heavy.length} ` +
        `consoleMs=${Math.round(p.consoleMs)} topEvents={${top || '-'}} err=${p.err || 'none'}`;
}

describe('Gesture long-task attribution probe (ADR-0026 follow-up)', () => {
    async function installInstrumentation(): Promise<void> {
        await browser.execute(() => {
            const w = window as any;
            w.__laRec = { events: [], on: false };
            w.__laAcc = { gbcr: 0, gbcrMs: 0, gcs: 0, gcsMs: 0, di: 0, diMs: 0, cr: 0, crMs: 0 };
            w.__laLT = [];
            w.__laMute = false;
            w.__laConsoleMs = 0;

            try {
                const obs = new PerformanceObserver((list) => {
                    for (const e of list.getEntries()) {
                        w.__laLT.push({ t: Math.round(e.startTime), d: Math.round((e as any).duration) });
                    }
                });
                obs.observe({ entryTypes: ['longtask'] });
            } catch { /* longtask unsupported */ }

            const parseEvent = (first: unknown): string | null => {
                if (typeof first !== 'string' || first.indexOf('%c') < 0) return null;
                const parts = first.split('%c');
                if (parts.length < 5) return null;
                const tail = parts[4].trim();
                const ev = tail.split(/\s+/)[0];
                return ev && ev.length < 64 ? ev : null;
            };
            for (const name of ['log', 'error', 'warn'] as const) {
                const orig = (console as any)[name].bind(console);
                (console as any)[name] = (...args: any[]) => {
                    if (w.__laRec.on) {
                        const ev = parseEvent(args[0]);
                        if (ev && w.__laRec.events.length < 40000) {
                            w.__laRec.events.push({ t: Math.round(performance.now() * 10) / 10, ev });
                        }
                        if (w.__laMute) return; // phase C: drop the actual sink
                    }
                    const t = performance.now();
                    const r = orig(...args);
                    if (w.__laRec.on) w.__laConsoleMs += performance.now() - t;
                    return r;
                };
            }

            const rafOrig = window.requestAnimationFrame.bind(window);
            (window as any).__laRafOrig = rafOrig;
            (window as any).requestAnimationFrame = (cb: FrameRequestCallback) => {
                if ((cb as any).__probeSelf) return rafOrig(cb);
                const wrapped: FrameRequestCallback = (ts: number) => {
                    const t = performance.now();
                    try { cb(ts); } finally {
                        const dur = performance.now() - t;
                        if (w.__laRec.on && dur > 0.5 && w.__laRec.raf.length < 40000) {
                            w.__laRec.raf.push({ t: Math.round(t * 10) / 10, d: Math.round(dur * 10) / 10 });
                        }
                    }
                };
                return rafOrig(wrapped);
            };
            w.__laRec.raf = [];

            const acc = w.__laAcc;
            const wrap = (obj: any, key: string, cKey: string, mKey: string) => {
                const orig = obj[key];
                if (typeof orig !== 'function') return;
                obj[key] = function (...a: any[]) {
                    const t = performance.now();
                    const r = orig.apply(this, a);
                    acc[mKey] += performance.now() - t;
                    acc[cKey] += 1;
                    return r;
                };
            };
            try {
                wrap(Element.prototype, 'getBoundingClientRect', 'gbcr', 'gbcrMs');
                wrap(window, 'getComputedStyle', 'gcs', 'gcsMs');
                const ctxProto = (window as any).CanvasRenderingContext2D?.prototype;
                if (ctxProto) {
                    wrap(ctxProto, 'drawImage', 'di', 'diMs');
                    wrap(ctxProto, 'clearRect', 'cr', 'crMs');
                }
            } catch { /* best effort */ }
        });
    }

    async function runPhase(label: string, verbose: boolean, mute: boolean): Promise<PhaseResult> {
        // Reload → identical starting state (zoom 1, fresh fixture) for every
        // phase; the ONLY variable left is the diagnostic configuration.
        await browser.reloadSession();
        await laHelpers.waitForApp();
        await laHelpers.loadFixturePdf(fixture);
        await browser.pause(1200);
        await installInstrumentation();

        await browser.execute((v: boolean, m: boolean) => {
            const w = window as any;
            w.__PDF_LAYOUT_TRACE_VERBOSE = v;
            w.__PDF_DIAGNOSTICS_VERBOSE = v;
            w.__laMute = m;
            w.__laConsoleMs = 0;
            // reset sinks
            w.__laLT.length = 0;
            w.__laRec.events.length = 0;
            w.__laRec.raf.length = 0;
            w.__laRec.on = true;
            const acc = w.__laAcc;
            for (const k of Object.keys(acc)) acc[k] = 0;
            w.__la = { frames: [], done: false, err: '', prev: { ...acc } };

            const rafOrig = w.__laRafOrig as (cb: FrameRequestCallback) => number;
            const sample = () => {
                try {
                    if (w.__la.done || w.__la.frames.length >= 3000) { w.__la.done = true; return; }
                    const now = performance.now();
                    const d: any = {};
                    for (const k of Object.keys(acc)) { d[k] = acc[k] - w.__la.prev[k]; }
                    w.__la.prev = { ...acc };
                    const zs = w.wasmv3?.readZoomState?.();
                    w.__la.frames.push({
                        t: Math.round(now * 10) / 10,
                        vis: zs ? +zs.visualZoom.toFixed(4) : -1,
                        ...d,
                    });
                    const self: FrameRequestCallback = () => sample();
                    (self as any).__probeSelf = true;
                    rafOrig(self);
                } catch (e: any) {
                    w.__la.err = String(e && e.message ? e.message : e);
                    w.__la.done = true;
                }
            };
            const boot: FrameRequestCallback = () => sample();
            (boot as any).__probeSelf = true;
            rafOrig(boot);

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
        }, verbose, mute);

        await browser.pause(GESTURE_END_MS + OBSERVE_MS);
        await browser.execute(() => { (window as any).__la.done = true; (window as any).__laRec.on = false; });
        await browser.pause(200);

        const state: any = await browser.execute(() => ({
            frames: (window as any).__la.frames,
            events: (window as any).__laRec.events.slice(),
            raf: (window as any).__laRec.raf.slice(),
            err: (window as any).__la.err,
            consoleMs: (window as any).__laConsoleMs || 0,
            visFirst: (window as any).__la.frames.length
                ? (window as any).__la.frames[0].vis : -1,
            visLast: (window as any).__la.frames.length
                ? (window as any).__la.frames[(window as any).__la.frames.length - 1].vis : -1,
        }));
        const longTasks: Array<{ t: number; d: number }> =
            await browser.execute(() => (window as any).__laLT.slice());
        await browser.pause(300);
        return {
            label, verbose, mute,
            frames: state.frames || [],
            events: state.events || [],
            raf: state.raf || [],
            longTasks,
            err: state.err || '',
            consoleMs: state.consoleMs || 0,
            // stashed for the sanity print below
            ...(state.visFirst !== undefined ? { visFirst: state.visFirst, visLast: state.visLast } : {}),
        } as PhaseResult & { visFirst: number; visLast: number };
    }

    it('attributes gesture long tasks: tracing ON / ON+muted sink / OFF (isolated phases)', async function (this: any) {
        // Three isolated phases (reload + gesture + collect each) — the suite
        // default 60s mocha timeout cannot fit them under 4-worker concurrency.
        this.timeout(300_000);
        const on = await runPhase('A', true, false);
        const muted = await runPhase('C', true, true);
        const off = await runPhase('B', false, false);

        for (const p of [on, muted, off]) {
            const extra = (p as any).visFirst !== undefined
                ? ` vis ${Number((p as any).visFirst).toFixed(3)}→${Number((p as any).visLast).toFixed(3)}`
                : '';
            console.log(`[lt-attr] ${summarize(p)}${extra}`);
            const lt = p.longTasks.filter((e) => e.t <= GESTURE_END_MS + OBSERVE_MS);
            const inWin = <T extends { t: number }>(arr: T[], s: number, e: number) =>
                arr.filter((x) => x.t >= s && x.t <= e);
            for (const e of lt) {
                const end = e.t + e.d;
                const evs = inWin(p.events, e.t, end);
                const evCount: Record<string, number> = {};
                for (const x of evs) evCount[x.ev] = (evCount[x.ev] || 0) + 1;
                const evTop = Object.entries(evCount).sort((a, b) => b[1] - a[1]).slice(0, 6)
                    .map(([k, v]) => `${k}×${v}`).join(',');
                const rcbs = inWin(p.raf, e.t, end).sort((a, b) => b.d - a.d).slice(0, 3)
                    .map((x) => `${x.t}:${x.d}ms`).join(',');
                console.log(`[lt-attr] ${p.label} lt t=${e.t} d=${e.d} events={${evTop || '-'}} raf=[${rcbs || '-'}]`);
            }
        }

        const logFile = laPath.join(repoRoot, 'e2e_longtask_attr.log');
        const dump = (p: PhaseResult) => [
            `## phase ${p.label} verbose=${p.verbose} mute=${p.mute}`,
            `# ${summarize(p)}`,
            '## longtasks (t,d)',
            ...p.longTasks.filter((e) => e.t <= GESTURE_END_MS + OBSERVE_MS).map((e) => `${e.t},${e.d}`),
            '## events (t,event)',
            ...p.events.map((e) => `${e.t},${e.ev}`),
            '## raf (t,durMs)',
            ...p.raf.map((e) => `${e.t},${e.d}`),
            '## frames (t,vis,gbcr,gbcrMs,gcs,gcsMs,di,diMs,cr,crMs)',
            ...p.frames.map((f: any) => `${f.t},${f.vis},${f.gbcr},${Math.round(f.gbcrMs * 100) / 100},${f.gcs},${Math.round(f.gcsMs * 100) / 100},${f.di},${Math.round(f.diMs * 100) / 100},${f.cr},${Math.round(f.crMs * 100) / 100}`),
            '',
        ];
        laFs.writeFileSync(logFile, [...dump(on), ...dump(muted), ...dump(off)].join('\n') + '\n');
    });
});
