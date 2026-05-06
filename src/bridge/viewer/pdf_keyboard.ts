import { VECTOR_CONTAINER_ID } from '../render/vector_host';

export type PdfKeyboardShortcutDeps = {
    isTextEditEnabled: () => boolean;
    getScrollContainer: () => HTMLElement | null;
    openFind: () => void;
    undo: () => void;
    redo: () => void;
    toggleBold: () => void;
    toggleItalic: () => void;
    toggleUnderline: () => void;
};

function isPdfViewerKeyboardScope(getScrollContainer: () => HTMLElement | null): boolean {
    const scrollContainer = getScrollContainer();
    if (!scrollContainer || scrollContainer.offsetParent === null) return false;
    const active = document.activeElement as HTMLElement | null;
    if (!active || active === document.body) return true;
    if (active.closest(`#${VECTOR_CONTAINER_ID}`)) return true;
    if (active.closest('#pdf-content-wrapper')) return true;
    if (active.closest('#pdf-scroll-container')) return true;
    if (active.closest('[data-plugin-id="pdf-viewer"]')) return true;
    return false;
}

function isPlainEditableTarget(target: EventTarget | null): boolean {
    const element = target as HTMLElement | null;
    if (!element) return false;
    const tag = element.tagName?.toLowerCase();
    return tag === 'input' || tag === 'textarea' || element.isContentEditable;
}

export function createPdfKeyboardShortcutHandler(
    deps: PdfKeyboardShortcutDeps,
): (event: KeyboardEvent) => void {
    return (event: KeyboardEvent) => {
        if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
        const key = event.key.toLowerCase();
        if (key === 'f' && !event.shiftKey) {
            if (!deps.isTextEditEnabled() && isPlainEditableTarget(event.target)) return;
            if (!isPdfViewerKeyboardScope(deps.getScrollContainer)) return;
            event.preventDefault();
            event.stopPropagation();
            deps.openFind();
            return;
        }
        if (key === 'b' && !event.shiftKey) {
            if (!isPdfViewerKeyboardScope(deps.getScrollContainer)) return;
            event.preventDefault();
            event.stopPropagation();
            deps.toggleBold();
            return;
        }
        if (key === 'i' && !event.shiftKey) {
            if (!isPdfViewerKeyboardScope(deps.getScrollContainer)) return;
            event.preventDefault();
            event.stopPropagation();
            deps.toggleItalic();
            return;
        }
        if (key === 'u' && !event.shiftKey) {
            if (!isPdfViewerKeyboardScope(deps.getScrollContainer)) return;
            event.preventDefault();
            event.stopPropagation();
            deps.toggleUnderline();
            return;
        }
        const wantsUndo = key === 'z' && !event.shiftKey;
        const wantsRedo = key === 'y' || (key === 'z' && event.shiftKey);
        if (!wantsUndo && !wantsRedo) return;

        if (!deps.isTextEditEnabled() && isPlainEditableTarget(event.target)) return;
        if (!isPdfViewerKeyboardScope(deps.getScrollContainer)) return;

        event.preventDefault();
        event.stopPropagation();
        if (wantsUndo) {
            deps.undo();
        } else {
            deps.redo();
        }
    };
}

