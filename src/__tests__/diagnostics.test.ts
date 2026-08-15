import { describe, it, expect, vi, beforeEach } from 'vitest';
import { formatPdfDiagnostic, verbosePdfDiagnosticsEnabled } from '../bridge/shared/diagnostics';

describe('diagnostics', () => {
  describe('formatPdfDiagnostic', () => {
    it('should format basic diagnostic message', () => {
      const result = formatPdfDiagnostic('render', 'frame_complete');
      expect(result).toMatch(/\d{2}:\d{2}:\d{2}\.\d{3}/); // timestamp
      expect(result).toContain('INFO');
      expect(result).toContain('RENDER');
      expect(result).toContain('frame_complete');
    });

    it('should include fields in the message', () => {
      const result = formatPdfDiagnostic('cache', 'hit', { page: 1, zoom: 1.5 });
      expect(result).toContain('page=1');
      expect(result).toContain('zoom=1.5');
    });

    it('should normalize layer names', () => {
      expect(formatPdfDiagnostic('prof', 'start')).toContain('PERF');
      expect(formatPdfDiagnostic('cache', 'miss')).toContain('CACHE');
      expect(formatPdfDiagnostic('layout', 'compute')).toContain('LAYOUT');
      expect(formatPdfDiagnostic('render-flow', 'start')).toContain('RENDER');
    });

    it('should infer error level from event name', () => {
      const result = formatPdfDiagnostic('render', 'decode_failed');
      expect(result).toContain('ERROR');
    });

    it('should infer warn level from event name', () => {
      const result = formatPdfDiagnostic('render', 'rejected');
      expect(result).toContain('WARN');
    });
  });

  describe('verbosePdfDiagnosticsEnabled', () => {
    it('should return false by default', () => {
      // Mock window object
      const mockWindow = {} as any;
      vi.stubGlobal('window', mockWindow);
      
      expect(verbosePdfDiagnosticsEnabled()).toBe(false);
      
      vi.unstubAllGlobals();
    });

    it('should return true when enabled', () => {
      // Mock window object with verbose enabled
      const mockWindow = { __PDF_DIAGNOSTICS_VERBOSE: true } as any;
      vi.stubGlobal('window', mockWindow);
      
      expect(verbosePdfDiagnosticsEnabled()).toBe(true);
      
      vi.unstubAllGlobals();
    });
  });
});
