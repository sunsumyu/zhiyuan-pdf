/**
 * Browser-facing zoom behavior checks.
 * Verifies the final DOM SetBox geometry and scroll state after a real zoom
 * request, plus the settled-state cleanup exposed by the runtime.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const zoomDomPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const zoomDomHelpers = require('../helpers/app') as typeof import('../helpers/app');

const zoomDomRepoRoot = zoomDomPath.resolve(__dirname, '..', '..', '..');
const zoomDomFixturePath = zoomDomPath.join(zoomDomRepoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

type Box = {
    left: number;
    top: number;
    width: number;
    height: number;
    scrollLeft: number;
    scrollTop: number;
};

type Snapshot = {
    zoom: any;
    container: Box | null;
    wrapper: Box | null;
    scroller: Box | null;
    main: Box | null;
    back: Box | null;
};

function isFiniteBox(box: Box | null): box is Box {
    return !!box && [box.left, box.top, box.width, box.height, box.scrollLeft, box.scrollTop].every(Number.isFinite);
}

describe('Zoom DOM behavior', () => {
    before(async () => {
        await zoomDomHelpers.waitForApp();
        await zoomDomHelpers.loadFixturePdf(zoomDomFixturePath);
        await browser.pause(1200);
    });

    it('applies settled SetBox geometry and preserves finite scroll state', async () => {
        const snapshot = async (): Promise<Snapshot> =>
            (await browser.execute(() => {
                const box = (node: Element | null): Box | null => {
                    if (!node) return null;
                    const element = node as HTMLElement;
                    const rect = element.getBoundingClientRect();
                    const scroller = node as HTMLElement;
                    return {
                        left: rect.left,
                        top: rect.top,
                        width: rect.width,
                        height: rect.height,
                        scrollLeft: scroller.scrollLeft,
                        scrollTop: scroller.scrollTop,
                    };
                };
                return {
                    zoom: (window as any).wasmv3?.getZoomState?.(),
                    container: box(document.getElementById('pdf-page-container')),
                    wrapper: box(document.getElementById('pdf-content-wrapper')),
                    scroller: box(document.getElementById('pdf-scroll-container')),
                    main: box(document.getElementById('pdf-vector-main-canvas')),
                    back: box(document.getElementById('pdf-vector-detail-canvas')),
                };
            })) as Snapshot;

        const before = await snapshot();
        await browser.execute(() => (window as any).pdfZoomChange?.('0.72'));
        await browser.pause(2500);
        const after = await snapshot();

        const result = {
            before: {
                zoom: before.zoom,
                container: before.container,
                scroller: before.scroller,
            },
            after: {
                zoom: after.zoom,
                container: after.container,
                wrapper: after.wrapper,
                scroller: after.scroller,
                main: after.main,
                back: after.back,
            },
            assertions: {
                zoomConverged:
                    Number.isFinite(after.zoom?.targetZoom) &&
                    Number.isFinite(after.zoom?.visualZoom) &&
                    Number.isFinite(after.zoom?.lastRenderedZoom) &&
                    Math.abs(after.zoom.targetZoom - after.zoom.visualZoom) < 0.001 &&
                    Math.abs(after.zoom.targetZoom - after.zoom.lastRenderedZoom) < 0.001,
                geometryFinite: [after.container, after.wrapper, after.scroller, after.main].every(isFiniteBox),
                geometryPositive: [after.container, after.wrapper, after.main].every(
                    (box) => !!box && box.width > 0 && box.height > 0,
                ),
                scrollFinite: isFiniteBox(after.scroller),
                previewSettled: after.zoom?.previewHost?.previewActive === false,
            },
        };
        console.log('[zoom-dom] snapshot:', JSON.stringify(result, null, 2));

        if (!result.assertions.zoomConverged) throw new Error('zoom state did not converge after DOM zoom request');
        if (!result.assertions.geometryFinite || !result.assertions.geometryPositive) {
            throw new Error('settled SetBox geometry is not finite and positive');
        }
        if (!result.assertions.scrollFinite) throw new Error('scroll state is not finite');
        if (!result.assertions.previewSettled) throw new Error('preview host remained active after settle');
    });
});
