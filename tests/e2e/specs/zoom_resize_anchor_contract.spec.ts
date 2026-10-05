/**
 * Resize anchor contract — closes HANDOFF 未决 #2 (ADR-0008 Negative).
 *
 * Recorded claim (ADR-0008 Negative, echoed in HANDOFF #2): "The
 * `syncHostLayout` path (resize handling) still writes the centered offset
 * into `visual_layout`, so a resize during a zoom gesture would reset the
 * anchor to centered."
 *
 * VERDICT (2026-10-05 forensics + this contract): the recorded mechanism is
 * UNREACHABLE in the current architecture — the claim is STALE, nothing to
 * fix. Evidence:
 *   - `syncLayoutBox`/`syncHostLayout` has exactly ONE caller: the debug
 *     `__pdfViewerGeometryProbe` (viewer_geometry_probe.ts). The only
 *     production window-resize listener is tile_layer's (ViewportGeometry
 *     invalidate + tile wake, ADR-0014).
 *   - Instrumented run: 0 `syncHostLayout` calls and 0 wrapper style
 *     mutations across a shrink+grow cycle; the wrapper box tracks the
 *     viewport via plain CSS flow (no inline width is ever set).
 *   - Scroll position is preserved EXACTLY across resize (content size is
 *     zoom-derived and viewport-independent), container offset inside the
 *     wrapper unchanged, zoom unchanged — nothing re-anchors the view.
 * What remains after resize is standard browser scroll preservation: the
 * viewport grows/shrinks around a fixed scroll offset, so the page point
 * that was at the viewport center moves within the (new) viewport. Keeping
 * that point centered across resizes would be an ENHANCEMENT (the
 * "preserve the anchor" follow-up ADR-0008 mentioned), not a defect fix.
 *
 * This spec pins the verified behavior so the stale path cannot silently
 * come back: across a shrink+grow cycle (content larger than viewport,
 * i.e. no clamping) the layout must not be rewritten and scroll must not
 * move. If a future change wires syncLayoutBox into resize — with or
 * without anchor preservation — this contract goes red and must be
 * revisited deliberately.
 *
 * Output: e2e_resize_anchor.log (repo-root; *.log gitignored, delete after
 * use per AGENTS.md §四).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const raPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const raHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = raPath.resolve(__dirname, '..', '..', '..');
const fixture = raPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

const ZOOM_TICKS = 8;
const TICK_MS = 120;
const SETTLE_MS = 2500; // convergence tail can run ~1.5s past the last tick

type Snap = {
    label: string;
    vpW: number; vpH: number;
    displayZoom: number;
    // anchor point in page space (under viewport center at snapshot time)
    pageX: number; pageY: number;
    // anchor point's screen offset relative to the viewport top-left
    sx: number; sy: number;
    containerOffsetInWrapper: { left: number; top: number };
    scrollTop: number; scrollLeft: number;
    wrapper: { w: number; h: number };
    scrollExtent: { w: number; h: number };
    displaySize: { w: number; h: number };
};

async function snapshot(label: string): Promise<Snap> {
    return await browser.execute((lbl: string) => {
        const scroller = document.getElementById('pdf-scroll-container') as HTMLElement;
        const container = document.getElementById('pdf-page-container') as HTMLElement;
        const wrapper = document.getElementById('pdf-content-wrapper') as HTMLElement;
        const zs = (window as any).wasmv3.readZoomState();
        const zoom = zs.visualZoom;
        const targetZoom = zs.targetZoom;
        const sRect = scroller.getBoundingClientRect();
        const cRect = container.getBoundingClientRect();
        const wRect = wrapper.getBoundingClientRect();
        const vpW = sRect.width;
        const vpH = sRect.height;
        // page-space point under the viewport center
        const pageX = (sRect.left + vpW / 2 - cRect.left) / zoom;
        const pageY = (sRect.top + vpH / 2 - cRect.top) / zoom;
        // its screen offset relative to the viewport top-left
        const sx = cRect.left - sRect.left + pageX * zoom;
        const sy = cRect.top - sRect.top + pageY * zoom;
        return {
            label: lbl,
            vpW, vpH,
            displayZoom: zoom,
            targetZoom,
            pageX, pageY, sx, sy,
            containerOffsetInWrapper: {
                left: cRect.left - wRect.left,
                top: cRect.top - wRect.top,
            },
            scrollTop: scroller.scrollTop,
            scrollLeft: scroller.scrollLeft,
            wrapper: { w: wRect.width, h: wRect.height },
            wrapperInlineWidth: (wrapper as HTMLElement).style.width || '(none)',
            scrollExtent: { w: scroller.scrollWidth, h: scroller.scrollHeight },
            displaySize: {
                w: parseFloat(container.style.width) || 0,
                h: parseFloat(container.style.height) || 0,
            },
        };
    }, label);
}

describe('Resize anchor probe (HANDOFF #2 / ADR-0008 Negative)', () => {
    before(async () => {
        await raHelpers.waitForApp();
        try { await browser.setWindowSize(1400, 900); } catch {}
        await raHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('tracks the zoom anchor across window shrink/grow', async () => {
        // cursor-anchored zoom-in at the viewport center
        await browser.execute((ticks: number, tickMs: number) => {
            const scroller = document.getElementById('pdf-scroll-container') as HTMLElement;
            const rr = scroller.getBoundingClientRect();
            const fire = (deltaY: number) => {
                scroller.dispatchEvent(new WheelEvent('wheel', {
                    bubbles: true, cancelable: true, ctrlKey: true,
                    deltaY, clientX: rr.left + rr.width / 2, clientY: rr.top + rr.height / 2,
                }));
            };
            let i = 0;
            const timer = setInterval(() => {
                fire(-120); i += 1; if (i >= ticks) clearInterval(timer);
            }, tickMs);
        }, ZOOM_TICKS, TICK_MS);
        await browser.pause(ZOOM_TICKS * TICK_MS + SETTLE_MS);

        // Decisive instrumentation: who calls syncHostLayout after this point?
        await browser.execute(() => {
            const w = window as any;
            w.__raSync = [];
            w.__raStyle = [];
            const api = w.wasmv3;
            if (api && typeof api.syncHostLayout === 'function') {
                const orig = api.syncHostLayout.bind(api);
                api.syncHostLayout = (...a: any[]) => {
                    w.__raSync.push({
                        t: Math.round(performance.now()),
                        displayZoom: a[0] && a[0].displayZoom,
                        stack: (new Error().stack || '').split('\n').slice(2, 6).join(' | '),
                    });
                    return orig(...a);
                };
            }
            const wrapper = document.getElementById('pdf-content-wrapper');
            if (wrapper) {
                new MutationObserver((muts) => {
                    for (const m of muts) {
                        w.__raStyle.push({
                            t: Math.round(performance.now()),
                            attr: m.attributeName,
                            old: (m as any).oldValue,
                            now: (wrapper as HTMLElement).getAttribute('style'),
                        });
                    }
                }).observe(wrapper, { attributes: true, attributeOldValue: true, attributeFilter: ['style'] });
            }
        });

        const a = await snapshot('A:zoomed-1400x900');
        await browser.execute(() => {
        });
        await browser.setWindowSize(900, 620);
        await browser.pause(1200);
        const b = await snapshot('B:shrunk-900x620');
        await browser.setWindowSize(1500, 1000);
        await browser.pause(1200);
        const c = await snapshot('C:grown-1500x1000');

        const syncCalls: any[] = await browser.execute(() => (window as any).__raSync || []);
        const styleMuts: any[] = await browser.execute(() => (window as any).__raStyle || []);
        const recentEvents: string[] = await browser.execute(() =>
            ((window as any).__PDF_DIAGNOSTICS_HISTORY || [])
                .slice(-60)
                .map((e: any) => `${e.event}`),
        );

        for (const s of [a, b, c]) {
            console.log(
                `[resize-anchor] ${s.label} vp=${Math.round(s.vpW)}x${Math.round(s.vpH)} zoom=${s.displayZoom.toFixed(3)} target=${(s as any).targetZoom?.toFixed?.(3) ?? '?'} ` +
                `anchorPage=(${s.pageX.toFixed(1)},${s.pageY.toFixed(1)}) screenOffset=(${s.sx.toFixed(1)},${s.sy.toFixed(1)}) ` +
                `containerInWrapper=(${s.containerOffsetInWrapper.left.toFixed(1)},${s.containerOffsetInWrapper.top.toFixed(1)}) ` +
                `scroll=(${s.scrollLeft.toFixed(0)},${s.scrollTop.toFixed(0)}) wrapper=${Math.round(s.wrapper.w)}x${Math.round(s.wrapper.h)} ` +
                `display=${Math.round(s.displaySize.w)}x${Math.round(s.displaySize.h)} extent=${Math.round(s.scrollExtent.w)}x${Math.round(s.scrollExtent.h)}`,
            );
        }
        const shift = (from: Snap, to: Snap) =>
            `dx=${(to.sx - from.sx).toFixed(1)}px dy=${(to.sy - from.sy).toFixed(1)}px`;
        console.log(`[resize-anchor] shift A→B ${shift(a, b)} | shift A→C ${shift(a, c)} | ` +
            `containerOffsetRewritten A→B=${Math.abs(b.containerOffsetInWrapper.left - a.containerOffsetInWrapper.left) > 1 || Math.abs(b.containerOffsetInWrapper.top - a.containerOffsetInWrapper.top) > 1}`);
        console.log(`[resize-anchor] syncHostLayout calls after A: ${syncCalls.length}` +
            (syncCalls.length ? ` → ${syncCalls.slice(0, 4).map((s) => `zoom=${s.displayZoom} @${s.stack}`).join(' ;; ')}` : ''));
        console.log(`[resize-anchor] wrapper style mutations: ${styleMuts.length}` +
            (styleMuts.length ? ` → ${styleMuts.slice(0, 4).map((m) => `${m.attr}: '${m.old}'→'${(m.now || '').slice(0, 60)}'`).join(' ;; ')}` : ''));
        console.log(`[resize-anchor] wrapper inline width: A='${(a as any).wrapperInlineWidth}' B='${(b as any).wrapperInlineWidth}' C='${(c as any).wrapperInlineWidth}'`);
        console.log(`[resize-anchor] recent events: ${[...new Set(recentEvents)].join(',')}`);

        // ── contract: resize must not rewrite layout, scroll, or zoom ────
        // Fixture content (1934px tall at 2.3x) is larger than either
        // viewport, so no scroll clamping is involved — preservation is exact.
        const offsetMoved = (p: Snap, q: Snap) =>
            Math.abs(q.containerOffsetInWrapper.left - p.containerOffsetInWrapper.left) > 0.5 ||
            Math.abs(q.containerOffsetInWrapper.top - p.containerOffsetInWrapper.top) > 0.5;
        expect(offsetMoved(a, b)).toBe(false);
        expect(offsetMoved(a, c)).toBe(false);
        expect(b.scrollLeft).toBe(a.scrollLeft);
        expect(b.scrollTop).toBe(a.scrollTop);
        expect(c.scrollLeft).toBe(a.scrollLeft);
        expect(c.scrollTop).toBe(a.scrollTop);
        // No re-zoom on resize (A and C are both fully settled).
        expect(Math.abs(c.displayZoom - a.displayZoom)).toBeLessThanOrEqual(0.005);
    });
});
