import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

/**
 * TDD: Verify zoom_controller.ts wiring matches the Rust RAF loop contract.
 *
 * The Rust RAF loop requires:
 * 1. onWheelEvent called on every wheel gesture (starts loop internally)
 * 2. commitRenderedFrameToQueue called for each rendered frame
 * 3. settle envelope knock installed (fixed global, no registrable callback — ADR-0001)
 * 4. startZoomRafLoop NOT called at bind time (loop self-manages)
 */

function readController(): string {
    return readFileSync(
        resolve(__dirname, '../bridge/zoom/zoom_controller.ts'),
        'utf8',
    );
}

function readRuntime(): string {
    return readFileSync(
        resolve(__dirname, '../bridge/viewer/pdf_runtime.ts'),
        'utf8',
    );
}

describe('zoom_controller.ts Rust RAF wiring', () => {
    it('wheel handler calls deps.onWheelEvent(input)', () => {
        const code = readController();
        expect(code).toMatch(/deps\.onWheelEvent\(input\)/);
    });

    it('wheel handler does NOT call deps.startZoomRafLoop', () => {
        const code = readController();
        // startZoomRafLoop must NOT be called in the wheel path or at bind time
        const wheelSection = code.substring(
            code.indexOf('scrollContainer.addEventListener'),
        );
        expect(wheelSection).not.toMatch(/deps\.startZoomRafLoop\(\)/);
    });

    it('commitRenderedFrame delegates to commitRenderedFrameToQueue', () => {
        const code = readController();
        expect(code).toMatch(/deps\.commitRenderedFrameToQueue\(frame\)/);
    });

    it('does NOT contain old TS RAF loop artifacts', () => {
        const code = readController();
        expect(code).not.toMatch(/startSmoothZoomPreview/);
        expect(code).not.toMatch(/applyZoomTransform/);
        expect(code).not.toMatch(/wheelZoomRafId/);
    });

    it('pdf_runtime installs the settle knock global (no registrable callback)', () => {
        const code = readRuntime();
        expect(code).toMatch(/__pdfDrainPendingRenderFrame/);
        expect(code).toMatch(/renderCurrentPage\('zoom',/);
        expect(code).not.toMatch(/onZoomSettle/);
    });

    it('pdf_runtime wraps startZoomRafLoop in try-catch', () => {
        const code = readRuntime();
        expect(code).toMatch(
            /try.*startZoomRafLoop.*catch/s,
        );
    });
});
