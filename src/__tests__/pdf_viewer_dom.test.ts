import { describe, it, expect, vi } from 'vitest';

// pdf_viewer_dom imports VECTOR_CONTAINER_ID from vector_host, which pulls in
// the wasm loader chain. Stub that dependency so the pure zoom math can be
// exercised in a node environment.
vi.mock('../bridge/render/vector_host', () => ({
  VECTOR_CONTAINER_ID: 'pdf-vector-container',
}));

import {
  clampZoom,
  getDynamicMaxZoom,
  MIN_ZOOM,
  MAX_ZOOM,
  DEFAULT_PAGE_WIDTH,
  DEFAULT_PAGE_HEIGHT,
} from '../bridge/viewer/pdf_viewer_dom';

describe('zoom math (pdf_viewer_dom)', () => {
  describe('clampZoom', () => {
    it('passes through in-range values untouched', () => {
      expect(clampZoom(0.1)).toBe(0.1);
      expect(clampZoom(1)).toBe(1);
      expect(clampZoom(12.5)).toBe(12.5);
      expect(clampZoom(30)).toBe(30);
    });

    it('clamps values below MIN_ZOOM up to MIN_ZOOM', () => {
      expect(clampZoom(0.01)).toBe(MIN_ZOOM);
      expect(clampZoom(-5)).toBe(MIN_ZOOM);
    });

    it('clamps values above MAX_ZOOM down to MAX_ZOOM', () => {
      expect(clampZoom(30.0001)).toBe(MAX_ZOOM);
      expect(clampZoom(1000)).toBe(MAX_ZOOM);
    });

    it('returns the neutral zoom 1.0 for any non-finite input', () => {
      expect(clampZoom(NaN)).toBe(1.0);
      expect(clampZoom(Infinity)).toBe(1.0);
      expect(clampZoom(-Infinity)).toBe(1.0);
    });
  });

  describe('getDynamicMaxZoom', () => {
    it('currently equals the static MAX_ZOOM', () => {
      expect(getDynamicMaxZoom()).toBe(MAX_ZOOM);
      expect(getDynamicMaxZoom()).toBe(30.0);
    });
  });

  describe('constants', () => {
    it('keeps a sane zoom range around the neutral zoom', () => {
      expect(MIN_ZOOM).toBeLessThan(1);
      expect(MAX_ZOOM).toBeGreaterThan(1);
    });

    it('defaults to A4 portrait page dimensions', () => {
      expect(DEFAULT_PAGE_WIDTH).toBe(595);
      expect(DEFAULT_PAGE_HEIGHT).toBe(842);
    });
  });
});
