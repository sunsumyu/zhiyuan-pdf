import { describe, expect, it, vi } from 'vitest';
import { createRenderWasmApi, type RenderWasmApi } from '../bridge/render/render_wasm_api';
import { ZoomTestTraceCollector } from './zoom_test_trace';

describe('RenderWasmApi zoom/frame boundary', () => {
  it('forwards frame arguments and preserves returned transition', () => {
    const settleRenderFrame = vi.fn(() => ({ accepted: true, nextFrame: null }));
    const commitRenderResult = vi.fn(() => ({ accepted: true, pageWidth: 600, pageHeight: 900 }));
    const wasm = {
      settleRenderFrame,
      commitRenderResult,
      isRenderFrameCurrent: vi.fn(() => true),
    } as unknown as Parameters<typeof createRenderWasmApi>[0] extends () => infer T ? T : never;
    const api = createRenderWasmApi(() => wasm);
    const trace = new ZoomTestTraceCollector('Z-TS-FRAME-001', 'bridge', 'render_wasm_api');
    trace.caseStart({ frame_token: 9, rendered_zoom: 1.25 });
    trace.stateBefore({ frame_token: 9 });

    const transition = api.settleRenderFrame(9, 1.25);
    const commit = api.commitRenderResult(9, 1.25, 600, 900);
    trace.decision({ transition, commit });
    trace.assertion(
      { settle: [9, 1.25], commit: [9, 1.25, 600, 900] },
      { settle: settleRenderFrame.mock.calls[0], commit: commitRenderResult.mock.calls[0] },
      'PASS',
    );
    trace.caseEnd('PASS');

    expect(transition).toEqual({ accepted: true, nextFrame: null });
    expect(commit).toEqual({ accepted: true, pageWidth: 600, pageHeight: 900 });
    expect(settleRenderFrame).toHaveBeenCalledWith(9, 1.25);
    expect(commitRenderResult).toHaveBeenCalledWith(9, 1.25, 600, 900);
    expect(trace.snapshot()).toHaveLength(6);
  });

  it('returns safe null/false values when optional wasm methods are absent', () => {
    const api = createRenderWasmApi(() => ({} as Parameters<typeof createRenderWasmApi>[0] extends () => infer T ? T : never));
    expect(api.settleRenderFrame(1, 1)).toBeNull();
    expect(api.commitRenderResult(1, 1, 600, 900)).toBeNull();
    expect(api.isRenderFrameCurrent(1)).toBe(true);
    expect(api.isImmediateMutationFrame('unknown')).toBe(false);
    expect(api.onWheelEvent({ deltaY: 1 })).toBeNull();
  });
});
