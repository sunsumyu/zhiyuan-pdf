import { getWasmApi } from '../shared/wasm_loader';

export type EditorFacadeResult = {
    changed: boolean;
    enabled?: boolean;
    caretIndex: number | null;
    draftText: string | null;
    renderFrame: unknown | null;
};

export type EditorActiveTarget = {
    paragraphId: string;
    regionId: string;
    pageIndex: number;
    text: string;
    left: number;
    top: number;
    width: number;
    height: number;
    initialCaretIndex: number;
    // Font properties for DOM positioning
    fontFamily: string;
    fontSizePx: number;
    fontWeight: string;
    fontStyle: string;
    color: string;
    textDecoration: string;
};

export type EditorInteractionTarget = {
    paragraphId: string;
    regionId: string;
    pageIndex: number;
    text: string;
    left: number;
    top: number;
    width: number;
    height: number;
    color: string;
    textDecoration: string;
    // Font properties for DOM rendering
    fontFamily: string;
    fontSize: number;
    fontWeight: string;
    fontStyle: string;
};

export type EditorSnapshotResult = {
    enabled: boolean;
    hasActiveTarget: boolean;
    paragraphId: string | null;
    draftText: string | null;
    caretIndex: number;
    hasPersistablePatches: boolean;
    targetCount: number;
    activeTarget: EditorActiveTarget | null;
    targets: EditorInteractionTarget[];
};

export type EditorOpenRequest = {
    paragraphId: string;
    clientX: number;
    clientY: number;
    referenceLeft: number;
    referenceTop: number;
    referenceWidth: number;
    referenceHeight: number;
    pageWidth: number;
    pageHeight: number;
};

export type EditorOpenRegionRequest = {
    pageIndex: number;
    regionId: string;
    kind: string;
    originalText: string;
};

export type EditorSyncInputRequest = {
    text: string;
    caretIndex: number;
};

export type EditorCommitRequest = {
    draftText: string;
    caretIndex: number;
};

export type EditorCommandRequest = {
    command: string;
    insertedText: string | null;
};

export type EditorMoveCaretRequest = {
    clientX: number;
    clientY: number;
    referenceLeft: number;
    referenceTop: number;
    referenceWidth: number;
    referenceHeight: number;
    pageWidth: number;
    pageHeight: number;
};

function callFacade<T>(fnName: string, arg?: unknown): T | null {
    const api = getWasmApi();
    const fn = (api as any)[fnName];
    if (typeof fn !== 'function') return null;
    try {
        return arg !== undefined ? fn(arg) : fn();
    } catch {
        return null;
    }
}

export function facadeOpenEditor(request: EditorOpenRequest): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeOpen', request);
}

export function facadeOpenRegionEditor(request: EditorOpenRegionRequest): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeOpenRegion', request);
}

export function facadeSyncInput(request: EditorSyncInputRequest): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeSyncInput', request);
}

export function facadeCommitEditor(request: EditorCommitRequest): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeCommit', request);
}

export function facadeCommitSilent(request: EditorCommitRequest): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeCommitSilent', request);
}

export function facadeApplyCommand(request: EditorCommandRequest): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeApplyCommand', request);
}

export function facadeCloseEditor(): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeClose');
}

export function facadeMoveCaret(request: EditorMoveCaretRequest): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeMoveCaret', request);
}

export function facadeApplyFormat(action: unknown): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeApplyFormat', action);
}

export function facadeReadSnapshot(displayZoom: number): EditorSnapshotResult | null {
    return callFacade<EditorSnapshotResult>('editorFacadeReadSnapshot', displayZoom);
}

export function facadeSetEditMode(enabled: boolean): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeSetEditMode', enabled);
}

export function facadeHasSessionChanges(): boolean {
    const api = getWasmApi();
    const fn = (api as any)['editorFacadeHasSessionChanges'];
    if (typeof fn !== 'function') return false;
    try {
        return !!fn();
    } catch {
        return false;
    }
}

