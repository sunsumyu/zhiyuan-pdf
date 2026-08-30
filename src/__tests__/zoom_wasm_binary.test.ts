import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

/**
 * TDD: Verify the actual WASM binary contains expected exported function names.
 *
 * wasm-opt can strip functions. wasm-pack caching can serve stale binaries.
 * This test reads the raw .wasm binary and checks for the UTF-8 function names
 * that wasm_bindgen embeds in the "name" section.
 *
 * This catches:
 * - Stale WASM binary (old build without new exports)
 * - wasm-opt stripping needed functions
 * - wasm-pack build not picking up Rust changes
 */

const WASM_PATH = resolve(
    __dirname,
    '../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui_bg.wasm',
);

function readWasmBytes(): Buffer {
    return readFileSync(WASM_PATH);
}

function wasmContainsUtf8(buf: Buffer, name: string): boolean {
    const needle = Buffer.from(name, 'utf8');
    return buf.indexOf(needle) !== -1;
}

describe('WASM binary contains zoom RAF exports', () => {
    const wasm = readWasmBytes();

    const REQUIRED_NAMES = [
        'onWheelEvent',
        'commitRenderedFrameToQueue',
        'startZoomRafLoop',
        'stopZoomRafLoop',
        'isZoomRafLoopRunning',
        '__pdfDrainPendingRenderFrame',
    ];

    for (const name of REQUIRED_NAMES) {
        it(`binary contains "${name}" in name section`, () => {
            expect(wasmContainsUtf8(wasm, name)).toBe(true);
        });
    }

    it('WASM file size is reasonable (>500KB, not stripped to empty)', () => {
        expect(wasm.length).toBeGreaterThan(500_000);
    });

    it('WASM file was recently built (not from months ago)', async () => {
        const { statSync } = await import('fs');
        const stat = statSync(WASM_PATH);
        const ageMs = Date.now() - stat.mtimeMs;
        // Should be less than 1 hour old
        expect(ageMs).toBeLessThan(3_600_000);
    });
});
