/**
 * Writer-locator probe for the 2026-09-30 blank-surface defect.
 *
 * A MutationObserver watches the `style` attribute of every link in the page
 * surface chain (wrapper / container / main canvas / tile layer / raster), so
 * any hide (display:none, visibility:hidden, opacity:0) is timestamped with the
 * element id. The app's diagnostics history is captured too, so the mutation
 * can be correlated with the semantic event (canvas-host.clear / raster.commit
 * / present / layout.sync / render-flow) that caused it.
 *
 * No assertions — measurement instrument. Output → e2e_writer_probe.log.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const wpPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const wpFs = require('node:fs') as typeof import('node:fs');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const wpHelpers = require('../helpers/app') as typeof import('../helpers/app');

const repoRoot = wpPath.resolve(__dirname, '..', '..', '..');
const fixture = wpPath.join(repoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');
const logFile = wpPath.join(repoRoot, 'e2e_writer_probe.log');

const IDS = ['pdf-content-wrapper', 'pdf-page-container', 'pdf-vector-main-canvas', 'pdf-tile-layer', 'pdf-render-target', 'pdf-vector-detail-canvas'];

describe('Page-surface writer locator', () => {
    before(async () => {
        await wpHelpers.waitForApp();
        await wpHelpers.loadFixturePdf(fixture);
        await browser.pause(1500);
    });

    it('records every visibility mutation on the surface chain during a zoom sweep', async () => {
        await browser.execute((ids: string[]) => {
            const w = window as any;
            w.__wp = [];
            w.__wpDone = false;
            w.__wpHistStart = (w.__PDF_DIAGNOSTICS_HISTORY || []).length;
            const snap = (el: HTMLElement) => {
                const c = getComputedStyle(el);
                return { d: c.display, v: c.visibility, o: c.opacity };
            };
            const t0 = performance.now();
            w.__wpT0 = t0;
            const obs = new MutationObserver((muts) => {
                for (const m of muts) {
                    const el = m.target as HTMLElement;
                    if (!el || !el.id) continue;
                    const c = getComputedStyle(el);
                    w.__wp.push({
                        ms: Math.round(performance.now() - t0),
                        id: el.id,
                        d: c.display, v: c.visibility, o: c.opacity,
                        inline: (el.getAttribute('style') || '').slice(0, 200),
                        stack: (new Error().stack || '').split('\n').slice(2, 6).join(' | ').slice(0, 400),
                    });
                }
            });
            for (const id of ids) {
                const el = document.getElementById(id);
                if (el) obs.observe(el, { attributes: true, attributeFilter: ['style'] });
            }
            w.__wpObs = obs;
            w.__wpSnap = () => ids.map((id) => {
                const el = document.getElementById(id);
                return { id, ...(el ? snap(el) : { d: '?', v: '?', o: '?' }) };
            });

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
            setTimeout(() => runSequence(120, 16), 300);
            setTimeout(() => runSequence(-120, 16), 300 + 16 * 90 + 1600);
            setTimeout(() => runSequence(-120, 12), 300 + 16 * 90 + 1600 + 16 * 90 + 1600);
        }, IDS);

        await browser.pause(16 * 90 + 1600 + 16 * 90 + 1600 + 12 * 90 + 2500);
        const out = await browser.execute(() => {
            const w = window as any;
            w.__wpDone = true;
            if (w.__wpObs) w.__wpObs.disconnect();
            const hist = (w.__PDF_DIAGNOSTICS_HISTORY || []).slice(w.__wpHistStart || 0)
                .filter((e: any) => /clear|present|raster|layout|render-flow|canvas|host/i.test(e.event || ''))
                .map((e: any) => `${Math.round(e.timestamp || 0)} ${e.level || ''} [${e.layer || ''}] ${e.event}: ${JSON.stringify(e.fields || {}).slice(0, 180)}`);
            return { muts: w.__wp || [], hist, snap: w.__wpSnap ? w.__wpSnap() : [] };
        });

        const lines: string[] = [];
        lines.push(`mutations=${out.muts.length}`);
        lines.push(`final snapshot: ${JSON.stringify(out.snap)}`);
        lines.push('--- surface mutations (only ones that change d/v/o to a HIDDEN state) ---');
        let last: Record<string, string> = {};
        for (const m of out.muts) {
            const key = `${m.d}/${m.v}/${m.o}`;
            const hidden = m.d === 'none' || m.v === 'hidden' || parseFloat(m.o || '1') <= 0.01;
            const prev = last[m.id];
            if (hidden && key !== prev) {
                lines.push(`  @${m.ms}ms ${m.id} -> d=${m.d} v=${m.v} o=${m.o} | inline="${m.inline}" | ${m.stack}`);
            }
            last[m.id] = key;
        }
        lines.push('--- all mutations (first 120) ---');
        for (const m of out.muts.slice(0, 120)) {
            lines.push(`  @${m.ms}ms ${m.id} d=${m.d} v=${m.v} o=${m.o}`);
        }
        lines.push('--- diagnostics history (filtered) ---');
        lines.push(...out.hist.slice(-400));
        wpFs.writeFileSync(logFile, lines.join('\n') + '\n');
        console.log(`[writer-probe] muts=${out.muts.length} hist=${out.hist.length}`);
    });
});
