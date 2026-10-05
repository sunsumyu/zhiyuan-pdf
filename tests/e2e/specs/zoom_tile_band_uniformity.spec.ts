/**
 * Tile band uniformity contract — postmortem 2026-10-05
 * (docs/bug-postmortems/2026-10-05-rest-blur-stale-tiles.md).
 *
 * Defect: the tile grid's clear rule inspected only `presentedZoom` (the band
 * of the LAST drawn tile), so a grid that accumulated gesture-era tiles never
 * cleared once the newest tile reached the target band. Stale tiles persisted
 * in the TOP layer (tile layer z=3 above the native detail patch z=2) at REST,
 * stretched by present scale — the user screenshot showed a hard horizontal
 * seam: 1.47-band tiles at visual 1.682 = 14.4% stretch above, native below.
 * This violates the P2 contract's second half: "settle 必然清晰".
 *
 * Contract: after a multi-band zoom gesture (+ a mid-tail scroll, the
 * reproduction's trap window) and full settle, EVERY visible tile's band must
 * sit within one scheduling quantization of the target zoom. Pre-fix measured
 * worst deviation 0.212 (band 1.47 @ target 1.682); post-fix the sweep re-lays
 * the grid at the target band (≤ ~0.05 by construction).
 *
 * Non-vacuity guards: the gesture must cross ≥2 quantized bands and the grid
 * must hold ≥2 visible tiles in the checked samples (a missing grid must not
 * pass vacuously).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const buPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const buHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = buPath.resolve(__dirname, '..', '..', '..');
const fixture = buPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

const TARGET_ZOOM = 1.65;   // crosses ≥3 quantized bands from the fit zoom
const TICK_MS = 120;
const MAX_TICKS = 12;
const SETTLE_MS = 3000;     // convergence tail (~0.9s) + sweep + pump drain
const SAMPLE_COUNT = 6;
const SAMPLE_GAP_MS = 300;

const MAX_BAND_DEV = 0.09;  // pre-fix 0.212 → red; post-fix ≤ ~0.05 (quantization)

describe('Tile band uniformity after settle (postmortem 2026-10-05)', () => {
    before(async () => {
        await buHelpers.waitForApp();
        await buHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
        await browser.execute(() => {
            (window as any).__PDF_LAYOUT_TRACE_VERBOSE = false;
            (window as any).__PDF_DIAGNOSTICS_VERBOSE = false;
        });
    });

    it('converges every visible tile to the settled band', async () => {
        // multi-band zoom gesture (cursor-anchored, viewport center)
        let targetMin = Infinity;
        let targetMax = -Infinity;
        for (let i = 0; i < MAX_TICKS; i++) {
            await browser.execute((dy: number) => {
                const scroller = document.getElementById('pdf-scroll-container') as HTMLElement;
                const rr = scroller.getBoundingClientRect();
                scroller.dispatchEvent(new WheelEvent('wheel', {
                    bubbles: true, cancelable: true, ctrlKey: true,
                    deltaY: dy, clientX: rr.left + rr.width / 2, clientY: rr.top + rr.height / 2,
                }));
            }, -120);
            await browser.pause(TICK_MS);
            const t: number = await browser.execute(() => {
                const zs = (window as any).wasmv3.readZoomState();
                return +zs.targetZoom.toFixed(3);
            });
            targetMin = Math.min(targetMin, t);
            targetMax = Math.max(targetMax, t);
            if (t >= TARGET_ZOOM) break;
        }
        // the reproduction's trap: scroll during the convergence tail
        await browser.pause(350);
        await browser.execute(() => {
            const scroller = document.getElementById('pdf-scroll-container') as HTMLElement;
            scroller.scrollTop += 400;
        });
        await browser.pause(SETTLE_MS);

        const bandRange = targetMax / targetMin;
        expect(bandRange).toBeGreaterThanOrEqual(1.1); // gesture crossed ≥2 bands

        // sample the settled grid
        const samples: Array<{ visual: number; tiles: number[] }> = [];
        for (let s = 0; s < SAMPLE_COUNT; s++) {
            const snap: any = await browser.execute(() => {
                const zs = (window as any).wasmv3.readZoomState();
                const layer = document.getElementById('pdf-tile-layer');
                const bands: number[] = [];
                if (layer) {
                    for (const el of Array.from(layer.children) as HTMLElement[]) {
                        const st = getComputedStyle(el);
                        if (st.display === 'none' || st.visibility === 'hidden' || parseFloat(st.opacity || '1') <= 0.01) continue;
                        const m = /scale\(([^)]+)\)/.exec(el.style.transform || '');
                        const scale = m ? parseFloat(m[1]) : 1;
                        if (!(scale > 0) || !(zs.visualZoom > 0)) continue;
                        bands.push(+(zs.visualZoom / scale).toFixed(4));
                    }
                }
                return {
                    visual: +zs.visualZoom.toFixed(4),
                    target: +zs.targetZoom.toFixed(4),
                    bands,
                    geom: (() => {
                        const c = document.getElementById('pdf-page-container') as HTMLElement;
                        const wr = document.getElementById('pdf-content-wrapper') as HTMLElement;
                        const mc = document.getElementById('pdf-vector-main-canvas') as HTMLCanvasElement | null;
                        const cr = c.getBoundingClientRect();
                        const wrr = wr.getBoundingClientRect();
                        return {
                            container: `${cr.width.toFixed(0)}x${cr.height.toFixed(0)}`,
                            wrapper: `${wrr.width.toFixed(0)}x${wrr.height.toFixed(0)}`,
                            mainBitmap: mc ? `${mc.width}x${mc.height}` : 'none',
                            lastR: +zs.lastRenderedZoom.toFixed(4),
                        };
                    })(),
                };
            });
            samples.push({ visual: snap.visual, tiles: snap.bands, geom: snap.geom });
            await browser.pause(SAMPLE_GAP_MS);
        }

        // non-vacuity: the grid must actually be populated while checked
        const populated = samples.filter((s) => s.tiles.length >= 2);
        const settledVisualEarly = samples[samples.length - 1].visual;
        let worstDevEarly = 0;
        for (const s of samples) {
            for (const band of s.tiles) {
                worstDevEarly = Math.max(worstDevEarly, Math.abs(settledVisualEarly - band));
            }
        }
        console.log(
            `[tile-bands] targetRange=${targetMin}→${targetMax} samples=${samples.length} ` +
            `populated=${populated.length} tilesPerSample=[${samples.map((s) => s.tiles.length).join(',')}] ` +
            `bands=[${[...new Set(samples.flatMap((s) => s.tiles))].sort((a, b) => a - b).join(',')}] ` +
            `worstDev=${worstDevEarly.toFixed(4)} (limit ${MAX_BAND_DEV})`,
        );
        const events: string[] = await browser.execute(() =>
            ((window as any).__PDF_DIAGNOSTICS_HISTORY || [])
                .slice(-150)
                .map((e: any) => e.event),
        );
        console.log(`[tile-bands] geom per sample: ${samples.map((s: any) => `${s.geom.container}/${s.geom.wrapper}/bmp=${s.geom.mainBitmap}/lastR=${s.geom.lastR}`).join(' | ')}`);
        console.log(`[tile-bands] recent events: ${[...new Set(events)].join(',')}`);
        expect(populated.length).toBeGreaterThanOrEqual(3);

        // the contract: every visible tile within one scheduling step
        const settledVisual = samples[samples.length - 1].visual;
        let worstDev = 0;
        for (const s of samples) {
            for (const band of s.tiles) {
                worstDev = Math.max(worstDev, Math.abs(settledVisual - band));
            }
        }
        expect(worstDev).toBeLessThanOrEqual(MAX_BAND_DEV);
    });
});
