import { emitPdfDiagnostic } from '../shared/diagnostics';
import type { PdfSaveResult } from '../document/document_edit_api';
import type { EditorFormatAction } from './types';
import * as api from './api';
import {
    ensureEditorHostView,
    hideInteractionTargets,
    hideEditorShell as hideEditorShellView,
    type EditorHostNodes,
} from './editor_host_view';
import {
    type EditorHostState,
    readTextareaCaret,
    clearDomSelection,
} from './textarea_helper';
import {
    type EditorHostDeps,
    type EditorContext,
    commitEditor,
    saveEdits,
    syncTargets,
    syncFormatButtons,
    readLegacySnapshot,
    hideEditorShell,
    setupActiveEditor,
} from './lifecycle';
import { createInputHandlers } from './input_handler';

export type { EditorHostDeps };

export type EditorHost = {
    syncTargets: (displayZoom: number) => void;
    clear: () => void;
    commitActiveEditor: () => Promise<void>;
    saveEdits: () => Promise<PdfSaveResult>;
    applyFormatAction: (action: EditorFormatAction) => Promise<void>;
    openRegionEditor: (
        pageIndex: number,
        regionId: string,
        kind: string,
        originalText: string,
    ) => Promise<void>;
    hasPendingEdits: () => boolean;
    setTextEditEnabled: (enabled: boolean) => void;
    isTextEditEnabled: () => boolean;
};

export function createEditorHost(deps: EditorHostDeps): EditorHost {
    function logEditorHostError(event: string, err: unknown): void {
        emitPdfDiagnostic('editor', event, { error: String(err) }, { level: 'ERROR' });
    }

    const state: EditorHostState = {
        suppressNativeInputCount: 0,
        lastRustCaretIndex: null,
        suppressBlurCommitForSave: false,
        suppressBlurCommitForOpen: false,
        suppressBlurCommitForRender: false,
        cachedDisplayZoom: 1.0,
    };

    let cachedNodes: EditorHostNodes | null = null;

    const ctx: EditorContext = {
        state,
        deps,
        ensureNodes: () => {
            if (!cachedNodes) {
                const handlers = createInputHandlers(ctx);
                cachedNodes = ensureEditorHostView({
                    readCaretIndex: readTextareaCaret,
                    writeCaretIndex: (textarea, caretIndex) => {
                        const charIndex = Math.max(0, caretIndex);
                        const converted = api.charToUtf16Offset(textarea.value, charIndex);
                        const nextCaret = Number.isFinite(converted) ? Math.max(0, converted) : charIndex;
                        textarea.selectionStart = nextCaret;
                        textarea.selectionEnd = nextCaret;
                        try {
                            textarea.setSelectionRange(nextCaret, nextCaret);
                        } catch {}
                    },
                    onCommitRequested: handlers.onCommitRequested,
                    onNavigationRequested: handlers.onNavigationRequested,
                    onBeforeInputRequested: handlers.onBeforeInputRequested,
                    onCompositionSyncRequested: handlers.onCompositionSyncRequested,
                    shouldSuppressNativeInput: handlers.shouldSuppressNativeInput,
                    shouldSuppressBlurCommit: handlers.shouldSuppressBlurCommit,
                    onBlurCommitSuppressed: handlers.onBlurCommitSuppressed,
                    onBlurCommitRequested: handlers.onBlurCommitRequested,
                    onShellPointerDown: handlers.onShellPointerDown,
                    onTargetPointerDown: handlers.onTargetPointerDown,
                    onRootPointerDown: handlers.onRootPointerDown,
                    onSelectionChanged: handlers.onSelectionChanged,
                    logNode: handlers.logNode,
                });
            }
            return cachedNodes;
        },
    };

    return {
        syncTargets: (displayZoom: number) => {
            try {
                syncTargets(ctx, displayZoom);
            } catch (err) {
                logEditorHostError('host.syncTargets.failed', err);
                throw err;
            }
        },
        clear: () => {
            const nodes = ctx.ensureNodes();
            if (!nodes) return;
            hideInteractionTargets(nodes);
            void commitEditor(ctx);
        },
        commitActiveEditor: () => commitEditor(ctx),
        saveEdits: () => saveEdits(ctx),
        applyFormatAction: async (action: EditorFormatAction) => {
            api.applyFormat(action);
            syncTargets(ctx, state.cachedDisplayZoom);
            syncFormatButtons();
        },
        openRegionEditor: async (pageIndex, regionId, kind, originalText) => {
            const nodes = ctx.ensureNodes();
            if (!nodes) return;
            ctx.state.suppressBlurCommitForOpen = true;
            clearDomSelection();

            const result = api.openRegion({ pageIndex, regionId, kind, originalText });
            if (!result?.ok || !result.data) {
                hideEditorShell(ctx);
                return;
            }

            const snapshot = readLegacySnapshot(ctx);
            const activeTarget = snapshot?.activeTarget;
            if (!activeTarget) {
                hideEditorShell(ctx);
                return;
            }

            const draftText = snapshot?.draftText ?? '';
            const caretIndex = snapshot?.caretIndex ?? 0;
            setupActiveEditor(ctx, nodes, activeTarget, draftText, caretIndex);
        },
        hasPendingEdits: () => api.hasUnsavedChanges(),
        setTextEditEnabled: (enabled: boolean) => {
            try {
                api.setEditMode(enabled);
                if (!enabled) {
                    const nodes = ctx.ensureNodes();
                    if (nodes) hideInteractionTargets(nodes);
                    void commitEditor(ctx);
                } else {
                    syncTargets(ctx, state.cachedDisplayZoom);
                }
            } catch (err) {
                logEditorHostError('host.setTextEditEnabled.failed', err);
                throw err;
            }
        },
        isTextEditEnabled: () => {
            try {
                return !!readLegacySnapshot(ctx)?.enabled;
            } catch (err) {
                logEditorHostError('host.isTextEditEnabled.failed', err);
                return false;
            }
        },
    };
}