export function facadeUtf16ToCharIndex(text: string, utf16Offset: number): number {
    const api = getWasmApi();
    const fn = (api as any)['editorFacadeUtf16ToCharIndex'];
    if (typeof fn !== 'function') return utf16Offset;
    try {
        return fn(text, utf16Offset) ?? utf16Offset;
    } catch {
        return utf16Offset;
    }
}

export function facadeCharToUtf16Offset(text: string, charIndex: number): number {
    const api = getWasmApi();
    const fn = (api as any)['editorFacadeCharToUtf16Offset'];
    if (typeof fn !== 'function') return charIndex;
    try {
        return fn(text, charIndex) ?? charIndex;
    } catch {
        return charIndex;
    }
}

// ─── Editor facade — runtime / diagnostics / format / paint ───────────────────

export function facadePaintCanvas(
    canvas: HTMLCanvasElement,
    displayZoom: number,
    draftText: string,
    caretIndex: number,
): boolean {
    const api = getWasmApi();
    const fn = (api as any)['editorFacadePaintCanvas'];
    if (typeof fn !== 'function') return false;
    try {
        return !!fn(canvas, displayZoom, draftText, caretIndex);
    } catch {
        return false;
    }
}

export function facadeReadDiagnostics(): unknown {
    return callFacade('editorFacadeReadDiagnostics');
}

export function facadeReadRuntime(): unknown {
    return callFacade('editorFacadeReadRuntime');
}

export function facadeResetRuntime(): void {
    const api = getWasmApi();
    (api as any)['editorFacadeResetRuntime']?.();
}

export function facadeSetDisplayZoom(zoom: number): void {
    const api = getWasmApi();
    (api as any)['editorFacadeSetDisplayZoom']?.(zoom);
}

export function facadeBeginCommit(): boolean {
    const api = getWasmApi();
    return !!(api as any)['editorFacadeBeginCommit']?.();
}

export function facadeFinishCommit(): void {
    const api = getWasmApi();
    (api as any)['editorFacadeFinishCommit']?.();
}

export function facadeToggleMode(): EditorFacadeResult | null {
    return callFacade<EditorFacadeResult>('editorFacadeToggleMode');
}

export function facadeReadFormatState(): unknown {
    return callFacade('editorFacadeReadFormatState');
}

export async function facadeSaveSession(path: string, pageIndex: number): Promise<unknown> {
    const api = getWasmApi();
    const fn = (api as any)['editorFacadeSaveSession'];
    if (typeof fn !== 'function') return null;
    try {
        return await fn(path, pageIndex);
    } catch {
        return null;
    }
}

// ─── Editor facade — STUB API (reserved, returns { implemented: false }) ─────

export type StubResult = { implemented: boolean; error: string };

export function facadeSelectRange(start: number, end: number): StubResult | null {
    const api = getWasmApi();
    return (api as any)['editorFacadeSelectRange']?.(start, end) ?? null;
}

export function facadeCut(): StubResult | null {
    return callFacade<StubResult>('editorFacadeCut');
}

export function facadeCopy(): StubResult | null {
    return callFacade<StubResult>('editorFacadeCopy');
}

export function facadePaste(text: string): StubResult | null {
    const api = getWasmApi();
    return (api as any)['editorFacadePaste']?.(text) ?? null;
}

export function facadeUndo(): StubResult | null {
    return callFacade<StubResult>('editorFacadeUndo');
}

export function facadeRedo(): StubResult | null {
    return callFacade<StubResult>('editorFacadeRedo');
}

export function facadeFindInActive(query: string, caseSensitive: boolean): StubResult | null {
    const api = getWasmApi();
    return (api as any)['editorFacadeFindInActive']?.(query, caseSensitive) ?? null;
}

export function facadeReplaceInActive(
    query: string,
    replacement: string,
    caseSensitive: boolean,
    replaceAll: boolean,
): StubResult | null {
    const api = getWasmApi();
    return (api as any)['editorFacadeReplaceInActive']?.(query, replacement, caseSensitive, replaceAll) ?? null;
}

