import { emitPdfDiagnostic } from '../shared/diagnostics';
import type { EditorContext } from './lifecycle';
import * as api from './api';
import {
    withSuppressedNativeInput,
    rememberRustCaret,
    writeTextareaCaret,
    readTextareaCaret,
    getLastDisplayZoom,
} from './textarea_helper';
import {
    readHostReferenceBox,
    type ParagraphInteractionTarget,
} from './editor_host_view';
import {
    commitEditor,
    setupActiveEditor,
    readLegacySnapshot,
    renderActiveEditor,
    hideEditorShell,
    syncTargets,
    openEditor,
} from './lifecycle';

export function createInputHandlers(ctx: EditorContext) {
    function logEditorDiagnostic(
        event: string,
        fields: Record<string, unknown> = {},
        level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR' = 'WARN',
        verboseOnly = false,
    ): void {
        emitPdfDiagnostic('editor', event, fields, { level, verboseOnly });
    }

    function textareaCharCount(textarea: HTMLTextAreaElement): number {
        return [...textarea.value].length;
    }

    function textareaCaretSnapshot(textarea: HTMLTextAreaElement): Record<string, unknown> {
        const hostCaret = readTextareaCaret(textarea);
        return {
            hostCaret,
            selectionStart: textarea.selectionStart,
            selectionEnd: textarea.selectionEnd,
            utf16Length: textarea.value.length,
            charCount: textareaCharCount(textarea),
            lastRustCaretIndex: ctx.state.lastRustCaretIndex,
        };
    }

    return {
        onCommitRequested: () => {
            void commitEditor(ctx).then(() => {
                api.setEditMode(false);
                syncTargets(ctx, getLastDisplayZoom(ctx.state));
            });
        },
        onNavigationRequested: (command: string, textarea: HTMLTextAreaElement) => {
            const before = textareaCaretSnapshot(textarea);
            const hostCaret = Number(before.hostCaret);
            logEditorDiagnostic('caret.navigation.before', {
                command,
                ...before,
            }, 'DEBUG', true);
            const syncResult = api.syncInput({ text: textarea.value, caretIndex: Math.max(0, hostCaret) });
            const result = api.applyCommand({ command, insertedText: null })?.data;
            logEditorDiagnostic('caret.navigation.afterCommand', {
                command,
                syncOk: syncResult?.ok,
                syncCaretIndex: syncResult?.data?.caretIndex,
                resultCaretIndex: result?.caretIndex,
                resultDraftCharCount: result?.draftText != null ? [...result.draftText].length : null,
                resultChanged: result?.changed,
            }, 'DEBUG', true);
            if (result && Number.isFinite(result.caretIndex) && result.caretIndex >= 0) {
                rememberRustCaret(ctx.state, result.caretIndex);
                withSuppressedNativeInput(ctx.state, () => {
                    if (result.draftText != null) textarea.value = result.draftText;
                    writeTextareaCaret(textarea, result.caretIndex);
                });
                logEditorDiagnostic('caret.navigation.afterWrite', {
                    command,
                    ...textareaCaretSnapshot(textarea),
                }, 'DEBUG', true);
            }
            renderActiveEditor(ctx);
        },
        onBeforeInputRequested: (command: string, text: string | null, textarea: HTMLTextAreaElement) => {
            const before = textareaCaretSnapshot(textarea);
            const hostCaret = Number(before.hostCaret);
            logEditorDiagnostic('caret.beforeinput.before', {
                command,
                insertedText: text,
                ...before,
            }, 'DEBUG', true);
            const syncResult = api.syncInput({ text: textarea.value, caretIndex: Math.max(0, hostCaret) });
            const result = api.applyCommand({ command, insertedText: text })?.data;
            logEditorDiagnostic('caret.beforeinput.afterCommand', {
                command,
                insertedText: text,
                syncOk: syncResult?.ok,
                syncCaretIndex: syncResult?.data?.caretIndex,
                resultCaretIndex: result?.caretIndex,
                resultDraftUtf16Length: result?.draftText != null ? result.draftText.length : null,
                resultDraftCharCount: result?.draftText != null ? [...result.draftText].length : null,
                resultChanged: result?.changed,
            }, 'DEBUG', true);
            if (result && Number.isFinite(result.caretIndex) && result.caretIndex >= 0) {
                rememberRustCaret(ctx.state, result.caretIndex);
                withSuppressedNativeInput(ctx.state, () => {
                    if (result.draftText != null) textarea.value = result.draftText;
                    writeTextareaCaret(textarea, result.caretIndex);
                });
                logEditorDiagnostic('caret.beforeinput.afterWrite', {
                    command,
                    insertedText: text,
                    ...textareaCaretSnapshot(textarea),
                }, 'DEBUG', true);
            }
            renderActiveEditor(ctx);
            if (result?.changed) {
                ctx.state.suppressBlurCommitForRender = true;
                const savedStart = textarea.selectionStart;
                const savedEnd = textarea.selectionEnd;
                void ctx.deps.renderCurrentPage('editorVisibility').finally(() => {
                    try {
                        const ta = textarea;
                        if (ta) {
                            withSuppressedNativeInput(ctx.state, () => {
                                if (document.activeElement !== ta) {
                                    ta.focus({ preventScroll: true });
                                }
                                if (savedStart !== null && savedEnd !== null) {
                                    ta.selectionStart = savedStart;
                                    ta.selectionEnd = savedEnd;
                                }
                            });
                        }
                    } finally {
                        ctx.state.suppressBlurCommitForRender = false;
                    }
                });
            }
        },
        onCompositionSyncRequested: (textarea: HTMLTextAreaElement) => {
            const caretIndex = readTextareaCaret(textarea);
            api.syncInput({ text: textarea.value, caretIndex: Math.max(0, caretIndex) });
            renderActiveEditor(ctx);
        },
        shouldSuppressNativeInput: () => ctx.state.suppressNativeInputCount > 0,
        shouldSuppressBlurCommit: () =>
            ctx.state.suppressBlurCommitForSave
            || ctx.state.suppressBlurCommitForOpen
            || ctx.state.suppressBlurCommitForRender,
        onBlurCommitSuppressed: () => {
            // No-op
        },
        onBlurCommitRequested: () => {
            if (!api.hasSessionChanges()) return;
            void commitEditor(ctx);
        },
        onShellPointerDown: (event: MouseEvent, shell: HTMLElement, textarea: HTMLTextAreaElement) => {
            const nodes = ctx.ensureNodes();
            if (!nodes) return;
            const referenceBox = readHostReferenceBox(nodes.root);
            void shell;
            const result = api.moveCaret({
                clientX: event.clientX,
                clientY: event.clientY,
                referenceLeft: referenceBox.left,
                referenceTop: referenceBox.top,
                referenceWidth: referenceBox.width,
                referenceHeight: referenceBox.height,
                pageWidth: ctx.deps.getPageWidth(),
                pageHeight: ctx.deps.getPageHeight(),
            });

            if (!result?.ok || !result.data) {
                void commitEditor(ctx).then(() => {
                    api.setEditMode(false);
                    syncTargets(ctx, getLastDisplayZoom(ctx.state));
                });
                return;
            }

            const nextCaret = result.data.caretIndex;
            withSuppressedNativeInput(ctx.state, () => {
                textarea.focus();
            });
            if (Number.isFinite(nextCaret) && nextCaret >= 0) {
                rememberRustCaret(ctx.state, nextCaret);
                withSuppressedNativeInput(ctx.state, () => {
                    writeTextareaCaret(textarea, nextCaret);
                });
                renderActiveEditor(ctx);
            }
        },
        onTargetPointerDown: (target: ParagraphInteractionTarget, event: MouseEvent) => {
            void openEditor(ctx, target, event);
        },
        onRootPointerDown: (event: MouseEvent) => {
            try {
                const beginResult = api.begin();
                if (beginResult && !beginResult.ok) {
                    logEditorDiagnostic('rootPointerDown.beginFailed', { beginResult }, 'WARN');
                }

                const nodes = ctx.ensureNodes();
                if (!nodes) return;

                event.preventDefault();
                event.stopPropagation();

                const referenceBox = readHostReferenceBox(nodes.root);
                const hitResult = api.hitTest({
                    clientX: event.clientX,
                    clientY: event.clientY,
                    referenceLeft: referenceBox.left,
                    referenceTop: referenceBox.top,
                    referenceWidth: referenceBox.width,
                    referenceHeight: referenceBox.height,
                    pageWidth: ctx.deps.getPageWidth(),
                    pageHeight: ctx.deps.getPageHeight(),
                });

                if (!hitResult?.ok || !hitResult.data?.blockId) {
                    logEditorDiagnostic('rootPointerDown.hitTestMiss', {
                        hitResult,
                        clientX: event.clientX,
                        clientY: event.clientY,
                    }, 'WARN');
                    api.discard();
                    hideEditorShell(ctx);
                    syncTargets(ctx, getLastDisplayZoom(ctx.state));
                    return;
                }

                const openResult = api.openBlock({
                    blockId: hitResult.data.blockId,
                    clientX: event.clientX,
                    clientY: event.clientY,
                    referenceLeft: referenceBox.left,
                    referenceTop: referenceBox.top,
                    referenceWidth: referenceBox.width,
                    referenceHeight: referenceBox.height,
                    pageWidth: ctx.deps.getPageWidth(),
                    pageHeight: ctx.deps.getPageHeight(),
                    fallbackPageX: hitResult.data.pageX,
                    fallbackPageY: hitResult.data.pageY,
                });

                if (!openResult?.ok || !openResult.data) {
                    logEditorDiagnostic('rootPointerDown.openBlockFailed', { openResult, hitResult }, 'ERROR');
                    api.discard();
                    hideEditorShell(ctx);
                    return;
                }

                const snapshot = readLegacySnapshot(ctx);
                const activeTarget = snapshot?.activeTarget;
                if (!activeTarget) {
                    logEditorDiagnostic('rootPointerDown.missingActiveTarget', { snapshot, hitResult }, 'ERROR');
                    api.discard();
                    hideEditorShell(ctx);
                    return;
                }

                setupActiveEditor(ctx, nodes, activeTarget, openResult.data.draftText, openResult.data.caretIndex);
            } catch (err) {
                logEditorDiagnostic('rootPointerDown.exception', {
                    error: String(err),
                    clientX: event.clientX,
                    clientY: event.clientY,
                }, 'ERROR');
                hideEditorShell(ctx);
            }
        },
        onSelectionChanged: (start: number, end: number, textarea: HTMLTextAreaElement) => {
            if (
                ctx.state.suppressNativeInputCount > 0
                || ctx.state.suppressBlurCommitForOpen
                || ctx.state.suppressBlurCommitForRender
            ) {
                return;
            }
            const charStart = api.utf16ToCharIndex(textarea.value, start);
            const charEnd = api.utf16ToCharIndex(textarea.value, end);
            if (charStart != null && charEnd != null) {
                api.setSelection(charStart, charEnd);
                if (ctx.state.lastRustCaretIndex !== charEnd) {
                    rememberRustCaret(ctx.state, charEnd);
                    renderActiveEditor(ctx);
                }
            }
        },
        logNode: () => {},
    };
}
