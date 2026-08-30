import { describe, it, expect } from 'vitest';

describe('zoom Rust RAF loop architecture', () => {
  it('zoom_controller.ts binds wheel to onWheelEvent', async () => {
    // The new architecture delegates all wheel logic to Rust via a single onWheelEvent call.
    const fs = await import('fs');
    const path = await import('path');
    const controller = fs.readFileSync(
      path.resolve(__dirname, '../bridge/zoom/zoom_controller.ts'),
      'utf8',
    );

    // Must NOT start the RAF loop at bind time — the loop self-stops after
    // settle, so a bind-time start would die before the first wheel event.
    // Restarting is Rust's job: onWheelEvent calls ensure_raf_loop_after_wheel.
    const hasBindTimeStart = /deps\.startZoomRafLoop\(\)/.test(controller);
    expect(hasBindTimeStart).toBe(false);

    // Must call onWheelEvent for wheel events
    const hasOnWheelEvent = /deps\.onWheelEvent\(input\)/.test(controller);
    expect(hasOnWheelEvent).toBe(true);

    // Must NOT have the old tick loop (startSmoothZoomPreview)
    const hasOldTickLoop = /startSmoothZoomPreview/.test(controller);
    expect(hasOldTickLoop).toBe(false);
  });

  it('commitRenderedFrame delegates to Rust queue', async () => {
    // The new architecture pushes committed frames to the Rust queue.
    const fs = await import('fs');
    const path = await import('path');
    const controller = fs.readFileSync(
      path.resolve(__dirname, '../bridge/zoom/zoom_controller.ts'),
      'utf8',
    );

    // Must call commitRenderedFrameToQueue
    const hasQueue = /deps\.commitRenderedFrameToQueue\(frame\)/.test(controller);
    expect(hasQueue).toBe(true);

    // Must NOT have the old complex applyCommittedFrame logic
    const hasOldApply = /function applyCommittedFrame/.test(controller);
    expect(hasOldApply).toBe(false);
  });

  it('RAF loop runs in Rust via web-sys', async () => {
    // The RAF loop is implemented in Rust using requestAnimationFrame + web-sys DOM ops.
    // DOM ops were extracted to raf_transform.rs and raf_committed.rs.
    const fs = await import('fs');
    const path = await import('path');
    const rafLoop = fs.readFileSync(
      path.resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_loop.rs'),
      'utf8',
    );
    const rafTransform = fs.readFileSync(
      path.resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_transform.rs'),
      'utf8',
    );
    const rafCommitted = fs.readFileSync(
      path.resolve(__dirname, '../../crates/pdf-viewer-ui/src/zoom/raf_committed.rs'),
      'utf8',
    );
    const all = rafLoop + rafTransform + rafCommitted;

    // Must use requestAnimationFrame
    const hasRaf = /request_animation_frame/.test(rafLoop);
    expect(hasRaf).toBe(true);

    // Must use web-sys for CSS transform (in raf_transform or raf_committed)
    const hasCssTransform = /set_property.*transform/.test(all);
    expect(hasCssTransform).toBe(true);

    // Must use web-sys for scroll (in raf_committed)
    const hasScroll = /set_scroll_left/.test(all);
    expect(hasScroll).toBe(true);

    // Wheel path must guarantee the loop is running (loop self-stops after settle)
    const hasWheelRestart = /fn ensure_raf_loop_after_wheel/.test(rafLoop);
    expect(hasWheelRestart).toBe(true);
  });

  it('vector canvas container uses overflow:visible so cssScale preview is not clipped', async () => {
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
    const fs = await import('fs');
    const path = await import('path');
    const canvasHost = fs.readFileSync(
      path.resolve(__dirname, '../bridge/render/vector_canvas_host.ts'),
      'utf8',
    );

    const setsVisible = /container\.style\.visibility\s*=\s*['"]visible['"]/.test(canvasHost);
    expect(setsVisible).toBe(true);
  });

  it('applyViewportCanvasFrame skips visible-canvas re-box when deferVisibleFrame is set', async () => {
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

    const guardedBox = /if \(!deferVisibleFrame\) \{[\s\S]*?applyCanvasCssBox\(refs\.mainCanvas[\s\S]*?applyCanvasCssBox\(refs\.backCanvas[\s\S]*?\n    \}/.test(fn);
    expect(guardedBox).toBe(true);

    const outside = fn.replace(/if \(!deferVisibleFrame\) \{[\s\S]*?\n    \}/, '');
    expect(/applyCanvasCssBox\(refs\.mainCanvas/.test(outside)).toBe(false);
    expect(/applyCanvasCssBox\(refs\.backCanvas/.test(outside)).toBe(false);

    expect(/ensureCanvasBitmap\(refs\.mainStageCanvas/.test(fn)).toBe(true);
  });
});
