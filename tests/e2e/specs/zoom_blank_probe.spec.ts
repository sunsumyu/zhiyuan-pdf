/**
 * Probe for the 2026-09-30 (video i5eOPYh5IH) defect: during a large zoom-in
 * burst the ENTIRE page surface goes blank (application background #1a1a1a
 * shows through) for ~2 frames, then restores.
 *
 * Dumps the full per-frame visibility CHAIN + zoom state to a CSV-ish log so
 * the blank can be located offline. No assertions — measurement instrument.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const bpPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const bpFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const bpHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = bpPath.resolve(__dirname, '..', '..', '..');
const fixture = bpPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');
const logFile = bpPath.join(repoRoot, 'e2e_blank_probe.log');

describe('Zoom blank-surface probe', () => {
    before(async () => {
        await bpHelpers.waitForApp();
        await bpHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('samples the page-surface visibility chain across a large zoom sweep', async () => {
        await browser.execute(() => {
            const w = window as any;
            w.__bp = [];
            w.__bpDone = false;
            const gv = (el: HTMLElement | null, p: string) => (el ? getComputedStyle(el).getPropertyValue(p) : '?');
            const rw = (el: HTMLElement | null) => (el ? Math.round(el.getBoundingClientRect().width) : -1);
            const visCount = (layer: HTMLElement | null) => {
                if (!layer) return -1;
                let n = 0;
                for (const el of Array.from(layer.children) as HTMLElement[]) {
                    const c = getComputedStyle(el);
                    if (c.display !== 'none' && c.visibility !== 'hidden' && parseFloat(c.opacity || '1') > 0.01) n++;
                }
                return n;
            };
            const t0 = performance.now();
            const sample = () => {
                const P = w.__bp;
                if (w.__bpDone || !P || P.length >= 6000) { w.__bpDone = true; return; }
                const zs = w.wasmv3?.readZoomState?.();
                const wrapper = document.getElementById('pdf-content-wrapper');
                const container = document.getElementById('pdf-page-container');
                const main = document.getElementById('pdf-vector-main-canvas');
                const layer = document.getElementById('pdf-tile-layer');
                const raster = document.getElementById('pdf-render-target');
                const mw = rw(main), cw = rw(container);
                const wd = gv(wrapper, 'display');
                const cd = gv(container, 'display');
                const md = gv(main, 'display');
                const cvis = gv(container, 'visibility');
                const mvis = gv(main, 'visibility');
                const mop = gv(main, 'opacity');
                const rast = gv(raster, 'display');
                const tiles = visCount(layer);
                const painted = wd !== 'none' && cd !== 'none' && cvis !== 'hidden' &&
                    ((mw > 50 && mvis !== 'hidden' && parseFloat(mop || '1') > 0.01) || tiles > 0 || rast !== 'none');
                P.push({
                    ms: Math.round(performance.now() - t0),
                    t: zs ? +zs.targetZoom.toFixed(4) : -1,
                    v: zs ? +zs.visualZoom.toFixed(4) : -1,
                    lr: zs ? +zs.lastRenderedZoom.toFixed(4) : -1,
                    painted: painted ? 1 : 0,
                    wd, cd, md, cvis, mvis, mop, rast, tiles, mw, cw,
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
            const runSequence = (deltaY: number, count: number, gapMs = 80) => {
                let i = 0;
                const timer = setInterval(() => {
                    fire(deltaY); i += 1; if (i >= count) clearInterval(timer);
                }, gapMs);
            };
            // out to ~0.1, settle; in to ~1.0, settle; big in to ~3.5
            setTimeout(() => runSequence(120, 20), 300);
            setTimeout(() => runSequence(-120, 20), 300 + 20 * 80 + 1600);
            setTimeout(() => runSequence(-120, 14), 300 + 20 * 80 + 1600 + 20 * 80 + 1600);
        });

        await browser.pause(20 * 80 + 1600 + 20 * 80 + 1600 + 14 * 80 + 2500);
        await browser.execute(() => { (window as any).__bpDone = true; });
        await browser.pause(150);
        const total: number = await browser.execute(() => (window as any).__bp.length);
        const frames: any[] = [];
        for (let s = 0; s < total; s += 400) {
            const chunk: any[] = await browser.execute((i: number) => (window as any).__bp.slice(i, i + 400), s);
            frames.push(...chunk);
        }
        const lines = ['ms,t,v,lr,painted,wd,cd,md,cvis,mvis,mop,rast,tiles,mw,cw'];
        for (const f of frames) {
            lines.push([f.ms, f.t, f.v, f.lr, f.painted, f.wd, f.cd, f.md, f.cvis, f.mvis, f.mop, f.rast, f.tiles, f.mw, f.cw].join(','));
        }
        const bad = frames.filter((f) => !f.painted).length;
        lines.unshift(`# frames=${frames.length} unpainted=${bad} maxTarget=${Math.max(...frames.map((f) => f.t))} minTarget=${Math.min(...frames.map((f) => f.t))}`);
        bpFs.writeFileSync(logFile, lines.join('\n') + '\n');
        console.log(`[blank-probe] frames=${frames.length} unpainted=${bad} maxT=${Math.max(...frames.map((f) => f.t))} minT=${Math.min(...frames.map((f) => f.t))}`);
    });
});
