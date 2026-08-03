import type { PdfSaveResult } from '../document/document_edit_api';
import type {
    EditorFormatAction,
    LegacyActiveTarget,
    LegacySnapshot,
} from './types';
import * as api from './api';
import {
    hideEditorShell as hideEditorShellView,
    hideInteractionTargets,
    positionEditorShell,
    readHostReferenceBox,
    renderInteractionTargets,
    suspendHostOverlays,
    type EditorHostNodes,
    type HostReferenceBox,
    type ParagraphInteractionTarget,
} from './editor_host_view';
import { syncEditorFormatButtons } from '../viewer/pdf_viewer_dom';
import { emitPdfDiagnostic } from '../shared/diagnostics';
import {
    type EditorHostState,
    withSuppressedNativeInput,
    readTextareaCaret,
    writeTextareaCaret,
    rememberRustCaret,
    clearDomSelection,
    getLastDisplayZoom,
} from './textarea_helper';

export interface EditorHostDeps {
    getWasmApi: () => any;
    getCurrentPath: () => string | null;
    getCurrentPage: () => number;
    getCurrentZoom: () => number;
    getPageWidth: () => number;
    getPageHeight: () => number;
    getVectorContainer: () => HTMLElement | null;
    buildRenderRequest: (reason?: 'default' | 'zoom' | 'editorVisibility' | 'documentMutation') => Record<string, number | string | boolean>;
    renderScheduledFrame: (frame: any | null) => Promise<void>;
    renderCurrentPage: (reason?: 'default' | 'zoom' | 'editorVisibility' | 'documentMutation') => Promise<void>;
    saveEditorSession: () => Promise<PdfSaveResult>;
    syncViewerState?: () => void;
}

export interface EditorContext {
    state: EditorHostState;
    deps: EditorHostDeps;
    ensureNodes: () => EditorHostNodes | null;
}

export function setupActiveEditor(
    ctx: EditorContext,
    nodes: EditorHostNodes,
    target: LegacyActiveTarget,
    draftText: string,
    caretIndex: number,
): void {
    logEditorDiagnostic('caret.setupActiveEditor', {
        caretIndex,
        draftUtf16Length: draftText.length,
        draftCharCount: [...draftText].length,
        paragraphId: target.paragraphId,
        markerText: target.markerText ?? null,
        markerKind: target.markerKind ?? null,
        markerAdvance: target.markerAdvance ?? null,
        markerRunCount: target.markerRunCount ?? 0,
        liveCaretIndex: target.liveCaretIndex ?? null,
        targetTextCharCount: [...target.text].length,
        shellLeft: target.left,
        shellTop: target.top,
        shellWidth: target.width,
        shellHeight: target.height,
    }, 'DEBUG', true);
    ctx.state.suppressBlurCommitForOpen = true;
    positionEditorShell(nodes, target);
    withSuppressedNativeInput(ctx.state, () => {
        nodes.textarea.value = draftText;
    });
    hideInteractionTargets(nodes);
    suspendHostOverlays(nodes);
    clearDomSelection();
    nodes.shell.style.display = 'block';
    withSuppressedNativeInput(ctx.state, () => {
        nodes.textarea.focus();
    });
    clearDomSelection();
    rememberRustCaret(ctx.state, caretIndex);
    withSuppressedNativeInput(ctx.state, () => {
        writeTextareaCaret(nodes.textarea, caretIndex);
    });
    renderActiveEditor(ctx);
    scheduleOpenFocusStabilization(ctx, nodes);
}

export function scheduleOpenFocusStabilization(ctx: EditorContext, nodes: EditorHostNodes): void {
    ctx.state.suppressBlurCommitForOpen = true;
    window.requestAnimationFrame(() => {
        window.setTimeout(() => {
            if (document.activeElement !== nodes.textarea) {
                try {
                    withSuppressedNativeInput(ctx.state, () => {
                        nodes.textarea.focus();
                    });
                } catch {
                    // Ignore
                }
            }
            if (ctx.state.lastRustCaretIndex !== null && Number.isFinite(ctx.state.lastRustCaretIndex)) {
                withSuppressedNativeInput(ctx.state, () => {
                    writeTextareaCaret(nodes.textarea, ctx.state.lastRustCaretIndex as number);
                });
            }
            ctx.state.suppressBlurCommitForOpen = false;
        }, 120);
    });
}

export function hideEditorShell(ctx: EditorContext): void {
    const nodes = ctx.ensureNodes();
    if (!nodes) return;
    clearDomSelection(true);
    hideEditorShellView(nodes);
    clearDomSelection(true);
}

