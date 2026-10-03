/**
 * ADR-0013 contract — hot paths must not log unconditionally.
 *
 * CPU profile evidence (tools/cdp-profile.mjs, 2026-10-01): a 16-step ctrl-wheel
 * gesture emitted 2456 console messages, 2245 of them "[WASM-ViewerSession]
 * read() is called" — a leftover debug print on the HOTTEST read path in the
 * app. The wasm→JS console bridge alone burned 506ms of self time (vs 12.6ms
 * for drawImage, i.e. the actual rendering), which is the dominant part of the
 * ~100ms per-wheel-step main-thread block.
 *
 * Follows the repo's established source-contract style (zoom_raf_contract /
 * zoom_rust_free_api): assert on the Rust source so the guard holds even for
 * code paths that unit tests never execute.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const uiSrc = resolve(__dirname, '../../crates/pdf-viewer-ui/src');

const read = (p: string) => readFileSync(resolve(uiSrc, p), 'utf8');

/** Extract the `read` method body from the ViewerSession wasm_bindgen impl. */
function viewerReadBody(): string {
    const src = read('viewer/viewer_api.rs');
    const start = src.indexOf('pub fn read(&self) -> JsValue');
    expect(start).toBeGreaterThanOrEqual(0);
    const end = src.indexOf('/// Bind a freshly opened document', start);
    return src.slice(start, end > 0 ? end : start + 2000);
}

describe('hot-path logging contract (ADR-0013)', () => {
    it('ViewerSession::read() does not write to the console', () => {
        // read() runs ~9x per animation frame during a zoom gesture; a console
        // write here flooded 2245 messages into one gesture.
        const body = viewerReadBody();
        expect(body).not.toMatch(/console::log_|log_1\s*\(/);
        expect(body).not.toMatch(/WASM-ViewerSession/);
    });

    it('the frame-plan builder does not log on every build', () => {
        // peek/schedule/followUp each build a plan per render — an
        // unconditional log here fires several times per wheel step.
        const src = read('present/plan_builder.rs');
        expect(src).not.toMatch(/log::info!/);
        expect(src).not.toMatch(/PAGE-SIZE/);
    });

    it('chain_trace remains the gated cross-cutting trace (gate exists)', () => {
        // The allowed logging route keeps its runtime gate.
        const gate = read('common/chain_trace.rs');
        expect(gate).toMatch(/CHAIN_ENABLED/);
        expect(gate).toMatch(/if !is_chain_trace_enabled\(\)/);
    });
});
