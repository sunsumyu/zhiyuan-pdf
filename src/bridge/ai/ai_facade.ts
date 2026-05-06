import { getWasmApi } from '../shared/wasm_loader';

export type ResumeAiSuggestion = {
    id: string;
    pageIndex: number;
    regionId: string;
    kind: string | null;
    originalText: string;
    suggestedText: string;
    state: string; // "pending" | "applied" | "failed"
    reasoning: string | null;
};

export type ResumeChatTurn = {
    role: string; // "user" | "assistant"
    content: string;
    timestamp: string | null;
};

export type ResumeAiThreadView = {
    suggestions: ResumeAiSuggestion[];
    turns: ResumeChatTurn[];
    notice: string | null;
};

export type ResumeAiSessionRequest = {
    path: string;
    pageIndex: number;
    scope: string; // "current-page" | "whole-document"
};

export type ResumeAiPromptRequest = {
    path: string;
    pageIndex: number;
    scope: string;
    prompt: string;
};

export type ResumeAiApplyRequest = {
    path: string;
    suggestionId: string;
};

export type ResumeAiApplyAllRequest = {
    path: string;
    suggestions: ResumeAiSuggestion[];
};

export type ResumeAiSaveAsRequest = {
    path: string;
    suggestions: ResumeAiSuggestion[];
    targetPath: string;
};

export type ResumeAiSaveAsResult = {
    path: string;
};

export type ResumeAiClearRequest = {
    path: string;
};

export type ResumeAiFacadeResult = {
    changed: boolean;
    threadView: ResumeAiThreadView | null;
    saveAsResult: ResumeAiSaveAsResult | null;
    renderFrame: unknown | null;
};

function callFacade<T>(fnName: string, arg?: unknown): T | null {
    const api = getWasmApi();
    const fn = (api as any)[fnName];
    if (typeof fn !== 'function') return null;
    try {
        return arg !== undefined ? fn(arg) : fn();
    } catch {
        return null;
    }
}

export function facadeSyncSession(path: string, pageIndex: number, scope: string): ResumeAiThreadView | null {
    return callFacade<ResumeAiThreadView>('resumeAiFacadeSyncSession', { path, pageIndex, scope });
}

export function facadeSubmitPrompt(path: string, pageIndex: number, scope: string, prompt: string): ResumeAiThreadView | null {
    return callFacade<ResumeAiThreadView>('resumeAiFacadeSubmitPrompt', { path, pageIndex, scope, prompt });
}

export function facadeApplySuggestion(path: string, suggestionId: string): ResumeAiFacadeResult | null {
    return callFacade<ResumeAiFacadeResult>('resumeAiFacadeApplySuggestion', { path, suggestionId });
}

export function facadeApplyAll(path: string, suggestions: ResumeAiSuggestion[]): ResumeAiFacadeResult | null {
    return callFacade<ResumeAiFacadeResult>('resumeAiFacadeApplyAll', { path, suggestions });
}

export function facadeSaveAs(path: string, suggestions: ResumeAiSuggestion[], targetPath: string): ResumeAiFacadeResult | null {
    return callFacade<ResumeAiFacadeResult>('resumeAiFacadeSaveAs', { path, suggestions, targetPath });
}

export function facadeClearSuggestions(path: string): ResumeAiThreadView | null {
    return callFacade<ResumeAiThreadView>('resumeAiFacadeClearSuggestions', { path });
}

