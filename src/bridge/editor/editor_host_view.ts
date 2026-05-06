const TARGET_LAYER_ID = 'pdf-editor-target-layer-vector';
const EDITOR_SHELL_ID = 'pdf-editor-shell-vector';
const EDITOR_CANVAS_ID = 'pdf-editor-canvas-vector';
const EDITOR_TEXTAREA_ID = 'pdf-editor-textarea-vector';
const VECTOR_INTERACTION_ROOT_ID = 'pdf-interaction-root-vector';
const VECTOR_INTERACTION_LAYER_ID = 'pdf-interaction-layer';
const VECTOR_CONTAINER_ID = 'pdf-page-container';
const LEGACY_INTERACTION_ROOT_ID = 'pdf-interaction-root';
const PDF_CONTENT_WRAPPER_ID = 'pdf-content-wrapper';
const HOST_OVERLAY_IDS = [
    'pdf-text-layer',
    'pdf-floating-editor-container',
    'pdf-annotation-overlay',
    'pdf-comment-overlay',
    'pdf-search-overlay',
    'pdf-comment-target-overlay',
    'pdf-annotation-target-overlay',
] as const;
const SELECTION_SOURCE_IDS = [
    PDF_CONTENT_WRAPPER_ID,
    VECTOR_CONTAINER_ID,
    VECTOR_INTERACTION_LAYER_ID,
    VECTOR_INTERACTION_ROOT_ID,
    LEGACY_INTERACTION_ROOT_ID,
    'pdf-text-layer',
    TARGET_LAYER_ID,
] as const;
const EDITOR_NAVIGATION_KEYS = new Set(['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End']);
const PRIMARY_POINTERDOWN_DATA_KEY = 'pdfLastPrimaryPointerDownAt';
const PRIMARY_POINTERDOWN_DEDUPE_MS = 600;

export type ParagraphInteractionTarget = {
    paragraphId: string;
    regionId: string;
    pageIndex: number;
    text: string;
    left: number;
    top: number;
    width: number;
    height: number;
    fontFamily: string;
    fontSize: number;
    fontWeight: string;
    fontStyle: string;
    color: string;
    textDecoration: string;
};

export type ActiveEditorTarget = {
    paragraphId: string;
    regionId: string;
    pageIndex: number;
    text: string;
    left: number;
    top: number;
    width: number;
    height: number;
    fontFamily: string;
    fontSizePx: number;
    fontWeight: string;
    fontStyle: string;
    color: string;
    textDecoration: string;
    initialCaretIndex: number;
};

export type HostReferenceBox = {
    left: number;
    top: number;
    width: number;
    height: number;
};

export type EditorHostNodes = {
    root: HTMLElement;
    targetLayer: HTMLElement;
    shell: HTMLElement;
    canvas: HTMLCanvasElement;
    textarea: HTMLTextAreaElement;
    overlays: HTMLElement[];
};

type BeforeInputCommand = 'insert' | 'backspace' | 'delete';

type EditorHostViewDeps = {
    readCaretIndex: (textarea: HTMLTextAreaElement) => number;
    writeCaretIndex: (textarea: HTMLTextAreaElement, caretIndex: number) => void;
    onCommitRequested: () => void;
    onNavigationRequested: (
        command: string,
        textarea: HTMLTextAreaElement,
    ) => void;
    onBeforeInputRequested: (
        command: BeforeInputCommand,
        text: string | null,
        textarea: HTMLTextAreaElement,
    ) => void;
    onCompositionSyncRequested: (textarea: HTMLTextAreaElement) => void;
    shouldSuppressNativeInput: () => boolean;
    shouldSuppressBlurCommit: () => boolean;
    onBlurCommitSuppressed: (textarea: HTMLTextAreaElement) => void;
    onBlurCommitRequested: () => void;
    onShellPointerDown: (
        event: MouseEvent,
        shell: HTMLElement,
        textarea: HTMLTextAreaElement,
    ) => void;
    onTargetPointerDown: (
        target: ParagraphInteractionTarget,
        event: MouseEvent,
    ) => void;
    onRootPointerDown: (event: MouseEvent) => void;
    logNode: (name: string, payload: Record<string, unknown>) => void;
};

function markPrimaryPointerDown(element: HTMLElement): void {
    element.dataset[PRIMARY_POINTERDOWN_DATA_KEY] = `${Date.now()}`;
}

