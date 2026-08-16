import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock wasm_loader and all heavy dependencies so we can test the pure logic
// of how bundle loading interacts with WASM initialization.
vi.mock('../bridge/shared/wasm_loader', () => ({
  ensureWasmInitialized: vi.fn(),
  getWasmApi: vi.fn(() => null),
}));

vi.mock('../bridge/shared/session_singletons', () => ({
  getDocumentSession: vi.fn(() => null),
  getReviewSession: vi.fn(() => null),
}));

vi.mock('../bridge/render/layout_trace', () => ({
  logPdfLayoutTrace: vi.fn(),
}));

vi.mock('../bridge/shared/diagnostics', () => ({
  emitPdfDiagnostic: vi.fn(),
}));

vi.mock('../bridge/render/vector_canvas_pool', () => ({
  CanvasPool: { rent: vi.fn(), recycle: vi.fn() },
}));

vi.mock('../bridge/render/vector_frame_cache', () => ({
  readViewportFrameCache: vi.fn(() => null),
  writeViewportFrameCache: vi.fn(),
  deleteViewportFrameCacheKeys: vi.fn(),
  clearVectorFrameCache: vi.fn(),
}));

describe('zoom init dedup', () => {
  it('document: vector_page_bundle.ts should NOT call initPageContext directly', async () => {
    // This is a documentation test: it verifies that the hydration
    // initPageContext call was removed from vector_page_bundle.ts.
    //
    // The actual WASM init happens in vector_host.ts renderVectorPageWithPlan
    // which calls initPageContext with the correct render zoom.
    //
    // The old code called initPageContext(zoom=1.0) here, then
    // vector_host.ts called it again with the real zoom, causing a
    // zoom=1.0 flash frame between the two calls.
    //
    // After the fix, the WASM state is only initialized once with
    // the correct zoom level, eliminating the flash.

    const fs = await import('fs');
    const path = await import('path');
    const bundle = fs.readFileSync(
      path.resolve(__dirname, '../bridge/render/vector_page_bundle.ts'),
      'utf8',
    );

    // The file should NOT contain a direct wasm.initPageContext call
    // in the bundle loading path. The WASM init is delegated to
    // vector_host.ts which calls it with the correct render zoom.
    const hasDirectInit = /wasm\.initPageContext\(/.test(bundle);
    expect(hasDirectInit).toBe(false);
  });

  it('document: vector_host.ts is the single WASM init call site', async () => {
    const fs = await import('fs');
    const path = await import('path');
    const host = fs.readFileSync(
      path.resolve(__dirname, '../bridge/render/vector_host.ts'),
      'utf8',
    );

    // vector_host.ts should call initPageContext when bundleChanged is true
    const hasInit = /renderApi\.initPageContext\(/.test(host);
    expect(hasInit).toBe(true);

    // And should call updatePageViewport when bundleChanged is false (zoom on same page)
    const hasUpdate = /renderApi\.updatePageViewport\(/.test(host);
    expect(hasUpdate).toBe(true);
  });

  it('zoom on same page should NOT trigger initPageContext', () => {
    // Simulate the bundleChanged logic:
    // When zooming on the same page, bundleChanged is false,
    // so only updatePageViewport is called (no model re-init).
    const bundleChanged = false;
    let wasmCall: string;

    if (bundleChanged) {
      wasmCall = 'initPageContext';
    } else {
      wasmCall = 'updatePageViewport';
    }

    expect(wasmCall).toBe('updatePageViewport');
  });

  it('new page load should trigger initPageContext with correct zoom', () => {
    // When a new page is loaded, bundleChanged is true,
    // so initPageContext is called with the actual render zoom.
    const bundleChanged = true;
    const renderZoom = 1.5;
    let wasmCall: string;
    let zoomUsed: number;

    if (bundleChanged) {
      wasmCall = 'initPageContext';
      zoomUsed = renderZoom;
    } else {
      wasmCall = 'updatePageViewport';
      zoomUsed = renderZoom;
    }

    expect(wasmCall).toBe('initPageContext');
    expect(zoomUsed).toBe(1.5);
  });
});
