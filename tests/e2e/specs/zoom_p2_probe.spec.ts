/**
 * P2 stutter probe (ADR-0012 investigation). During a real ctrl-wheel gesture,
 * sample per rAF frame:
 *   - tile-layer DOM canvas count (are streaming tiles actually covering?)
 *   - Rust tile stats (queue/ready/rendering)
 *   - main canvas CSS box size (full-page reknock size)
 *   - zoom state
 *   - main-thread long tasks (>50ms)
 * No assertions — measurement instrument. Writes e2e_p2_probe.log.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const p2Path = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const p2Fs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const p2Helpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = p2Path.resolve(__dirname, '..', '..', '..');
const fixture = p2Path.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');
const logFile = p2Path.join(repoRoot, 'e2e_p2_probe.log');

describe('P2 stutter probe', () => {
    before(async () => {
        await p2Helpers.waitForApp();
        await p2Helpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('samples tile coverage + main-thread long tasks across a wheel gesture', async () => {
        await browser.execute(() => {
            const w = window as any;
            w.__p2 = { frames: [], longtasks: [], done: false };
            const po = new PerformanceObserver((list) => {
                for (const e of list.getEntries()) {
                    w.__p2.longtasks.push({ t: Math.round(e.startTime), d: Math.round(e.duration) });
                }
            });
            try { po.observe({ entryTypes: ['longtask'] }); } catch { /* unsupported */ }

            const t0 = performance.now();
            const sample = () => {
                if (w.__p2.done || w.__p2.frames.length >= 6000) { w.__p2.done = true; return; }
                const zs = w.wasmv3?.readZoomState?.();
                const layer = document.getElementById('pdf-tile-layer');
                let tileCount = 0, tileVisible = 0;
                if (layer) {
                    for (const el of Array.from(layer.children) as HTMLElement[]) {
                        tileCount++;
                        const c = getComputedStyle(el);
                        if (c.display !== 'none' && c.visibility !== 'hidden' && parseFloat(c.opacity || '1') > 0.01) tileVisible++;
                    }
                }
                const cv = document.getElementById('pdf-vector-main-canvas') as HTMLElement | null;
                const r = cv ? cv.getBoundingClientRect() : null;
                let stats: any = null;
                try { stats = w.wasmv3?.renderFacadeTileStats?.(); } catch { /* ignore */ }
                w.__p2.frames.push({
                    ms: Math.round(performance.now() - t0),
                    v: zs ? +zs.visualZoom.toFixed(4) : -1,
                    t: zs ? +zs.targetZoom.toFixed(4) : -1,
                    tileCount, tileVisible,
                    queue: stats?.queue_size ?? -1,
                    ready: stats?.cache?.ready ?? -1,
                    rendering: stats?.cache?.rendering ?? -1,
                    animating: stats?.is_animating ? 1 : 0,
                    cvW: r ? Math.round(r.width) : -1,
                    cvH: r ? Math.round(r.height) : -1,
                });
                requestAnimationFrame(sample);
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
            const timer = setInterval(() => { fire(-120); i += 1; if (i >= 16) clearInterval(timer); }, 120);
        });

        await browser.pause(16 * 120 + 3500);
        await browser.execute(() => { (window as any).__p2.done = true; });
        await browser.pause(150);

        const total: number = await browser.execute(() => (window as any).__p2.frames.length);
        const frames: any[] = [];
        for (let s = 0; s < total; s += 500) {
            const chunk: any[] = await browser.execute((i: number) => (window as any).__p2.frames.slice(i, i + 500), s);
            frames.push(...chunk);
        }
        const lt: any[] = await browser.execute(() => (window as any).__p2.longtasks);

        const lines = ['ms,v,t,tileCount,tileVisible,queue,ready,rendering,animating,cvW,cvH'];
        for (const f of frames) {
            lines.push([f.ms, f.v, f.t, f.tileCount, f.tileVisible, f.queue, f.ready, f.rendering, f.animating, f.cvW, f.cvH].join(','));
        }
        const maxTile = Math.max(...frames.map((f) => f.tileVisible), 0);
        const maxQueue = Math.max(...frames.map((f) => f.queue), 0);
        const maxReady = Math.max(...frames.map((f) => f.ready), 0);
        const ltMax = lt.length ? Math.max(...lt.map((e) => e.d)) : 0;
        lines.unshift(`# frames=${frames.length} maxTileVisible=${maxTile} maxQueue=${maxQueue} maxReady=${maxReady} longtasks=${lt.length} ltMax=${ltMax}ms`);
        lines.push('# longtasks: ' + lt.map((e) => `${e.t}:${e.d}`).join(' '));
        p2Fs.writeFileSync(logFile, lines.join('\n') + '\n');
        console.log(`[p2-probe] frames=${frames.length} maxTileVisible=${maxTile} maxQueue=${maxQueue} maxReady=${maxReady} longtasks=${lt.length} ltMax=${ltMax}ms`);
    });
});
