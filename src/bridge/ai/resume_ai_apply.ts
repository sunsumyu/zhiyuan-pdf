import type { ViewerSessionSnapshot } from '../viewer/viewer_session';
import type { DocumentEditApi, PdfRegionTextEdit } from '../document/document_edit_api';
import {
    applyResumeAiEdits,
    markResumeAiSuggestionApplied,
    markResumeAiSuggestionFailed,
} from './resume_ai_client';
import type { ResumeAiSuggestion, ResumeAiThreadView } from './resume_ai_types';

function describeError(error: unknown): string {
    if (error instanceof Error) {
        return error.message;
    }
    return String(error);
}

function normalizePath(value: string): string {
    return value.replace(/\//g, '\\').toLowerCase();
}

export type ApplyContext = {
    getViewerSession: () => ViewerSessionSnapshot;
    documentEdits: DocumentEditApi;
    setCurrentPage: (pageIndex: number) => void;
    logAiChain: (node: string, payload: Record<string, unknown>) => void;
};

function toTextEdit(suggestion: ResumeAiSuggestion): PdfRegionTextEdit {
    return {
        id: suggestion.id,
        pageIndex: suggestion.pageIndex,
        regionId: suggestion.regionId,
        kind: suggestion.kind,
        originalText: suggestion.originalText,
        newText: suggestion.suggestedText,
    };
}

export async function applySingleSuggestion(
    ctx: ApplyContext,
    suggestion: ResumeAiSuggestion,
): Promise<ResumeAiThreadView> {
    const currentSession = ctx.getViewerSession();
    const isCurrentDocument = !!currentSession.path
        && normalizePath(suggestion.path) === normalizePath(currentSession.path);

    ctx.logAiChain('ts.apply_one.invoke', {
        suggestionId: suggestion.id,
        path: currentSession.path || suggestion.path,
    });

    if (isCurrentDocument && currentSession.currentPage !== suggestion.pageIndex) {
        ctx.setCurrentPage(suggestion.pageIndex);
    }
    ctx.documentEdits.editRegionText(toTextEdit(suggestion), 'ai-apply-one');
    const saveResult = await ctx.documentEdits.saveEdits('ai-apply-one');
    if (!saveResult.saved) {
        throw new Error(saveResult.errorMessage || '编辑器保存失败');
    }
    const view = await markResumeAiSuggestionApplied({
        path: currentSession.path || suggestion.path,
        suggestionId: suggestion.id,
    });
    ctx.logAiChain('ts.apply_one.result', {
        suggestionId: suggestion.id,
        notice: view.notice ?? null,
    });
    return view;
}

export async function applyAllPendingSuggestions(
    ctx: ApplyContext,
    suggestions: ResumeAiSuggestion[],
): Promise<{ applied: number; failed: Array<{ id: string; message: string }>; views: ResumeAiThreadView[] }> {
    const currentSession = ctx.getViewerSession();
    ctx.logAiChain('ts.apply_all.click', { count: suggestions.length, path: currentSession.path });

    const failed: Array<{ id: string; message: string }> = [];
    const views: ResumeAiThreadView[] = [];
    const grouped = new Map<number, ResumeAiSuggestion[]>();
    for (const suggestion of suggestions) {
        const bucket = grouped.get(suggestion.pageIndex) || [];
        bucket.push(suggestion);
        grouped.set(suggestion.pageIndex, bucket);
    }

    for (const [pageIndex, pageSuggestions] of grouped) {
        if (currentSession.currentPage !== pageIndex) {
            ctx.setCurrentPage(pageIndex);
        }
        let pageSaveFailed = false;
        for (const suggestion of pageSuggestions) {
            try {
                ctx.documentEdits.editRegionText(toTextEdit(suggestion), 'ai-apply-all');
            } catch (error) {
                failed.push({ id: suggestion.id, message: describeError(error) });
            }
        }
        const saveResult = await ctx.documentEdits.saveEdits('ai-apply-all');
        if (!saveResult.saved) {
            pageSaveFailed = true;
            const message = saveResult.errorMessage || `第 ${pageIndex + 1} 页保存失败`;
            for (const suggestion of pageSuggestions) {
                if (!failed.find((item) => item.id === suggestion.id)) {
                    failed.push({ id: suggestion.id, message });
                }
            }
        }
        for (const suggestion of pageSuggestions) {
            if (pageSaveFailed || failed.find((item) => item.id === suggestion.id)) {
                continue;
            }
            try {
                const view = await markResumeAiSuggestionApplied({
                    path: currentSession.path!,
                    suggestionId: suggestion.id,
                });
                views.push(view);
            } catch (error) {
                failed.push({ id: suggestion.id, message: describeError(error) });
            }
        }
    }

    for (const item of failed) {
        try {
            const view = await markResumeAiSuggestionFailed({
                path: currentSession.path!,
                suggestionId: item.id,
                errorMessage: item.message,
            });
            views.push(view);
        } catch (error) {
            ctx.logAiChain('ts.apply_all.error.secondary', { message: describeError(error) });
        }
    }

    return { applied: suggestions.length - failed.length, failed, views };
}

export async function saveAsSeparatePdf(
    ctx: ApplyContext,
    suggestions: ResumeAiSuggestion[],
    targetPath: string,
): Promise<{ path: string }> {
    const session = ctx.getViewerSession();
    return await applyResumeAiEdits({
        path: session.path!,
        suggestions,
        targetPath,
    });
}
