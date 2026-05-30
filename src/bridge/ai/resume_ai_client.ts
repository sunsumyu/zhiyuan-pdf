/**
 * Resume AI client (Stage C — minimum viable chat).
 *
 * 历史背景: 这一层原本是 9 个 `targetInvokeV3` 调用，对应 Rust 后端 `submit_resume_prompt`
 * 等命令。但 Rust 端命令处理器从未实现 (`src-tauri/src/lib.rs::invoke_handler!` 中无注册),
 * 导致前端发送任何请求都会 silently 失败。
 *
 * 本实现绕开 Rust 后端, 直接在 TS 侧:
 *   - 用 `fetch` 调 Gemini REST API (Generative Language API v1beta)
 *   - 维护 per-path 内存会话状态 (turns / suggestions / scope)
 *   - 不生成 PDF 修改 patch (suggestions 始终为空), 仅做对话
 *
 * 后续如果要加 PDF 改写能力, 在 `submitResumeAiPrompt` 里把 Gemini 返回的修改建议
 * 翻译成 `ResumeAiSuggestion[]` 即可 — 控制器代码无需改动。
 */
import type {
  ResumeAiScope,
  ResumeAiSuggestion,
  ResumeChatTurn,
  ResumeAiPlanResult,
  ResumeAiThreadView,
} from './resume_ai_types';

export interface PlanResumeAiRequest {
  apiKey: string;
  currentPage: number;
  history: ResumeChatTurn[];
  path: string;
  prompt: string;
  scope: ResumeAiScope;
}

export interface ApplyResumeAiRequest {
  path: string;
  suggestions: ResumeAiSuggestion[];
  targetPath?: string;
}

export interface SyncResumeAiSessionRequest {
  path?: string;
  currentPage?: number;
  scope?: ResumeAiScope;
}

export interface SubmitResumeAiPromptRequest {
  apiKey: string;
  path: string;
  prompt: string;
  currentPage: number;
  scope: ResumeAiScope;
}

export interface ApplyResumeAiSuggestionRequest {
  path: string;
  suggestionId: string;
}

// ── In-memory session state ──────────────────────────────────────────────

interface SessionState {
  scope: ResumeAiScope;
  currentPage: number;
  turns: ResumeChatTurn[];
  suggestions: ResumeAiSuggestion[];
}

const SESSIONS = new Map<string, SessionState>();
const EMPTY_SESSION_KEY = '__no_path__';

function sessionKey(path?: string): string {
  return path && path.length > 0 ? path : EMPTY_SESSION_KEY;
}

function getOrCreateSession(path?: string, scope: ResumeAiScope = 'current-page', currentPage = 0): SessionState {
  const key = sessionKey(path);
  let state = SESSIONS.get(key);
  if (!state) {
    state = { scope, currentPage, turns: [], suggestions: [] };
    SESSIONS.set(key, state);
  }
  return state;
}

function makeView(state: SessionState, path: string | undefined, notice?: string): ResumeAiThreadView {
  return {
    path,
    currentPage: state.currentPage,
    scope: state.scope,
    turns: [...state.turns],
    suggestions: [...state.suggestions],
    phase: 'idle',
    busy: false,
    notice,
  };
}

// ── Gemini REST call ─────────────────────────────────────────────────────

const GEMINI_MODEL = 'gemini-2.5-flash';
const GEMINI_ENDPOINT = `https://generativelanguage.googleapis.com/v1beta/models/${GEMINI_MODEL}:generateContent`;

interface GeminiPart { text: string }
interface GeminiContent { role?: 'user' | 'model'; parts: GeminiPart[] }
interface GeminiResponse {
  candidates?: Array<{
    content?: { parts?: GeminiPart[] };
    finishReason?: string;
  }>;
  promptFeedback?: { blockReason?: string };
  error?: { message?: string };
}

function toGeminiContents(history: ResumeChatTurn[], userPrompt: string): GeminiContent[] {
  const contents: GeminiContent[] = history.map((turn) => ({
    role: turn.role === 'assistant' ? 'model' : 'user',
    parts: [{ text: turn.text }],
  }));
  contents.push({ role: 'user', parts: [{ text: userPrompt }] });
  return contents;
}

