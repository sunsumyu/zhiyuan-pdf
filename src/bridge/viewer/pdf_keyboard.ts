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
    prevPage: () => void;
    nextPage: () => void;
    runPageTurnBench?: (direction: 'next' | 'prev') => void;
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
    // Toolbar buttons / selects等非编辑控件不应拦截方向键翻页。
    if (!isPlainEditableTarget(active)) return true;
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
        // --- Page navigation with arrow keys / PageUp / PageDown ---
        if (!event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey) {
            const navKey = event.key;
            const isPrev = navKey === 'ArrowLeft' || navKey === 'ArrowUp' || navKey === 'PageUp';
            const isNext = navKey === 'ArrowRight' || navKey === 'ArrowDown' || navKey === 'PageDown';
            if (isPrev || isNext) {
                const editable = isPlainEditableTarget(event.target);
                if (editable) return;
                const scope = isPdfViewerKeyboardScope(deps.getScrollContainer);
                if (!scope) return;
                event.preventDefault();
                event.stopPropagation();
                if (isPrev) deps.prevPage(); else deps.nextPage();
                return;
            }
        }

        if (event.ctrlKey && event.shiftKey && event.altKey && !event.metaKey) {
            const navKey = event.key;
            const isPrevBench = navKey === 'ArrowLeft' || navKey === 'ArrowUp' || navKey === 'PageUp';
            const isNextBench = navKey === 'ArrowRight' || navKey === 'ArrowDown' || navKey === 'PageDown';
            if ((isPrevBench || isNextBench) && deps.runPageTurnBench) {
                if (!isPdfViewerKeyboardScope(deps.getScrollContainer)) return;
                event.preventDefault();
                event.stopPropagation();
                deps.runPageTurnBench(isPrevBench ? 'prev' : 'next');
                return;
            }
        }

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

        // --- Undo (Ctrl+Z) & Redo (Ctrl+Y / Ctrl+Shift+Z) ---
        const wantsUndo = key === 'z' && !event.shiftKey;
        const wantsRedo = key === 'y' || (key === 'z' && event.shiftKey);
        if (!wantsUndo && !wantsRedo) return;

        const targetEl = event.target as HTMLElement | null;
        if (targetEl && (targetEl.id === 'search-input' || targetEl.closest('#pdf-find-bar') || targetEl.closest('#find-container'))) {
            return;
        }

        const scrollContainer = deps.getScrollContainer();
        const active = document.activeElement as HTMLElement | null;
        const inScope = !active || active === document.body ||
            active.closest(`#${VECTOR_CONTAINER_ID}`) ||
            active.closest('#pdf-content-wrapper') ||
            active.closest('#pdf-scroll-container') ||
            active.closest('[data-plugin-id="pdf-viewer"]') ||
            (scrollContainer && scrollContainer.contains(active));

        if (!inScope) return;

        event.preventDefault();
        event.stopPropagation();
        if (wantsUndo) {
            deps.undo();
        } else {
            deps.redo();
        }
    };
}
