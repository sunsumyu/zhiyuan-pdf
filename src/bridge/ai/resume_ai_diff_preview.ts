type DiffToken = {
  text: string;
  type: 'unchanged' | 'removed' | 'added';
};

function tokenizeForDiff(value: string): string[] {
  return value.match(/[\u4e00-\u9fff]|[A-Za-z0-9_+#./%-]+|\s+|[^\s]/g) || [];
}

function buildDiffTokens(originalText: string, nextText: string): DiffToken[] {
  const originalTokens = tokenizeForDiff(originalText);
  const nextTokens = tokenizeForDiff(nextText);
  const rowCount = originalTokens.length + 1;
  const colCount = nextTokens.length + 1;
  const table: number[][] = Array.from({ length: rowCount }, () => Array<number>(colCount).fill(0));

  for (let row = 1; row < rowCount; row += 1) {
    for (let col = 1; col < colCount; col += 1) {
      if (originalTokens[row - 1] === nextTokens[col - 1]) {
        table[row][col] = table[row - 1][col - 1] + 1;
      } else {
        table[row][col] = Math.max(table[row - 1][col], table[row][col - 1]);
      }
    }
  }

  const result: DiffToken[] = [];
  let row = originalTokens.length;
  let col = nextTokens.length;
  while (row > 0 && col > 0) {
    if (originalTokens[row - 1] === nextTokens[col - 1]) {
      result.push({ text: nextTokens[col - 1], type: 'unchanged' });
      row -= 1;
      col -= 1;
    } else if (table[row - 1][col] >= table[row][col - 1]) {
      result.push({ text: originalTokens[row - 1], type: 'removed' });
      row -= 1;
    } else {
      result.push({ text: nextTokens[col - 1], type: 'added' });
      col -= 1;
    }
  }
  while (row > 0) {
    result.push({ text: originalTokens[row - 1], type: 'removed' });
    row -= 1;
  }
  while (col > 0) {
    result.push({ text: nextTokens[col - 1], type: 'added' });
    col -= 1;
  }

  return result.reverse();
}

export function countDiffStats(originalText: string, nextText: string): { added: number; removed: number } {
  return buildDiffTokens(originalText, nextText).reduce((stats, token) => {
    if (token.type === 'added' && token.text.trim()) {
      stats.added += 1;
    }
    if (token.type === 'removed' && token.text.trim()) {
      stats.removed += 1;
    }
    return stats;
  }, { added: 0, removed: 0 });
}

export function createDiffPreview(originalText: string, nextText: string): HTMLDivElement {
  const container = document.createElement('div');
  container.className = 'pdf-ai-diff-preview';

  buildDiffTokens(originalText, nextText).forEach((token) => {
    const span = document.createElement('span');
    span.className = `pdf-ai-diff-token pdf-ai-diff-token-${token.type}`;
    span.textContent = token.text;
    container.appendChild(span);
  });

  return container;
}
