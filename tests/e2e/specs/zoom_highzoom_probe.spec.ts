/**
 * High-zoom blank probe. The 2026-09-30 video ends with a large zoom-in step
 * and a ~2-frame whole-surface blank. Large zoom drives the canvas bitmap past
 * MAX_CANVAS_DIM (10240) — the clamp path ADR-0010 §4 flags as a divergent
 * box-space scenario. This probe sweeps deep into high zoom and samples the
 * surface visibility chain + canvas bitmap size on every frame.
 *
 * No assertions — measurement instrument. Output → e2e_highzoom_probe.log.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const hpPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const hpFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const hpHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = hpPath.resolve(__dirname, '..', '..', '..');
const fixture = hpPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');
const logFile = hpPath.join(repoRoot, 'e2e_highzoom_probe.log');

describe('High-zoom blank probe', () => {
    before(async () => {
        await hpHelpers.waitForApp();
        await hpHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('samples surface + canvas bitmap across a deep zoom-in sweep', async () => {
        await browser.execute(() => {
            const w = window as any;
            w.__hp = [];
            w.__hpDone = false;
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
            const t0 = performance.now();
            const sample = () => {
                const P = w.__hp;
                if (w.__hpDone || !P || P.length >= 6000) { w.__hpDone = true; return; }
                const zs = w.wasmv3?.readZoomState?.();
                const wrapper = document.getElementById('pdf-content-wrapper');
                const container = document.getElementById('pdf-page-container');
                const main = document.getElementById('pdf-vector-main-canvas') as HTMLCanvasElement | null;
                const layer = document.getElementById('pdf-tile-layer');
                const raster = document.getElementById('pdf-render-target');
                const mw = main ? Math.round(main.getBoundingClientRect().width) : -1;
                const bw = main ? main.width : -1;
                const bh = main ? main.height : -1;
                const cw = container ? Math.round(container.getBoundingClientRect().width) : -1;
                const cd = gv(container, 'display'), cvis = gv(container, 'visibility');
                const md = gv(main, 'display'), mvis = gv(main, 'visibility'), mop = gv(main, 'opacity');
                const tiles = visCount(layer);
                const painted = gv(wrapper, 'display') !== 'none' && cd !== 'none' && cvis !== 'hidden' &&
                    ((mw > 50 && mvis !== 'hidden' && parseFloat(mop || '1') > 0.01) || tiles > 0 || gv(raster, 'display') !== 'none');
                P.push({
                    ms: Math.round(performance.now() - t0),
                    t: zs ? +zs.targetZoom.toFixed(4) : -1,
                    v: zs ? +zs.visualZoom.toFixed(4) : -1,
                    lr: zs ? +zs.lastRenderedZoom.toFixed(4) : -1,
                    painted: painted ? 1 : 0,
                    cd, cvis, md, mvis, mop, tiles, mw, cw, bw, bh,
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
            const runSequence = (deltaY: number, count: number, gapMs = 90) => {
                let i = 0;
                const timer = setInterval(() => { fire(deltaY); i += 1; if (i >= count) clearInterval(timer); }, gapMs);
            };
            // deep in: 40 steps of ~1.11x ≈ 30x
            setTimeout(() => runSequence(-120, 40), 300);
        });

        await browser.pause(40 * 90 + 3000);
        await browser.execute(() => { (window as any).__hpDone = true; });
        await browser.pause(150);
        const total: number = await browser.execute(() => (window as any).__hp.length);
        const frames: any[] = [];
        for (let s = 0; s < total; s += 400) {
            const chunk: any[] = await browser.execute((i: number) => (window as any).__hp.slice(i, i + 400), s);
            frames.push(...chunk);
        }
        const lines = ['ms,t,v,lr,painted,cd,cvis,md,mvis,mop,tiles,mw,cw,bw,bh'];
        for (const f of frames) lines.push([f.ms, f.t, f.v, f.lr, f.painted, f.cd, f.cvis, f.md, f.mvis, f.mop, f.tiles, f.mw, f.cw, f.bw, f.bh].join(','));
        const bad = frames.filter((f) => !f.painted).length;
        lines.unshift(`# frames=${frames.length} unpainted=${bad} maxT=${Math.max(...frames.map((f) => f.t))} maxBW=${Math.max(...frames.map((f) => f.bw))}`);
        hpFs.writeFileSync(logFile, lines.join('\n') + '\n');
        console.log(`[highzoom-probe] frames=${frames.length} unpainted=${bad} maxT=${Math.max(...frames.map((f) => f.t))} maxBW=${Math.max(...frames.map((f) => f.bw))}`);
    });
});
