import { countDiffStats, createDiffPreview } from './resume_ai_diff_preview';
import type { ResumeAiSuggestion, ResumeChatTurn } from './resume_ai_types';

type ApplySuggestionSource = 'button-pointerdown' | 'button-click';

type RenderResumeAiConversationArgs = {
  container: HTMLElement;
  turns: ResumeChatTurn[];
  suggestions: ResumeAiSuggestion[];
  isBusy: boolean;
  onApplySuggestion: (suggestionId: string, source: ApplySuggestionSource) => void;
  onApplyPointerDownLog: (suggestionId: string) => void;
};

type SyncResumeAiSummaryArgs = {
  summary: HTMLElement;
  applyAllButton: HTMLButtonElement | null;
  saveAsButton: HTMLButtonElement | null;
  clearButton: HTMLButtonElement | null;
  suggestions: ResumeAiSuggestion[];
  isBusy: boolean;
};

export function renderResumeAiConversation(args: RenderResumeAiConversationArgs): void {
  const { container, turns, suggestions } = args;
  container.innerHTML = '';

  if (turns.length === 0) {
    container.appendChild(
      createMessageBubble(
        'assistant',
        '打开 PDF 后，直接用自然语言告诉我你想怎么改。你可以先改当前页，也可以切到整份简历模式统一改写。',
      ),
    );
  } else {
    turns.forEach((turn) => {
      container.appendChild(createMessageBubble(turn.role, turn.text));
    });
  }

  if (suggestions.length > 0) {
    const threadSection = document.createElement('section');
    threadSection.className = 'pdf-ai-thread-suggestions';

    const summary = document.createElement('div');
    summary.className = 'pdf-ai-thread-suggestions-summary';
    const counts = countSuggestionStates(suggestions);
    summary.textContent = `当前建议 ${suggestions.length} 条 · 待应用 ${counts.pendingCount} · 已应用 ${counts.appliedCount}${counts.failedCount > 0 ? ` · 失败 ${counts.failedCount}` : ''}`;
    threadSection.appendChild(summary);

    appendSuggestionGroups(threadSection, suggestions, args);
    container.appendChild(threadSection);
  }

  container.scrollTop = container.scrollHeight;
}

export function syncResumeAiSuggestionSummary(args: SyncResumeAiSummaryArgs): void {
  const { summary, applyAllButton, saveAsButton, clearButton, suggestions, isBusy } = args;
  const counts = countSuggestionStates(suggestions);

  if (applyAllButton) {
    applyAllButton.disabled = counts.pendingCount === 0 || isBusy;
  }
  if (saveAsButton) {
    saveAsButton.disabled = counts.pendingCount === 0 || isBusy;
  }
  if (clearButton) {
    clearButton.disabled = suggestions.length === 0 || isBusy;
  }

  if (suggestions.length === 0) {
    summary.textContent = '暂无建议';
    return;
  }

  summary.textContent = `已整合到上方对话：共 ${suggestions.length} 条，待应用 ${counts.pendingCount} 条，已应用 ${counts.appliedCount} 条${counts.failedCount > 0 ? `，失败${counts.failedCount} 条` : ''}`;
}

function appendSuggestionGroups(
  container: HTMLElement,
  suggestions: ResumeAiSuggestion[],
  args: RenderResumeAiConversationArgs,
): void {
  const groupedByPage = new Map<number, ResumeAiSuggestion[]>();
  suggestions.forEach((suggestion) => {
    const current = groupedByPage.get(suggestion.pageIndex) || [];
    current.push(suggestion);
    groupedByPage.set(suggestion.pageIndex, current);
  });

  Array.from(groupedByPage.entries())
    .sort((left, right) => left[0] - right[0])
    .forEach(([pageIndex, pageSuggestions]) => {
      const pageSection = document.createElement('section');
      pageSection.className = 'pdf-ai-page-group';

      const pageHeader = document.createElement('div');
      pageHeader.className = 'pdf-ai-page-group-header';
      pageHeader.textContent = `第${pageIndex + 1} 页 ${pageSuggestions.length} 条建议`;
      pageSection.appendChild(pageHeader);

      pageSuggestions.forEach((suggestion) => {
        pageSection.appendChild(createSuggestionCard(suggestion, args));
      });

      container.appendChild(pageSection);
    });
}

