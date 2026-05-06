import { targetInvokeV3 } from '../shared/wasm_loader';
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

export async function planResumeAiEdits(
  request: PlanResumeAiRequest,
): Promise<ResumeAiPlanResult> {
  return targetInvokeV3('plan_resume_edits', {
    request: {
      apiKey: request.apiKey,
      currentPage: request.currentPage,
      history: request.history,
      path: request.path,
      prompt: request.prompt,
      scope: request.scope,
    },
  });
}

export async function applyResumeAiEdits(
  request: ApplyResumeAiRequest,
): Promise<{ path: string }> {
  return targetInvokeV3('apply_resume_edits', {
    request: {
      path: request.path,
      suggestions: request.suggestions,
      targetPath: request.targetPath,
    },
  });
}

export async function syncResumeAiSession(
  request: SyncResumeAiSessionRequest,
): Promise<ResumeAiThreadView> {
  return targetInvokeV3('sync_resume_session', {
    request: {
      path: request.path,
      currentPage: request.currentPage,
      scope: request.scope,
    },
  });
}

export async function submitResumeAiPrompt(
  request: SubmitResumeAiPromptRequest,
): Promise<ResumeAiThreadView> {
  return targetInvokeV3('submit_resume_prompt', {
    request: {
      apiKey: request.apiKey,
      path: request.path,
      prompt: request.prompt,
      currentPage: request.currentPage,
      scope: request.scope,
    },
  });
}

export async function applyResumeAiSuggestion(
  request: ApplyResumeAiSuggestionRequest,
): Promise<ResumeAiThreadView> {
  return targetInvokeV3('apply_resume_suggestion', {
    request: {
      path: request.path,
      suggestionId: request.suggestionId,
    },
  });
}

export async function markResumeAiSuggestionApplied(
  request: ApplyResumeAiSuggestionRequest,
): Promise<ResumeAiThreadView> {
  return targetInvokeV3('mark_suggestion_applied', {
    request: {
      path: request.path,
      suggestionId: request.suggestionId,
    },
  });
}

export async function markResumeAiSuggestionFailed(
  request: { path: string; suggestionId: string; errorMessage: string },
): Promise<ResumeAiThreadView> {
  return targetInvokeV3('mark_suggestion_failed', {
    request: {
      path: request.path,
      suggestionId: request.suggestionId,
      errorMessage: request.errorMessage,
    },
  });
}

export async function applyAllResumeAiSuggestions(
  request: { path: string },
): Promise<ResumeAiThreadView> {
  return targetInvokeV3('apply_all_suggestions', {
    request: {
      path: request.path,
    },
  });
}

export async function markAllResumeAiSuggestionsApplied(
  request: { path: string },
): Promise<ResumeAiThreadView> {
  return targetInvokeV3('mark_all_suggestions_applied', {
    request: {
      path: request.path,
    },
  });
}

export async function clearResumeAiSuggestions(
  request: { path?: string },
): Promise<ResumeAiThreadView> {
  return targetInvokeV3('clear_suggestions', {
    request: {
      path: request.path,
    },
  });
}

