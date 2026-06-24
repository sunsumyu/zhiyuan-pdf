import { utf16ToCharIndex, charToUtf16Offset } from './api';

export const EDITOR_TEXTAREA_ID = 'pdf-editor-textarea-vector';

export interface EditorHostState {
    suppressNativeInputCount: number;
    lastRustCaretIndex: number | null;
    suppressBlurCommitForSave: boolean;
    suppressBlurCommitForOpen: boolean;
    suppressBlurCommitForRender: boolean;
    cachedDisplayZoom: number;
}

export function withSuppressedNativeInput<T>(state: EditorHostState, fn: () => T): T {
    state.suppressNativeInputCount++;
    try {
        return fn();
    } finally {
        setTimeout(() => {
            state.suppressNativeInputCount = Math.max(0, state.suppressNativeInputCount - 1);
        }, 0);
    }
}

export function readTextareaCaret(textarea: HTMLTextAreaElement): number {
    const utf16Offset = Math.max(0, textarea.selectionStart ?? textarea.value.length);
    const converted = utf16ToCharIndex(textarea.value, utf16Offset);
    return Number.isFinite(converted) ? Math.max(0, converted) : utf16Offset;
}

export function writeTextareaCaret(textarea: HTMLTextAreaElement, caretIndex: number): void {
    const charIndex = Math.max(0, caretIndex);
    const converted = charToUtf16Offset(textarea.value, charIndex);
    const nextCaret = Number.isFinite(converted) ? Math.max(0, converted) : charIndex;
    textarea.selectionStart = nextCaret;
    textarea.selectionEnd = nextCaret;
    try {
        textarea.setSelectionRange(nextCaret, nextCaret);
    } catch {
        // Ignore
    }
}

export function rememberRustCaret(state: EditorHostState, caretIndex: unknown): void {
    if (typeof caretIndex === 'number' && Number.isFinite(caretIndex) && caretIndex >= 0) {
        state.lastRustCaretIndex = Math.max(0, caretIndex);
    }
}

export function clearDomSelection(blurEditorInput = false): void {
    try {
        window.getSelection?.()?.removeAllRanges();
        if (
            document.activeElement instanceof HTMLElement
            && (blurEditorInput || document.activeElement.id !== EDITOR_TEXTAREA_ID)
        ) {
            document.activeElement.blur();
        }
    } catch {
        // Ignore
    }
}

export function getLastDisplayZoom(state: EditorHostState): number {
    return state.cachedDisplayZoom > 0 ? state.cachedDisplayZoom : 1.0;
}
