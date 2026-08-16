import { describe, it, expect } from 'vitest';

describe('zoom commit frame anti-flash', () => {
  it('applyCommittedFrame should NOT clear transform before syncLayoutBox', async () => {
    // The flash is caused by clearing container.style.transform = ''
    // BEFORE syncLayoutBox re-applies it. This creates a single frame
    // where the container has no CSS transform, causing a visual jump.
    //
    // The fix: remove the explicit transform clear; syncLayoutBox sets
    // the transform directly via the cssScale computation.
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

    // The old code had: container.style.transform = '';
    // followed by deps.syncLayoutBox(...)
    // This pattern causes a flash because the transform is cleared
    // before being re-applied.
    const hasTransformClear = /container\.style\.transform\s*=\s*['"]{2}/.test(fn);
    expect(hasTransformClear).toBe(false);
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
});
