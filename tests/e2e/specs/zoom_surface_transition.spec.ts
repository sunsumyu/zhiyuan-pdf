/**
 * Surface transition: a real ctrl-wheel must switch the active surface
 * from the raster sibling to the vector container (ADR-0002 I3).
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const surfPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const surfHelpers = require('../helpers/app') as typeof import('../helpers/app');

const surfRoot = surfPath.resolve(__dirname, '..', '..', '..');
const surfFixture = surfPath.join(surfRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

type SurfaceState = {
    raster: { display: string; inDom: boolean };
    vector: { display: string; inDom: boolean };
    mainCanvasVisible: boolean;
    settled: boolean;
};

describe('Surface transition on wheel', () => {
    before(async () => {
        await surfHelpers.waitForApp();
        await surfHelpers.loadFixturePdf(surfFixture);
        await browser.pause(1200);
    });

    it('switches active surface from raster to vector during a real wheel gesture', async () => {
        const readSurface = async (): Promise<SurfaceState> => (await browser.execute(() => {
            const style = (el: HTMLElement | null) => ({
                display: el ? el.style.display || '(default)' : '(missing)',
                inDom: !!el,
            });
            const raster = document.getElementById('pdf-render-target') as HTMLElement | null;
            const vector = document.getElementById('pdf-page-container') as HTMLElement | null;
            const main = document.getElementById('pdf-vector-main-canvas') as HTMLCanvasElement | null;
            const state = (window as any).wasmv3?.getZoomState?.();
            return {
                raster: style(raster),
                vector: style(vector),
                mainCanvasVisible: !!(main && main.style.visibility !== 'hidden'),
                settled: state?.previewHost?.previewActive === false,
            };
        })) as SurfaceState;

        const before = await readSurface();

        // Real ctrl-wheel → RAF loop starts → surface switch happens.
        await browser.execute(() => {
            const scroller = document.getElementById('pdf-scroll-container');
            if (!scroller) throw new Error('scroll container missing');
            const r = scroller.getBoundingClientRect();
            scroller.dispatchEvent(new WheelEvent('wheel', {
                bubbles: true,
                cancelable: true,
                ctrlKey: true,
                deltaY: -120,
                clientX: r.left + r.width / 2,
                clientY: r.top + r.height / 2,
            }));
        });
        await browser.pause(1500);
        const during = await readSurface();
        await browser.pause(1800);
        const after = await readSurface();

        const result = { before, during, after };
        console.log('[surface] snapshot:', JSON.stringify(result, null, 2));

        const assertVectorActive = (s: SurfaceState, label: string) => {
            if (!s.vector.inDom) throw new Error(`vector container missing ${label}`);
            if (s.vector.display !== 'block') {
                throw new Error(`vector container not active (display=${s.vector.display}) ${label}`);
            }
        };
        // During the gesture the vector container must be the active surface.
        assertVectorActive(during, 'during');
        // After settle the vector surface must remain the presented surface.
        assertVectorActive(after, 'after');
        if (!after.mainCanvasVisible) throw new Error('main vector canvas not visible after settle');
        if (!after.settled) throw new Error('not settled after surface transition');
    });
});