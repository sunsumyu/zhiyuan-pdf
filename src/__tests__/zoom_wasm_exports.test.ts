import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

/**
 * TDD: Verify WASM module exports the required zoom RAF loop API.
 *
 * This catches the class of bug where:
 * - wasm-pack didn't rebuild after Rust changes
 * - wasm-opt stripped needed functions
 * - #[wasm_bindgen] attributes were changed/removed
 *
 * (onZoomSettle removed from the contract — settle is now an envelope
 * push with a fixed global knock function, ADR-0001.)
 */

const WASM_DTS_PATH = resolve(
    __dirname,
    '../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui.d.ts',
);

function readDts(): string {
    return readFileSync(WASM_DTS_PATH, 'utf8');
}

describe('WASM zoom RAF loop exports', () => {
    const REQUIRED_EXPORTS = [
        'startZoomRafLoop',
        'stopZoomRafLoop',
        'isZoomRafLoopRunning',
        'onWheelEvent',
        'commitRenderedFrameToQueue',
    ] as const;

    for (const fnName of REQUIRED_EXPORTS) {
        it(`d.ts declares ${fnName} as a standalone function`, () => {
            const dts = readDts();
            const pattern = new RegExp(
                `^export function ${fnName}\\(`,
                'm',
            );
            expect(dts).toMatch(pattern);
        });
    }

    it('onWheelEvent returns any (not void)', () => {
        const dts = readDts();
        const match = dts.match(
            /^export function onWheelEvent\(input_js: any\):\s*(\S+)/m,
        );
        expect(match).not.toBeNull();
        expect(match![1]).not.toBe('void');
    });

    it('onZoomSettle is no longer exported (envelope replaces callback)', () => {
        const dts = readDts();
        expect(dts).not.toMatch(/^export function onZoomSettle\(/m);
    });
});
