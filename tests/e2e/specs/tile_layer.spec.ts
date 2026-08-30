/**
 * Tile layer end-to-end probe (ADR-0003 chain):
 * open a PDF → settle → the Rust TileManager must have rendered viewport
 * tiles through the vector worker, and tile canvases must be present in the
 * DOM inside the vector container.
 */
// eslint-disable-next-line @typescript-eslint/no-var-requires
const tilePdfPath = require('node:path') as typeof import('node:path');
// eslint-disable-next-line @typescript-eslint/no-var-requires
const tileHelpers = require('../helpers/app') as typeof import('../helpers/app');

const tileRepoRoot = tilePdfPath.resolve(__dirname, '..', '..', '..');
const tileFixturePath = tilePdfPath.join(tileRepoRoot, 'tests', 'e2e', 'fixtures', 'multipage.pdf');

describe('Tile layer chain', () => {
    before(async () => {
        await tileHelpers.waitForApp();
        await tileHelpers.loadFixturePdf(tileFixturePath);
    });

    it('renders viewport tiles after settle (Rust stats + DOM canvases)', async () => {
        // Wait for the settle → schedule → worker render → present chain.
        await browser.waitUntil(
            async () => {
                const ready = (await browser.execute(() => {
                    const w = window as any;
                    const stats = w.wasmv3?.renderFacadeTileStats?.();
                    return Number(stats?.cache?.ready ?? 0);
                })) as number;
                return ready > 0;
            },
            { timeout: 15_000, interval: 500 },
        );

        const probe = (await browser.execute(() => {
            const w = window as any;
            const stats = w.wasmv3?.renderFacadeTileStats?.();
            const layer = document.getElementById('pdf-tile-layer');
            const canvases = layer
                ? Array.from(layer.querySelectorAll('canvas')).map((c) => ({
                    visible: (c as HTMLCanvasElement).style.display === 'block',
                    width: (c as HTMLCanvasElement).width,
                }))
                : [];
            return {
                ready: Number(stats?.cache?.ready ?? 0),
                pending: Number(stats?.cache?.pending ?? 0),
                queueSize: Number(stats?.queue_size ?? 0),
                layerExists: !!layer,
                canvasCount: canvases.length,
                visibleCanvases: canvases.filter((c) => c.visible).length,
                bitmapSizes: Array.from(new Set(canvases.map((c) => c.width))),
            };
        })) as any;

        console.log('[e2e] tile probe:', JSON.stringify(probe));
        if (!probe.layerExists) throw new Error('#pdf-tile-layer missing from DOM');
        if (probe.ready <= 0) throw new Error(`expected ready tiles > 0, got ${probe.ready}`);
        if (probe.visibleCanvases <= 0) throw new Error('no visible tile canvases presented');
        // Tile bitmaps are 512px cells scaled by dpr (edge tiles clipped).
        if (!probe.bitmapSizes.every((s: number) => s > 0 && s <= 2048)) {
            throw new Error(`unexpected tile bitmap sizes: ${probe.bitmapSizes}`);
        }
    });
});
