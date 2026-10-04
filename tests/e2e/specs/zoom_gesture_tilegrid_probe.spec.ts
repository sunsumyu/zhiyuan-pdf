/**
 * Gesture tile-grid probe (postmortem P2 follow-up). MEASUREMENT ONLY — no
 * assertions, no product change.
 *
 * Purpose: characterize what the tile-layer DOM grid does during a fast
 * ctrl-wheel zoom, and where the mid-gesture content actually comes from.
 *
 * Findings (2026-10-04, baseline — see docs/bug-postmortems P2):
 *   - During the gesture the tile-layer grid is FROZEN: it holds the page-open
 *     settle tiles (renderZoom == 1), CSS-stretched by scale(visual/1) up to
 *     ~5x. The grid only re-lays at settle (ADR-0009: the DOM grid is the
 *     settle path; the mid-gesture tile stream goes to the TileManager, not the
 *     DOM).
 *   - The mid-gesture CONTENT source is the detail overlay (backCanvas,
 *     `backVis` below), refreshed on the reknock throttle
 *     (PREVIEW_REKNOCK_INTERVAL_MS = 60ms + PREVIEW_REKNOCK_BLUR_THRESHOLD =
 *     0.02, ADR-0009/0012). Its content band lag CANNOT be derived from DOM
 *     geometry (its CSS box is only the viewport rect + the ADR-0024 visual
 *     mapping, not the rendered band) — quantifying it needs render-path
 *     instrumentation, not this probe.
 *
 * So this probe measures the tile-grid lag (which is ~0 at settle and large
 * mid-gesture by design) and the backCanvas visibility window, and its real
 * value is to STOP the "patch lags 1-3 bands" reading from being mapped onto
 * the DOM grid, which is a different (and intentionally frozen) surface.
 *
 * Per rAF frame it derives each grid tile's render band from its CSS transform
 * (s = visualZoom / renderZoom => renderZoom = visualZoom / s), and records the
 * nearest-band lag + main-thread long tasks.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const cbPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const cbFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const cbHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = cbPath.resolve(__dirname, '..', '..', '..');
const fixture = cbPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

const BAND = 0.03; // GESTURE_TILE_ZOOM_STEP
const BURST_STEPS = 16;
const BURST_GAP_MS = 120; // matches zoom_p2_probe
const BURST_DELAY_MS = 0;
const GESTURE_END_MS = BURST_DELAY_MS + BURST_STEPS * BURST_GAP_MS;

describe('Gesture tile-grid probe (P2)', () => {
    before(async () => {
        await cbHelpers.waitForApp();
        await cbHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('measures tile content band lag + long tasks across a fast zoom-in', async () => {
        await browser.execute((gestureEnd: number, BAND: number) => {
            const w = window as any;
            w.__cb = { frames: [], done: false };
            w.__cbLT = [];
            try {
                const obs = new PerformanceObserver((list) => {
                    for (const e of list.getEntries()) {
                        w.__cbLT.push({ t: Math.round(e.startTime), d: Math.round((e as any).duration) });
                    }
                });
                obs.observe({ entryTypes: ['longtask'] });
            } catch { /* longtask unsupported */ }
            const t0 = performance.now();
            const scaleOf = (el: HTMLElement): number => {
                const t = el.style.transform || '';
                const m = /scale\(([^)]+)\)/.exec(t);
                return m ? parseFloat(m[1]) : 1;
            };
            const sample = () => {
                w.__cb.ticks = (w.__cb.ticks || 0) + 1;
                try {
                    if (w.__cb.done || w.__cb.frames.length >= 3000) { w.__cb.done = true; return; }
                const zs = w.wasmv3?.readZoomState?.();
                const vis = zs ? zs.visualZoom : -1;
                const layer = document.getElementById('pdf-tile-layer');
                const bands: number[] = [];
                if (layer && vis > 0) {
                    for (const el of Array.from(layer.children) as HTMLElement[]) {
                        const c = getComputedStyle(el);
                        if (c.display === 'none' || c.visibility === 'hidden') continue;
                        if (parseFloat(c.opacity || '1') <= 0.01) continue;
                        const s = scaleOf(el);
                        if (!(s > 0)) continue;
                        bands.push(vis / s); // renderZoom of this tile
                    }
                }
                let nearestLag = -1, minBand = -1, maxBand = -1, n = bands.length;
                // Detail overlay (backCanvas) — the mid-gesture content source.
                let backVis = 0;
                const back = document.getElementById('pdf-vector-detail-canvas') as HTMLElement | null;
                if (back) {
                    const bc = getComputedStyle(back);
                    if (bc.display !== 'none' && bc.visibility !== 'hidden' && parseFloat(bc.opacity || '1') > 0.01) {
                        const bb = back.getBoundingClientRect();
                        if (bb.width > 0 && bb.height > 0) backVis = 1;
                    }
                }
                if (n > 0 && vis > 0) {
                    minBand = Math.min(...bands);
                    maxBand = Math.max(...bands);
                    let best = Infinity;
                    for (const b of bands) {
                        const d = Math.abs(b - vis);
                        if (d < best) best = d;
                    }
                    nearestLag = best / BAND; // bands behind the freshest content
                }
                w.__cb.frames.push({
                    ms: Math.round(performance.now() - t0),
                    vis: vis > 0 ? +vis.toFixed(4) : -1,
                    n, minBand: minBand > 0 ? +minBand.toFixed(4) : -1,
                    maxBand: maxBand > 0 ? +maxBand.toFixed(4) : -1,
                    lag: nearestLag >= 0 ? +nearestLag.toFixed(2) : -1,
                    bv: backVis,
                });
                requestAnimationFrame(sample);
                } catch (e: any) {
                    w.__cb.err = String(e && e.message ? e.message : e);
                    w.__cb.done = true;
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
        }, GESTURE_END_MS, BAND);

        await browser.pause(GESTURE_END_MS + 2500);
        await browser.execute(() => { (window as any).__cb.done = true; });
        await browser.pause(150);

        const total: number = await browser.execute(() => (window as any).__cb.frames.length);
        const ticks: number = await browser.execute(() => (window as any).__cb.ticks || 0);
        const probeErr: string = await browser.execute(() => (window as any).__cb.err || '');
        const frames: Array<{ ms: number; vis: number; n: number; minBand: number; maxBand: number; lag: number }> = [];
        for (let s = 0; s < total; s += 500) {
            const chunk = await browser.execute((i: number) => (window as any).__cb.frames.slice(i, i + 500), s);
            frames.push(...(chunk as any[]));
        }
        const longTasks: Array<{ t: number; d: number }> = await browser.execute(() => (window as any).__cbLT || []);

        const gesture = frames.filter((f) => f.ms <= GESTURE_END_MS + 200 && f.lag >= 0);
        const lags = gesture.map((f) => f.lag).sort((a, b) => a - b);
        const q = (p: number) => (lags.length ? lags[Math.min(lags.length - 1, Math.floor(lags.length * p))] : -1);
        const visSpan = gesture.length ? Math.max(...gesture.map((f) => f.vis)) / Math.min(...gesture.map((f) => f.vis)) : 0;
        const tileCounts = gesture.map((f) => f.n);
        const ltMax = longTasks.length ? Math.max(...longTasks.map((e) => e.d)) : 0;

        console.log(
            `[band-lag] frames=${frames.length} ticks=${ticks} err=${probeErr || 'none'} gestureFrames=${gesture.length} visSpan=${visSpan.toFixed(2)}x ` +
            `tilesPerFrame(max)=${tileCounts.length ? Math.max(...tileCounts) : 0} ` +
            `lagBands p50=${q(0.5)} p90=${q(0.9)} max=${lags.length ? lags[lags.length - 1] : -1} ` +
            `longtasks=${longTasks.length} ltMax=${ltMax}ms`,
        );
        // Raw per-frame dump for offline inspection (repo-root; deleted after).
        const logFile = cbPath.join(repoRoot, 'e2e_band_probe.log');
        const lines = [
            `# frames=${frames.length} gestureFrames=${gesture.length} visSpan=${visSpan.toFixed(3)} ` +
            `lag p50=${q(0.5)} p90=${q(0.9)} max=${lags.length ? lags[lags.length - 1] : -1} ` +
            `longtasks=${longTasks.length} ltMax=${ltMax}ms`,
            'ms,vis,n,minBand,maxBand,lagBands,backVis',
            ...frames.map((f) => `${f.ms},${f.vis},${f.n},${f.minBand},${f.maxBand},${f.lag},${(f as any).bv}`),
        ];
        cbFs.writeFileSync(logFile, lines.join('\n') + '\n');
    });
});
