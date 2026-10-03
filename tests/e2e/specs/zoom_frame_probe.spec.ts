/**
 * Runtime probe for the 2026-09-27 zoom frame analysis (see
 * docs/bug-postmortems/zoom-frame-analysis-2026-09-27.md).
 *
 * Replays the recorded gesture (ctrl-wheel out 100%→~26%, pause, back in to
 * ~105%) against the real Tauri webview while a page-side rAF sampler dumps
 * per-frame zoom state + presentation-surface geometry. Output lands in
 * e2e_probe.log at the repo root for offline analysis.
 *
 * This spec makes no assertions — it is a measurement instrument.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const probePath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const probeFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const probeHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = probePath.resolve(__dirname, '..', '..', '..');
const fixture = probePath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');
const logFile = probePath.join(repoRoot, 'e2e_probe.log');

const WHEEL_EVENTS = 14; // ×2^(-±120/800) per event: ~0.90 out / ~1.11 in
const WHEEL_INTERVAL_MS = 150;
const SETTLE_WINDOW_MS = 2500;

describe('Zoom frame probe', () => {
    before(async () => {
        await probeHelpers.waitForApp();
        await probeHelpers.loadFixturePdf(fixture);
        await browser.pause(1200);
    });

    it('samples every frame across out → pause → in gesture', async () => {
        // ── 1. Install the in-page rAF sampler ──
        await browser.execute(() => {
            const w = window as any;
            w.__probe = [];
            w.__probeDone = false;
            const round = (v: number) => (Number.isFinite(v) ? Math.round(v * 10) / 10 : null);
            const box = (el: HTMLElement | null) => {
                if (!el) return null;
                const r = el.getBoundingClientRect();
                const cs = getComputedStyle(el);
                return {
                    l: round(r.left), t: round(r.top), w: round(r.width), h: round(r.height),
                    vis: cs.visibility, disp: cs.display, tf: cs.transform,
                };
            };
            const t0 = performance.now();
            const sample = () => {
                const P = w.__probe;
                if (w.__probeDone || !P || P.length >= 2000) {
                    w.__probeDone = true;
                    return;
                }
                const zs = w.wasmv3?.readZoomState?.();
                const tileLayer = document.getElementById('pdf-tile-layer');
                const tileKids: any[] = [];
                if (tileLayer) {
                    for (const el of Array.from(tileLayer.children).slice(0, 6)) {
                        const c = el as HTMLElement;
                        const r = c.getBoundingClientRect();
                        const cs = getComputedStyle(c);
                        tileKids.push({ l: round(r.left), t: round(r.top), w: round(r.width), h: round(r.height), d: cs.display, v: cs.visibility });
                    }
                }
                const label = document.getElementById('pdf-zoom-select') as HTMLSelectElement | null;
                P.push({
                    ms: Math.round(performance.now() - t0),
                    z: zs ? [round(zs.targetZoom), round(zs.visualZoom), round(zs.lastRenderedZoom)] : null,
                    raf: !!w.wasmv3?.isZoomRafLoopRunning?.(),
                    label: label ? label.value : null,
                    cont: box(document.getElementById('pdf-page-container')),
                    cv: box(document.getElementById('pdf-vector-main-canvas')),
                    bk: box(document.getElementById('pdf-vector-detail-canvas')),
                    tiles: tileLayer ? tileLayer.childElementCount : -1,
                    tk: tileKids,
                    raster: box(document.getElementById('pdf-render-target')),
                });
                requestAnimationFrame(sample);
            };
            requestAnimationFrame(sample);

            // ── 2. Gesture driver: out, pause, in (page-side timers) ──
            const scroller = document.getElementById('pdf-scroll-container') as HTMLElement | null;
            if (!scroller) return;
            const r = scroller.getBoundingClientRect();
            const cx = r.left + r.width / 2;
            const cy = r.top + r.height / 2;
            const fire = (deltaY: number) => {
                scroller.dispatchEvent(new WheelEvent('wheel', {
                    bubbles: true, cancelable: true, ctrlKey: true,
                    deltaY, clientX: cx, clientY: cy,
                }));
            };
            const runSequence = (deltaY: number, count: number) => {
                let i = 0;
                const timer = setInterval(() => {
                    fire(deltaY);
                    i += 1;
                    if (i >= count) clearInterval(timer);
                }, 150);
            };
            setTimeout(() => runSequence(120, 14), 200);
            setTimeout(() => runSequence(-120, 14), 200 + 14 * 150 + 2500);
        });

        // out: 200 + 14×150 (gesture) + 2500 (settle/stall window) + margin
        await browser.pause(1100);
        await browser.takeScreenshot().then((img) =>
            probeFs.writeFileSync(probePath.join(repoRoot, 'scratch', 'probe_mid_out.png'), Buffer.from(img, 'base64')));
        await browser.pause(2500);
        await browser.takeScreenshot().then((img) =>
            probeFs.writeFileSync(probePath.join(repoRoot, 'scratch', 'probe_out_settled.png'), Buffer.from(img, 'base64')));
        await browser.pause(400);
        // in: 14×150 + settle window + margin
        await browser.pause(2400);
        await browser.takeScreenshot().then((img) =>
            probeFs.writeFileSync(probePath.join(repoRoot, 'scratch', 'probe_mid_in.png'), Buffer.from(img, 'base64')));
        await browser.pause(3200);

        // ── 3. Stop the sampler and pull the data in chunks ──
        await browser.execute(() => {
            (window as any).__probeDone = true;
        });
        await browser.pause(100);
        const total: number = await browser.execute(() => (window as any).__probe.length);
        const rows: any[] = [];
        for (let start = 0; start < total; start += 400) {
            const chunk: any[] = await browser.execute(
                (s: number) => (window as any).__probe.slice(s, s + 400),
                start,
            );
            rows.push(...chunk);
        }

        probeFs.writeFileSync(logFile, JSON.stringify(rows));
        console.log(`[zoom-frame-probe] wrote ${rows.length} frames to ${logFile}`);
        console.log(`[zoom-frame-probe] duration: ${rows.length ? rows[rows.length - 1].ms : 0}ms`);
    });
});