function shouldIgnoreCompatibilityMouseDown(element: HTMLElement): boolean {
    const raw = element.dataset[PRIMARY_POINTERDOWN_DATA_KEY];
    if (!raw) return false;
    const at = Number(raw);
    return Number.isFinite(at) && (Date.now() - at) <= PRIMARY_POINTERDOWN_DEDUPE_MS;
}

function bindPrimaryPress(
    element: HTMLElement,
    handler: (event: MouseEvent) => void,
): void {
    element.onpointerdown = (event: PointerEvent) => {
        markPrimaryPointerDown(element);
        handler(event);
    };
    element.onmousedown = (event: MouseEvent) => {
        if (shouldIgnoreCompatibilityMouseDown(element)) {
            return;
        }
        handler(event);
    };
}

function ensureInteractionRoot(): HTMLElement | null {
    let root = document.getElementById(VECTOR_INTERACTION_ROOT_ID) as HTMLElement | null;
    if (root) return root;

    const container = document.getElementById(VECTOR_CONTAINER_ID) as HTMLElement | null;
    if (!container) return null;

    let layer = document.getElementById(VECTOR_INTERACTION_LAYER_ID) as HTMLElement | null;
    if (!layer) {
        layer = document.createElement('div');
        layer.id = VECTOR_INTERACTION_LAYER_ID;
        layer.style.cssText = 'position:absolute;inset:0;pointer-events:auto;z-index:12000;';
        container.appendChild(layer);
    }

    root = document.getElementById(VECTOR_INTERACTION_ROOT_ID) as HTMLElement | null;
    if (!root) {
        root = document.createElement('div');
        root.id = VECTOR_INTERACTION_ROOT_ID;
        root.style.cssText = 'position:absolute;inset:0;pointer-events:auto;z-index:12000;';
        layer.appendChild(root);
    }

    return root;
}

export function ensureEditorHostView(deps: EditorHostViewDeps): EditorHostNodes | null {
    const root = ensureInteractionRoot();
    if (!root) return null;

    root.style.pointerEvents = 'auto';
    root.style.position = 'absolute';
    root.style.inset = '0';
    root.style.userSelect = 'none';
    (root.style as any).webkitUserSelect = 'none';

    // Host-only responsibility: suppress browser-native text selection visuals
    // across the interaction root and PDF overlay layers. If this is not owned
    // here, native DOM selection can leak into edit mode as a fake blue bar.
    let styleTag = document.getElementById('pdf-interaction-suppression-style');
    if (!styleTag) {
        styleTag = document.createElement('style');
        styleTag.id = 'pdf-interaction-suppression-style';
        document.head.appendChild(styleTag);
    }
    const selectionSelectors = [
        `#${PDF_CONTENT_WRAPPER_ID} *`,
        `#${VECTOR_CONTAINER_ID} *`,
        `#${VECTOR_INTERACTION_LAYER_ID} *`,
        `#${LEGACY_INTERACTION_ROOT_ID} *`,
        `#${VECTOR_INTERACTION_ROOT_ID} *`,
        '.pdf-interaction-target-layer *',
        ...HOST_OVERLAY_IDS.map((id) => `#${id} *`),
    ];
    const hardSuppressionSelectors = SELECTION_SOURCE_IDS.flatMap((id) => [`#${id}`, `#${id} *`]);
    const hostSelectionCss = `
        ${hardSuppressionSelectors.join(',\n        ')} {
            user-select: none !important;
            -webkit-user-select: none !important;
            -webkit-touch-callout: none !important;
        }
        ${selectionSelectors.join(',\n        ')},
        ${selectionSelectors.map((selector) => `${selector}::selection`).join(',\n        ')} {
            background: transparent !important;
            background-color: transparent !important;
            color: inherit !important;
            -webkit-text-fill-color: currentColor !important;
        }
        #${EDITOR_TEXTAREA_ID},
        #${EDITOR_TEXTAREA_ID}::selection {
            background: transparent !important;
            background-color: transparent !important;
            color: transparent !important;
            caret-color: transparent !important;
            text-decoration: none !important;
            text-shadow: none !important;
            -webkit-text-fill-color: transparent !important;
        }
    `;
    if (styleTag.textContent !== hostSelectionCss) {
        styleTag.textContent = hostSelectionCss;
    }

    let targetLayer = document.getElementById(TARGET_LAYER_ID) as HTMLElement | null;
    if (!targetLayer) {
        targetLayer = document.createElement('div');
        targetLayer.id = TARGET_LAYER_ID;
        targetLayer.className = 'pdf-interaction-target-layer';
        targetLayer.style.cssText = 'position:absolute;inset:0;pointer-events:auto;z-index:12012;user-select:none;-webkit-user-select:none;';
        root.appendChild(targetLayer);
    }
    targetLayer.onselectstart = () => false;

    let shell = document.getElementById(EDITOR_SHELL_ID) as HTMLElement | null;
    if (!shell) {
        shell = document.createElement('div');
        shell.id = EDITOR_SHELL_ID;
        shell.style.cssText = 'position:absolute;display:none;z-index:12020;background:transparent;border:none;box-shadow:none;border-radius:0;padding:0;margin:0;box-sizing:border-box;pointer-events:auto;overflow:visible;cursor:text;';
        root.appendChild(shell);
    }

    let canvas = document.getElementById(EDITOR_CANVAS_ID) as HTMLCanvasElement | null;
    if (!canvas) {
        canvas = document.createElement('canvas');
        canvas.id = EDITOR_CANVAS_ID;
        canvas.style.cssText = 'position:absolute;left:0;top:0;width:100%;height:100%;display:block;pointer-events:none;';
        shell.appendChild(canvas);
    }

    let textarea = document.getElementById(EDITOR_TEXTAREA_ID) as HTMLTextAreaElement | null;
    if (!textarea) {
        textarea = document.createElement('textarea');
        textarea.id = EDITOR_TEXTAREA_ID;
        // Off-screen invisible textarea: only captures input/IME/caret. All visual
        // rendering (text, marker, caret position) is performed by the canvas via the
        // Rust PDF-Glyph paint backend so the editor visual is identical to the
        // original PDF (single render chain — no browser-font fork).
        textarea.style.cssText = 'position:fixed;left:-10000px;top:0;width:1px;height:1px;opacity:0;border:none;outline:none;resize:none;background:transparent;color:transparent;caret-color:transparent;text-decoration:none;padding:0;margin:0;overflow:hidden;line-height:1;box-sizing:border-box;white-space:pre-wrap;overflow-wrap:break-word;pointer-events:none;';
        shell.appendChild(textarea);
        bindTextareaEvents(textarea, deps);
    }
    textarea.style.setProperty('-webkit-text-fill-color', 'transparent', 'important');

    bindPrimaryPress(shell, (event: MouseEvent) => {
        event.preventDefault();
        event.stopPropagation();
        deps.onShellPointerDown(event, shell!, textarea!);
    });
    shell.onselectstart = () => false;

    root.onselectstart = () => false;
    bindPrimaryPress(root, (event: MouseEvent) => {
        if (event.defaultPrevented) return;
        deps.onRootPointerDown(event);
    });

    const overlays = HOST_OVERLAY_IDS
        .map((id) => document.getElementById(id) as HTMLElement | null)
        .filter((node): node is HTMLElement => !!node);

    return { root, targetLayer, shell, canvas, textarea, overlays };
}

