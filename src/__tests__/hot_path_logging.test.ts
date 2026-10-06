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
const bridgeRenderSrc = resolve(__dirname, '../../src/bridge/render');

const read = (p: string) => readFileSync(resolve(uiSrc, p), 'utf8');
const readBridgeRender = (p: string) => readFileSync(resolve(bridgeRenderSrc, p), 'utf8');

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

    it('canvas/page.rs paint path does not write to the console (ADR-0013)', () => {
        // render_page runs once per page render; an unconditional log here
        // fired on every vector page paint ("[CANVAS-DBG] render_page
        // finished", found 2026-10-06).
        const src = read('render/canvas/page.rs');
        expect(src).not.toMatch(/console::log_1/);
        expect(src).not.toMatch(/CANVAS-DBG/);
    });

    it('bridge render-path modules do not log info-level messages (ADR-0022)', () => {
        // Stale-frame aborts fire per frame during gestures; per ADR-0022 they
        // degrade to DEBUG via emitPdfDiagnostic (no IPC, no console flood),
        // never console.log. console.error/warn stay allowed: the error stream
        // keeps real failures only.
        for (const file of ['vector_host.ts', 'vector_page_bundle.ts', 'tile_layer.ts', 'render_flow.ts']) {
            const src = readBridgeRender(file);
            expect(src, `${file} must not console.log/info/debug`).not.toMatch(/console\.(log|info|debug)\(/);
        }
    });
});
