/**
 * Diagnostic: doubled-page / black-region symptom at 48% zoom.
 * Dumps every presentation surface's CSS box + bitmap size and screenshots.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const diagPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const diagHelpers = require('../helpers/app') as typeof import('../helpers/app');

const diagRepoRoot = diagPath.resolve(__dirname, '..', '..', '..');
const diagFixturePath = diagPath.join(diagRepoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

describe('Diagnose doubled page at 48%', () => {
    before(async () => {
        await diagHelpers.waitForApp();
        await diagHelpers.loadFixturePdf(diagFixturePath);
        await browser.pause(1500);
    });

    it('dumps surface geometry at 48% zoom', async () => {
        await browser.execute(() => {
            (window as any).pdfZoomChange?.('0.48');
        });
        await browser.pause(2500);

        const geo = (await browser.execute(() => {
            const box = (el: Element | null) => {
                if (!el) return null;
                const r = el.getBoundingClientRect();
                const c = el as HTMLCanvasElement;
                return {
                    left: Math.round(r.left),
                    top: Math.round(r.top),
                    w: Math.round(r.width),
                    h: Math.round(r.height),
                    bitmap: `${c.width}x${c.height}`,
                    display: (c as HTMLElement).style.display || '(default)',
                    visibility: (c as HTMLElement).style.visibility || '(default)',
                    opacity: (c as HTMLElement).style.opacity || '(default)',
                    zIndex: (c as HTMLElement).style.zIndex || '(default)',
                };
            };
            const container = document.getElementById('pdf-page-container');
            const wrapper = document.getElementById('pdf-content-wrapper');
            const scroller = document.getElementById('pdf-scroll-container') || document.querySelector('.scroll-container');
            const tiles = document.getElementById('pdf-tile-layer');
            const tileCanvases = tiles
                ? Array.from(tiles.querySelectorAll('canvas')).map((c) => box(c))
                : [];
            return {
                zoomState: (window as any).wasmv3?.getZoomState?.(),
                wrapper: box(wrapper),
                scroller: box(scroller as Element | null),
                container: box(container),
                containerStyle: container
                    ? {
                          transform: container.style.transform || '(none)',
                          width: container.style.width,
                          height: container.style.height,
                      }
                    : null,
                mainCanvas: box(document.getElementById('pdf-vector-main-canvas')),
                backCanvas: box(document.getElementById('pdf-vector-detail-canvas')),
                mainStage: box(document.getElementById('pdf-vector-main-stage-canvas')),
                tileLayer: box(tiles),
                tileCanvasCount: tileCanvases.length,
                tileCanvases: tileCanvases.slice(0, 14),
            };
        })) as any;

        console.log('[diag] geometry:', JSON.stringify(geo, null, 1));

        await browser.saveScreenshot('tests/e2e/diag-48.png');

        // Isolate surfaces: hide tiles → shot B; hide tiles+main → shot C.
        await browser.execute(() => {
            const t = document.getElementById('pdf-tile-layer');
            if (t) t.style.display = 'none';
        });
        await browser.pause(300);
        await browser.saveScreenshot('tests/e2e/diag-48-notiles.png');

        await browser.execute(() => {
            const m = document.getElementById('pdf-vector-main-canvas');
            if (m) m.style.visibility = 'hidden';
        });
        await browser.pause(300);
        await browser.saveScreenshot('tests/e2e/diag-48-nocanvas.png');

        await browser.execute(() => {
            const t = document.getElementById('pdf-tile-layer');
            if (t) t.style.display = '';
            const m = document.getElementById('pdf-vector-main-canvas');
            if (m) m.style.visibility = 'visible';
        });
    });
});
