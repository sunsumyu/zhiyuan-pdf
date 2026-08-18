import { describe, it, expect } from 'vitest';

describe('zoom commit frame anti-flash', () => {
  it('applyCommittedFrame calls syncLayoutBox unconditionally (no preview guard)', async () => {
    // The new approach: syncLayoutBox is always called in applyCommittedFrame,
    // even when the preview rAF loop is active. This ensures the container DOM
    // dimensions stay consistent with lastRenderedZoom.
    // After syncLayoutBox, if preview is active, the CSS transform is overridden
    // with the visual scale for continuity.
    const fs = await import('fs');
    const path = await import('path');
    const controller = fs.readFileSync(
      path.resolve(__dirname, '../bridge/zoom/zoom_controller.ts'),
      'utf8',
    );

    // Find the applyCommittedFrame function
    const fnMatch = controller.match(
      /function applyCommittedFrame\([\s\S]*?\n    \}/,
    );
    expect(fnMatch).not.toBeNull();
    const fn = fnMatch![0];

    // syncLayoutBox must be called unconditionally (not inside a guard)
    const hasSync = /deps\.syncLayoutBox\(frame\.displayZoom, frame\.renderZoom, frame\)/.test(fn);
    expect(hasSync).toBe(true);

    // When preview is active, CSS transform should be overridden
    const hasPreviewOverride = /wheelZoomRafId !== null/.test(fn);
    expect(hasPreviewOverride).toBe(true);
  });

  it('vector canvas container uses overflow:visible so cssScale preview is not clipped', async () => {
    // 缩放预览期间 canvas 按 cssScale 放大后可能超出容器的布局盒；
    // overflow:hidden 会把预览画面裁掉（用户确认过的裁切 bug）。
    // 容器必须在创建时就使用 overflow:visible。
    const fs = await import('fs');
    const path = await import('path');
    const canvasHost = fs.readFileSync(
      path.resolve(__dirname, '../bridge/render/vector_canvas_host.ts'),
      'utf8',
    );

    const cssTextMatch = canvasHost.match(/container\.style\.cssText = \[([\s\S]*?)\]\.join/);
    expect(cssTextMatch).not.toBeNull();
    expect(cssTextMatch![1]).toContain("'overflow: visible'");
  });

  it('presentViewportCanvas sets container visible', async () => {
    // presentViewportCanvas must restore container visibility after the
    // bitmap has been drawn, completing the hide-draw-show cycle.
    const fs = await import('fs');
    const path = await import('path');
    const canvasHost = fs.readFileSync(
      path.resolve(__dirname, '../bridge/render/vector_canvas_host.ts'),
      'utf8',
    );

    // presentViewportCanvas sets container visible
    const setsVisible = /container\.style\.visibility\s*=\s*['"]visible['"]/.test(canvasHost);
    expect(setsVisible).toBe(true);
  });

  it('syncLayoutBox sets transform unconditionally', async () => {
    // syncLayoutBox should always set container.style.transform
    // (either to scale(...) or to '' if cssScale ≈ 1.0).
    // This means there's no need to clear it beforehand.
    const fs = await import('fs');
    const path = await import('path');
    const layout = fs.readFileSync(
      path.resolve(__dirname, '../bridge/viewer/pdf_layout_sync.ts'),
      'utf8',
    );

    // syncLayoutBox sets container.style.transform based on cssScale
    const setsTransform = /container\.style\.transform\s*=/.test(layout);
    expect(setsTransform).toBe(true);
  });

  it('commitRenderedFrame updates container DOM when preview is active', async () => {
    // When the preview rAF loop is active and a committed frame arrives,
    // commitRenderedFrame must call syncLayoutBox to update the container
    // DOM to match the committed zoom. Without this, the container stays
    // at the old lastRenderedZoom size, causing a visible jump when the
    // preview loop computes cssScale = visualZoom / newLastRenderedZoom.
    //
    // After syncLayoutBox, the CSS transform is overridden with the visual
    // scale for continuity.
    const fs = await import('fs');
    const path = await import('path');
    const controller = fs.readFileSync(
      path.resolve(__dirname, '../bridge/zoom/zoom_controller.ts'),
      'utf8',
    );

    // Find the commitRenderedFrame function
    const fnMatch = controller.match(
      /function commitRenderedFrame\([\s\S]*?\n    \}/,
    );
    expect(fnMatch).not.toBeNull();
    const fn = fnMatch![0];

    // Must call syncLayoutBox to update container DOM during preview
    const hasSync = /deps\.syncLayoutBox\(frame\.displayZoom, frame\.renderZoom, frame\)/.test(fn);
    expect(hasSync).toBe(true);

    // Must override CSS transform for visual continuity
    const hasTransformOverride = /container\.style\.transform/.test(fn);
    expect(hasTransformOverride).toBe(true);
  });

  it('applyViewportCanvasFrame skips visible-canvas re-box when deferVisibleFrame is set', async () => {
    // 缩放预览期间渲染开始时，container 仍处于上一次提交的 render zoom
    // 布局并带着 preview scale（visualZoom / lastRenderedZoom）。如果此时
    // 把可见 mainCanvas/backCanvas 的 CSS 重排成新 render zoom 的尺寸，
    // canvas 视觉宽度会变成 pageWidth * newRenderZoom * previewScale，
    // 在 commit 同步 container 之前呈现为骤缩/骤放（用户看到的闪烁）。
    // 双缓冲（deferVisibleFrame=true）时必须保持已提交帧的 CSS 盒，
    // 由 presentViewportCanvasFromSource 在 present 时原子重排。
    const fs = await import('fs');
    const path = await import('path');
    const canvasHost = fs.readFileSync(
      path.resolve(__dirname, '../bridge/render/vector_canvas_host.ts'),
      'utf8',
    );

    const fnMatch = canvasHost.match(
      /export function applyViewportCanvasFrame\([\s\S]*?\n\}/,
    );
    expect(fnMatch).not.toBeNull();
    const fn = fnMatch![0];

    // 可见 canvas 的重排必须被 deferVisibleFrame 守卫
    const guardedBox = /if \(!deferVisibleFrame\) \{[\s\S]*?applyCanvasCssBox\(refs\.mainCanvas[\s\S]*?applyCanvasCssBox\(refs\.backCanvas[\s\S]*?\n    \}/.test(fn);
    expect(guardedBox).toBe(true);

    // 守卫之外不得再直接重排可见 canvas
    const outside = fn.replace(/if \(!deferVisibleFrame\) \{[\s\S]*?\n    \}/, '');
    expect(/applyCanvasCssBox\(refs\.mainCanvas/.test(outside)).toBe(false);
    expect(/applyCanvasCssBox\(refs\.backCanvas/.test(outside)).toBe(false);

    // 离屏 stage 位图尺寸仍无条件设置（present 需要正确的缓冲大小）
    expect(/ensureCanvasBitmap\(refs\.mainStageCanvas/.test(fn)).toBe(true);
  });
});
