import type { RustRenderFrame } from '../render/frame_plan';
import type { PdfSaveResult } from '../document/document_edit_api';
import type { EditorFormatAction } from './editor_wasm_api';
import {
    facadeOpenEditor,
    facadeOpenRegionEditor,
    facadeSyncInput,
    facadeCommitEditor,
    facadeCommitSilent,
    facadeCloseEditor,
    facadeMoveCaret,
    facadeApplyFormat,
    facadeApplyCommand,
    facadePaintCanvas,
    facadeReadSnapshot,
    facadeSetEditMode,
    facadeHasSessionChanges,
    facadeUtf16ToCharIndex,
    facadeCharToUtf16Offset,
    type EditorFacadeResult,
    type EditorSnapshotResult,
    type EditorOpenRequest,
    type EditorOpenRegionRequest,
    type EditorSyncInputRequest,
    type EditorCommitRequest,
    type EditorMoveCaretRequest,
} from './editor_facade';
import { createEditorHostDiagnostics } from './editor_host_diagnostics';
import { syncEditorFormatButtons } from '../viewer/pdf_viewer_dom';
import {
    ensureEditorHostView,
    hideEditorShell as hideEditorShellView,
    hideInteractionTargets,
    positionEditorShell,
    readHostReferenceBox,
    renderInteractionTargets,
    snapshotHostOverlays,
    suspendHostOverlays,
    type ActiveEditorTarget,
    type HostReferenceBox,
    type ParagraphInteractionTarget,
} from './editor_host_view';

const EDITOR_TEXTAREA_ID = 'pdf-editor-textarea-vector';

type EditorHostDeps = {
    getWasmApi: () => any;
    getCurrentPath: () => string | null;
    getCurrentPage: () => number;
    getCurrentZoom: () => number;
    getPageWidth: () => number;
    getPageHeight: () => number;
    getVectorContainer: () => HTMLElement | null;
    buildRenderRequest: (reason?: 'default' | 'zoom' | 'editorVisibility' | 'documentMutation') => Record<string, number | string | boolean>;
    renderScheduledFrame: (frame: RustRenderFrame | null) => Promise<void>;
    saveEditorSession: () => Promise<PdfSaveResult>;
    syncViewerState?: () => void;
};

