/**
 * TDD: settle envelope — 推帧而非推回调（ADR-0001 候选2）
 *
 * 不变量：
 * E1. raf_loop.rs 不再持有 ON_SETTLE_CALLBACK / notify_settle() ——
 *     settle 时 Rust 直接构建 FramePlanRequest 并调度信封。
 * E2. free_api.rs 不再导出 onZoomSettle。
 * E3. TS 侧不再有 registerZoomSettleCallback / onZoomSettle 注册链。
 * E4. 新敲门函数 drainPendingRenderFrame 存在且由 Rust 在 settle 后直呼。
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const RAF_LOOP = readFileSync(
    resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_loop.rs'),
    'utf8',
);
const RAF_DISPATCH = readFileSync(
    resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_dispatch.rs'),
    'utf8',
);
const FREE_API = readFileSync(
    resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/free_api.rs'),
    'utf8',
);
const PDF_RUNTIME = readFileSync(
    resolve(__dirname, '../bridge/viewer/pdf_runtime.ts'),
    'utf8',
);
const RENDER_FLOW = readFileSync(
    resolve(__dirname, '../bridge/render/render_flow.ts'),
    'utf8',
);
const RENDER_WASM_API = readFileSync(
    resolve(__dirname, '../bridge/render/render_wasm_api.ts'),
    'utf8',
);

describe('settle envelope: push frames, not callbacks', () => {
    it('E1: raf_loop has no callback storage or notify_settle', () => {
        expect(RAF_LOOP).not.toMatch(/ON_SETTLE_CALLBACK/);
        expect(RAF_LOOP).not.toMatch(/fn notify_settle/);
        expect(RAF_LOOP).not.toMatch(/on_settle_callback/);
    });

    it('E1: raf_loop dispatches settle via knock function, not direct scheduling', () => {
        // ADR-0001: RAF loop knocks TS render loop; TS reads zoom state and schedules itself.
        // The knock function is in raf_dispatch.rs (extracted from raf_loop).
        expect(RAF_LOOP + RAF_DISPATCH).toMatch(/knock_render_loop|__pdfDrainPendingRenderFrame/);
        expect(RAF_LOOP).not.toMatch(/schedule_render_frame_request/);
    });

    it('E2: free_api no longer exports onZoomSettle', () => {
        expect(FREE_API).not.toMatch(/onZoomSettle/);
    });

    it('E3: TS runtime has no registerZoomSettleCallback registration chain', () => {
        expect(PDF_RUNTIME).not.toMatch(/registerZoomSettleCallback/);
        expect(PDF_RUNTIME).not.toMatch(/onZoomSettle/);
    });

    it('E3: render_wasm_api no longer wraps onZoomSettle', () => {
        expect(RENDER_WASM_API).not.toMatch(/onZoomSettle/);
    });

    it('E4: a fixed global knock function exists for Rust to invoke', () => {
        // Rust calls window.__pdfDrainPendingRenderFrame() once after settle.
        // The constant is defined in raf_dispatch.rs (extracted from raf_loop).
        expect(RAF_DISPATCH).toMatch(/__pdfDrainPendingRenderFrame/);
    });

    it('E5: knock renders at visualZoom so preview bitmaps track the animation (C1)', () => {
        // ADR-0002: mid-gesture commits need renderZoom == displayZoom to be
        // seamless; at settle visualZoom == targetZoom so settle is unchanged.
        expect(PDF_RUNTIME).toMatch(/renderCurrentPage\('zoom',\s*readZoomState\(\)\.visualZoom\)/);
    });

    it('E6: render flow accepts a zoom override for knock renders', () => {
        expect(RENDER_FLOW).toMatch(/zoomOverride/);
    });

    it('E7: raf_loop reknocks mid-animation via the throttled decision', () => {
        expect(RAF_LOOP).toMatch(/should_reknock_preview_render/);
        expect(RAF_LOOP).toMatch(/in_flight_frame_token/);
    });
});
