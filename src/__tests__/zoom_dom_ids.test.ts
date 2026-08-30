import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

/**
 * TDD: The Rust RAF loop applies CSS transforms directly to DOM elements
 * via web-sys `getElementById`. If the IDs hardcoded in raf_loop.rs do not
 * match the IDs actually created by the TS bridge, init_dom_cache() returns
 * None and EVERY apply_css_transform silently no-ops — zoom appears
 * completely dead while all logic tests still pass.
 *
 * This test cross-references:
 *   - crates/pdf-viewer-ui/src/zoom/raf_loop.rs (Rust DOM lookups)
 *   - src/bridge/render/vector_canvas_host.ts (real container creation)
 *   - index.html (static scroll container)
 */
function readRafLoop(): string {
    // DOM ID constants were extracted to raf_dom_cache.rs; read both files
    // and concatenate so the regex searches still find them.
    const loop = readFileSync(
        resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_loop.rs'),
        'utf8',
    );
    const domCache = readFileSync(
        resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_dom_cache.rs'),
        'utf8',
    );
    return loop + '\n' + domCache;
}

function readCanvasHost(): string {
    return readFileSync(
        resolve(__dirname, '../bridge/render/vector_canvas_host.ts'),
        'utf8',
    );
}

describe('raf_loop DOM ids match real bridge elements', () => {
    it('raf_loop container id equals VECTOR_CONTAINER_ID from vector_canvas_host.ts', () => {
        const canvasHost = readCanvasHost();
        const tsId = canvasHost.match(/VECTOR_CONTAINER_ID\s*=\s*'([^']+)'/)?.[1];
        expect(tsId).toBeTruthy();

        const rafLoop = readRafLoop();
        const rustId = rafLoop.match(/const\s+VECTOR_CONTAINER_ID[^=]*=\s*"([^"]+)"/)?.[1];
        expect(rustId).toBeTruthy();

        expect(rustId).toBe(tsId);
    });

    it('raf_loop scroll container id exists in index.html', () => {
        const html = readFileSync(resolve(__dirname, '../../index.html'), 'utf8');
        const rafLoop = readRafLoop();
        const rustScrollId = rafLoop.match(/const\s+SCROLL_CONTAINER_ID[^=]*=\s*"([^"]+)"/)?.[1];
        expect(rustScrollId).toBeTruthy();
        expect(html).toContain(`id="${rustScrollId}"`);
    });

    it('raf_loop does not reference any id that is absent from the TS bridge', () => {
        const canvasHost = readCanvasHost();
        const knownIds = new Set(
            [...canvasHost.matchAll(/'([a-z][a-z0-9-]+)'/g)].map((m) => m[1]),
        );
        const html = readFileSync(resolve(__dirname, '../../index.html'), 'utf8');
        for (const m of html.matchAll(/id="([^"]+)"/g)) knownIds.add(m[1]);

        const rafLoop = readRafLoop();
        const rustIds = [
            ...rafLoop.matchAll(/const\s+\w*CONTAINER_ID[^=]*=\s*"([^"]+)"/g),
        ].map((m) => m[1]);

        for (const id of rustIds) {
            expect(
                knownIds.has(id),
                `"${id}" used by raf_loop.rs must exist in TS bridge or index.html`,
            ).toBe(true);
        }
    });
});
