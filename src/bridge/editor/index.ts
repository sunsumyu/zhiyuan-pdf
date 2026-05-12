import type { RustRenderFrame } from '../render/frame_plan';
import type { PdfSaveResult } from '../document/document_edit_api';
import type {
    EditorFormatAction,
    LegacyActiveTarget,
    LegacyInteractionTarget,
    LegacySnapshot,
    EditorResponse,
} from './types';
import * as api from './api';
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
    type EditorHostNodes,
    type HostReferenceBox,
    type ParagraphInteractionTarget,
} from './editor_host_view';
import { syncEditorFormatButtons } from '../viewer/pdf_viewer_dom';

const EDITOR_TEXTAREA_ID = 'pdf-editor-textarea-vector';

// ── Deps (injected by caller, same as old EditorHostDeps) ───────

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
    renderCurrentPage: (reason?: 'default' | 'zoom' | 'editorVisibility' | 'documentMutation') => Promise<void>;
    saveEditorSession: () => Promise<PdfSaveResult>;
    syncViewerState?: () => void;
};

// ── Public interface ────────────────────────────────────────────

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

// ── Implementation ──────────────────────────────────────────────

export function createEditorHost(deps: EditorHostDeps): EditorHost {
    let suppressNativeInput = false;
    let lastRustCaretIndex: number | null = null;
    let suppressBlurCommitForSave = false;
    let suppressBlurCommitForOpen = false;
    // Programmatic page-canvas re-renders during typing can transiently steal
    // focus from the editor textarea, firing a spurious blur that would
    // otherwise commit-and-close the editor on every keystroke. Suppress blur
    // commits while we drive a render and refocus immediately after.
    let suppressBlurCommitForRender = false;
    let cachedDisplayZoom = 1.0;

    // ── Textarea helpers (must stay in TS — DOM-only) ───────────

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
        const converted = api.utf16ToCharIndex(textarea.value, utf16Offset);
        return Number.isFinite(converted) ? Math.max(0, converted) : utf16Offset;
    }

    function writeTextareaCaret(textarea: HTMLTextAreaElement, caretIndex: number): void {
        const charIndex = Math.max(0, caretIndex);
        const converted = api.charToUtf16Offset(textarea.value, charIndex);
        const nextCaret = Number.isFinite(converted) ? Math.max(0, converted) : charIndex;
        textarea.selectionStart = nextCaret;
        textarea.selectionEnd = nextCaret;
        try {
            textarea.setSelectionRange(nextCaret, nextCaret);
        } catch {
            // Ignore
        }
    }

    function rememberRustCaret(caretIndex: unknown): void {
        if (typeof caretIndex === 'number' && Number.isFinite(caretIndex) && caretIndex >= 0) {
            lastRustCaretIndex = Math.max(0, caretIndex);
        }
    }

    function clearDomSelection(blurEditorInput = false): void {
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

    function getLastDisplayZoom(): number {
        return cachedDisplayZoom > 0 ? cachedDisplayZoom : 1.0;
    }

    // ── DOM node creation (delegates to editor_host_view.ts) ────

    function ensureNodes(): EditorHostNodes | null {
        return ensureEditorHostView({
            readCaretIndex: readTextareaCaret,
            writeCaretIndex: writeTextareaCaret,
            onCommitRequested: () => {
                void commitEditor().then(() => {
                    api.setEditMode(false);
                    syncTargets(getLastDisplayZoom());
                });
            },
            onNavigationRequested: (command, textarea) => {
                // Sync host caret to Rust before navigation — user may have clicked
                // inside the textarea to reposition cursor without Rust knowing.
                const hostCaret = readTextareaCaret(textarea);
                api.syncInput({ text: textarea.value, caretIndex: Math.max(0, hostCaret) });
                const result = api.applyCommand({ command, insertedText: null })?.data;
                if (result && Number.isFinite(result.caretIndex) && result.caretIndex >= 0) {
                    rememberRustCaret(result.caretIndex);
                    withSuppressedNativeInput(() => {
                        if (result.draftText != null) textarea.value = result.draftText;
                        writeTextareaCaret(textarea, result.caretIndex);
                    });
                }
                renderActiveEditor(getLastDisplayZoom());
            },
            onBeforeInputRequested: (command, text, textarea) => {
                // Sync host caret to Rust before applying command. This is critical
                // because the textarea allows native click-to-position which Rust
                // doesn't observe, leaving Rust's internal caret stale.
                const hostCaret = readTextareaCaret(textarea);
                api.syncInput({ text: textarea.value, caretIndex: Math.max(0, hostCaret) });
                const result = api.applyCommand({ command, insertedText: text })?.data;
                if (result && Number.isFinite(result.caretIndex) && result.caretIndex >= 0) {
                    rememberRustCaret(result.caretIndex);
                    withSuppressedNativeInput(() => {
                        if (result.draftText != null) textarea.value = result.draftText;
                        writeTextareaCaret(textarea, result.caretIndex);
                    });
                }
                renderActiveEditor(getLastDisplayZoom());
                if (result?.changed) {
                    // Page canvas re-render can transiently shift DOM/layout
                    // and pull focus off the textarea. Suppress blur-commit
                    // for the duration and refocus afterward so the user can
                    // keep typing without the editor closing each keystroke.
                    suppressBlurCommitForRender = true;
                    void deps.renderCurrentPage('editorVisibility').finally(() => {
                        try {
                            const ta = textarea;
                            if (ta && document.activeElement !== ta) {
                                ta.focus({ preventScroll: true });
                            }
                        } finally {
                            suppressBlurCommitForRender = false;
                        }
                    });
                }
            },
            onCompositionSyncRequested: (textarea) => {
                const caretIndex = readTextareaCaret(textarea);
                api.syncInput({ text: textarea.value, caretIndex: Math.max(0, caretIndex) });
                renderActiveEditor(getLastDisplayZoom());
            },
            shouldSuppressNativeInput: () => suppressNativeInput,
            shouldSuppressBlurCommit: () =>
                suppressBlurCommitForSave
                || suppressBlurCommitForOpen
                || suppressBlurCommitForRender,
            onBlurCommitSuppressed: () => {
                // No-op: blur suppressed by save/open flow
            },
            onBlurCommitRequested: () => {
                if (!api.hasSessionChanges()) return;
                void commitEditor();
            },
            onShellPointerDown: (event, shell, textarea) => {
                // FIX: reference box must span the full rendered page (matching
                // pageWidth/pageHeight), NOT the shell rect. Using the shell rect
                // here produced an incorrect scale (shellWidth / pageWidth ≪ true
                // display scale), making client→page transforms misplace the caret.
                // Use the interaction root, same as onRootPointerDown / openEditor.
                const nodes = ensureNodes();
                if (!nodes) return;
                const referenceBox = readHostReferenceBox(nodes.root);
                void shell; // shell no longer needed for reference math
                const result = api.moveCaret({
                    clientX: event.clientX,
                    clientY: event.clientY,
                    referenceLeft: referenceBox.left,
                    referenceTop: referenceBox.top,
                    referenceWidth: referenceBox.width,
                    referenceHeight: referenceBox.height,
                    pageWidth: deps.getPageWidth(),
                    pageHeight: deps.getPageHeight(),
                });

                if (!result?.ok || !result.data) {
                    // No active editor state (edge case) → close block → Viewing
                    void commitEditor().then(() => {
                        api.setEditMode(false);
                        syncTargets(getLastDisplayZoom());
                    });
                    return;
                }

                const nextCaret = result.data.caretIndex;
                textarea.focus();
                if (Number.isFinite(nextCaret) && nextCaret >= 0) {
                    rememberRustCaret(nextCaret);
                    writeTextareaCaret(textarea, nextCaret);
                    renderActiveEditor(getLastDisplayZoom());
                }
            },
            onTargetPointerDown: (target, event) => {
                void openEditor(target, event);
            },
            onRootPointerDown: (event) => {
                // New API flow: begin → hitTest → openBlock / discard
                const beginResult = api.begin();
                if (!beginResult?.ok) {
                    // Already in editing state — just do hit test
                }

                const nodes = ensureNodes();
                if (!nodes) return;

                event.preventDefault();
                event.stopPropagation();

                // Hit test via new API
                const referenceBox = readHostReferenceBox(nodes.root);
                const hitResult = api.hitTest({
                    clientX: event.clientX,
                    clientY: event.clientY,
                    referenceLeft: referenceBox.left,
                    referenceTop: referenceBox.top,
                    referenceWidth: referenceBox.width,
                    referenceHeight: referenceBox.height,
                    pageWidth: deps.getPageWidth(),
                    pageHeight: deps.getPageHeight(),
                });

                if (!hitResult?.ok || !hitResult.data?.blockId) {
                    // Hit miss → discard → Viewing
                    api.discard();
                    hideEditorShell();
                    syncTargets(getLastDisplayZoom());
                    return;
                }

                // Open the block via new API
                const openResult = api.openBlock({
                    blockId: hitResult.data.blockId,
                    clientX: event.clientX,
                    clientY: event.clientY,
                    referenceLeft: referenceBox.left,
                    referenceTop: referenceBox.top,
                    referenceWidth: referenceBox.width,
                    referenceHeight: referenceBox.height,
                    pageWidth: deps.getPageWidth(),
                    pageHeight: deps.getPageHeight(),
                });

                if (!openResult?.ok || !openResult.data) {
                    api.discard();
                    hideEditorShell();
                    return;
                }

                // Read legacy snapshot for positioning (legacy snapshot has DOM coords)
                const snapshot = readLegacySnapshot();
                const activeTarget = snapshot?.activeTarget;
                if (!activeTarget) {
                    api.discard();
                    hideEditorShell();
                    return;
                }

                setupActiveEditor(nodes, activeTarget, openResult.data.draftText, openResult.data.caretIndex);
            },
            logNode: () => {
                // Diagnostics removed — Rust structured logging handles this
            },
        });
    }

    // ── Core editor lifecycle ───────────────────────────────────

    function setupActiveEditor(
        nodes: EditorHostNodes,
        target: LegacyActiveTarget,
        draftText: string,
        caretIndex: number,
    ): void {
        positionEditorShell(nodes, target);
        withSuppressedNativeInput(() => {
            nodes.textarea.value = draftText;
        });
        hideInteractionTargets(nodes);
        suspendHostOverlays(nodes);
        clearDomSelection();
        nodes.shell.style.display = 'block';
        nodes.textarea.focus();
        clearDomSelection();
        rememberRustCaret(caretIndex);
        withSuppressedNativeInput(() => {
            writeTextareaCaret(nodes.textarea, caretIndex);
        });
        renderActiveEditor(getLastDisplayZoom());
        scheduleOpenFocusStabilization(nodes);
    }

    function scheduleOpenFocusStabilization(nodes: EditorHostNodes): void {
        suppressBlurCommitForOpen = true;
        window.requestAnimationFrame(() => {
            window.setTimeout(() => {
                if (document.activeElement !== nodes.textarea) {
                    try {
                        nodes.textarea.focus();
                    } catch {
                        // Ignore
                    }
                }
                if (lastRustCaretIndex !== null && Number.isFinite(lastRustCaretIndex)) {
                    withSuppressedNativeInput(() => {
                        writeTextareaCaret(nodes.textarea, lastRustCaretIndex as number);
                    });
                }
                suppressBlurCommitForOpen = false;
            }, 120);
        });
    }

    function hideEditorShell(): void {
        const nodes = ensureNodes();
        if (!nodes) return;
        clearDomSelection(true);
        hideEditorShellView(nodes);
        clearDomSelection(true);
    }

    function readLegacySnapshot(): LegacySnapshot | null {
        return api.readLegacySnapshot(getLastDisplayZoom());
    }

    function renderActiveEditor(displayZoom = getLastDisplayZoom()): void {
        const nodes = ensureNodes();
        if (!nodes) return;
        const snapshot = readLegacySnapshot();
        if (!snapshot?.activeTarget) return;
        positionEditorShell(nodes, snapshot.activeTarget);

        const draftText = snapshot.draftText;
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

        // Paint editor canvas via Rust glyph backend
        api.paintCanvas(nodes.canvas, displayZoom, draftText, caretIndex);
        syncFormatButtons();
    }

    async function commitEditor(): Promise<void> {
        const snapshot = readLegacySnapshot();
        if (!snapshot?.activeTarget && snapshot?.draftText == null) return;

        const nodes = ensureNodes();
        if (!nodes) return;
        const path = deps.getCurrentPath();
        if (!path) {
            closeEditor(true);
            return;
        }

        const draftText = snapshot?.draftText;
        if (draftText == null) return;

        const caretIndex = Math.max(
            0,
            lastRustCaretIndex ?? snapshot?.caretIndex ?? readTextareaCaret(nodes.textarea),
        );

        // Use new API commit
        const result = api.commit({ draftText, caretIndex });
        hideEditorShell();
        syncTargets(getLastDisplayZoom());
        syncFormatButtons();
    }

    async function commitForSave(): Promise<void> {
        const snapshot = readLegacySnapshot();
        if (!snapshot?.activeTarget && snapshot?.draftText == null) return;

        const nodes = ensureNodes();
        if (!nodes) return;

        const draftText = snapshot?.draftText;
        if (draftText == null) return;

        const caretIndex = Math.max(
            0,
            lastRustCaretIndex ?? snapshot?.caretIndex ?? readTextareaCaret(nodes.textarea),
        );

        api.commit({ draftText, caretIndex });
        hideEditorShell();
        syncTargets(getLastDisplayZoom());
        syncFormatButtons();
    }

    function closeEditor(clearActive = true): void {
        hideEditorShell();
        lastRustCaretIndex = null;
        if (clearActive) {
            api.closeBlock();
        }
        syncTargets(getLastDisplayZoom());
        syncFormatButtons();
    }

    function syncFormatButtons(): void {
        syncEditorFormatButtons({
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
        });
    }

    // ── Open editor from target click ───────────────────────────

    async function openEditor(target: ParagraphInteractionTarget, event: MouseEvent): Promise<void> {
        const nodes = ensureNodes();
        if (!nodes) return;
        suppressBlurCommitForOpen = true;
        clearDomSelection();

        // Ensure edit mode is on
        const beginResult = api.begin();
        // begin may fail if already in editing state — that's ok

        const referenceBox = resolveTargetReferenceBox(target, event, nodes.root);
        const hitResult = api.hitTest({
            clientX: event.clientX,
            clientY: event.clientY,
            referenceLeft: referenceBox.left,
            referenceTop: referenceBox.top,
            referenceWidth: referenceBox.width,
            referenceHeight: referenceBox.height,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
        });

        const blockId = hitResult?.data?.blockId ?? target.paragraphId;

        const openResult = api.openBlock({
            blockId,
            clientX: event.clientX,
            clientY: event.clientY,
            referenceLeft: referenceBox.left,
            referenceTop: referenceBox.top,
            referenceWidth: referenceBox.width,
            referenceHeight: referenceBox.height,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
        });

        if (!openResult?.ok || !openResult.data) {
            hideEditorShell();
            return;
        }

        const snapshot = readLegacySnapshot();
        const activeTarget = snapshot?.activeTarget;
        if (!activeTarget) {
            hideEditorShell();
            return;
        }

        setupActiveEditor(nodes, activeTarget, openResult.data.draftText, openResult.data.caretIndex);
    }

    function resolveTargetReferenceBox(
        target: ParagraphInteractionTarget,
        event: MouseEvent,
        root: HTMLElement,
    ): HostReferenceBox {
        const rootRect = root.getBoundingClientRect();
        return {
            left: rootRect.left,
            top: rootRect.top,
            width: rootRect.width,
            height: rootRect.height,
        };
    }

    // ── Sync targets / save ─────────────────────────────────────

    function syncTargets(displayZoom: number): void {
        cachedDisplayZoom = displayZoom;
        const nodes = ensureNodes();
        if (!nodes) return;
        const container = deps.getVectorContainer();
        if (!container) {
            hideInteractionTargets(nodes);
            return;
        }

        const snapshot = readLegacySnapshot();
        if (!snapshot?.enabled) {
            hideInteractionTargets(nodes);
            hideEditorShell();
            syncFormatButtons();
            return;
        }

        if (snapshot.activeTarget) {
            hideInteractionTargets(nodes);
            positionEditorShell(nodes, snapshot.activeTarget);
            nodes.shell.style.display = 'block';
            if (document.activeElement !== nodes.textarea) {
                const draftText = snapshot.draftText;
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
            renderInteractionTargets(
                nodes,
                Array.isArray(snapshot.targets) ? snapshot.targets : [],
                (target, event) => {
                    void openEditor(target, event);
                },
            );
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
            const result = await deps.saveEditorSession();
            if (!result?.saved) {
                return {
                    saved: false,
                    hadPersistablePatches: (result as any)?.hadPersistablePatches,
                    errorMessage: (result as any)?.errorMessage ?? '保存失败',
                };
            }
            return result;
        } finally {
            suppressBlurCommitForSave = false;
        }
    }

    // ── Public API ──────────────────────────────────────────────

    return {
        syncTargets,
        clear: () => {
            const nodes = ensureNodes();
            if (!nodes) return;
            hideInteractionTargets(nodes);
            void commitEditor();
        },
        commitActiveEditor: commitEditor,
        saveEdits,
        applyFormatAction: async (action: EditorFormatAction) => {
            api.applyFormat(action);
            syncTargets(getLastDisplayZoom());
            syncFormatButtons();
        },
        openRegionEditor: async (pageIndex, regionId, kind, originalText) => {
            const nodes = ensureNodes();
            if (!nodes) return;
            suppressBlurCommitForOpen = true;
            clearDomSelection();

            const result = api.openRegion({ pageIndex, regionId, kind, originalText });
            if (!result?.ok || !result.data) {
                hideEditorShell();
                return;
            }

            const snapshot = readLegacySnapshot();
            const activeTarget = snapshot?.activeTarget;
            if (!activeTarget) {
                hideEditorShell();
                return;
            }

            const draftText = snapshot?.draftText ?? '';
            const caretIndex = snapshot?.caretIndex ?? 0;
            setupActiveEditor(nodes, activeTarget, draftText, caretIndex);
        },
        hasPendingEdits: () => api.hasUnsavedChanges(),
        setTextEditEnabled: (enabled: boolean) => {
            api.setEditMode(enabled);
            if (!enabled) {
                const nodes = ensureNodes();
                if (nodes) hideInteractionTargets(nodes);
                void commitEditor();
            } else {
                syncTargets(getLastDisplayZoom());
            }
        },
        isTextEditEnabled: () => !!readLegacySnapshot()?.enabled,
    };
}