export function hideEditorShell(nodes: EditorHostNodes): void {
    nodes.shell.style.display = 'none';
    nodes.textarea.value = '';
    nodes.canvas.width = 0;
    nodes.canvas.height = 0;
    restoreHostOverlays(nodes);
}

export function hideInteractionTargets(nodes: EditorHostNodes): void {
    nodes.targetLayer.innerHTML = '';
    nodes.targetLayer.style.display = 'none';
}

export function showInteractionTargets(nodes: EditorHostNodes): void {
    nodes.targetLayer.style.display = 'block';
}

export function snapshotHostOverlays(nodes: EditorHostNodes): Array<Record<string, unknown>> {
    return nodes.overlays.map((overlay) => ({
        id: overlay.id,
        display: overlay.style.display || '(empty)',
        childCount: overlay.childElementCount,
        hiddenForEditor: overlay.dataset.editorHidden === '1',
    }));
}

export function suspendHostOverlays(nodes: EditorHostNodes): void {
    for (const overlay of nodes.overlays) {
        if (overlay.dataset.editorHidden === '1') {
            continue;
        }
        overlay.dataset.editorPrevDisplay = overlay.style.display || '';
        overlay.dataset.editorHidden = '1';
        overlay.style.display = 'none';
    }
}

export function restoreHostOverlays(nodes: EditorHostNodes): void {
    for (const overlay of nodes.overlays) {
        if (overlay.dataset.editorHidden !== '1') {
            continue;
        }
        overlay.style.display = overlay.dataset.editorPrevDisplay || '';
        delete overlay.dataset.editorPrevDisplay;
        delete overlay.dataset.editorHidden;
    }
}

