import { save } from '@tauri-apps/plugin-dialog';
import type { ViewerSessionSnapshot } from '../viewer/viewer_session';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import type { DocumentEditApi } from '../document/document_edit_api';
import { renderResumeAiConversation, syncResumeAiSuggestionSummary } from './resume_ai_panel_view';
import {
  applyResumeAiBusyState,
  applyResumeAiPanelOpen,
  applyResumeAiWideMode,
  setResumeAiStatus,
  syncResumeAiApiKeySection,
} from './resume_ai_panel_state_view';
import {
  clearResumeAiSuggestions,
  markResumeAiSuggestionFailed,
  submitResumeAiPrompt,
  syncResumeAiSession,
} from './resume_ai_client';
import type { ResumeAiScope, ResumeAiSuggestion, ResumeAiThreadView, ResumeChatTurn } from './resume_ai_types';
import { loadAiSettings, saveAiSettings } from '../../../utils/ai-settings';
import {
  applySingleSuggestion,
  applyAllPendingSuggestions,
  saveAsSeparatePdf,
  type ApplyContext,
} from './resume_ai_apply';

type StatusTone = 'idle' | 'working' | 'success' | 'error';

type ResumeAiControllerDeps = {
  getViewerSession: () => ViewerSessionSnapshot;
  documentEdits: DocumentEditApi;
  openPdfPath: (path: string) => Promise<void>;
  renderCurrentPage: (reason?: 'default' | 'zoom' | 'editorVisibility' | 'documentMutation') => Promise<void>;
  setCurrentPage: (pageIndex: number) => void;
};

export type ResumeAiController = {
  applyAllSuggestions: () => Promise<void>;
  initialize: () => void;
  saveAsAiVersion: () => Promise<void>;
  syncViewerState: () => void;
  togglePanel: () => void;
};

function getElement<T extends HTMLElement>(id: string): T | null {
  return document.getElementById(id) as T | null;
}

function describeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function normalizePath(value: string): string {
  return value.replace(/\//g, '\\').toLowerCase();
}

function buildSuggestedPdfCopyPath(sourcePath: string): string {
  if (/\.pdf$/i.test(sourcePath)) {
    return sourcePath.replace(/\.pdf$/i, '.ai.pdf');
  }
  return `${sourcePath}.ai.pdf`;
}

function buildApplyContext(deps: ResumeAiControllerDeps, logFn: (node: string, payload: Record<string, unknown>) => void): ApplyContext {
  return {
    getViewerSession: deps.getViewerSession,
    documentEdits: deps.documentEdits,
    setCurrentPage: deps.setCurrentPage,
    logAiChain: logFn,
  };
}

function formatScopeLabel(scope: ResumeAiScope): string {
  return scope === 'whole-document' ? '整份简历' : '当前页';
}

class PdfResumeAiController implements ResumeAiController {
  private readonly deps: ResumeAiControllerDeps;

  private initialized = false;

  private initializingRetryTimer: number | null = null;

  private isBusy = false;

  private lastPath: string | null = null;

  private lastPageIndex: number | null = null;

  private scope: ResumeAiScope = 'current-page';

  private suggestions: ResumeAiSuggestion[] = [];

  private turns: ResumeChatTurn[] = [];

  private isWide = false;

  private isEditingApiKey = false;

  private statusResetTimer: number | null = null;

  private aiPanelActionBridgeBound = false;

  private lastApplyTriggerAt = 0;

  constructor(deps: ResumeAiControllerDeps) {
    this.deps = deps;
  }

  initialize(): void {
    const panel = getElement<HTMLElement>('pdf-ai-panel');
    const toggleButton = getElement<HTMLButtonElement>('pdf-ai-toggle-btn');
    const sendButton = getElement<HTMLButtonElement>('pdf-ai-send-btn');
    const input = getElement<HTMLTextAreaElement>('pdf-ai-chat-input');
    const chatMessages = getElement<HTMLElement>('pdf-ai-chat-messages');
    const saveKeyButton = getElement<HTMLButtonElement>('pdf-ai-save-key');
    const editKeyButton = getElement<HTMLButtonElement>('pdf-ai-key-edit-btn');
    const cancelKeyEditButton = getElement<HTMLButtonElement>('pdf-ai-cancel-key-edit');
    const scopeSelect = getElement<HTMLSelectElement>('pdf-ai-scope-select');
    const applyAllButton = getElement<HTMLButtonElement>('pdf-ai-apply-all-btn');
    const saveAsButton = getElement<HTMLButtonElement>('pdf-ai-save-as-btn');
    const clearButton = getElement<HTMLButtonElement>('pdf-ai-clear-btn');
    const closeButton = getElement<HTMLButtonElement>('pdf-ai-close-btn');
    const wideButton = getElement<HTMLButtonElement>('pdf-ai-wide-btn');

    if (!panel || !toggleButton || !sendButton || !input || !chatMessages || !saveKeyButton || !editKeyButton || !cancelKeyEditButton || !scopeSelect || !applyAllButton || !saveAsButton || !clearButton || !closeButton) {
      if (this.initializingRetryTimer === null) {
        this.initializingRetryTimer = window.setTimeout(() => {
          this.initializingRetryTimer = null;
          this.initialize();
        }, 250);
      }
      return;
    }

    if (!this.initialized) {
      toggleButton.onclick = () => this.togglePanel();
      closeButton.onclick = () => this.setPanelOpen(false);
      if (wideButton) {
        wideButton.onclick = () => {
          void this.toggleWideMode();
        };
      }
      saveKeyButton.onclick = () => this.saveApiKey();
      editKeyButton.onclick = () => this.expandApiKeyEditor();
      cancelKeyEditButton.onclick = () => this.cancelApiKeyEditing();
      scopeSelect.onchange = () => {
        this.scope = (scopeSelect.value === 'whole-document' ? 'whole-document' : 'current-page');
        void this.syncRustSession();
      };
      sendButton.onclick = () => {
        void this.handleSendMessage();
      };
      input.onkeydown = (event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
          event.preventDefault();
          void this.handleSendMessage();
        }
      };
      input.oninput = () => {
        input.style.height = 'auto';
        input.style.height = `${Math.min(input.scrollHeight, 240)}px`;
      };
      if (!this.aiPanelActionBridgeBound) {
        const routeActionEvent = (event: Event) => {
          this.handleAiPanelActionEvent(event);
        };
        panel.addEventListener('pointerdown', routeActionEvent, true);
        panel.addEventListener('click', routeActionEvent, true);
        this.aiPanelActionBridgeBound = true;
      }
      applyAllButton.onclick = () => {
        void this.applyAllSuggestions();
      };
      saveAsButton.onclick = () => {
        void this.saveAsAiVersion();
      };
      clearButton.onclick = () => this.clearSuggestions();
      this.initialized = true;
    }

    void this.restoreApiKey();
    scopeSelect.value = this.scope;

    panel.style.display = 'none';
    this.renderMessages();
    this.renderSuggestions();
    this.syncViewerState();
  }

  syncViewerState(): void {
    const meta = getElement<HTMLElement>('pdf-ai-doc-meta');
    const toggleButton = getElement<HTMLButtonElement>('pdf-ai-toggle-btn');
    const session = this.deps.getViewerSession();
    if (meta) {
      if (session.path) {
        meta.textContent = `范围: ${formatScopeLabel(this.scope)} · ? ${session.currentPage + 1} / ${session.pageCount} 页`;
      } else {
        meta.textContent = '当前没有打开 PDF';
      }
    }

    if (toggleButton) {
      toggleButton.disabled = false;
      toggleButton.style.opacity = session.path ? '1' : '0.8';
    }

    if (!session.path) {
      if (this.lastPath !== null) {
        this.lastPath = null;
        this.lastPageIndex = null;
        this.turns = [];
        this.suggestions = [];
        this.renderMessages();
        this.renderSuggestions();
      }
      return;
    }

    if (this.lastPath !== session.path) {
      this.lastPath = session.path;
      this.lastPageIndex = session.currentPage;
      void this.syncRustSession();
      return;
    }

    if (this.lastPageIndex !== session.currentPage) {
      this.lastPageIndex = session.currentPage;
      void this.syncRustSession();
    }
  }

  togglePanel(): void {
    const panel = getElement<HTMLElement>('pdf-ai-panel');
    if (!panel) {
      return;
    }
    const nextOpenState = panel.style.display === 'none' || !panel.style.display;
    this.setPanelOpen(nextOpenState);
  }

  async applyAllSuggestions(): Promise<void> {
    const pendingSuggestions = this.suggestions.filter((item) => item.state === 'pending');
    if (pendingSuggestions.length === 0) {
      this.setStatus('当前没有待应用的建议', 'idle');
      return;
    }
    if (this.isBusy) {
      return;
    }

    const currentSession = this.deps.getViewerSession();
    if (!currentSession.path) {
      this.setStatus('请先打开一份 PDF 简历', 'error');
      return;
    }
    this.setBusy(true, `正在通过编辑器应用 ${pendingSuggestions.length} 条建议...`);
    try {
      const ctx = buildApplyContext(this.deps, (n, p) => this.logAiChain(n, p));
      const { applied, views } = await applyAllPendingSuggestions(ctx, pendingSuggestions);
      for (const view of views) {
        this.applyThreadView(view);
      }
      this.setStatus(`已应用 ${applied} 条建议`, 'success');
      this.scheduleIdleStatusSync();
    } catch (error) {
      this.logAiChain('ts.apply_all.error', { message: describeError(error) });
      await this.syncRustSession();
      this.setStatus('批量应用失败，请查看对话区错误详情', 'error');
      this.scheduleIdleStatusSync();
    } finally {
      this.setBusy(false);
    }
  }

  async saveAsAiVersion(): Promise<void> {
    const pendingSuggestions = this.suggestions.filter((item) => item.state === 'pending');
    const session = this.deps.getViewerSession();
    if (pendingSuggestions.length === 0) {
      this.setStatus('当前没有待另存的 AI 修改', 'idle');
      return;
    }
    if (!session.path || this.isBusy) {
      return;
    }

    const targetPath = await save({
      defaultPath: buildSuggestedPdfCopyPath(session.path),
      filters: [
        {
          name: 'PDF Documents',
          extensions: ['pdf'],
        },
      ],
    });

    if (!targetPath) {
      this.setStatus('已取消另存', 'idle');
      return;
    }

    if (normalizePath(targetPath) === normalizePath(session.path)) {
      this.setStatus('另存路径需要和原 PDF 不同', 'error');
      return;
    }

    this.setBusy(true, '正在生成 AI 改写副本...');
    try {
      const ctx = buildApplyContext(this.deps, (n, p) => this.logAiChain(n, p));
      const result = await saveAsSeparatePdf(ctx, pendingSuggestions, targetPath);

      await this.deps.openPdfPath(result.path);
      this.pushTurn('assistant', '已生成 AI 改写副本，并自动切换到新文件。原始PDF 没有被覆盖');
      this.setStatus('AI 版本已另存并打开', 'success');
      this.scheduleIdleStatusSync();
    } catch (error) {
      const message = describeError(error);
      this.pushTurn('assistant', `另存失败: ${message}`);
      this.setStatus('另存失败，请查看对话区错误详情', 'error');
      this.scheduleIdleStatusSync();
    } finally {
      this.setBusy(false);
    }
  }

  private clearSuggestions(): void {
    const session = this.deps.getViewerSession();
    void clearResumeAiSuggestions({ path: session.path || undefined })
      .then((view) => {
        this.applyThreadView(view);
        this.setStatus(view.notice || '建议列表已清空', 'idle');
      })
      .catch((error) => {
        this.setStatus(`清空失败: ${describeError(error)}`, 'error');
        this.scheduleIdleStatusSync();
      });
  }

  private async applySuggestion(suggestionId: string): Promise<void> {
    const suggestion = this.suggestions.find((item) => item.id === suggestionId);
    if (!suggestion || suggestion.state === 'applied' || this.isBusy) {
      return;
    }

    const currentSession = this.deps.getViewerSession();
    const isCurrentDocument = !!currentSession.path && normalizePath(suggestion.path) === normalizePath(currentSession.path);

    this.setBusy(true, '正在通过编辑器写回...');
    try {
      const ctx = buildApplyContext(this.deps, (n, p) => this.logAiChain(n, p));
      const view = await applySingleSuggestion(ctx, suggestion);
      this.applyThreadView(view);
      if (isCurrentDocument && this.deps.getViewerSession().currentPage !== suggestion.pageIndex) {
        await this.showSuggestionPage(suggestion.pageIndex);
      }
      this.setStatus(view.notice || '修改已通过编辑器写回', 'success');
      this.scheduleIdleStatusSync();
    } catch (error) {
      const message = describeError(error);
      this.logAiChain('ts.apply_one.error', { suggestionId, message });
      try {
        const view = await markResumeAiSuggestionFailed({
          path: currentSession.path || suggestion.path,
          suggestionId,
          errorMessage: message,
        });
        this.applyThreadView(view);
      } catch (secondaryError) {
        this.logAiChain('ts.apply_one.error.secondary', { suggestionId, message: describeError(secondaryError) });
        await this.syncRustSession();
      }
      this.setStatus('应用失败，请查看对话区错误详情', 'error');
      this.scheduleIdleStatusSync();
    } finally {
      this.setBusy(false);
    }
  }

  private async handleApplySuggestionClick(suggestionId: string): Promise<void> {
    this.logAiChain('ts.apply_one.click', { suggestionId });
    await this.applySuggestion(suggestionId);
  }

  private triggerApplySuggestion(suggestionId: string, source: string): void {
    const now = Date.now();
    if (now - this.lastApplyTriggerAt < 200) {
      return;
    }
    this.lastApplyTriggerAt = now;
    this.logAiChain('ts.apply_one.trigger', { suggestionId, source });
    void this.handleApplySuggestionClick(suggestionId);
  }

  private handleAiPanelActionEvent(event: Event): void {
    const target = event.target as HTMLElement | null;
    const actionButton = target?.closest<HTMLButtonElement>('[data-ai-action]');
    if (!actionButton) {
      return;
    }

    const action = actionButton.dataset.aiAction;
    if (action !== 'apply-suggestion') {
      return;
    }

    const suggestionId = actionButton.dataset.suggestionId;
    if (!suggestionId || actionButton.disabled) {
      return;
    }

    if (event.type === 'pointerdown') {
      event.preventDefault();
      event.stopPropagation();
      this.logAiChain('ts.apply_one.pointerdown', { suggestionId });
      this.triggerApplySuggestion(suggestionId, 'panel-pointerdown');
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    this.triggerApplySuggestion(suggestionId, 'panel-click');
  }

  private async handleSendMessage(): Promise<void> {
    if (this.isBusy) {
      return;
    }

    const input = getElement<HTMLTextAreaElement>('pdf-ai-chat-input');
    const apiKeyInput = getElement<HTMLInputElement>('pdf-ai-api-key');
    const session = this.deps.getViewerSession();

    if (!input || !apiKeyInput) {
      return;
    }

    const prompt = input.value.trim();
    if (!prompt) {
      return;
    }

    const apiKey = apiKeyInput.value.trim();
    if (!apiKey) {
      this.expandApiKeyEditor();
      this.setStatus('请输入有效的 Gemini API Key', 'error');
      return;
    }

    if (!session.path) {
      this.setStatus('请先打开一份 PDF 简历', 'error');
      return;
    }

    await saveAiSettings({ geminiApiKey: apiKey });
    input.value = '';
    input.style.height = '84px';
    this.setBusy(true, `正在分析 ${formatScopeLabel(this.scope)} 简历内容...`);

    try {
      const view = await submitResumeAiPrompt({
        apiKey,
        currentPage: session.currentPage,
        path: session.path,
        prompt,
        scope: this.scope,
      });
      this.applyThreadView(view);
      this.setStatus(
        view.notice || (view.suggestions.length > 0
          ? `已生成 ${view.suggestions.length} 条建议`
          : '没有生成可安全应用的建议'),

        view.suggestions.length > 0 ? 'success' : 'idle',
      );
      this.scheduleIdleStatusSync();
    } catch (error) {
      await this.syncRustSession();
      this.setStatus('请求失败，请检查网络或代理', 'error');
      this.scheduleIdleStatusSync();
    } finally {
      this.setBusy(false);
    }
  }

  private pushTurn(role: ResumeChatTurn['role'], text: string): void {
    this.turns = [...this.turns, { role, text }];
    this.renderMessages();
  }

  private applyThreadView(view: ResumeAiThreadView): void {
    this.scope = view.scope;
    this.turns = view.turns || [];
    this.suggestions = view.suggestions || [];
    this.renderMessages();
    this.renderSuggestions();
  }

  private async syncRustSession(): Promise<void> {
    try {
      const session = this.deps.getViewerSession();
      const view = await syncResumeAiSession({
        path: session.path || undefined,
        currentPage: session.path ? session.currentPage : undefined,
        scope: this.scope,
      });
      this.applyThreadView(view);
      if (view.notice) {
        this.setStatus(view.notice, 'idle');
        this.scheduleIdleStatusSync(1200);
      }
    } catch (error) {
      this.setStatus(`同步 AI 会话失败: ${describeError(error)}`, 'error');
      this.scheduleIdleStatusSync();
    }
  }

  private renderMessages(): void {
    const container = getElement<HTMLElement>('pdf-ai-chat-messages');
    if (!container) {
      return;
    }
    renderResumeAiConversation({
      container,
      turns: this.turns,
      suggestions: this.suggestions,
      isBusy: this.isBusy,
      onApplyPointerDownLog: (suggestionId) => {
        this.logAiChain('ts.apply_one.pointerdown.direct', { suggestionId });
      },
      onApplySuggestion: (suggestionId, source) => {
        this.triggerApplySuggestion(suggestionId, source);
      },
    });
  }

  private renderSuggestions(): void {
    const summary = getElement<HTMLElement>('pdf-ai-actions-summary');
    const applyAllButton = getElement<HTMLButtonElement>('pdf-ai-apply-all-btn');
    const saveAsButton = getElement<HTMLButtonElement>('pdf-ai-save-as-btn');
    const clearButton = getElement<HTMLButtonElement>('pdf-ai-clear-btn');
    if (!summary) {
      return;
    }

    this.renderMessages();
    syncResumeAiSuggestionSummary({
      summary,
      applyAllButton,
      saveAsButton,
      clearButton,
      suggestions: this.suggestions,
      isBusy: this.isBusy,
    });
  }

  private saveApiKey(): void {
    const apiKeyInput = getElement<HTMLInputElement>('pdf-ai-api-key');
    if (!apiKeyInput) {
      return;
    }

    const apiKey = apiKeyInput.value.trim();
    if (!apiKey) {
      this.setStatus('请输入有效的 Gemini API Key', 'error');
      return;
    }

    void saveAiSettings({ geminiApiKey: apiKey })
      .then(() => {
        this.isEditingApiKey = false;
        this.syncApiKeySection();
        this.setStatus('API Key 已保存在本地配置文件', 'success');
        this.scheduleIdleStatusSync(1500);
      })
      .catch((error) => {
        this.setStatus(`保存失败: ${describeError(error)}`, 'error');
        this.scheduleIdleStatusSync();
      });
  }

  private async restoreApiKey(): Promise<void> {
    const apiKeyInput = getElement<HTMLInputElement>('pdf-ai-api-key');
    if (!apiKeyInput || apiKeyInput.value) {
      return;
    }

    try {
      const settings = await loadAiSettings();
      apiKeyInput.value = settings.geminiApiKey || '';
      this.isEditingApiKey = !apiKeyInput.value.trim();
      this.syncApiKeySection();
      this.syncIdleStatus();
    } catch (error) {
      this.setStatus(`读取 AI 配置失败: ${describeError(error)}`, 'error');
      this.scheduleIdleStatusSync();
    }
  }

  private setBusy(nextBusy: boolean, statusText?: string): void {
    if (nextBusy) {
      this.clearStatusResetTimer();
    }
    this.isBusy = nextBusy;
    applyResumeAiBusyState({ busy: nextBusy, suggestions: this.suggestions });
    if (statusText) {
      this.setStatus(statusText, nextBusy ? 'working' : 'idle');
    }
  }

  private async toggleWideMode(): Promise<void> {
    this.isWide = !this.isWide;
    applyResumeAiWideMode(this.isWide);
    await this.refreshViewer();
  }

  private setPanelOpen(isOpen: boolean): void {
    applyResumeAiPanelOpen(isOpen);
    if (isOpen) {
      this.syncViewerState();
      this.syncIdleStatus();
    }
    void this.refreshViewer();
  }

  private async showSuggestionPage(
    pageIndex: number,
    reason: 'default' | 'zoom' | 'editorVisibility' | 'documentMutation' = 'default',
  ): Promise<void> {
    const session = this.deps.getViewerSession();
    if (pageIndex !== session.currentPage) {
      this.deps.setCurrentPage(pageIndex);
    }
    await this.deps.renderCurrentPage(reason);
  }

  private async refreshViewer(): Promise<void> {
    const session = this.deps.getViewerSession();
    if (!session.path) {
      return;
    }
    await new Promise<void>((resolve) => {
      window.requestAnimationFrame(() => resolve());
    });
    await this.deps.renderCurrentPage();
  }

  private setStatus(text: string, tone: StatusTone): void {
    this.clearStatusResetTimer();
    setResumeAiStatus(text, tone);
  }

  private syncIdleStatus(): void {
    if (this.isBusy) {
      return;
    }
    const apiKeyInput = getElement<HTMLInputElement>('pdf-ai-api-key');
    const hasKey = !!apiKeyInput?.value.trim();
    this.setStatus(hasKey ? '' : '未配置', 'idle');
  }

  private scheduleIdleStatusSync(delayMs = 2800): void {
    this.clearStatusResetTimer();
    this.statusResetTimer = window.setTimeout(() => {
      this.statusResetTimer = null;
      this.syncIdleStatus();
    }, delayMs);
  }

  private clearStatusResetTimer(): void {
    if (this.statusResetTimer !== null) {
      window.clearTimeout(this.statusResetTimer);
      this.statusResetTimer = null;
    }
  }

  private expandApiKeyEditor(): void {
    this.isEditingApiKey = true;
    this.syncApiKeySection();
  }

  private cancelApiKeyEditing(): void {
    const apiKeyInput = getElement<HTMLInputElement>('pdf-ai-api-key');
    const hasKey = !!apiKeyInput?.value.trim();
    this.isEditingApiKey = !hasKey;
    this.syncApiKeySection();
    this.syncIdleStatus();
  }

  private syncApiKeySection(): void {
    syncResumeAiApiKeySection(this.isEditingApiKey);
  }

  private logAiChain(node: string, payload: Record<string, unknown>): void {
    emitPdfDiagnostic('ai', node, payload, { verboseOnly: true });
  }
}

export function createResumeAiController(deps: ResumeAiControllerDeps): ResumeAiController {
  return new PdfResumeAiController(deps);
}
