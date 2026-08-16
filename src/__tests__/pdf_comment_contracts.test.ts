import { describe, it, expect } from 'vitest';

import {
  EMPTY_REVIEW_RESULT,
  EMPTY_REVIEW_PANEL,
  EMPTY_OVERLAY_DISPLAY,
  EMPTY_TARGET_OVERLAY_DISPLAY,
  normalizeReviewSession,
  normalizeOverlayDisplay,
  normalizeTargetOverlayDisplay,
  normalizeReviewDisplay,
  type CommentReviewSession,
  type PdfCommentOverlayDisplay,
  type PdfCommentTargetOverlayDisplay,
  type PdfCommentReviewResult,
} from '../bridge/comment/pdf_comment_contracts';

describe('comment contracts (pdf_comment_contracts)', () => {
  describe('normalizeReviewSession', () => {
    it('returns full defaults for null/undefined', () => {
      for (const raw of [null, undefined]) {
        expect(normalizeReviewSession(raw as CommentReviewSession | null | undefined)).toEqual({
          panelOpen: false,
          scope: 'page',
          query: '',
          selectedCommentId: null,
        });
      }
    });

    it('preserves valid session fields', () => {
      const raw: CommentReviewSession = {
        panelOpen: true,
        scope: 'document',
        query: '发票',
        selectedCommentId: 'c-12',
      };
      expect(normalizeReviewSession(raw)).toEqual(raw);
    });

    it('coerces an invalid scope back to page scope', () => {
      const raw = { panelOpen: true, scope: 'galaxy', query: '', selectedCommentId: null };
      expect(normalizeReviewSession(raw as unknown as CommentReviewSession).scope).toBe('page');
    });

    it('defaults missing optional fields', () => {
      const out = normalizeReviewSession({ panelOpen: false, scope: 'page', query: 'x' });
      expect(out.selectedCommentId).toBeNull();
    });
  });

  describe('normalizeOverlayDisplay / normalizeTargetOverlayDisplay', () => {
    it('fall back to the shared EMPTY constants for null/undefined', () => {
      expect(normalizeOverlayDisplay(null)).toBe(EMPTY_OVERLAY_DISPLAY);
      expect(normalizeOverlayDisplay(undefined)).toBe(EMPTY_OVERLAY_DISPLAY);
      expect(normalizeTargetOverlayDisplay(null)).toBe(EMPTY_TARGET_OVERLAY_DISPLAY);
      expect(normalizeTargetOverlayDisplay(undefined)).toBe(EMPTY_TARGET_OVERLAY_DISPLAY);
    });

    it('pass a populated display through by reference', () => {
      const overlay: PdfCommentOverlayDisplay = {
        comments: [{ id: 'm1', title: '备注', frame: { leftPercent: 0, topPercent: 0, widthPercent: 10, heightPercent: 10 }, selected: false }],
      };
      expect(normalizeOverlayDisplay(overlay)).toBe(overlay);

      const targets: PdfCommentTargetOverlayDisplay = {
        targets: [{ id: 't1', kind: 'paragraph', pageIndex: 0, label: 'P1', title: '段落', frame: { leftPercent: 0, topPercent: 0, widthPercent: 10, heightPercent: 10 } }],
      };
      expect(normalizeTargetOverlayDisplay(targets)).toBe(targets);
    });
  });

  describe('normalizeReviewDisplay', () => {
    it('fills every section with defaults for null/undefined', () => {
      for (const raw of [null, undefined]) {
        const out = normalizeReviewDisplay(raw);
        expect(out.session).toEqual({ panelOpen: false, scope: 'page', query: '', selectedCommentId: null });
        expect(out.review).toBe(EMPTY_REVIEW_RESULT);
        expect(out.panel).toBe(EMPTY_REVIEW_PANEL);
        expect(out.overlay).toBe(EMPTY_OVERLAY_DISPLAY);
      }
    });

    it('normalizes sections independently and keeps provided payloads', () => {
      const review: PdfCommentReviewResult = {
        totalComments: 2,
        filteredComments: 1,
        pagesWithComments: 1,
        summaries: [{ pageIndex: 0, totalComments: 2, filteredComments: 1 }],
        comments: [],
      };
      const out = normalizeReviewDisplay({
        session: { panelOpen: true, scope: 'document', query: '' },
        review,
      });
      expect(out.session.scope).toBe('document');
      expect(out.review).toBe(review);
      expect(out.panel).toBe(EMPTY_REVIEW_PANEL);
      expect(out.overlay).toBe(EMPTY_OVERLAY_DISPLAY);
    });
  });

  describe('EMPTY constants', () => {
    it('are internally consistent zero states', () => {
      expect(EMPTY_REVIEW_RESULT.totalComments).toBe(0);
      expect(EMPTY_REVIEW_RESULT.summaries).toHaveLength(0);
      expect(EMPTY_REVIEW_RESULT.comments).toHaveLength(0);
      expect(EMPTY_REVIEW_PANEL.empty).toBe(true);
      expect(EMPTY_OVERLAY_DISPLAY.comments).toHaveLength(0);
      expect(EMPTY_TARGET_OVERLAY_DISPLAY.targets).toHaveLength(0);
    });
  });
});