function createSuggestionCard(
  suggestion: ResumeAiSuggestion,
  args: RenderResumeAiConversationArgs,
): HTMLDivElement {
  const card = document.createElement('div');
  card.className = 'pdf-ai-suggestion-card';

  const header = document.createElement('div');
  header.className = 'pdf-ai-suggestion-header';

  const title = document.createElement('div');
  title.className = 'pdf-ai-suggestion-title';
  title.textContent = suggestion.summary;

  const badge = document.createElement('span');
  badge.className = `pdf-ai-suggestion-badge pdf-ai-suggestion-badge-${suggestion.state}`;
  badge.textContent = suggestion.state === 'pending' ? `${Math.round(suggestion.confidence * 100)}%`
    : suggestion.state === 'applied' ? '已应用'
      : '失败';

  const actionButton = document.createElement('button');
  actionButton.className = 'pdf-ai-suggestion-action pdf-ai-suggestion-primary-action';
  actionButton.type = 'button';
  actionButton.dataset.aiAction = 'apply-suggestion';
  actionButton.dataset.suggestionId = suggestion.id;
  actionButton.disabled = suggestion.state === 'applied' || args.isBusy;
  actionButton.textContent = suggestion.state === 'applied' ? '已应' : '应用这条';
  actionButton.onpointerdown = (event) => {
    event.preventDefault();
    event.stopPropagation();
    args.onApplyPointerDownLog(suggestion.id);
    args.onApplySuggestion(suggestion.id, 'button-pointerdown');
  };
  actionButton.onclick = (event) => {
    event.preventDefault();
    event.stopPropagation();
    if (actionButton.disabled) {
      return;
    }
    args.onApplySuggestion(suggestion.id, 'button-click');
  };

  const headerActions = document.createElement('div');
  headerActions.style.display = 'flex';
  headerActions.style.alignItems = 'center';
  headerActions.style.gap = '8px';
  headerActions.appendChild(badge);
  headerActions.appendChild(actionButton);

  header.appendChild(title);
  header.appendChild(headerActions);

  const originalLabel = document.createElement('div');
  originalLabel.className = 'pdf-ai-suggestion-label';
  originalLabel.textContent = '原文';

  const originalText = document.createElement('pre');
  originalText.className = 'pdf-ai-suggestion-block';
  originalText.textContent = suggestion.originalText;

  const nextLabel = document.createElement('div');
  nextLabel.className = 'pdf-ai-suggestion-label';
  nextLabel.textContent = '建议改写';

  const nextText = document.createElement('pre');
  nextText.className = 'pdf-ai-suggestion-block pdf-ai-suggestion-block-next';
  nextText.textContent = suggestion.suggestedText;

  const diffLabel = document.createElement('div');
  diffLabel.className = 'pdf-ai-suggestion-label';
  const diffStats = countDiffStats(suggestion.originalText, suggestion.suggestedText);
  diffLabel.textContent = `差异预览  +${diffStats.added} / -${diffStats.removed}`;

  const diffPreview = createDiffPreview(suggestion.originalText, suggestion.suggestedText);

  const footer = document.createElement('div');
  footer.className = 'pdf-ai-suggestion-footer';

  const pageMeta = document.createElement('span');
  pageMeta.className = 'pdf-ai-suggestion-meta';
  pageMeta.textContent = `第${suggestion.pageIndex + 1} 页 ${suggestion.kind === 'list-item-region' ? '项目 bullet' : '段落'}`;

  footer.appendChild(pageMeta);

  card.appendChild(header);
  card.appendChild(originalLabel);
  card.appendChild(originalText);
  card.appendChild(nextLabel);
  card.appendChild(nextText);
  card.appendChild(diffLabel);
  card.appendChild(diffPreview);
  if (suggestion.errorMessage) {
    const errorText = document.createElement('div');
    errorText.className = 'pdf-ai-suggestion-error';
    errorText.textContent = suggestion.errorMessage;
    card.appendChild(errorText);
  }
  card.appendChild(footer);
  return card;
}

function countSuggestionStates(suggestions: ResumeAiSuggestion[]): {
  pendingCount: number;
  appliedCount: number;
  failedCount: number;
} {
  return suggestions.reduce((counts, suggestion) => {
    if (suggestion.state === 'pending') {
      counts.pendingCount += 1;
    } else if (suggestion.state === 'applied') {
      counts.appliedCount += 1;
    } else if (suggestion.state === 'failed') {
      counts.failedCount += 1;
    }
    return counts;
  }, { pendingCount: 0, appliedCount: 0, failedCount: 0 });
}

function createMessageBubble(role: ResumeChatTurn['role'], text: string): HTMLDivElement {
  const bubble = document.createElement('div');
  bubble.className = role === 'user' ? 'pdf-ai-message-user' : 'pdf-ai-message-assistant';
  bubble.textContent = text;
  return bubble;
}
