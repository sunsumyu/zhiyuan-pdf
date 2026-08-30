import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';

/**
 * TDD: Verify free_api.rs exports match what TS expects.
 *
 * The WASM bridge contract:
 * - onWheelEvent(input_js) → JsValue  (must call ensure_raf_loop_after_wheel)
 * - commitRenderedFrameToQueue(frame_js) → void
 * - startZoomRafLoop() → void
 * - stopZoomRafLoop() → void
 *
 * (onZoomSettle removed — settle is now an envelope push, ADR-0001.)
 */

function readFreeApi(): string {
    return readFileSync(
        resolve(
            __dirname,
            '../../crates/pdf-viewer-ui/src/zoom/free_api.rs',
        ),
        'utf8',
    );
}

function readRafLoop(): string {
    return readFileSync(
        resolve(
            __dirname,
            '../../crates/pdf-viewer-ui/src/zoom/raf_loop.rs',
        ),
        'utf8',
    );
}

function readRafCommitted(): string {
    return readFileSync(
        resolve(
            __dirname,
            '../../crates/pdf-viewer-ui/src/zoom/raf_committed.rs',
        ),
        'utf8',
    );
}

describe('free_api.rs WASM export contract', () => {
    const freeApi = readFreeApi();
    const rafLoop = readRafLoop();
    const rafCommitted = readRafCommitted();

    it('onWheelEvent calls ensure_raf_loop_after_wheel', () => {
        // The WASM export must guarantee the RAF loop is running
        const fnBody = freeApi.substring(
            freeApi.indexOf('pub fn on_wheel_event(input_js'),
            freeApi.indexOf('pub fn on_wheel_event(input_js') + 500,
        );
        expect(fnBody).toMatch(/ensure_raf_loop_after_wheel\(\)/);
    });

    it('commitRenderedFrameToQueue calls raf_loop::commit_rendered_frame', () => {
        expect(freeApi).toMatch(
            /raf_loop::commit_rendered_frame/,
        );
    });

    it('raf_loop.rs defines ensure_raf_loop_after_wheel', () => {
        expect(rafLoop).toMatch(/pub fn ensure_raf_loop_after_wheel/);
    });

    it('raf_loop.rs start_zoom_raf_loop is idempotent', () => {
        // Must check RAF_HANDLE before starting
        const startFn = rafLoop.substring(
            rafLoop.indexOf('pub fn start_zoom_raf_loop()'),
            rafLoop.indexOf('pub fn start_zoom_raf_loop()') + 300,
        );
        expect(startFn).toMatch(/is_some/); // early return if already running
    });

    it('raf_loop.rs commit_rendered_frame handles idle loop', () => {
        // When RAF loop is not running, must apply frame directly.
        // The function body is in raf_committed.rs (extracted from raf_loop).
        const fn = rafCommitted.substring(
            rafCommitted.indexOf('pub fn commit_rendered_frame('),
            rafCommitted.indexOf('pub fn commit_rendered_frame(') + 400,
        );
        expect(fn).toMatch(/is_raf_loop_running/);
        expect(fn).toMatch(/apply_committed_frame/);
    });
});