type EditorHost = {
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
    const diagnostics = createEditorHostDiagnostics(() => null);
    let suppressNativeInput = false;
    let lastRustCaretIndex: number | null = null;
    let suppressBlurCommitForSave = false;
    let suppressBlurCommitForOpen = false;

    function withSuppressedNativeInput<T>(fn: () => T): T {
        suppressNativeInput = true;
        try {
            return fn();
        } finally {
            suppressNativeInput = false;
        }
    }

    function readTextareaCaret(textarea: HTMLTextAreaElement): number {
        const utf16Offset = Math.max(0, textarea.selectionStart ?? textarea.value.length);
        const converted = facadeUtf16ToCharIndex(textarea.value, utf16Offset);
        return Number.isFinite(converted) ? Math.max(0, converted) : utf16Offset;
    }

    function clearDomSelection(blurEditorInput = false): void {
        try {
            window.getSelection?.()?.removeAllRanges();
            // Clear stale DOM selections without blurring the hidden editor input.
            // The Rust editor owns caret/selection visuals; the textarea only captures input.
            if (
                document.activeElement instanceof HTMLElement
                && (blurEditorInput || document.activeElement.id !== EDITOR_TEXTAREA_ID)
            ) {
                document.activeElement.blur();
            }
        } catch {
            // Some hosts partially implement Selection APIs; ignore safely.
        }
    }

    function rememberRustCaret(caretIndex: unknown): void {
        if (typeof caretIndex === 'number' && Number.isFinite(caretIndex) && caretIndex >= 0) {
            lastRustCaretIndex = Math.max(0, caretIndex);
        }
    }

    function writeTextareaCaret(textarea: HTMLTextAreaElement, caretIndex: number): void {
        const charIndex = Math.max(0, caretIndex);
        const converted = facadeCharToUtf16Offset(textarea.value, charIndex);
        const nextCaret = Number.isFinite(converted) ? Math.max(0, converted) : charIndex;
        textarea.selectionStart = nextCaret;
        textarea.selectionEnd = nextCaret;
        try {
            textarea.setSelectionRange(nextCaret, nextCaret);
        } catch {
            // Hidden textarea selection APIs can be partially supported depending on host.
        }
    }

    function resolveRustOwnedOpenCaret(snapshot: EditorSnapshotResult | null | undefined): number {
        const snapshotCaret = snapshot?.caretIndex;
        if (Number.isFinite(snapshotCaret) && (snapshotCaret as number) >= 0) {
            return Math.max(0, snapshotCaret as number);
        }
        return 0;
    }

    function scanBlueRunInCanvas(
        canvas: HTMLCanvasElement | null,
        box: { left: number; top: number; width: number; height: number },
    ): Record<string, unknown> | null {
        if (!canvas || box.width <= 0 || box.height <= 0) return null;
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        if (!ctx) return null;

        const cssWidth = canvas.clientWidth || 1;
        const cssHeight = canvas.clientHeight || 1;
        const scaleX = canvas.width / cssWidth;
        const scaleY = canvas.height / cssHeight;
        const left = Math.max(0, Math.floor(box.left * scaleX));
        const top = Math.max(0, Math.floor((box.top - 8) * scaleY));
        const width = Math.max(1, Math.ceil(box.width * scaleX));
        const height = Math.max(1, Math.ceil((box.height + 16) * scaleY));
        const safeWidth = Math.min(width, canvas.width - left);
        const safeHeight = Math.min(height, canvas.height - top);
        if (safeWidth <= 0 || safeHeight <= 0) return null;

        let imageData: ImageData;
        try {
            imageData = ctx.getImageData(left, top, safeWidth, safeHeight);
        } catch {
            return null;
        }

        let bestRun = 0;
        let bestRow = 0;
        for (let row = 0; row < safeHeight; row += 1) {
            let currentRun = 0;
            for (let col = 0; col < safeWidth; col += 1) {
                const offset = (row * safeWidth + col) * 4;
                const r = imageData.data[offset];
                const g = imageData.data[offset + 1];
                const b = imageData.data[offset + 2];
                const a = imageData.data[offset + 3];
                const isBlue = a > 160 && b > 130 && g > 60 && r < 90;
                if (isBlue) {
                    currentRun += 1;
                    if (currentRun > bestRun) {
                        bestRun = currentRun;
                        bestRow = row;
                    }
                } else {
                    currentRun = 0;
                }
            }
        }

        return {
            runCssWidth: Math.round((bestRun / scaleX) * 10) / 10,
            rowCssTop: Math.round((bestRow / scaleY) * 10) / 10,
            thresholdHit: bestRun / scaleX >= box.width * 0.45,
        };
    }

    function scanActiveBlueEvidence(target: ActiveEditorTarget): Record<string, unknown> {
        const mainCanvas = document.getElementById('pdf-vector-main-canvas') as HTMLCanvasElement | null;
        const detailCanvas = document.getElementById('pdf-vector-detail-canvas') as HTMLCanvasElement | null;
        const editorCanvas = document.getElementById('pdf-editor-canvas-vector') as HTMLCanvasElement | null;
        return {
            paragraphId: target.paragraphId,
            targetLeft: target.left,
            targetTop: target.top,
            targetWidth: target.width,
            targetHeight: target.height,
            main: scanBlueRunInCanvas(mainCanvas, target),
            detail: scanBlueRunInCanvas(detailCanvas, target),
            editor: scanBlueRunInCanvas(editorCanvas, {
                left: 0,
                top: 0,
                width: target.width,
                height: target.height,
            }),
            domSelectionText: window.getSelection?.()?.toString() ?? '',
        };
    }

    function scheduleOpenFocusStabilization(nodes: NonNullable<ReturnType<typeof ensureNodes>>): void {
        const stabilize = () => {
            if (document.activeElement !== nodes.textarea) {
                try {
                    nodes.textarea.focus();
                } catch {
                    // Ignore host focus failures here; the main goal is avoiding an immediate blur-commit loop.
                }
            }
            suppressBlurCommitForOpen = false;
            diagnostics.logNode('ts.open.focus-stabilized', {
                activeElementIsTextarea: document.activeElement === nodes.textarea,
            });
        };
        window.requestAnimationFrame(() => {
            window.setTimeout(stabilize, 120);
        });
    }

    function buildPointerPayload(
        event: MouseEvent,
        referenceBox: HostReferenceBox,
    ): Record<string, number> {
        return {
            clientX: event.clientX,
            clientY: event.clientY,
            referenceLeft: referenceBox.left,
            referenceTop: referenceBox.top,
            referenceWidth: referenceBox.width,
            referenceHeight: referenceBox.height,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
        };
    }

    function resolveTargetReferenceBox(
        target: ParagraphInteractionTarget,
        event: MouseEvent,
        root: HTMLElement,
    ): HostReferenceBox {
        const eventTarget = event.currentTarget as HTMLElement | null;
        const eventTargetMatchesParagraph = eventTarget?.dataset?.paragraphId === target.paragraphId;
        const targetRect = eventTargetMatchesParagraph ? eventTarget.getBoundingClientRect() : null;
        const rootRect = root.getBoundingClientRect();
        return {
            left: targetRect?.left ?? (rootRect.left + target.left),
            top: targetRect?.top ?? (rootRect.top + target.top),
            width: targetRect?.width ?? target.width,
            height: targetRect?.height ?? target.height,
        };
    }

    function scheduleRustDiagnosticsFlush(reason: string): void {
        const frameSchedule = typeof window !== 'undefined' && typeof window.requestAnimationFrame === 'function'
            ? window.requestAnimationFrame.bind(window)
            : (callback: FrameRequestCallback) => window.setTimeout(() => callback(performance.now()), 0);

        frameSchedule(() => {
            diagnostics.logRustDiagnostics(`${reason}:raf`);
            window.setTimeout(() => {
                diagnostics.logRustDiagnostics(`${reason}:timeout`);
            }, 0);
        });
    }

    async function runFacadeRender(result: EditorFacadeResult | null | undefined): Promise<void> {
        if (!result?.renderFrame) return;
        await deps.renderScheduledFrame(result.renderFrame as RustRenderFrame | null);
        deps.syncViewerState?.();
    }

    async function syncEditorInputState(text: string, caretIndex: number): Promise<EditorFacadeResult | null> {
        const result = facadeSyncInput({ text, caretIndex: Math.max(0, caretIndex) });
        const nodes = ensureNodes();
        if (
            nodes &&
            Number.isFinite(result?.caretIndex) &&
            (result?.caretIndex ?? -1) >= 0
        ) {
            withSuppressedNativeInput(() => {
                writeTextareaCaret(nodes.textarea, result!.caretIndex!);
            });
        }
        await runFacadeRender(result);
        return result ?? null;
    }

    function applyEditorCommandViaRustState(
        command: string,
        insertedText: string | null,
        text: string,
        caretIndex: number,
    ): EditorFacadeResult | null {
        const snapshot = readEditorSnapshot(getLastDisplayZoom());
        const rustText = snapshot?.draftText ?? null;
        if (rustText == null) return null;
        const rustCaretIndex = Math.max(0, lastRustCaretIndex ?? snapshot?.caretIndex ?? caretIndex);
        // Apply the command (delete/backspace/insert) via WASM API
        const result = facadeApplyCommand({ command, insertedText });
        if (typeof result?.caretIndex === 'number' && result.caretIndex >= 0) {
            rememberRustCaret(result.caretIndex);
            const nodes = ensureNodes();
            if (nodes && result.draftText) {
                withSuppressedNativeInput(() => {
                    nodes.textarea.value = result.draftText!;
                    writeTextareaCaret(nodes.textarea, result.caretIndex!);
                });
            }
        }
        return result ?? null;
    }

    let cachedDisplayZoom: number = 1.0;

    function getLastDisplayZoom(): number {
        return cachedDisplayZoom > 0 ? cachedDisplayZoom : 1.0;
    }

    function readEditorSnapshot(displayZoom = getLastDisplayZoom()): EditorSnapshotResult | null {
        try {
            const snapshot = facadeReadSnapshot(displayZoom);
            return snapshot && typeof snapshot === 'object' ? snapshot : null;
        } catch {
            return null;
        }
    }

    function readRequiredRustDraftText(
        snapshot: EditorSnapshotResult | null | undefined,
        _reason: string,
        _details: Record<string, unknown> = {},
    ): string | null {
        if (typeof snapshot?.draftText === 'string') {
            return snapshot.draftText;
        }
        return null;
    }

    function isTextEditEnabled(): boolean {
        return !!readEditorSnapshot(getLastDisplayZoom())?.enabled;
    }

    function enableTextEditModeForPointer(displayZoom = getLastDisplayZoom()): EditorSnapshotResult | null {
        const before = readEditorSnapshot(displayZoom);
        if (before?.enabled) return before;
        facadeSetEditMode(true);
        syncTargets(displayZoom);
        return readEditorSnapshot(displayZoom);
    }

    function ensureNodes() {
        return ensureEditorHostView({
            readCaretIndex: readTextareaCaret,
            writeCaretIndex: writeTextareaCaret,
            onCommitRequested: () => {
                void commitEditor();
            },
            onNavigationRequested: (command, textarea) => {
                const result = applyEditorCommandViaRustState(
                    command,
                    null,
                    textarea.value,
                    readTextareaCaret(textarea),
                );
                if (Number.isFinite(result?.caretIndex) && (result?.caretIndex ?? -1) >= 0) {
                    withSuppressedNativeInput(() => {
                        writeTextareaCaret(textarea, result!.caretIndex!);
                    });
                }
                void runFacadeRender(result as EditorFacadeResult | null | undefined).then(() => {
                    renderActiveEditor(getLastDisplayZoom());
                });
            },
            onBeforeInputRequested: (command, text, textarea) => {
                const result = applyEditorCommandViaRustState(
                    command,
                    text,
                    textarea.value,
                    readTextareaCaret(textarea),
                );
                void runFacadeRender(result as EditorFacadeResult | null | undefined).then(() => {
                    renderActiveEditor(getLastDisplayZoom());
                });
            },
            onCompositionSyncRequested: (textarea) => {
                const caretIndex = readTextareaCaret(textarea);
                void syncEditorInputState(textarea.value, caretIndex).then(() => {
                    renderActiveEditor(getLastDisplayZoom());
                });
            },
            shouldSuppressNativeInput: () => suppressNativeInput,
            shouldSuppressBlurCommit: () =>
                suppressBlurCommitForSave
                || suppressBlurCommitForOpen
                || !readEditorSnapshot()?.activeTarget
                || !facadeHasSessionChanges(),
            onBlurCommitSuppressed: (textarea) => {
                diagnostics.logNode('ts.blur.commit-suppressed', {
                    textareaValue: textarea.value,
                    caretIndex: readTextareaCaret(textarea),
                    suppressForSave: suppressBlurCommitForSave,
                    suppressForOpen: suppressBlurCommitForOpen,
                    sessionDirty: facadeHasSessionChanges(),
                });
            },
            onBlurCommitRequested: () => {
                if (!facadeHasSessionChanges()) {
                    diagnostics.logNode('ts.blur.commit-suppressed', {
                        reason: 'clean-session-before-commit',
                        suppressForSave: suppressBlurCommitForSave,
                        suppressForOpen: suppressBlurCommitForOpen,
                        sessionDirty: facadeHasSessionChanges(),
                    });
                    return;
                }
                diagnostics.logNode('ts.blur.commit-requested', {
                    suppressForSave: suppressBlurCommitForSave,
                    suppressForOpen: suppressBlurCommitForOpen,
                    sessionDirty: facadeHasSessionChanges(),
                });
                void commitEditor();
            },
            onShellPointerDown: (event, shell, textarea) => {
                const referenceBox = readHostReferenceBox(shell);
                diagnostics.logNode('ts.shell-mousedown.input', {
                    clientX: event.clientX,
                    clientY: event.clientY,
                    referenceLeft: referenceBox.left,
                    referenceTop: referenceBox.top,
                    referenceWidth: referenceBox.width,
                    referenceHeight: referenceBox.height,
                    textareaValue: textarea.value,
                    selectionStart: textarea.selectionStart,
                    selectionEnd: textarea.selectionEnd,
                });
                const moveRequest: EditorMoveCaretRequest = {
                    clientX: event.clientX,
                    clientY: event.clientY,
                    referenceLeft: referenceBox.left,
                    referenceTop: referenceBox.top,
                    referenceWidth: referenceBox.width,
                    referenceHeight: referenceBox.height,
                    pageWidth: deps.getPageWidth(),
                    pageHeight: deps.getPageHeight(),
                };
                const caretResult = facadeMoveCaret(moveRequest);
                const nextCaret = caretResult?.caretIndex ?? 0;
                diagnostics.logNode('ts.shell-mousedown.result', { nextCaret });
                textarea.focus();
                if (Number.isFinite(nextCaret) && nextCaret >= 0) {
                    rememberRustCaret(nextCaret);
                    writeTextareaCaret(textarea, nextCaret);
                    renderActiveEditor(getLastDisplayZoom());
                }
            },
            onTargetPointerDown: (target, event) => {
                diagnostics.logNode('ts.target.pointerdown', {
                    paragraphId: target.paragraphId,
                    targetText: target.text,
                    clientX: event.clientX,
                    clientY: event.clientY,
                });
                void openEditor(target, event);
            },
            onRootPointerDown: (event) => {
                const nodes = ensureNodes();
                if (!nodes) return;
                const snapshotBefore = readEditorSnapshot(getLastDisplayZoom());
                if (!snapshotBefore?.enabled) {
                    enableTextEditModeForPointer(getLastDisplayZoom());
                    syncFormatButtons();
                }
                const snapAfter = readEditorSnapshot(getLastDisplayZoom());
                const targetCount = snapAfter?.targets?.length ?? 0;
                console.log('[EDITOR-DIAG] root.pointerdown', {
                    clientX: event.clientX,
                    clientY: event.clientY,
                    enabledBefore: !!snapshotBefore?.enabled,
                    enabledAfter: !!snapAfter?.enabled,
                    targetCount,
                    pageW: deps.getPageWidth(),
                    pageH: deps.getPageHeight(),
                });
                diagnostics.logNode('ts.root.pointerdown', {
                    clientX: event.clientX,
                    clientY: event.clientY,
                    targetCount,
                    delegatedToRustHitTest: true,
                });
                event.preventDefault();
                event.stopPropagation();
                void openEditorFromRootPoint(event);
            },
            logNode: diagnostics.logNode,
        });
    }

    function hideEditorShell(): void {
        const nodes = ensureNodes();
        if (!nodes) return;
        clearDomSelection(true);
        hideEditorShellView(nodes);
        clearDomSelection(true);
        diagnostics.logNode('ts.overlay.restore', {
            overlays: snapshotHostOverlays(nodes),
        });
    }

    function hideTargetsForActiveEdit(nodes: NonNullable<ReturnType<typeof ensureNodes>>): void {
        hideInteractionTargets(nodes);
        diagnostics.logNode('ts.target-layer.hidden', {
            displayed: nodes.targetLayer.style.display !== 'none',
            childCount: nodes.targetLayer.childElementCount,
        });
    }

    function syncFormatButtons(): void {
        // Format state is now handled by facade
        const formatState = {
            bold: false,
            italic: false,
            underline: false,
            color: '#111827',
            fontFamily: 'Microsoft YaHei',
            fontSize: 12,
            charSpacing: 0,
            lineHeight: 1.2,
            alignment: 'left',
            listKind: 'none',
        };
        syncEditorFormatButtons(formatState);
    }

    async function applyEditorFormatAction(
        label: string,
        action: () => unknown,
        details?: Record<string, unknown>,
    ): Promise<void> {
        const result = action() as EditorFacadeResult | null | undefined;
        diagnostics.logNode(`ts.format.${label}`, { ...details, result });
        await runFacadeRender(result);
        syncTargets(getLastDisplayZoom());
        syncFormatButtons();
    }

    function closeEditor(clearActive = true): void {
        hideEditorShell();
        lastRustCaretIndex = null;
        if (clearActive) {
            const result = facadeCloseEditor();
            diagnostics.logNode('ts.close', { clearActive, result });
            void runFacadeRender(result).then(() => {
                syncTargets(getLastDisplayZoom());
                syncFormatButtons();
            });
        } else {
            diagnostics.logNode('ts.close', { clearActive });
            syncFormatButtons();
        }
    }

    async function commitEditor(): Promise<void> {
        const initialSnapshot = readEditorSnapshot();
        if (!initialSnapshot?.activeTarget && initialSnapshot?.draftText == null) return;
        const began = true; // Facade handles begin/finish internally
        if (!began) return;
        const nodes = ensureNodes();
        if (!nodes) {
            // editorApi.finishCommit();
            return;
        }
        const path = deps.getCurrentPath();
        if (!path) {
            closeEditor(true);
            // editorApi.finishCommit();
            return;
        }
        try {
            const snapshot = readEditorSnapshot(getLastDisplayZoom());
            const draftText = readRequiredRustDraftText(snapshot, 'commit', {
                textareaValue: nodes.textarea.value,
            });
            if (draftText == null) return;
            const caretIndex = Math.max(0, lastRustCaretIndex ?? snapshot?.caretIndex ?? readTextareaCaret(nodes.textarea));
            const commitResult = facadeCommitEditor({ draftText, caretIndex });
            const committed = !!commitResult?.changed;
            hideEditorShell();
            diagnostics.logNode('ts.commit', { draftText, textareaValue: nodes.textarea.value, caretIndex, commitResult, committed });
            await runFacadeRender(commitResult);
            syncTargets(getLastDisplayZoom());
            syncFormatButtons();
        } finally {
            // editorApi.finishCommit();
        }
    }

    async function commitForSave(): Promise<void> {
        const initialSnapshot = readEditorSnapshot();
        if (!initialSnapshot?.activeTarget && initialSnapshot?.draftText == null) return;
        const began = true; // Facade handles begin/finish internally
        if (!began) return;
        const nodes = ensureNodes();
        if (!nodes) {
            // editorApi.finishCommit();
            return;
        }
        try {
            const snapshot = readEditorSnapshot(getLastDisplayZoom());
            const draftText = readRequiredRustDraftText(snapshot, 'save-commit', {
                textareaValue: nodes.textarea.value,
            });
            if (draftText == null) return;
            const caretIndex = Math.max(0, lastRustCaretIndex ?? snapshot?.caretIndex ?? readTextareaCaret(nodes.textarea));
            const commitResult = facadeCommitSilent({ draftText, caretIndex });
            hideEditorShell();
            diagnostics.logNode('ts.save.commit-silent', {
                draftText,
                textareaValue: nodes.textarea.value,
                caretIndex,
                commitResult,
            });
            await runFacadeRender(commitResult);
            syncTargets(getLastDisplayZoom());
            syncFormatButtons();
        } finally {
            // editorApi.finishCommit();
        }
    }

    function renderActiveEditor(displayZoom = getLastDisplayZoom()): void {
        const nodes = ensureNodes();
        if (!nodes) return;
        const snapshot = readEditorSnapshot(displayZoom);
        if (!snapshot?.activeTarget) return;
        positionEditorShell(nodes, snapshot.activeTarget);
        diagnostics.logNode('ts.shell.positioned', {
            paragraphId: snapshot.activeTarget.paragraphId,
            targetLeft: snapshot.activeTarget.left,
            targetTop: snapshot.activeTarget.top,
            targetWidth: snapshot.activeTarget.width,
            targetHeight: snapshot.activeTarget.height,
        });
        const draftText = readRequiredRustDraftText(snapshot, 'render-active-editor');
        if (draftText == null) return;
        const caretIndex = Math.max(
            0,
            (Number.isFinite(lastRustCaretIndex) && (lastRustCaretIndex as number) >= 0)
                ? (lastRustCaretIndex as number)
                : (snapshot.caretIndex ?? draftText.length),
        );
        rememberRustCaret(caretIndex);
        if (nodes.textarea.value !== draftText) {
            withSuppressedNativeInput(() => {
                nodes.textarea.value = draftText;
            });
        }
        withSuppressedNativeInput(() => {
            writeTextareaCaret(nodes.textarea, caretIndex);
        });
        // Paint the editor's own canvas (inside shell) via the Rust PDF-Glyph
        // backend through the editor facade. The page-level renderFrame paints
        // the main page canvas but NOT this shell-internal canvas; without this
        // call the editor canvas stays blank in edit mode (the textarea is
        // off-screen by design — see docs/editor-render-architecture.md).
        facadePaintCanvas(nodes.canvas, displayZoom, draftText, caretIndex);
        diagnostics.logNode('ts.render-active-editor', {
            displayZoom,
            draftText,
            caretIndex,
            textareaValue: nodes.textarea.value,
            selectionStart: nodes.textarea.selectionStart,
            selectionEnd: nodes.textarea.selectionEnd,
        });
        diagnostics.logNode('ts.blue-scan', scanActiveBlueEvidence(snapshot.activeTarget));
        diagnostics.logRustDiagnostics('renderActiveEditor');
        scheduleRustDiagnosticsFlush('renderActiveEditor');
        syncFormatButtons();
    }

    async function applyEditorFormat(action: EditorFormatAction): Promise<void> {
        const label = action.type;
        await applyEditorFormatAction(
            label,
            () => facadeApplyFormat(action),
            action as unknown as Record<string, unknown>,
        );
    }

    async function openEditor(target: ParagraphInteractionTarget, event: MouseEvent): Promise<void> {
        const nodes = ensureNodes();
        if (!nodes) return;
        suppressBlurCommitForOpen = true;
        clearDomSelection();

        const displayZoom = Math.max(0.1, getLastDisplayZoom());
        const referenceBox = resolveTargetReferenceBox(target, event, nodes.root);
        diagnostics.logNode('ts.open.input', {
            paragraphId: target.paragraphId,
            targetText: target.text,
            targetColor: target.color,
            targetTextDecoration: target.textDecoration,
            clientX: event.clientX,
            clientY: event.clientY,
            targetClientLeft: referenceBox.left,
            targetClientTop: referenceBox.top,
            targetWidth: referenceBox.width,
            targetHeight: referenceBox.height,
            displayZoom,
        });
        const openRequest: EditorOpenRequest = {
            paragraphId: target.paragraphId,
            clientX: event.clientX,
            clientY: event.clientY,
            referenceLeft: referenceBox.left,
            referenceTop: referenceBox.top,
            referenceWidth: referenceBox.width,
            referenceHeight: referenceBox.height,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
        };
        const openedResult = facadeOpenEditor(openRequest);
        diagnostics.logNode('ts.open.result', { openedResult });
        const opened = !!openedResult?.changed;

        if (!opened) {
            hideEditorShell();
            return;
        }
        await runFacadeRender(openedResult);
        const activeTarget = readEditorSnapshot()?.activeTarget;
        if (!activeTarget) {
            hideEditorShell();
            return;
        }

        positionEditorShell(nodes, activeTarget);
        diagnostics.logNode('ts.shell.positioned', {
            paragraphId: activeTarget.paragraphId,
            targetLeft: activeTarget.left,
            targetTop: activeTarget.top,
            targetWidth: activeTarget.width,
            targetHeight: activeTarget.height,
        });
        const openedSnapshot = readEditorSnapshot();
        const draftText = readRequiredRustDraftText(openedSnapshot, 'open-editor', {
            paragraphId: activeTarget.paragraphId,
            targetText: activeTarget.text,
        });
        if (draftText == null) {
            hideEditorShell();
            return;
        }
        withSuppressedNativeInput(() => {
            nodes.textarea.value = draftText;
        });
        diagnostics.logNode('ts.overlay.snapshot.before-open', {
            overlays: snapshotHostOverlays(nodes),
        });
        hideTargetsForActiveEdit(nodes);
        suspendHostOverlays(nodes);
        clearDomSelection();
        diagnostics.logNode('ts.overlay.snapshot.after-open', {
            overlays: snapshotHostOverlays(nodes),
        });
        nodes.shell.style.display = 'block';
        nodes.textarea.focus();
        clearDomSelection();
        const caretIndex = resolveRustOwnedOpenCaret(openedSnapshot);
        diagnostics.logNode('ts.open.rust-caret', {
            clientX: event.clientX,
            clientY: event.clientY,
            snapshotCaretIndex: openedSnapshot?.caretIndex ?? null,
            activeInitialCaretIndex: openedSnapshot?.activeTarget?.initialCaretIndex ?? null,
            resolvedCaretIndex: caretIndex,
        });
        rememberRustCaret(caretIndex);
        withSuppressedNativeInput(() => {
            writeTextareaCaret(nodes.textarea, caretIndex);
        });
        diagnostics.logRustDiagnostics('openEditor');
        renderActiveEditor(getLastDisplayZoom());
        scheduleOpenFocusStabilization(nodes);
    }

    async function openEditorFromRootPoint(event: MouseEvent): Promise<void> {
        const nodes = ensureNodes();
        if (!nodes) return;
        suppressBlurCommitForOpen = true;
        clearDomSelection();

        const displayZoom = Math.max(0.1, getLastDisplayZoom());
        const referenceBox = readHostReferenceBox(nodes.root);
        diagnostics.logNode('ts.open.root.input', {
            clientX: event.clientX,
            clientY: event.clientY,
            targetClientLeft: referenceBox.left,
            targetClientTop: referenceBox.top,
            targetWidth: referenceBox.width,
            targetHeight: referenceBox.height,
            displayZoom,
        });
        const openRequest: EditorOpenRequest = {
            paragraphId: '',
            clientX: event.clientX,
            clientY: event.clientY,
            referenceLeft: referenceBox.left,
            referenceTop: referenceBox.top,
            referenceWidth: referenceBox.width,
            referenceHeight: referenceBox.height,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
        };
        const openedResult = facadeOpenEditor(openRequest);
        console.log('[EDITOR-DIAG] open.root.result', {
            changed: !!openedResult?.changed,
            enabled: !!openedResult?.enabled,
            caretIndex: openedResult?.caretIndex,
            openRequest,
        });
        diagnostics.logNode('ts.open.root.result', { openedResult });
        if (!openedResult?.changed) {
            console.warn('[EDITOR-DIAG] open.root: not-changed → hide');
            hideEditorShell();
            return;
        }
        await runFacadeRender(openedResult);
        const postOpenSnap = readEditorSnapshot();
        const activeTarget = postOpenSnap?.activeTarget;
        console.log('[EDITOR-DIAG] open.root.post-render', {
            hasActiveTarget: !!activeTarget,
            hasActiveTargetBool: (postOpenSnap as any)?.hasActiveTarget,
            paragraphId: activeTarget?.paragraphId,
            left: activeTarget?.left,
            top: activeTarget?.top,
            width: activeTarget?.width,
            height: activeTarget?.height,
            draftText: postOpenSnap?.draftText?.slice(0, 40),
            enabled: postOpenSnap?.enabled,
            snapKeys: postOpenSnap ? Object.keys(postOpenSnap) : null,
        });
        if (!activeTarget) {
            console.warn('[EDITOR-DIAG] open.root: no activeTarget → hide');
            hideEditorShell();
            return;
        }

        positionEditorShell(nodes, activeTarget);
        diagnostics.logNode('ts.shell.positioned', {
            paragraphId: activeTarget.paragraphId,
            targetLeft: activeTarget.left,
            targetTop: activeTarget.top,
            targetWidth: activeTarget.width,
            targetHeight: activeTarget.height,
        });
        const openedSnapshot = readEditorSnapshot();
        const draftText = readRequiredRustDraftText(openedSnapshot, 'open-editor-root', {
            paragraphId: activeTarget.paragraphId,
            targetText: activeTarget.text,
        });
        if (draftText == null) {
            console.warn('[EDITOR-DIAG] open.root: draftText null → hide');
            hideEditorShell();
            return;
        }
        withSuppressedNativeInput(() => {
            nodes.textarea.value = draftText;
        });
        hideTargetsForActiveEdit(nodes);
        suspendHostOverlays(nodes);
        clearDomSelection();
        nodes.shell.style.display = 'block';
        nodes.textarea.focus();
        clearDomSelection();
        const shellRect = nodes.shell.getBoundingClientRect();
        console.log('[EDITOR-DIAG] open.root.shell-shown', {
            display: nodes.shell.style.display,
            shellLeft: nodes.shell.style.left,
            shellTop: nodes.shell.style.top,
            shellWidth: nodes.shell.style.width,
            shellHeight: nodes.shell.style.height,
            rectLeft: shellRect.left,
            rectTop: shellRect.top,
            rectWidth: shellRect.width,
            rectHeight: shellRect.height,
            draftLen: draftText.length,
            focused: document.activeElement === nodes.textarea,
        });
        const caretIndex = resolveRustOwnedOpenCaret(openedSnapshot);
        rememberRustCaret(caretIndex);
        withSuppressedNativeInput(() => {
            writeTextareaCaret(nodes.textarea, caretIndex);
        });
        diagnostics.logRustDiagnostics('openEditorFromRootPoint');
        renderActiveEditor(getLastDisplayZoom());
        scheduleOpenFocusStabilization(nodes);
    }

    async function openRegionEditor(
        pageIndex: number,
        regionId: string,
        kind: string,
        originalText: string,
    ): Promise<void> {
        const nodes = ensureNodes();
        if (!nodes) return;
        suppressBlurCommitForOpen = true;
        clearDomSelection();

        const openedResult = facadeOpenRegionEditor({
            pageIndex,
            regionId,
            kind,
            originalText,
        });
        diagnostics.logNode('ts.open.region.result', {
            pageIndex,
            regionId,
            kind,
            originalText,
            openedResult,
        });
        if (!openedResult?.changed) {
            hideEditorShell();
            return;
        }

        await runFacadeRender(openedResult);
        const snapshot = readEditorSnapshot();
        const activeTarget = snapshot?.activeTarget;
        if (!activeTarget) {
            hideEditorShell();
            return;
        }

        positionEditorShell(nodes, activeTarget);
        diagnostics.logNode('ts.shell.positioned', {
            paragraphId: activeTarget.paragraphId,
            targetLeft: activeTarget.left,
            targetTop: activeTarget.top,
            targetWidth: activeTarget.width,
            targetHeight: activeTarget.height,
        });
        const draftText = readRequiredRustDraftText(snapshot, 'open-region-editor', {
            paragraphId: activeTarget.paragraphId,
            targetText: activeTarget.text,
        });
        if (draftText == null) {
            hideEditorShell();
            return;
        }
        const caretIndex = resolveRustOwnedOpenCaret(snapshot);
        rememberRustCaret(caretIndex);
        withSuppressedNativeInput(() => {
            nodes.textarea.value = draftText;
            writeTextareaCaret(nodes.textarea, caretIndex);
        });
        diagnostics.logNode('ts.overlay.snapshot.before-open', {
            overlays: snapshotHostOverlays(nodes),
        });
        hideTargetsForActiveEdit(nodes);
        suspendHostOverlays(nodes);
        diagnostics.logNode('ts.overlay.snapshot.after-open', {
            overlays: snapshotHostOverlays(nodes),
        });
        nodes.shell.style.display = 'block';
        nodes.textarea.focus();
        diagnostics.logRustDiagnostics('openRegionEditor');
        renderActiveEditor(getLastDisplayZoom());
        scheduleOpenFocusStabilization(nodes);
    }

    function syncTargets(displayZoom: number): void {
        // Display zoom is now cached locally
        const nodes = ensureNodes();
        if (!nodes) {
            diagnostics.logNode('ts.sync-targets.result', {
                reason: 'missing-nodes',
                displayZoom,
            });
            return;
        }
        const container = deps.getVectorContainer();
        if (!container) {
            diagnostics.logNode('ts.sync-targets.result', {
                reason: 'missing-vector-container',
                displayZoom,
            });
            hideTargetsForActiveEdit(nodes);
            return;
        }
        const snapshot = readEditorSnapshot(getLastDisplayZoom());
        if (!snapshot?.enabled) {
            diagnostics.logNode('ts.sync-targets.result', {
                reason: 'disabled',
                displayZoom,
                enabled: !!snapshot?.enabled,
                active: !!snapshot?.activeTarget,
                targetCount: Array.isArray(snapshot?.targets) ? snapshot.targets.length : 0,
            });
            hideTargetsForActiveEdit(nodes);
            hideEditorShell();
            syncFormatButtons();
            return;
        }
        if (snapshot.activeTarget) {
            diagnostics.logNode('ts.sync-targets.result', {
                reason: 'active-editor',
                displayZoom,
                enabled: true,
                active: true,
                paragraphId: snapshot.activeTarget.paragraphId,
            });
            hideTargetsForActiveEdit(nodes);
            positionEditorShell(nodes, snapshot.activeTarget);
            nodes.shell.style.display = 'block';
            if (document.activeElement !== nodes.textarea) {
                const draftText = readRequiredRustDraftText(snapshot, 'sync-targets-active', {
                    paragraphId: snapshot.activeTarget.paragraphId,
                    targetText: snapshot.activeTarget.text,
                });
                if (draftText == null) return;
                withSuppressedNativeInput(() => {
                    nodes.textarea.value = draftText;
                });
                const caretIndex = Math.max(
                    0,
                    (Number.isFinite(lastRustCaretIndex) && (lastRustCaretIndex as number) >= 0)
                        ? (lastRustCaretIndex as number)
                        : (snapshot.caretIndex ?? draftText.length),
                );
                rememberRustCaret(caretIndex);
                withSuppressedNativeInput(() => {
                    writeTextareaCaret(nodes.textarea, caretIndex);
                });
            }
            renderActiveEditor(getLastDisplayZoom());
        } else {
            diagnostics.logNode('ts.sync-targets.result', {
                reason: 'targets',
                displayZoom,
                enabled: true,
                active: false,
                targetCount: Array.isArray(snapshot.targets) ? snapshot.targets.length : 0,
            });
            renderInteractionTargets(
                nodes,
                Array.isArray(snapshot.targets) ? snapshot.targets : [],
                (target, event) => {
                    void openEditor(target, event);
                },
            );
            diagnostics.logNode('ts.target-layer.rendered', {
                displayed: nodes.targetLayer.style.display !== 'none',
                targetCount: Array.isArray(snapshot.targets) ? snapshot.targets.length : 0,
                childCount: nodes.targetLayer.childElementCount,
            });
            hideEditorShell();
            syncFormatButtons();
        }
    }

    async function saveEdits(): Promise<PdfSaveResult> {
        const path = deps.getCurrentPath();
        if (!path) {
            return { saved: false, errorMessage: 'missing-path' };
        }
        suppressBlurCommitForSave = true;
        try {
            await commitForSave();
            diagnostics.logNode('ts.save.start', {
                path,
                page: deps.getCurrentPage(),
                zoom: deps.getCurrentZoom(),
                hasPersistablePatches: !!readEditorSnapshot()?.hasPersistablePatches,
            });
            const result = await deps.saveEditorSession();
            diagnostics.logNode('ts.save.result', { result });
            if (!result?.saved) {
                const errorMessage = result?.errorMessage
                    ?? (result?.hadPersistablePatches ? '编辑器保存失败' : '没有可保存的修改');
                diagnostics.logNode('ts.save.error', { errorMessage, result });
                return {
                    saved: false,
                    hadPersistablePatches: result?.hadPersistablePatches,
                    errorMessage,
                };
            }
            diagnostics.logNode('ts.save.success', {
                page: deps.getCurrentPage(),
                zoom: deps.getCurrentZoom(),
            });
            return result;
        } finally {
            suppressBlurCommitForSave = false;
        }
    }

    return {
        syncTargets,
        clear: () => {
            const nodes = ensureNodes();
            if (!nodes) return;
            hideTargetsForActiveEdit(nodes);
            void commitEditor();
        },
        commitActiveEditor: commitEditor,
        saveEdits,
        applyFormatAction: applyEditorFormat,
        openRegionEditor,
        hasPendingEdits: () => !!readEditorSnapshot()?.hasPersistablePatches,
        setTextEditEnabled: (enabled: boolean) => {
            const result = facadeSetEditMode(enabled);
            diagnostics.logNode('ts.mode.set', {
                enabled: !!result?.enabled,
                changed: !!result?.changed,
                snapshotEnabled: !!readEditorSnapshot()?.enabled,
                displayZoom: getLastDisplayZoom(),
            });
            if (!enabled) {
                const nodes = ensureNodes();
                if (nodes) {
                    hideTargetsForActiveEdit(nodes);
                }
                void commitEditor();
            } else {
                syncTargets(getLastDisplayZoom());
            }
        },
        isTextEditEnabled,
    };
}