export function positionEditorShell(
    nodes: EditorHostNodes,
    target: Pick<
        ActiveEditorTarget,
        'left' | 'top' | 'width' | 'height' | 'fontFamily' | 'fontSizePx' | 'fontWeight' | 'fontStyle' | 'color'
        | 'textDecoration'
    >,
): void {
    nodes.shell.style.left = `${target.left}px`;
    nodes.shell.style.top = `${target.top}px`;
    nodes.shell.style.width = `${target.width}px`;
    nodes.shell.style.height = `${target.height}px`;
    nodes.canvas.style.width = `${target.width}px`;
    nodes.canvas.style.height = `${target.height}px`;
    nodes.textarea.style.fontFamily = target.fontFamily || 'serif';
    nodes.textarea.style.fontSize = `${Math.max(10, target.fontSizePx)}px`;
    nodes.textarea.style.fontWeight = target.fontWeight || 'normal';
    nodes.textarea.style.fontStyle = target.fontStyle || 'normal';
    nodes.textarea.style.color = 'transparent';
    nodes.textarea.style.caretColor = 'transparent';
    nodes.textarea.style.textDecoration = 'none';
    nodes.textarea.style.setProperty('-webkit-text-fill-color', 'transparent', 'important');
}

export function renderInteractionTargets(
    nodes: EditorHostNodes,
    targets: ParagraphInteractionTarget[],
    onTargetPointerDown: (target: ParagraphInteractionTarget, event: MouseEvent) => void,
): void {
    showInteractionTargets(nodes);
    nodes.targetLayer.innerHTML = '';
    for (const target of targets) {
        const box = document.createElement('div');
        box.dataset.paragraphId = target.paragraphId;
        box.style.cssText = [
            'position:absolute',
            `left:${target.left}px`,
            `top:${target.top}px`,
            `width:${target.width}px`,
            `height:${target.height}px`,
            'pointer-events:auto',
            'cursor:text',
            'background:transparent',
            'border:none',
            'border-radius:0',
            'user-select:none',
            '-webkit-user-select:none',
        ].join(';');
        bindPrimaryPress(box, (event: MouseEvent) => {
            event.preventDefault();
            event.stopPropagation();
            onTargetPointerDown(target, event);
        });
        nodes.targetLayer.appendChild(box);
    }
}

export function readHostReferenceBox(element: HTMLElement): HostReferenceBox {
    const rect = element.getBoundingClientRect();
    return {
        left: rect.left,
        top: rect.top,
        width: rect.width,
        height: rect.height,
    };
}

function bindTextareaEvents(textarea: HTMLTextAreaElement, deps: EditorHostViewDeps): void {
    let composing = false;

    textarea.addEventListener('keydown', (event: KeyboardEvent) => {
        if (event.key === 'Escape') {
            event.preventDefault();
            deps.onCommitRequested();
            return;
        }
        if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
            event.preventDefault();
            deps.onCommitRequested();
            return;
        }
        if (EDITOR_NAVIGATION_KEYS.has(event.key)) {
            event.preventDefault();
            deps.onNavigationRequested(event.key, textarea);
        }
    });

    textarea.addEventListener('beforeinput', (event: InputEvent) => {
        const inputType = event.inputType || '';
        let command: BeforeInputCommand | null = null;
        let text: string | null = null;
        if (inputType === 'deleteContentBackward') {
            command = 'backspace';
        } else if (inputType === 'deleteContentForward') {
            command = 'delete';
        } else if (inputType === 'insertText' || inputType === 'insertLineBreak') {
            command = 'insert';
            text = inputType === 'insertLineBreak' ? '\n' : (event.data ?? '');
        }
        if (!command) return;

        event.preventDefault();
        deps.logNode('ts.beforeinput', {
            inputType,
            command,
            text,
            textareaValue: textarea.value,
            selectionStart: textarea.selectionStart,
            selectionEnd: textarea.selectionEnd,
            caretIndex: deps.readCaretIndex(textarea),
        });
        deps.onBeforeInputRequested(command, text, textarea);
    });

    textarea.addEventListener('compositionstart', () => {
        composing = true;
    });

    textarea.addEventListener('compositionend', () => {
        composing = false;
        deps.onCompositionSyncRequested(textarea);
    });

    textarea.addEventListener('blur', () => {
        if (deps.shouldSuppressBlurCommit()) {
            deps.onBlurCommitSuppressed(textarea);
            return;
        }
        deps.onBlurCommitRequested();
    });

    textarea.addEventListener('input', () => {
        if (deps.shouldSuppressNativeInput()) return;
        if (!composing) return;
        deps.onCompositionSyncRequested(textarea);
    });
}
