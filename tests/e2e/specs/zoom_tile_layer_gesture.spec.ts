/**
 * Contract for ADR-0009 gesture tile streaming (docs/adr/
 * 0009-gesture-tile-streaming.md), superseding the 2026-09-27 hide-on-gesture
 * contract: during a ctrl-wheel gesture the tile layer STAYS VISIBLE and
 * streams quantized-visual-zoom tiles (Rust incremental scheduler), each
 * presented with the unified formula scale(visualZoom / renderZoom). Tiles at
 * any render zoom coexist aligned with the CSS-scaled main canvas — page
 * point p lands at p × visualZoom on both surfaces — so no hide/clear switch
 * exists anywhere in the gesture lifecycle, and no multi-surface misalignment
 * frame is possible by construction.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const tlPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const tlHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = tlPath.resolve(__dirname, '..', '..', '..');
const fixture = tlPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

type TileLayerState = {
    display: string;
    childCount: number;
    visibleTileCount: number;
    alignedTileCount: number;
    zoom: { targetZoom: number; visualZoom: number; lastRenderedZoom: number } | null;
};

describe('Zoom tile layer gesture streaming (ADR-0009)', () => {
    before(async () => {
        await tlHelpers.waitForApp();
        await tlHelpers.loadFixturePdf(fixture);
        await browser.pause(1200);
    });

    function readState(): Promise<TileLayerState> {
        return browser.execute(() => {
            const layer = document.getElementById('pdf-tile-layer');
            if (!layer) {
                return {
                    display: '(missing)',
                    childCount: -1,
                    visibleTileCount: -1,
                    alignedTileCount: -1,
                    misalignedTiles: -1,
                    misalignedDetail: [],
                    zoom: null,
                };
            }
            // A presented tile carries the ADR-0009 present formula:
            //   left = tileX × 512 × s,  top = tileY × 512 × s,  scale = s
            // with s = visual / renderZoom. Two invariants are checkable from
            // the DOM alone, for ANY band and ANY zoom:
            //   1. scale sanity: s within one quantized band of identity
            //      (|a − 1| ≤ ~6%) for current-band tiles;
            //   2. GRID POSITION: left / (512 × a) and top / (512 × a) must
            //      be near-integers (±0.02). This catches the 2026-09-30
            //      shear bug (left frozen at render-space 512 while the
            //      scale changed → k = 1.087, clearly off-grid) that a
            //      scale-only assertion cannot see.
            const zoomState = (window as any).wasmv3?.readZoomState?.() ?? null;
            let visibleTileCount = 0;
            let currentBandCount = 0;
            let misalignedTiles = 0;
            const misalignedDetail: string[] = [];
            for (const child of Array.from(layer.children) as HTMLElement[]) {
                if (child.style.display === 'none') continue;
                visibleTileCount += 1;
                const m = new DOMMatrixReadOnly(
                    child.style.transform && child.style.transform !== 'none'
                        ? child.style.transform
                        : 'matrix(1,0,0,1,0,0)',
                );
                const a = m.a;
                const left = parseFloat(child.style.left || '0');
                const top = parseFloat(child.style.top || '0');
                const kx = left / (512 * a);
                const ky = top / (512 * a);
                // Alignment = the tile sits on the 512 grid after its scale
                // is applied. Scale magnitude is NOT an alignment criterion:
                // old-band fallback tiles legitimately carry |a−1| up to the
                // full zoom delta while remaining pixel-aligned (verified
                // 2026-09-30: a=0.9013 at kx=1.000 — perfect alignment).
                const offGrid =
                    Math.abs(kx - Math.round(kx)) > 0.02 ||
                    Math.abs(ky - Math.round(ky)) > 0.02;
                if (!offGrid) {
                    if (Math.abs(a - 1) <= 0.061) currentBandCount += 1;
                } else {
                    misalignedTiles += 1;
                    misalignedDetail.push(
                        `pos=(${child.style.left},${child.style.top}) ` +
                        `a=${a.toFixed(4)} kx=${kx.toFixed(3)} ky=${ky.toFixed(3)} OFF-GRID`,
                    );
                }
            }
            return {
                display: layer.style.display,
                childCount: layer.childElementCount,
                visibleTileCount,
                alignedTileCount: currentBandCount,
                misalignedTiles,
                misalignedDetail,
                zoom: zoomState,
            };
        }) as Promise<TileLayerState>;
    }

    function dispatchWheel(deltaY: number): Promise<void> {
        return browser.execute((dy: number) => {
            const scroller = document.getElementById('pdf-scroll-container');
            if (!scroller) throw new Error('scroll container missing');
            const r = scroller.getBoundingClientRect();
            scroller.dispatchEvent(new WheelEvent('wheel', {
                bubbles: true,
                cancelable: true,
                ctrlKey: true,
                deltaY: dy,
                clientX: r.left + r.width / 2,
                clientY: r.top + r.height / 2,
            }));
        }, deltaY).then(() => undefined) as Promise<void>;
    }

    function settled(s: TileLayerState): boolean {
        return !!s.zoom &&
            Math.abs(s.zoom.visualZoom - s.zoom.targetZoom) < 0.001 &&
            Math.abs(s.zoom.lastRenderedZoom - s.zoom.targetZoom) < 0.001;
    }

    function dumpTileDiagnostics(): Promise<void> {
        return browser.execute(() => {
            const history = ((window as any).__PDF_DIAGNOSTICS_HISTORY ?? []) as Array<{
                event: string;
                message: string;
            }>;
            return history
                .filter((h) =>
                    h.event.startsWith('gesture-stream') ||
                    h.event === 'tile-layer.schedule-viewport' ||
                    h.event === 'tile-layer.tile-render-failed')
                .slice(-30)
                .map((h) => h.message);
        }).then((lines: string[]) => {
            console.log('[tile-stream-diag]', JSON.stringify(lines, null, 1));
        }) as Promise<void>;
    }

    it('keeps the layer visible through gestures and aligns every tile to the canvas', async () => {
        const before = await readState();
        if (before.display === '(missing)') throw new Error('tile layer host missing');
        if (before.childCount === 0) throw new Error('expected initial tiles to be presented');

        // Single wheel-out: target 1.0 → 0.901, gap 0.099 > NEAR_SETTLE_EPS —
        // the gesture streaming branch engages. The layer must NEVER hide.
        await dispatchWheel(120);
        const during = await readState();
        if (!during.zoom) throw new Error('zoom state unreadable');
        if (Math.abs(during.zoom.targetZoom - 0.901) > 0.01) {
            throw new Error(`wheel did not change target zoom: ${during.zoom.targetZoom}`);
        }
        if (during.display === 'none') {
            throw new Error('tile layer hid during gesture — ADR-0009 violation');
        }

        // Mid-gesture or at settle: the layer stays visible, and after settle
        // at least a viewport's worth (≥4) of current-band aligned tiles has
        // been presented (the streamed/settle surface covers the viewport).
        // Every presented tile — any band — must sit on the 512-px grid
        // (position × scale consistency, see readState).
        await waitAligned(() => readState(), {
            timeout: 6000,
            interval: 60,
            failMsg: 'settle did not produce a viewport of aligned current-band tiles',
        });
        await dumpTileDiagnostics();

        // Reverse gesture: the layer must stay visible again (the f_015 ghost
        // class is impossible by construction — no hide, unified formula).
        // Zooming back may land on a zoom whose tiles are still Ready in the
        // Rust cache — no render requests fire, so the DOM keeps the previous
        // band's tiles (still grid-aligned by the unified formula; the settle
        // canvas carries the sharp image). Assert visibility + grid life.
        await dispatchWheel(-120);
        await waitAligned(() => readState(), {
            timeout: 6000,
            interval: 60,
            failMsg: 'reverse settle did not keep a visible, grid-aligned tile layer',
        });
    });

    /**
     * Wait until the zoom state settles, the layer is visible, a viewport of
     * current-band tiles is presented, and every presented tile is on-grid.
     * On failure, re-reads the state and rethrows with the tile detail so the
     * wdio log carries actionable geometry.
     */
    async function waitAligned(
        read: () => Promise<TileLayerState>,
        opts: { timeout: number; interval: number; failMsg: string },
    ): Promise<void> {
        try {
            await browser.waitUntil(async () => {
                const s = await read();
                if (s.display === 'none') return false;
                return settled(s) && s.alignedTileCount >= 4 && s.misalignedTiles === 0;
            }, opts);
        } catch (err) {
            const s = await read();
            throw new Error(
                `${opts.failMsg}\n` +
                `layer=${s.display} visible=${s.visibleTileCount} ` +
                `aligned=${s.alignedTileCount} misaligned=${s.misalignedTiles}\n` +
                `${s.misalignedDetail.join('\n')}`,
            );
        }
    }
});
