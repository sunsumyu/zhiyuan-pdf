/**
 * ADR-0024 detail-overlay geometry contract.
 *
 * The 2026-10-03 recording (video r51dNMZ5vl) showed a fast zoom-OUT leaving a
 * stale viewport-tile patch on the detail overlay (backCanvas,
 * #pdf-vector-detail-canvas) that stuck out beyond the displayed page and
 * persisted at rest for 0.5-1.4s (permanently when settle took the skip-render
 * reuse branch). Root cause: backCanvas was the only presented surface without
 * per-frame geometry compensation; DetailOverlayOwner now re-derives
 * left/top/transform every tick under the same ADR-0009 formula as the main
 * canvas and tiles.
 *
 * Frame-level invariant (the postmortem's P4 contract): at every composited
 * frame where the detail patch is visible, its on-screen rect must lie INSIDE
 * the main canvas's on-screen rect — the patch's base rect is clamped to the
 * page box in its own zoom space, and both surfaces map page point q to
 * q × visualZoom, so containment is the pixel-free proxy for "页面框外无白色
 * 表面". Pre-fix, a patch frozen at an old band exceeds the page during
 * zoom-out by hundreds of px; post-fix the worst case is floor/ceil rounding.
 *
 * Two checks:
 *   1. NON-VACUITY — the detector flags deliberately displaced surfaces.
 *   2. INVARIANT — fast zoom-out (≥3 bands) + 2s rest, rAF-sampled: every
 *      visible-patch frame is contained, rest window included (the P1
 *      permanent-variant defusal).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const scPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const scHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = scPath.resolve(__dirname, '..', '..', '..');
const fixture = scPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

// Sub-pixel headroom for floor/ceil'd tile rects and gBCR rounding. The
// pre-fix defect violates this by hundreds of px, not by 8.
const TOL = 8;

const BURST_STEPS = 12;
const BURST_GAP_MS = 60;
const BURST_DELAY_MS = 300;
const GESTURE_END_MS = BURST_DELAY_MS + BURST_STEPS * BURST_GAP_MS;
const REST_MS = 2000;

describe('Detail overlay geometry contract (ADR-0024)', () => {
    before(async () => {
        await scHelpers.waitForApp();
        await scHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('detector is non-vacuous: a displaced detail surface is flagged', async () => {
        try {
            await browser.setWindowSize(1400, 900);
        } catch {
            /* best effort; some drivers cap window size */
        }
        await browser.pause(300);

        const result = await browser.execute((tol: number) => {
            const back = document.getElementById('pdf-vector-detail-canvas') as HTMLElement | null;
            const main = document.getElementById('pdf-vector-main-canvas') as HTMLElement | null;
            if (!back || !main) return { ok: false, error: 'surfaces missing' };

            const withinMain = (): boolean => {
                const b = back!.getBoundingClientRect();
                const m = main!.getBoundingClientRect();
                return b.left >= m.left - tol && b.top >= m.top - tol &&
                    b.right <= m.right + tol && b.bottom <= m.bottom + tol;
            };

            const prev = {
                vis: back.style.visibility,
                op: back.style.opacity,
                left: back.style.left,
                top: back.style.top,
                transform: back.style.transform,
            };
            // Force-show the (never-yet-presented, default-boxed) surface so the
            // detector has something to look at.
            back.style.visibility = 'visible';
            back.style.opacity = '1';
            const baselineInside = withinMain();

            // Violation A: push past the displayed page's right edge.
            back.style.left = `${main.getBoundingClientRect().width + 60}px`;
            const rightFlagged = !withinMain();
            // Violation B: push past the bottom edge.
            back.style.left = prev.left;
            back.style.top = `${main.getBoundingClientRect().height + 60}px`;
            const bottomFlagged = !withinMain();

            // Restore every touched style in the SAME JS turn — nothing of the
            // mutation is ever composited.
            back.style.visibility = prev.vis;
            back.style.opacity = prev.op;
            back.style.left = prev.left;
            back.style.top = prev.top;
            back.style.transform = prev.transform;
            return { ok: baselineInside && rightFlagged && bottomFlagged, baselineInside, rightFlagged, bottomFlagged };
        }, TOL);

        if (!result || result.ok !== true) {
            throw new Error(`detector non-vacuity failed: ${JSON.stringify(result)}`);
        }
    });

    it('keeps the detail patch inside the displayed page on every frame of a fast zoom-out + 2s rest', async () => {
        try {
            await browser.setWindowSize(1400, 900);
        } catch {
            /* best effort */
        }
        await browser.pause(300);

        // ── 1. In-page rAF sampler + burst driver ──
        await browser.execute((gestureEnd: number) => {
            const w = window as any;
            w.__doFrames = [];
            w.__doDone = false;
            const TOL = 8;
            const t0 = performance.now();
            const sample = () => {
                if (w.__doDone || w.__doFrames.length >= 3000) {
                    w.__doDone = true;
                    return;
                }
                const back = document.getElementById('pdf-vector-detail-canvas') as HTMLElement | null;
                const main = document.getElementById('pdf-vector-main-canvas') as HTMLElement | null;
                const zs = w.wasmv3?.readZoomState?.();
                let vis = 0;
                let ok = 1;
                let info: any = null;
                if (back && main) {
                    const cs = getComputedStyle(back);
                    const shown = cs.display !== 'none' && cs.visibility !== 'hidden' &&
                        parseFloat(cs.opacity || '1') > 0.01;
                    const presented = back.style.width !== '' && back.style.height !== '';
                    if (shown && presented) {
                        const b = back.getBoundingClientRect();
                        const m = main.getBoundingClientRect();
                        const within = b.left >= m.left - TOL && b.top >= m.top - TOL &&
                            b.right <= m.right + TOL && b.bottom <= m.bottom + TOL;
                        if (b.width > 0 && b.height > 0) vis = 1;
                        ok = within ? 1 : 0;
                        if (!within) {
                            info = {
                                b: [b.left, b.top, b.right, b.bottom].map(Math.round),
                                m: [m.left, m.top, m.right, m.bottom].map(Math.round),
                            };
                        }
                    }
                }
                w.__doFrames.push({
                    ms: Math.round(performance.now() - t0),
                    z: zs ? [zs.targetZoom, zs.visualZoom] : null,
                    vis,
                    ok,
                    info,
                });
                requestAnimationFrame(sample);
            };
            requestAnimationFrame(sample);

            const scroller = document.getElementById('pdf-scroll-container') as HTMLElement | null;
            if (!scroller) return;
            const r = scroller.getBoundingClientRect();
            const fire = (deltaY: number) => {
                scroller.dispatchEvent(new WheelEvent('wheel', {
                    bubbles: true, cancelable: true, ctrlKey: true,
                    deltaY, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2,
                }));
            };
            // Fast zoom-out burst, exactly the recorded failure direction
            // (patches grow larger than the shrinking page and stick out).
            let i = 0;
            const timer = setInterval(() => {
                fire(120);
                i += 1;
                if (i >= 12) clearInterval(timer);
            }, 60);
            void gestureEnd;
        }, GESTURE_END_MS);

        await browser.pause(GESTURE_END_MS + REST_MS + 600);
        await browser.execute(() => {
            (window as any).__doDone = true;
        });
        await browser.pause(150);

        // ── 2. Chunked frame fetch ──
        const total: number = await browser.execute(() => (window as any).__doFrames.length);
        const frames: Array<{ ms: number; z: [number, number] | null; vis: number; ok: number; info: any }> = [];
        for (let s = 0; s < total; s += 500) {
            const chunk = await browser.execute(
                (i: number) => (window as any).__doFrames.slice(i, i + 500),
                s,
            );
            frames.push(...(chunk as any[]));
        }
        if (frames.length < 60) {
            throw new Error(`sampler captured too few frames: ${frames.length}`);
        }

        // ── 3. Gesture sanity: the burst really fired, spanning ≥3 bands ──
        const zsFrames = frames.filter((f) => f.z);
        if (zsFrames.length < 10) {
            throw new Error(`readZoomState never available (${zsFrames.length} frames)`);
        }
        const finalTarget = zsFrames[zsFrames.length - 1].z![0];
        const maxVisual = Math.max(...zsFrames.map((f) => f.z![1]));
        if (!(finalTarget < 0.8)) {
            throw new Error(`burst did not zoom out (finalTarget=${finalTarget}) — sampler inert, not a geometry verdict`);
        }
        if (!(maxVisual / finalTarget > 1.25)) {
            throw new Error(`zoom span under 3 bands (maxVisual=${maxVisual}, finalTarget=${finalTarget})`);
        }

        // ── 4. Path engagement: the viewport-tile patch must have been shown ──
        const visible = frames.filter((f) => f.vis === 1);
        if (visible.length === 0) {
            throw new Error('detail patch never visible — viewport-tile path did not engage; contract would be vacuous');
        }

        // ── 5. THE invariant: every visible frame keeps the patch inside the page ──
        const violations = visible.filter((f) => f.ok !== 1);
        if (violations.length > 0) {
            const first = violations[0];
            throw new Error(
                `${violations.length}/${visible.length} visible frames had the patch outside the displayed page; ` +
                `first at ms=${first.ms} z=${JSON.stringify(first.z)} rects=${JSON.stringify(first.info)}`,
            );
        }

        // ── 6. P1 permanent variant: the 2s REST window was sampled and clean ──
        const restFrames = frames.filter((f) => f.ms >= GESTURE_END_MS + 500);
        if (restFrames.length < 30) {
            throw new Error(`rest window under-sampled (${restFrames.length} frames)`);
        }
        const restViolations = restFrames.filter((f) => f.vis === 1 && f.ok !== 1);
        if (restViolations.length > 0) {
            throw new Error(`rest-window violation (stale patch at stillness): ${JSON.stringify(restViolations[0])}`);
        }
    });
});