export function readLegacySnapshot(ctx: EditorContext): LegacySnapshot | null {
    return api.readLegacySnapshot(getLastDisplayZoom(ctx.state));
}

function logEditorDiagnostic(
    event: string,
    fields: Record<string, unknown> = {},
    level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR' = 'WARN',
    verboseOnly = false,
): void {
    emitPdfDiagnostic('editor', event, fields, { level, verboseOnly });
}

export function renderActiveEditor(ctx: EditorContext, displayZoom = getLastDisplayZoom(ctx.state)): void {
    const nodes = ctx.ensureNodes();
    if (!nodes) return;
    const snapshot = readLegacySnapshot(ctx);
    if (!snapshot?.activeTarget) return;
    positionEditorShell(nodes, snapshot.activeTarget);

    const draftText = snapshot.draftText;
    if (draftText == null) return;

    const caretIndex = Math.max(
        0,
        (Number.isFinite(ctx.state.lastRustCaretIndex) && (ctx.state.lastRustCaretIndex as number) >= 0)
            ? (ctx.state.lastRustCaretIndex as number)
            : (snapshot.caretIndex ?? draftText.length),
    );
    rememberRustCaret(ctx.state, caretIndex);

    if (nodes.textarea.value !== draftText) {
        withSuppressedNativeInput(ctx.state, () => {
            nodes.textarea.value = draftText;
        });
    }
    withSuppressedNativeInput(ctx.state, () => {
        writeTextareaCaret(nodes.textarea, caretIndex);
    });

    api.paintCanvas(nodes.canvas, displayZoom, draftText, caretIndex);
    syncFormatButtons();
}

export async function commitEditor(ctx: EditorContext): Promise<void> {
    const snapshot = readLegacySnapshot(ctx);
    if (!snapshot?.activeTarget && snapshot?.draftText == null) return;

    const nodes = ctx.ensureNodes();
    if (!nodes) return;
    const path = ctx.deps.getCurrentPath();
    if (!path) {
        closeEditor(ctx, true);
        return;
    }

    const draftText = snapshot?.draftText;
    if (draftText == null) return;

    const caretIndex = Math.max(
        0,
        ctx.state.lastRustCaretIndex ?? snapshot?.caretIndex ?? readTextareaCaret(nodes.textarea),
    );

    api.commit({ draftText, caretIndex });
    hideEditorShell(ctx);
    syncTargets(ctx, getLastDisplayZoom(ctx.state));
    syncFormatButtons();
}

export async function commitForSave(ctx: EditorContext): Promise<void> {
    const snapshot = readLegacySnapshot(ctx);
    if (!snapshot?.activeTarget && snapshot?.draftText == null) return;

    const nodes = ctx.ensureNodes();
    if (!nodes) return;

    const draftText = snapshot?.draftText;
    if (draftText == null) return;

    const caretIndex = Math.max(
        0,
        ctx.state.lastRustCaretIndex ?? snapshot?.caretIndex ?? readTextareaCaret(nodes.textarea),
    );

    api.commit({ draftText, caretIndex });
    hideEditorShell(ctx);
    syncTargets(ctx, getLastDisplayZoom(ctx.state));
    syncFormatButtons();
}

export function closeEditor(ctx: EditorContext, clearActive = true): void {
    hideEditorShell(ctx);
    ctx.state.lastRustCaretIndex = null;
    if (clearActive) {
        api.closeBlock();
    }
    syncTargets(ctx, getLastDisplayZoom(ctx.state));
    syncFormatButtons();
}

