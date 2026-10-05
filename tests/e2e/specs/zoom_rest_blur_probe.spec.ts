/**
 * Rest-state blur reproduction probe — HANDOFF #1 "settle 后仍模糊" trigger
 * (user screenshot 2026-10-05: at rest 154%, upper viewport blurry/stretched,
 * lower sharp, horizontal seam mid-viewport). MEASUREMENT ONLY — no product
 * change, no assertions beyond reproduction sanity.
 *
 * Static forensics: at ~154% (< ADR-0016 budget_zoom ≈ 2.0) the settle path is
 * a FULL-PAGE native render, so any persistent blur at rest is off-contract.
 * Three hypotheses:
 *   H1  scroll-refresh render dropped (resolveHostScrollRefresh.shouldRefresh
 *       false, or scheduler recentCommit-120ms suppress — one-shot, no retry)
 *       → gesture-era viewport patch stays, base stays old-zoom stretched.
 *   H2  tile grid half-old (stale band tiles stretched, no re-lay).
 *   H3  settle full-page render never landed (lastRenderedZoom < visual).
 *
 * Sequence: zoom to ~1.54 → mid-tail scroll (+400px) → settle → structural
 * snapshot (S2) → scroll-liveness (+1px, watch 700ms for a render) → heal test
 * (zoom nudge out+in, snapshot S3). Screenshots saved for visual confirmation.
 *
 * Output: e2e_rest_blur.log + tmp_repro_*.png (repo-root; delete after use per
 * AGENTS.md §四).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const rbPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const rbFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const rbHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = rbPath.resolve(__dirname, '..', '..', '..');
const fixture = rbPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

const TARGET_ZOOM = 1.52;
const TICK_MS = 120;
const MAX_TICKS = 12;

type Snap = any;

async function fireTick(deltaY: number): Promise<void> {
    await browser.execute((dy: number) => {
        const scroller = document.getElementById('pdf-scroll-container') as HTMLElement;
        const rr = scroller.getBoundingClientRect();
        scroller.dispatchEvent(new WheelEvent('wheel', {
            bubbles: true, cancelable: true, ctrlKey: true,
            deltaY: dy, clientX: rr.left + rr.width / 2, clientY: rr.top + rr.height / 2,
        }));
    }, deltaY);
}

async function snapshot(label: string): Promise<Snap> {
    return await browser.execute((lbl: string) => {
        const w = window as any;
        const scroller = document.getElementById('pdf-scroll-container') as HTMLElement;
        const container = document.getElementById('pdf-page-container') as HTMLElement;
        const main = document.getElementById('pdf-vector-main-canvas') as HTMLCanvasElement | null;
        const detail = document.getElementById('pdf-vector-detail-canvas') as HTMLCanvasElement | null;
        const tileLayer = document.getElementById('pdf-tile-layer') as HTMLElement | null;
        const zs = w.wasmv3.readZoomState();
        const sRect = scroller.getBoundingClientRect();
        const cRect = container.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        const visual = zs.visualZoom;
        // page CSS width per unit zoom (container width = page × displayZoom)
        const pageW = (parseFloat(container.style.width) || cRect.width) / (visual || 1);

        const tiles: any[] = [];
        if (tileLayer) {
            for (const el of Array.from(tileLayer.children) as HTMLElement[]) {
                const st = getComputedStyle(el);
                if (st.display === 'none' || st.visibility === 'hidden' || parseFloat(st.opacity || '1') <= 0.01) continue;
                const m = /scale\(([^)]+)\)/.exec(el.style.transform || '');
                const scale = m ? parseFloat(m[1]) : 1;
                const r = el.getBoundingClientRect();
                tiles.push({
                    renderZoom: +(visual / scale).toFixed(3),
                    rect: { l: +r.left.toFixed(0), t: +r.top.toFixed(0), w: +r.width.toFixed(0), h: +r.height.toFixed(0) },
                });
            }
        }
        const mainRect = main ? main.getBoundingClientRect() : null;
        const detailRect = detail ? detail.getBoundingClientRect() : null;
        const detailStyle = detail ? {
            left: detail.style.left, top: detail.style.top,
            width: detail.style.width, height: detail.style.height,
            transform: detail.style.transform, display: getComputedStyle(detail).display,
        } : null;
        return {
            label: lbl,
            zoom: { visual: +visual.toFixed(4), target: +zs.targetZoom.toFixed(4), lastR: +zs.lastRenderedZoom.toFixed(4) },
            viewport: { w: +sRect.width.toFixed(0), h: +sRect.height.toFixed(0), top: +sRect.top.toFixed(0), scrollY: +scroller.scrollTop.toFixed(0) },
            pageW: +pageW.toFixed(1),
            dpr,
            main: main ? {
                bitmap: `${main.width}x${main.height}`,
                // bitmap covers the whole page → bitmap zoom = bitmapW / (pageW × dpr)
                baseRenderZoom: +((main.width) / (pageW * dpr)).toFixed(3),
                cssW: main.style.width, transform: main.style.transform,
                rect: mainRect ? { t: +mainRect.top.toFixed(0), h: +mainRect.height.toFixed(0) } : null,
            } : null,
            detail: detail ? {
                bitmap: `${detail.width}x${detail.height}`,
                style: detailStyle,
                rect: detailRect ? { l: +detailRect.left.toFixed(0), t: +detailRect.top.toFixed(0), w: +detailRect.width.toFixed(0), h: +detailRect.height.toFixed(0) } : null,
            } : null,
            tiles,
        };
    }, label);
}

describe('Rest-state blur reproduction (HANDOFF #1 trigger)', () => {
    before(async () => {
        await rbHelpers.waitForApp();
        await rbHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
        await browser.execute(() => {
            (window as any).__PDF_LAYOUT_TRACE_VERBOSE = false;
            (window as any).__PDF_DIAGNOSTICS_VERBOSE = false;
            const w = window as any;
            w.__rb = { prof: [], on: true };
            const names = new Set(['page-render-duration', 'plan-build-timing']);
            const parse = (first: unknown): string | null => {
                if (typeof first !== 'string' || first.indexOf('%c') < 0) return null;
                const parts = first.split('%c');
                return parts.length < 5 ? null : parts[4].trim().split(/\s+/)[0];
            };
            for (const name of ['log', 'error', 'warn'] as const) {
                const orig = (console as any)[name].bind(console);
                (console as any)[name] = (...args: any[]) => {
                    if (w.__rb.on) {
                        const ev = parse(args[0]);
                        if (ev && names.has(ev) && w.__rb.prof.length < 2000) {
                            w.__rb.prof.push({ t: Math.round(performance.now()), ev });
                        }
                    }
                    return orig(...args);
                };
            }
        });
    });

    it('reproduces the at-rest partial blur and discriminates H1/H2/H3', async () => {
        // S0: pre-zoom reference
        const s0 = await snapshot('S0:initial');

        // zoom to ~1.54
        for (let i = 0; i < MAX_TICKS; i++) {
            await fireTick(-120);
            await browser.pause(TICK_MS);
            const z: any = await browser.execute(() => {
                const zs = (window as any).wasmv3.readZoomState();
                return +zs.targetZoom.toFixed(3);
            });
            if (z >= TARGET_ZOOM) break;
        }
        // scroll DURING the convergence tail (the trap window)
        await browser.pause(350);
        await browser.execute(() => {
            const scroller = document.getElementById('pdf-scroll-container') as HTMLElement;
            scroller.scrollTop += 400;
        });
        const scrollT = Date.now();
        // liveness: does ANY render complete within 700ms of the scroll?
        await browser.pause(700);
        const liveness1: any = await browser.execute(() => (window as any).__rb.prof.slice());

        // full settle
        await browser.pause(2500);
        const s2 = await snapshot('S2:after-scroll-rest');
        try {
            const b64 = await browser.takeScreenshot();
            rbFs.writeFileSync(rbPath.join(repoRoot, 'tmp_repro_broken.png'), Buffer.from(b64, 'base64'));
        } catch {}

        // scroll liveness again: +1px scroll, watch 700ms
        await browser.execute(() => {
            (window as any).__rb.prof.length = 0;
            const scroller = document.getElementById('pdf-scroll-container') as HTMLElement;
            scroller.scrollTop += 1;
        });
        await browser.pause(700);
        const liveness2: any = await browser.execute(() => (window as any).__rb.prof.slice());

        // heal test: zoom nudge out+in
        await fireTick(120);
        await browser.pause(900);
        await fireTick(-120);
        await browser.pause(2200);
        const s3 = await snapshot('S3:after-heal');
        try {
            const b64 = await browser.takeScreenshot();
            rbFs.writeFileSync(rbPath.join(repoRoot, 'tmp_repro_healed.png'), Buffer.from(b64, 'base64'));
        } catch {}

        for (const s of [s0, s2, s3]) {
            console.log(`[rest-blur] ${s.label} zoom=${JSON.stringify(s.zoom)} viewport=${s.viewport.w}x${s.viewport.h}@scrollY${s.viewport.scrollY} ` +
                `base=${s.main ? `${s.main.bitmap} baseRenderZoom=${s.main.baseRenderZoom} css=${s.main.cssW} tf=${s.main.transform}` : 'none'} ` +
                `detail=${s.detail ? `${s.detail.bitmap} rect=${JSON.stringify(s.detail.rect)} disp=${s.detail.style?.display}` : 'none'} ` +
                `tiles=${s.tiles.length}${s.tiles.length ? ` bands=[${[...new Set(s.tiles.map((t: any) => t.renderZoom))].join(',')}]` : ''}`);
        }
        const rendersIn = (arr: any[], fromTs: number) => {
            void fromTs;
            return arr.filter((p) => p.ev === 'page-render-duration').length;
        };
        console.log(`[rest-blur] renders within 700ms after mid-tail scroll: ${rendersIn(liveness1)} | after +1px scroll: ${rendersIn(liveness2)}`);

        // classification
        const visual = s2.zoom.visual;
        const baseZ = s2.main ? s2.main.baseRenderZoom : -1;
        const baseStretched = baseZ > 0 && baseZ < visual * 0.98;
        const lastRGap = Math.abs(s2.zoom.lastR - visual);
        const oldTiles = s2.tiles.filter((t: any) => t.renderZoom < visual * 0.98);
        console.log(`[rest-blur] verdict inputs: baseStretched=${baseStretched} (base ${baseZ} vs visual ${visual.toFixed(3)}) ` +
            `lastR-gap=${lastRGap.toFixed(3)} oldTiles=${oldTiles.length}/${s2.tiles.length} ` +
            `detailCoversViewport=${s2.detail && s2.detail.rect ? (s2.detail.rect.t <= s2.viewport.top + 2 && s2.detail.rect.h >= s2.viewport.h - 2) : 'n/a'}`);
        if (baseStretched && lastRGap > 0.02) console.log('[rest-blur] → H3 leaning: settle full-page render never landed (lastR behind visual) AND base old-zoom stretched.');
        else if (baseStretched && lastRGap <= 0.02) console.log('[rest-blur] → lastR claims current zoom but base bitmap is older → commit/settle recorded without a full-page paint (H3 variant).');
        else if (oldTiles.length > 0) console.log('[rest-blur] → H2 leaning: base is current; stale-band tiles paint the blur.');
        else console.log('[rest-blur] → no stretched surface found in structure; blur may be inside a single bitmap (patch band) — inspect screenshots.');

        const lines = [s0, s2, s3].map((s) => JSON.stringify(s));
        rbFs.writeFileSync(rbPath.join(repoRoot, 'e2e_rest_blur.log'), lines.join('\n') + '\n');
    });
});