async function callGemini(apiKey: string, contents: GeminiContent[]): Promise<string> {
  const url = `${GEMINI_ENDPOINT}?key=${encodeURIComponent(apiKey)}`;
  const body = JSON.stringify({ contents });
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body,
  });

  let payload: GeminiResponse;
  try {
    payload = (await response.json()) as GeminiResponse;
  } catch {
    throw new Error(`Gemini 响应不是合法 JSON (HTTP ${response.status})`);
  }

  if (!response.ok) {
    const apiMsg = payload.error?.message || `HTTP ${response.status}`;
    throw new Error(`Gemini API 错误: ${apiMsg}`);
  }
  if (payload.promptFeedback?.blockReason) {
    throw new Error(`请求被 Gemini 拦截: ${payload.promptFeedback.blockReason}`);
  }
  const text = payload.candidates?.[0]?.content?.parts?.map((p) => p.text).join('') ?? '';
  if (!text.trim()) {
    throw new Error('Gemini 返回空内容');
  }
  return text.trim();
}

// ── Public API (signatures kept identical to original Rust-backed version) ──

export async function planResumeAiEdits(
  _request: PlanResumeAiRequest,
): Promise<ResumeAiPlanResult> {
  // Stage C 不做 PDF 改写。返回空建议。
  return { reply: '', suggestions: [], warnings: [] };
}

export async function applyResumeAiEdits(
  request: ApplyResumeAiRequest,
): Promise<{ path: string }> {
  return { path: request.targetPath ?? request.path };
}

export async function syncResumeAiSession(
  request: SyncResumeAiSessionRequest,
): Promise<ResumeAiThreadView> {
  const state = getOrCreateSession(
    request.path,
    request.scope ?? 'current-page',
    request.currentPage ?? 0,
  );
  if (request.scope) state.scope = request.scope;
  if (typeof request.currentPage === 'number') state.currentPage = request.currentPage;
  return makeView(state, request.path);
}

export async function submitResumeAiPrompt(
  request: SubmitResumeAiPromptRequest,
): Promise<ResumeAiThreadView> {
  const state = getOrCreateSession(request.path, request.scope, request.currentPage);
  state.scope = request.scope;
  state.currentPage = request.currentPage;

  // 先记录用户消息, 即便后续 API 失败用户也能看到自己说了什么
  state.turns.push({ role: 'user', text: request.prompt });

  try {
    const reply = await callGemini(request.apiKey, toGeminiContents(state.turns.slice(0, -1), request.prompt));
    state.turns.push({ role: 'assistant', text: reply });
    return makeView(state, request.path);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    state.turns.push({ role: 'assistant', text: `[出错] ${message}` });
    return makeView(state, request.path, `请求失败: ${message}`);
  }
}

export async function applyResumeAiSuggestion(
  request: ApplyResumeAiSuggestionRequest,
): Promise<ResumeAiThreadView> {
  const state = getOrCreateSession(request.path);
  const suggestion = state.suggestions.find((s) => s.id === request.suggestionId);
  if (suggestion) suggestion.state = 'applied';
  return makeView(state, request.path);
}

export async function markResumeAiSuggestionApplied(
  request: ApplyResumeAiSuggestionRequest,
): Promise<ResumeAiThreadView> {
  return applyResumeAiSuggestion(request);
}

export async function markResumeAiSuggestionFailed(
  request: { path: string; suggestionId: string; errorMessage: string },
): Promise<ResumeAiThreadView> {
  const state = getOrCreateSession(request.path);
  const suggestion = state.suggestions.find((s) => s.id === request.suggestionId);
  if (suggestion) {
    suggestion.state = 'failed';
    suggestion.errorMessage = request.errorMessage;
  }
  return makeView(state, request.path);
}

export async function applyAllResumeAiSuggestions(
  request: { path: string },
): Promise<ResumeAiThreadView> {
  const state = getOrCreateSession(request.path);
  for (const s of state.suggestions) {
    if (s.state === 'pending') s.state = 'applied';
  }
  return makeView(state, request.path);
}

export async function markAllResumeAiSuggestionsApplied(
  request: { path: string },
): Promise<ResumeAiThreadView> {
  return applyAllResumeAiSuggestions(request);
}

export async function clearResumeAiSuggestions(
  request: { path?: string },
): Promise<ResumeAiThreadView> {
  const state = getOrCreateSession(request.path);
  state.suggestions = [];
  return makeView(state, request.path, '建议列表已清空');
}

