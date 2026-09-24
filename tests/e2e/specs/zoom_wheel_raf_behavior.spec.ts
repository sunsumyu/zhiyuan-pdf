/** Browser-facing wheel → RAF → settle behavior. */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const wheelPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const wheelHelpers = require('../helpers/app') as typeof import('../helpers/app');

const wheelRoot = wheelPath.resolve(__dirname, '..', '..', '..');
const wheelFixture = wheelPath.join(wheelRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

type State = {
    zoom: any;
    container: { width: number; height: number; left: number; top: number } | null;
    scroller: { scrollLeft: number; scrollTop: number } | null;
};

describe('Zoom wheel RAF behavior', () => {
    before(async () => {
        await wheelHelpers.waitForApp();
        await wheelHelpers.loadFixturePdf(wheelFixture);
        await browser.pause(1200);
    });

    it('wakes RAF from a real ctrl-wheel and settles without invalid geometry', async () => {
        const readState = async (): Promise<State> => (await browser.execute(() => {
            const read = (id: string) => {
                const e = document.getElementById(id);
                if (!e) return null;
                const r = e.getBoundingClientRect();
                return { width: r.width, height: r.height, left: r.left, top: r.top };
            };
            const scroller = document.getElementById('pdf-scroll-container');
            return {
                zoom: (window as any).wasmv3?.getZoomState?.(),
                container: read('pdf-page-container'),
                scroller: scroller ? { scrollLeft: scroller.scrollLeft, scrollTop: scroller.scrollTop } : null,
            };
        })) as State;

        const before = await readState();
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
        const immediate = await readState();
        // Sample during the RAF animation window. A single wheel tick's zoom
        // animation converges in ~60ms, and SETTLE_DRAWING_DELAY_MS=30ms after
        // that the settle render fires and updates geometry to the final zoom.
        // Sampling at 80ms lands on that settle boundary and flakes; 20ms is
        // unambiguously inside the active gesture, where geometry must stay
        // the wheel-owned value.
        await browser.pause(20);
        const during = await readState();
        await browser.pause(2200);
        const after = await readState();

        const result = {
            before,
            immediate,
            during,
            after,
            assertions: {
                targetChanged: Math.abs((during.zoom?.targetZoom ?? 1) - (before.zoom?.targetZoom ?? 1)) > 0.0001,
                duringFinite: Number.isFinite(during.zoom?.targetZoom) && Number.isFinite(during.zoom?.visualZoom),
                immediateGeometryFinite: !!immediate.container &&
                    [immediate.container.width, immediate.container.height, immediate.container.left, immediate.container.top].every(Number.isFinite),
                rafPreservedWheelGeometry: !!immediate.container && !!during.container &&
                    Math.abs(immediate.container.width - during.container.width) < 0.5 &&
                    Math.abs(immediate.container.height - during.container.height) < 0.5 &&
                    Math.abs(immediate.container.left - during.container.left) < 0.5 &&
                    Math.abs(immediate.container.top - during.container.top) < 0.5,
                afterConverged: Number.isFinite(after.zoom?.targetZoom) &&
                    Math.abs(after.zoom.targetZoom - after.zoom.visualZoom) < 0.001 &&
                    Math.abs(after.zoom.targetZoom - after.zoom.lastRenderedZoom) < 0.001,
                afterGeometryFinite: !!after.container &&
                    [after.container.width, after.container.height, after.container.left, after.container.top].every(Number.isFinite),
                afterScrollFinite: !!after.scroller &&
                    Number.isFinite(after.scroller.scrollLeft) && Number.isFinite(after.scroller.scrollTop),
                settled: after.zoom?.previewHost?.previewActive === false && after.zoom?.drawingDelay?.active === false,
            },
        };
        console.log('[zoom-wheel] snapshot:', JSON.stringify(result, null, 2));

        if (!result.assertions.targetChanged) throw new Error('ctrl-wheel did not change target zoom');
        if (!result.assertions.duringFinite) throw new Error('wheel RAF state became non-finite');
        if (!result.assertions.immediateGeometryFinite || !result.assertions.rafPreservedWheelGeometry) {
            throw new Error('RAF changed wheel-owned geometry during the active gesture');
        }
        if (!result.assertions.afterConverged) throw new Error('wheel zoom did not converge after settle');
        if (!result.assertions.afterGeometryFinite || !result.assertions.afterScrollFinite) {
            throw new Error('wheel settle produced invalid DOM geometry or scroll');
        }
        if (!result.assertions.settled) throw new Error('wheel settle left preview or drawing delay active');
    });
});
