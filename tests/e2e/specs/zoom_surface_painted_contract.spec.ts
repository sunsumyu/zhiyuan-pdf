/**
 * ADR-0011 page-surface painted contract.
 *
 * The 2026-09-30 recording (video i5eOPYh5IH) showed the ENTIRE page surface
 * vanish for ~2 frames during a large zoom-in — the app background (#1a1a1a)
 * showed through — then restore. Root cause class: page-surface visibility had
 * >=6 writers with no single owner, so a "both surfaces hidden" intermediate
 * state could be composited. PresentationSurfaceOwner is now the single writer
 * and performs every swap show-target-before-hide-source.
 *
 * Two checks, run against the real Tauri webview:
 *   1. NON-VACUITY — the detector flags a deliberately hidden surface. Without
 *      this, a "0 unpainted frames" result could just mean a blind sampler.
 *   2. INVARIANT — across a large zoom sweep (heavy window + burst), the page
 *      surface is painted on EVERY frame, and never goes
 *      painted → unpainted → painted (the flicker signature).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const scPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const scHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = scPath.resolve(__dirname, '..', '..', '..');
const fixture = scPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

describe('Zoom page-surface painted contract (ADR-0011)', () => {
    before(async () => {
        await scHelpers.waitForApp();
        await scHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('detector is non-vacuous: a hidden surface is flagged unpainted', async () => {
        // Install the sampler (same predicate the invariant test uses).
        await browser.execute(() => {
            const w = window as any;
            const gv = (el: HTMLElement | null, p: string) =>
                el ? getComputedStyle(el).getPropertyValue(p) : '?';
            w.__surfPainted = () => {
                const wrapper = document.getElementById('pdf-content-wrapper');
                const container = document.getElementById('pdf-page-container');
                const main = document.getElementById('pdf-vector-main-canvas');
                const raster = document.getElementById('pdf-render-target');
                const layer = document.getElementById('pdf-tile-layer');
                const wd = gv(wrapper, 'display');
                const cd = gv(container, 'display');
                const cvis = gv(container, 'visibility');
                const md = gv(main, 'display');
                const mvis = gv(main, 'visibility');
                const mop = gv(main, 'opacity');
                const mw = main ? main.getBoundingClientRect().width : 0;
                let tiles = 0;
                if (layer) {
                    for (const el of Array.from(layer.children) as HTMLElement[]) {
                        const c = getComputedStyle(el);
                        if (c.display !== 'none' && c.visibility !== 'hidden' && parseFloat(c.opacity || '1') > 0.01) tiles++;
                    }
                }
                const mainPainted = mw > 50 && md !== 'none' && mvis !== 'hidden' && parseFloat(mop || '1') > 0.01;
                const rasterPainted = !!raster && gv(raster, 'display') !== 'none';
                return wd !== 'none' && cd !== 'none' && cvis !== 'hidden' &&
                    (mainPainted || rasterPainted || tiles > 0);
            };
        });

        const paintedNow: boolean = await browser.execute(() => (window as any).__surfPainted());
        expect(paintedNow).toBe(true);

        // Hide the container for a beat, sample, then restore.
        const detected: boolean = await browser.execute(() => {
            const container = document.getElementById('pdf-page-container') as HTMLElement | null;
            if (!container) return false;
            const prevDisplay = container.style.display;
            const prevVis = container.style.visibility;
            container.style.display = 'none';
            container.style.visibility = 'hidden';
            const detectedHidden = !(window as any).__surfPainted();
            container.style.display = prevDisplay;
            container.style.visibility = prevVis;
            return detectedHidden;
        });
        expect(detected).toBe(true);
    });

    it('keeps the page surface painted on every frame of a large zoom sweep', async () => {
        // Heavy window — the recorded blank appeared under big-bitmap load.
        try {
            await browser.setWindowSize(1500, 950);
        } catch {
            /* best effort; some drivers cap window size */
        }
        await browser.pause(400);

        await browser.execute(() => {
            const w = window as any;
            w.__spFrames = [];
            w.__spDone = false;
            const t0 = performance.now();
            const sample = () => {
                if (w.__spDone || w.__spFrames.length >= 6000) {
                    w.__spDone = true;
                    return;
                }
                const zs = w.wasmv3?.readZoomState?.();
                w.__spFrames.push({
                    ms: Math.round(performance.now() - t0),
                    v: zs ? +zs.visualZoom.toFixed(4) : -1,
                    painted: w.__surfPainted() ? 1 : 0,
                });
                requestAnimationFrame(sample);
            };
            requestAnimationFrame(sample);

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
            const runSequence = (deltaY: number, count: number, gapMs: number) => {
                let i = 0;
                const timer = setInterval(() => {
                    fire(deltaY);
                    i += 1;
                    if (i >= count) clearInterval(timer);
                }, gapMs);
            };
            // out to ~0.5, settle, then a fast burst back in past 1.0 (the
            // recorded 0.54→2.45 jump class).
            setTimeout(() => runSequence(120, 16, 90), 300);
            setTimeout(() => runSequence(-120, 24, 35), 300 + 16 * 90 + 1500);
        });

        await browser.pause(300 + 16 * 90 + 1500 + 24 * 35 + 2500);
        await browser.execute(() => {
            (window as any).__spDone = true;
        });
        await browser.pause(150);

        const total: number = await browser.execute(() => (window as any).__spFrames.length);
        const frames: Array<{ ms: number; v: number; painted: number }> = [];
        for (let s = 0; s < total; s += 500) {
            const chunk = await browser.execute(
                (i: number) => (window as any).__spFrames.slice(i, i + 500),
                s,
            );
            frames.push(...(chunk as any[]));
        }
        if (frames.length < 60) {
            throw new Error(`sampler captured too few frames: ${frames.length}`);
        }

        // Unpainted frames + painted→unpainted→painted transitions.
        const unpainted: number[] = [];
        let flickers = 0;
        for (let i = 0; i < frames.length; i++) {
            if (!frames[i].painted) unpainted.push(i);
            if (i >= 2 && frames[i].painted && !frames[i - 1].painted && frames[i - 2].painted) {
                flickers += 1;
            }
        }
        const maxVisual = Math.max(...frames.map((f) => f.v));
        const summary = `frames=${frames.length} unpainted=${unpainted.length} flickers=${flickers} maxVisual=${maxVisual}`;
        console.log(`[surface-contract] ${summary}`);
        if (unpainted.length > 0 || flickers > 0) {
            const at = unpainted.slice(0, 8).map((i) => `f${i}@${frames[i].ms}ms(v=${frames[i].v})`).join(' ');
            throw new Error(`page surface went unpainted (${summary})\n${at}`);
        }
    });
});
