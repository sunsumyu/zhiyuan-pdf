import { describe, it, expect, vi, beforeEach } from 'vitest';

// vector_frame_cache touches the DOM via layout_trace and clones canvases via
// CanvasPool. Stub both so the pure Map-based cache semantics can be tested.
vi.mock('../bridge/render/layout_trace', () => ({
  logPdfLayoutTrace: () => {},
}));

vi.mock('../bridge/render/vector_canvas_pool', () => ({
  CanvasPool: {
    rent: vi.fn(
      (width: number, height: number): unknown =>
        ({ width, height, getContext: () => ({ drawImage: () => {} }) }),
    ),
    recycle: vi.fn(),
  },
}));

import { CanvasPool } from '../bridge/render/vector_canvas_pool';
import {
  clearVectorFrameCache,
  readViewportFrameCache,
  writeViewportFrameCache,
  deleteViewportFrameCacheKeys,
} from '../bridge/render/vector_frame_cache';

function fakeSourceCanvas(width: number, height: number): HTMLCanvasElement {
  return { width, height } as HTMLCanvasElement;
}

beforeEach(() => {
  clearVectorFrameCache();
  vi.clearAllMocks();
});

describe('vector frame cache (vector_frame_cache)', () => {
  it('misses (returns null) on an empty cache', () => {
    expect(readViewportFrameCache('doc.pdf:0:1.0')).toBeNull();
  });

  it('writes a pool-rented clone, not the source canvas', () => {
    const source = fakeSourceCanvas(320, 200);
    writeViewportFrameCache('doc.pdf:0:1.0', source);

    const cached = readViewportFrameCache('doc.pdf:0:1.0');
    expect(cached).not.toBeNull();
    expect(cached).not.toBe(source);
    expect(cached!.width).toBe(320);
    expect(cached!.height).toBe(200);
    expect(CanvasPool.rent).toHaveBeenCalledWith(320, 200);
  });

  it('keys are exact: different zoom is a different entry', () => {
    writeViewportFrameCache('doc.pdf:0:1.0', fakeSourceCanvas(10, 10));
    writeViewportFrameCache('doc.pdf:0:1.5', fakeSourceCanvas(10, 10));

    expect(readViewportFrameCache('doc.pdf:0:1.0')).not.toBeNull();
    expect(readViewportFrameCache('doc.pdf:0:1.5')).not.toBeNull();
    expect(readViewportFrameCache('doc.pdf:0:2.0')).toBeNull();
  });

  it('overwriting the same key replaces the entry (no duplicate growth)', () => {
    writeViewportFrameCache('k', fakeSourceCanvas(1, 1));
    const first = readViewportFrameCache('k');
    writeViewportFrameCache('k', fakeSourceCanvas(2, 2));
    const second = readViewportFrameCache('k');

    expect(second).not.toBe(first);
    expect(second!.width).toBe(2);
  });

  it('deleteViewportFrameCacheKeys removes listed keys and recycles their canvases', () => {
    writeViewportFrameCache('a', fakeSourceCanvas(1, 1));
    writeViewportFrameCache('b', fakeSourceCanvas(1, 1));
    deleteViewportFrameCacheKeys(['a', 'b']);

    expect(readViewportFrameCache('a')).toBeNull();
    expect(readViewportFrameCache('b')).toBeNull();
    expect(vi.mocked(CanvasPool.recycle)).toHaveBeenCalledTimes(2);
  });

  it('skips empty-string keys and leaves other entries alone', () => {
    writeViewportFrameCache('keep', fakeSourceCanvas(1, 1));
    deleteViewportFrameCacheKeys(['', 'missing']);

    expect(readViewportFrameCache('keep')).not.toBeNull();
    expect(vi.mocked(CanvasPool.recycle)).not.toHaveBeenCalled();
  });

  it('clearVectorFrameCache recycles every entry and empties the cache', () => {
    writeViewportFrameCache('a', fakeSourceCanvas(1, 1));
    writeViewportFrameCache('b', fakeSourceCanvas(1, 1));
    clearVectorFrameCache();

    expect(readViewportFrameCache('a')).toBeNull();
    expect(readViewportFrameCache('b')).toBeNull();
    expect(vi.mocked(CanvasPool.recycle)).toHaveBeenCalledTimes(2);
  });
});
