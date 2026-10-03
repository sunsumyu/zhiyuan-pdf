/**
 * Exact-condition repro probe for the 2026-09-30 blank-surface defect.
 * The recording was a large (2560x1360) window with fast, large-magnitude
 * ctrl-wheel bursts. This probe matches: resized window + deltaY=±240 at
 * ~35ms cadence, and samples the surface painted-ness chain every rAF frame
 * PLUS records max main-thread frame gap (starvation).
 *
 * No assertions — measurement instrument. Output → e2e_bigburst_probe.log.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const bqPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const bqFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const bqHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = bqPath.resolve(__dirname, '..', '..', '..');
const fixture = bqPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');
const logFile = bqPath.join(repoRoot, 'e2e_bigburst_probe.log');

describe('Big-burst blank repro', () => {
    before(async () => {
        await bqHelpers.waitForApp();
        try { await browser.setWindowSize(1500, 950); } catch {}
        await bqHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('samples surface painted-ness across fast large-magnitude bursts', async () => {
        await browser.execute(() => {
            const w = window as any;
            w.__bq = [];
            w.__bqDone = false;
            const gv = (el: HTMLElement | null, p: string) => (el ? getComputedStyle(el).getPropertyValue(p) : '?');
            const visCount = (layer: HTMLElement | null) => {
                if (!layer) return -1;
                let n = 0;
                for (const el of Array.from(layer.children) as HTMLElement[]) {
                    const c = getComputedStyle(el);
                    if (c.display !== 'none' && c.visibility !== 'hidden' && parseFloat(c.opacity || '1') > 0.01) n++;
                }
                return n;
            };
            let last = performance.now();
            const t0 = last;
            const sample = () => {
                const now = performance.now();
                const P = w.__bq;
                if (w.__bqDone || !P || P.length >= 6000) { w.__bqDone = true; return; }
                const gap = Math.round(now - last); last = now;
                const zs = w.wasmv3?.readZoomState?.();
                const wrapper = document.getElementById('pdf-content-wrapper');
                const container = document.getElementById('pdf-page-container');
                const main = document.getElementById('pdf-vector-main-canvas') as HTMLCanvasElement | null;
                const layer = document.getElementById('pdf-tile-layer');
                const raster = document.getElementById('pdf-render-target');
                const mw = main ? Math.round(main.getBoundingClientRect().width) : -1;
                const cd = gv(container, 'display'), cvis = gv(container, 'visibility');
                const md = gv(main, 'display'), mvis = gv(main, 'visibility'), mop = gv(main, 'opacity');
                const tiles = visCount(layer);
                const painted = gv(wrapper, 'display') !== 'none' && cd !== 'none' && cvis !== 'hidden' &&
                    ((mw > 50 && mvis !== 'hidden' && parseFloat(mop || '1') > 0.01) || tiles > 0 || gv(raster, 'display') !== 'none');
                P.push({
                    ms: Math.round(now - t0), gap,
                    t: zs ? +zs.targetZoom.toFixed(4) : -1,
                    v: zs ? +zs.visualZoom.toFixed(4) : -1,
                    painted: painted ? 1 : 0,
                    cd, cvis, md, mvis, mop, tiles, mw,
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
                    bubbles: true, cancelable: true, ctrlKey: true, deltaY, clientX: cx, clientY: cy,
                }));
            };
            const runSequence = (deltaY: number, count: number, gapMs = 35) => {
                let i = 0;
                const timer = setInterval(() => { fire(deltaY); i += 1; if (i >= count) clearInterval(timer); }, gapMs);
            };
            setTimeout(() => runSequence(240, 20), 300);
            setTimeout(() => runSequence(-240, 20), 300 + 20 * 35 + 1400);
            setTimeout(() => runSequence(-240, 16), 300 + 20 * 35 + 1400 + 20 * 35 + 1400);
        });

        await browser.pause(20 * 35 + 1400 + 20 * 35 + 1400 + 16 * 35 + 2500);
        await browser.execute(() => { (window as any).__bqDone = true; });
        await browser.pause(150);
        const total: number = await browser.execute(() => (window as any).__bq.length);
        const frames: any[] = [];
        for (let s = 0; s < total; s += 400) {
            const chunk: any[] = await browser.execute((i: number) => (window as any).__bq.slice(i, i + 400), s);
            frames.push(...chunk);
        }
        const lines = ['ms,gap,t,v,painted,cd,cvis,md,mvis,mop,tiles,mw'];
        for (const f of frames) lines.push([f.ms, f.gap, f.t, f.v, f.painted, f.cd, f.cvis, f.md, f.mvis, f.mop, f.tiles, f.mw].join(','));
        const bad = frames.filter((f) => !f.painted).length;
        const maxGap = Math.max(...frames.map((f) => f.gap));
        lines.unshift(`# frames=${frames.length} unpainted=${bad} maxGap=${maxGap} maxT=${Math.max(...frames.map((f) => f.t))} minT=${Math.min(...frames.map((f) => f.t))}`);
        bqFs.writeFileSync(logFile, lines.join('\n') + '\n');
        console.log(`[bigburst-probe] frames=${frames.length} unpainted=${bad} maxGap=${maxGap} maxT=${Math.max(...frames.map((f) => f.t))} minT=${Math.min(...frames.map((f) => f.t))}`);
    });
});
