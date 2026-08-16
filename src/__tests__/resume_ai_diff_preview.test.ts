import { describe, it, expect } from 'vitest';

// countDiffStats is the only function we need; createDiffPreview touches the DOM
// (document.createElement) so we can't test it in node. We exercise the pure
// diff-tokenizer indirectly via countDiffStats.

import { countDiffStats } from '../bridge/ai/resume_ai_diff_preview';

describe('diff preview (resume_ai_diff_preview)', () => {
  describe('countDiffStats', () => {
    it('reports zero added/removed for identical strings', () => {
      expect(countDiffStats('hello', 'hello')).toEqual({ added: 0, removed: 0 });
    });

    it('reports pure addition (empty original)', () => {
      // tokenizeForDiff groups consecutive ASCII alphanum into one token
      expect(countDiffStats('', 'abc')).toEqual({ added: 1, removed: 0 });
    });

    it('reports pure deletion (empty next)', () => {
      expect(countDiffStats('abc', '')).toEqual({ added: 0, removed: 1 });
    });

    it('counts single-token substitution as one removal + one addition', () => {
      expect(countDiffStats('abc', 'adc')).toEqual({ added: 1, removed: 1 });
    });

    it('treats CJK characters as atomic tokens (one per character)', () => {
      expect(countDiffStats('你好世界', '你好地球')).toEqual({ added: 2, removed: 2 });
    });

    it('ignores whitespace-only tokens in the count', () => {
      expect(countDiffStats('a b', 'a b')).toEqual({ added: 0, removed: 0 });
      // insertion of only spaces does not inflate added count
      expect(countDiffStats('a', 'a   ')).toEqual({ added: 0, removed: 0 });
    });

    it('handles both empty strings', () => {
      expect(countDiffStats('', '')).toEqual({ added: 0, removed: 0 });
    });

    it('handles a full replacement (all ASCII is one token)', () => {
      expect(countDiffStats('ABCD', 'EFGH')).toEqual({ added: 1, removed: 1 });
    });
  });
});
