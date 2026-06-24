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
    return {
        onCommitRequested: () => {
            void commitEditor(ctx).then(() => {
                api.setEditMode(false);
                syncTargets(ctx, getLastDisplayZoom(ctx.state));
            });
        },
        onNavigationRequested: (command: string, textarea: HTMLTextAreaElement) => {
            const hostCaret = readTextareaCaret(textarea);
            api.syncInput({ text: textarea.value, caretIndex: Math.max(0, hostCaret) });
            const result = api.applyCommand({ command, insertedText: null })?.data;
            if (result && Number.isFinite(result.caretIndex) && result.caretIndex >= 0) {
                rememberRustCaret(ctx.state, result.caretIndex);
                withSuppressedNativeInput(ctx.state, () => {
                    if (result.draftText != null) textarea.value = result.draftText;
                    writeTextareaCaret(textarea, result.caretIndex);
                });
            }
            renderActiveEditor(ctx);
        },
        onBeforeInputRequested: (command: string, text: string | null, textarea: HTMLTextAreaElement) => {
            const hostCaret = readTextareaCaret(textarea);
            console.log('[CARET-DIAG]', command, {
                hostCaret,
                selStart: textarea.selectionStart,
                selEnd: textarea.selectionEnd,
                valLen: textarea.value.length,
                lastRust: ctx.state.lastRustCaretIndex,
            });
            api.syncInput({ text: textarea.value, caretIndex: Math.max(0, hostCaret) });
            const result = api.applyCommand({ command, insertedText: text })?.data;
            if (result && Number.isFinite(result.caretIndex) && result.caretIndex >= 0) {
                rememberRustCaret(ctx.state, result.caretIndex);
                withSuppressedNativeInput(ctx.state, () => {
                    if (result.draftText != null) textarea.value = result.draftText;
                    writeTextareaCaret(textarea, result.caretIndex);
                });
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
            const beginResult = api.begin();
            if (beginResult && !beginResult.ok) {
                console.warn('[EDITOR-DIAG] rootPointerDown begin failed', beginResult);
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
                console.warn('[EDITOR-DIAG] rootPointerDown hitTest missed', { hitResult });
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
                console.warn('[EDITOR-DIAG] rootPointerDown openBlock failed', { openResult, hitResult });
                api.discard();
                hideEditorShell(ctx);
                return;
            }

            const snapshot = readLegacySnapshot(ctx);
            const activeTarget = snapshot?.activeTarget;
            if (!activeTarget) {
                console.warn('[EDITOR-DIAG] rootPointerDown missing activeTarget', { snapshot, hitResult });
                api.discard();
                hideEditorShell(ctx);
                return;
            }

            setupActiveEditor(ctx, nodes, activeTarget, openResult.data.draftText, openResult.data.caretIndex);
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