export function syncFormatButtons(): void {
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

export async function openEditor(
    ctx: EditorContext,
    target: ParagraphInteractionTarget,
    event: MouseEvent,
): Promise<void> {
    try {
        const nodes = ctx.ensureNodes();
        if (!nodes) return;
        ctx.state.suppressBlurCommitForOpen = true;
        clearDomSelection();

        const beginResult = api.begin();
        if (beginResult && !beginResult.ok) {
            logEditorDiagnostic('openEditor.beginFailed', { beginResult }, 'WARN');
        }

        const referenceBox = resolveTargetReferenceBox(target, event, nodes.root);
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
            logEditorDiagnostic('openEditor.hitTestMiss', {
                paragraphId: target.paragraphId,
                hitResult,
                clientX: event.clientX,
                clientY: event.clientY,
            }, 'WARN');
        }

        const blockId = hitResult?.data?.blockId ?? target.paragraphId;

        const openResult = api.openBlock({
            blockId,
            clientX: event.clientX,
            clientY: event.clientY,
            referenceLeft: referenceBox.left,
            referenceTop: referenceBox.top,
            referenceWidth: referenceBox.width,
            referenceHeight: referenceBox.height,
            pageWidth: ctx.deps.getPageWidth(),
            pageHeight: ctx.deps.getPageHeight(),
            fallbackPageX: hitResult?.data?.pageX,
            fallbackPageY: hitResult?.data?.pageY,
        });

        if (!openResult?.ok || !openResult.data) {
            logEditorDiagnostic('openEditor.openBlockFailed', { openResult, blockId, hitResult }, 'ERROR');
            hideEditorShell(ctx);
            return;
        }

        const snapshot = readLegacySnapshot(ctx);
        const activeTarget = snapshot?.activeTarget;
        if (!activeTarget) {
            logEditorDiagnostic('openEditor.missingActiveTarget', { snapshot, blockId, hitResult }, 'ERROR');
            hideEditorShell(ctx);
            return;
        }

        setupActiveEditor(ctx, nodes, activeTarget, openResult.data.draftText, openResult.data.caretIndex);
    } catch (err) {
        logEditorDiagnostic('openEditor.exception', {
            error: String(err),
            paragraphId: target.paragraphId,
            clientX: event.clientX,
            clientY: event.clientY,
        }, 'ERROR');
        hideEditorShell(ctx);
    }
}

export function resolveTargetReferenceBox(
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

export function syncTargets(ctx: EditorContext, displayZoom: number): void {
    ctx.state.cachedDisplayZoom = displayZoom;
    const nodes = ctx.ensureNodes();
    if (!nodes) return;
    const container = ctx.deps.getVectorContainer();
    if (!container) {
        hideInteractionTargets(nodes);
        return;
    }

    const snapshot = readLegacySnapshot(ctx);
    if (!snapshot?.enabled) {
        logEditorDiagnostic('targets.disabled', {
            pageIndex: ctx.deps.getCurrentPage(),
            displayZoom,
            hasSnapshot: !!snapshot,
        }, 'DEBUG');
        hideInteractionTargets(nodes);
        hideEditorShell(ctx);
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
            withSuppressedNativeInput(ctx.state, () => {
                nodes.textarea.value = draftText;
            });
            const caretIndex = Math.max(
                0,
                (Number.isFinite(ctx.state.lastRustCaretIndex) && (ctx.state.lastRustCaretIndex as number) >= 0)
                    ? (ctx.state.lastRustCaretIndex as number)
                    : (snapshot.caretIndex ?? draftText.length),
            );
            rememberRustCaret(ctx.state, caretIndex);
            withSuppressedNativeInput(ctx.state, () => {
                writeTextareaCaret(nodes.textarea, caretIndex);
            });
        }
        renderActiveEditor(ctx);
    } else {
        const targets = Array.isArray(snapshot.targets) ? snapshot.targets : [];
        if (targets.length === 0) {
            logEditorDiagnostic('targets.empty', {
                pageIndex: ctx.deps.getCurrentPage(),
                displayZoom,
                path: ctx.deps.getCurrentPath(),
                pageWidth: ctx.deps.getPageWidth(),
                pageHeight: ctx.deps.getPageHeight(),
            }, 'WARN');
        } else {
            logEditorDiagnostic('targets.ready', {
                pageIndex: ctx.deps.getCurrentPage(),
                displayZoom,
                count: targets.length,
            }, 'DEBUG');
        }
        renderInteractionTargets(
            nodes,
            targets,
            (target, event) => {
                void openEditor(ctx, target, event);
            },
        );
        hideEditorShell(ctx);
        syncFormatButtons();
    }
}

export async function saveEdits(ctx: EditorContext): Promise<PdfSaveResult> {
    const path = ctx.deps.getCurrentPath();
    if (!path) {
        return { saved: false, errorMessage: 'missing-path' };
    }
    ctx.state.suppressBlurCommitForSave = true;
    try {
        await commitForSave(ctx);
        const result = await ctx.deps.saveEditorSession();
        if (!result?.saved) {
            return {
                saved: false,
                hadPersistablePatches: (result as any)?.hadPersistablePatches,
                errorMessage: (result as any)?.errorMessage ?? '保存失败',
            };
        }
        return result;
    } finally {
        ctx.state.suppressBlurCommitForSave = false;
    }
}
