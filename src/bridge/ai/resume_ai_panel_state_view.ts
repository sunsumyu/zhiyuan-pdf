import type { ResumeAiSuggestion } from './resume_ai_types';

type StatusTone = 'idle' | 'working' | 'success' | 'error';

type BusyStateArgs = {
  busy: boolean;
  suggestions: ResumeAiSuggestion[];
};

export function applyResumeAiBusyState(args: BusyStateArgs): void {
  const sendButton = getElement<HTMLButtonElement>('pdf-ai-send-btn');
  const applyAllButton = getElement<HTMLButtonElement>('pdf-ai-apply-all-btn');
  const saveAsButton = getElement<HTMLButtonElement>('pdf-ai-save-as-btn');
  const input = getElement<HTMLTextAreaElement>('pdf-ai-chat-input');
  const pendingCount = args.suggestions.filter((item) => item.state === 'pending').length;

  if (sendButton) {
    sendButton.disabled = args.busy;
    sendButton.textContent = args.busy ? '处理中' : '发送';
  }
  if (applyAllButton) {
    applyAllButton.disabled = args.busy || pendingCount === 0;
  }
  if (saveAsButton) {
    saveAsButton.disabled = args.busy || pendingCount === 0;
  }
  if (input) {
    input.disabled = args.busy;
  }
}

export function applyResumeAiWideMode(isWide: boolean): void {
  const panel = getElement<HTMLElement>('pdf-ai-panel');
  const wideButton = getElement<HTMLButtonElement>('pdf-ai-wide-btn');
  if (panel) {
    if (isWide) {
      panel.style.flexBasis = 'min(860px, 76vw)';
      panel.style.width = 'min(860px, 76vw)';
    } else {
      panel.style.flexBasis = 'clamp(460px, 48vw, 620px)';
      panel.style.width = 'clamp(460px, 48vw, 620px)';
    }
  }
  if (wideButton) {
    wideButton.style.background = isWide ? '#89b4fa' : 'transparent';
    wideButton.style.color = isWide ? '#11111b' : '#cdd6f4';
  }
}

export function applyResumeAiPanelOpen(isOpen: boolean): void {
  const panel = getElement<HTMLElement>('pdf-ai-panel');
  const toggleButton = getElement<HTMLButtonElement>('pdf-ai-toggle-btn');
  if (panel) {
    panel.style.display = isOpen ? 'flex' : 'none';
  }
  if (toggleButton) {
    toggleButton.setAttribute('aria-pressed', isOpen ? 'true' : 'false');
    toggleButton.style.background = isOpen ? '#89b4fa' : '#313244';
    toggleButton.style.color = isOpen ? '#11111b' : '#cdd6f4';
  }
}

export function setResumeAiStatus(text: string, tone: StatusTone): void {
  const status = getElement<HTMLElement>('pdf-ai-status');
  if (!status) {
    return;
  }

  if (!text.trim()) {
    status.textContent = '';
    status.style.display = 'none';
    status.style.color = '#a6adc8';
    status.style.borderColor = '#45475a';
    return;
  }

  status.style.display = 'inline-flex';
  status.textContent = text;
  if (tone === 'working') {
    status.style.color = '#f9e2af';
    status.style.borderColor = '#f9e2af';
  } else if (tone === 'success') {
    status.style.color = '#a6e3a1';
    status.style.borderColor = '#a6e3a1';
  } else if (tone === 'error') {
    status.style.color = '#f38ba8';
    status.style.borderColor = '#f38ba8';
  } else {
    status.style.color = '#a6adc8';
    status.style.borderColor = '#45475a';
  }
}

export function syncResumeAiApiKeySection(isEditingApiKey: boolean): void {
  const apiKeyInput = getElement<HTMLInputElement>('pdf-ai-api-key');
  const keyEditor = getElement<HTMLElement>('pdf-ai-key-editor');
  const keyCompactActions = getElement<HTMLElement>('pdf-ai-key-compact-actions');
  const keySummary = getElement<HTMLElement>('pdf-ai-key-summary-inline');
  const cancelKeyEditButton = getElement<HTMLButtonElement>('pdf-ai-cancel-key-edit');
  if (!apiKeyInput || !keyEditor || !keyCompactActions || !keySummary || !cancelKeyEditButton) {
    return;
  }

  const apiKey = apiKeyInput.value.trim();
  const hasKey = !!apiKey;
  const showEditor = isEditingApiKey || !hasKey;

  keyEditor.style.display = showEditor ? 'flex' : 'none';
  keyCompactActions.style.display = showEditor ? 'none' : 'inline-flex';
  cancelKeyEditButton.style.display = hasKey ? 'inline-flex' : 'none';
  keySummary.textContent = hasKey ? '已配置' : '未配置 Gemini API Key';
}

function getElement<T extends HTMLElement>(id: string): T | null {
  return document.getElementById(id) as T | null;
}
