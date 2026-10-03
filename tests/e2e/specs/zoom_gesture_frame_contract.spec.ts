/**
 * ADR-0009 gesture frame contract — per-frame geometry assertions across a
 * live ctrl-wheel gesture (out → settle → in), run against the real Tauri
 * webview. Guards the two defect classes found via 2026-09-30 recordings:
 *
 *   1. TILE SHEAR (present-formula translation bug): every presented tile
 *      must sit on the 512-px grid after its present scale —
 *      kx = left/(512·a), ky = top/(512·a) integral within ±0.02 — on EVERY
 *      sampled frame, for ANY band (current or fallback).
 *   2. SURFACE JUMP (present-frame continuity): the main canvas visual width
 *      must track pageWidth × visualZoom within 4% on every frame. Catches
 *      the reknock-present jump where the canvas showed page×Z_new
 *      (transform 'none') for a compositor frame while tiles sat at
 *      page×visualZoom.
 *
 * Also asserted: the tile layer never hides during the gesture (ADR-0009).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const gcPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const gcHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = gcPath.resolve(__dirname, '..', '..', '..');
const fixture = gcPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');
const PAGE_WIDTH = 595; // fixture page width in CSS px
const SCALE_TOLERANCE = 0.04; // canvas visual width vs page×visual
const GRID_TOLERANCE = 0.02; // kx/ky integer proximity

type FrameSample = {
    ms: number;
    z: [number, number, number] | null;
    cvW: number;
    layerDisplay: string;
    tiles: Array<{ l: number; t: number; a: number }>;
};

type Violation = { frame: number; ms: number; kind: string; detail: string };

const WHEEL_COUNT = 14; // ×2^(-±120/800) per event: ~0.90 out / ~1.11 in
// 14 bands span a 4.8× zoom range (1.0→0.21→1.0), matching the user's
// recorded 100%→30%→329% bursts. This is the scenario where the
// commit-without-present desync (box left in a stale band while lastRendered
// moves on) produces 6-12% visual drift — the red-light scenario for ADR-0010.

describe('Zoom gesture frame contract (ADR-0009/0010)', () => {
    before(async () => {
        await gcHelpers.waitForApp();
        await gcHelpers.loadFixturePdf(fixture);
        await browser.pause(1200);
    });

    it('keeps every surface grid-aligned and continuous on every gesture frame', async () => {
        // ── 1. In-page rAF sampler ──
        await browser.execute(() => {
            const w = window as any;
            w.__gcFrames = [];
            w.__gcDone = false;
            const t0 = performance.now();
            const sample = () => {
                if (w.__gcDone || w.__gcFrames.length >= 4000) {
                    w.__gcDone = true;
                    return;
                }
                const zs = w.wasmv3?.readZoomState?.();
                const cv = document.getElementById('pdf-vector-main-canvas') as HTMLElement | null;
                const layer = document.getElementById('pdf-tile-layer');
                const tiles: Array<{ l: number; t: number; a: number }> = [];
                if (layer && layer.style.display !== 'none') {
                    for (const el of Array.from(layer.children) as HTMLElement[]) {
                        if (el.style.display === 'none') continue;
                        const m = new DOMMatrixReadOnly(
                            el.style.transform && el.style.transform !== 'none'
                                ? el.style.transform
                                : 'matrix(1,0,0,1,0,0)',
                        );
                        tiles.push({
                            l: parseFloat(el.style.left || '0'),
                            t: parseFloat(el.style.top || '0'),
                            a: m.a,
                        });
                    }
                }
                w.__gcFrames.push({
                    ms: Math.round(performance.now() - t0),
                    z: zs ? [zs.targetZoom, zs.visualZoom, zs.lastRenderedZoom] : null,
                    cvW: cv ? cv.getBoundingClientRect().width : 0,
                    layerDisplay: layer ? layer.style.display : '(missing)',
                    tiles,
                });
                requestAnimationFrame(sample);
            };
            requestAnimationFrame(sample);

            // ── 2. Gesture driver: 14 out, settle, 14 in (page-side timers) ──
            const scroller = document.getElementById('pdf-scroll-container') as HTMLElement | null;
            if (!scroller) return;
            const r = scroller.getBoundingClientRect();
            const fire = (deltaY: number) => {
                scroller.dispatchEvent(new WheelEvent('wheel', {
                    bubbles: true, cancelable: true, ctrlKey: true,
                    deltaY, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2,
                }));
            };
            const runSequence = (deltaY: number, count: number) => {
                let i = 0;
                const timer = setInterval(() => {
                    fire(deltaY);
                    i += 1;
                    if (i >= count) clearInterval(timer);
                }, 150);
            };
            // WHEEL_COUNT=14 inlined — browser.execute body cannot see Node consts.
            setTimeout(() => runSequence(120, 14), 200);
            setTimeout(() => runSequence(-120, 14), 200 + 14 * 150 + 2200);
        });

        // out: 200 + 14×150 + settle window + margin; in: 14×150 + settle + margin
        await browser.pause(4200);
        await browser.pause(4200);
        await browser.pause(1500);

        // ── 3. Stop sampler, pull frames ──
        await browser.execute(() => {
            (window as any).__gcDone = true;
        });
        await browser.pause(120);
        const total: number = await browser.execute(() => (window as any).__gcFrames.length);
        const frames: FrameSample[] = [];
        for (let start = 0; start < total; start += 400) {
            const chunk: FrameSample[] = await browser.execute(
                (s: number) => (window as any).__gcFrames.slice(s, s + 400),
                start,
            );
            frames.push(...chunk);
        }
        if (frames.length < 100) {
            throw new Error(`sampler captured too few frames: ${frames.length}`);
        }

        // ── 4. Assertions over every frame ──
        const violations: Violation[] = [];
        let hiddenFrames = 0;
        let canvasCheckedFrames = 0;
        let tilesCheckedFrames = 0;
        const canvasDrifts: number[] = [];
        for (let i = 0; i < frames.length; i++) {
            const f = frames[i];
            if (f.layerDisplay === 'none') {
                hiddenFrames += 1;
                if (violations.length < 8) {
                    violations.push({
                        frame: i, ms: f.ms, kind: 'LAYER-HIDDEN',
                        detail: `visual=${f.z?.[1]}`,
                    });
                }
                continue;
            }
            const visual = f.z?.[1] ?? 0;
            // (2) Canvas visual width continuity — only once the canvas has a
            // real present (pre-first-present frames have a placeholder box).
            if (f.cvW > 50 && visual > 0) {
                canvasCheckedFrames += 1;
                const expected = PAGE_WIDTH * visual;
                const drift = Math.abs(f.cvW - expected) / expected;
                canvasDrifts.push(drift);
                if (drift > SCALE_TOLERANCE && violations.length < 8) {
                    violations.push({
                        frame: i, ms: f.ms, kind: 'CANVAS-WIDTH-JUMP',
                        detail: `cvW=${f.cvW.toFixed(1)} expected≈${expected.toFixed(1)} ` +
                            `drift=${(drift * 100).toFixed(1)}% z=[${f.z?.map((v) => v.toFixed(3)).join(',')}]`,
                    });
                }
            }
            // (1) Tile grid alignment — every visible tile, every frame.
            if (f.tiles.length > 0) {
                tilesCheckedFrames += 1;
                for (const tile of f.tiles) {
                    if (!(tile.a > 0)) continue;
                    const kx = tile.l / (512 * tile.a);
                    const ky = tile.t / (512 * tile.a);
                    const offGrid =
                        Math.abs(kx - Math.round(kx)) > GRID_TOLERANCE ||
                        Math.abs(ky - Math.round(ky)) > GRID_TOLERANCE;
                    if (offGrid && violations.length < 8) {
                        violations.push({
                            frame: i, ms: f.ms, kind: 'TILE-OFF-GRID',
                            detail: `l=${tile.l.toFixed(1)} t=${tile.t.toFixed(1)} ` +
                                `a=${tile.a.toFixed(4)} kx=${kx.toFixed(3)} ky=${ky.toFixed(3)}`,
                        });
                    }
                }
            }
        }

        const summary = `frames=${frames.length} hidden=${hiddenFrames} ` +
            `canvasChecked=${canvasCheckedFrames} tileFrames=${tilesCheckedFrames}`;
        if (canvasDrifts.length) {
            const sorted = [...canvasDrifts].sort((a, b) => a - b);
            const q = (p: number) => sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * p))];
            console.log(`[frame-contract] canvas drift vs page×visual: ` +
                `p50=${(q(0.5) * 100).toFixed(2)}% p90=${(q(0.9) * 100).toFixed(2)}% ` +
                `p99=${(q(0.99) * 100).toFixed(2)}% max=${(sorted[sorted.length - 1] * 100).toFixed(2)}% ` +
                `over${(SCALE_TOLERANCE * 100).toFixed(0)}%=${canvasDrifts.filter((d) => d > SCALE_TOLERANCE).length}/${canvasDrifts.length}`);
        }
        if (hiddenFrames > 0) {
            throw new Error(`tile layer hidden during gesture (${summary})\n` +
                violations.map((v) => `${v.kind} f${v.frame}@${v.ms}ms ${v.detail}`).join('\n'));
        }
        if (violations.length > 0) {
            throw new Error(`gesture frame contract violated (${summary})\n` +
                violations.map((v) => `${v.kind} f${v.frame}@${v.ms}ms ${v.detail}`).join('\n'));
        }
    });
});
