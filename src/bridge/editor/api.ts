import { getWasmApi } from '../shared/wasm_loader';
import type { WasmModule } from '../shared/wasm_loader';
import type { EditorSession } from '../../../crates/pdf-viewer-ui/pkg/pdf_viewer_ui';
import type {
    EditorResponse,
    HitTestResult,
    OpenBlockResult,
    MoveCaretResult,
    CommitResult,
    SnapshotResult,
    TextBlockInfo,
    HitTestRequest,
    OpenBlockRequest,
    MoveCaretRequest,
    CommitRequest,
    SyncInputRequest,
    SyncInputResult,
    ApplyCommandRequest,
    ApplyCommandResult,
    SetEditModeResult,
    LegacySnapshot,
    EditorFormatAction,
    BoundingBox,
    TextSelection,
    TextLine,
    PagePoint,
    ClientPoint,
} from './types';

// ── Singleton EditorSession WASM instance ───────────────────────

let _session: EditorSession | null = null;

function getSession(): EditorSession {
    if (!_session) {
        const api = getWasmApi();
        if (typeof api?.EditorSession === 'function') {
            _session = new api.EditorSession();
        }
    }
    if (!_session) {
        throw new Error('EditorSession WASM API is unavailable');
    }
    return _session;
}

// ── New API (typed, EditorResponse<T>) ──────────────────────────

export function begin(): EditorResponse<TextBlockInfo[]> | null {
    return getSession()?.begin() ?? null;
}

export function hitTest(request: HitTestRequest): EditorResponse<HitTestResult> | null {
    return getSession()?.hitTest(request) ?? null;
}

export function openBlock(request: OpenBlockRequest): EditorResponse<OpenBlockResult> | null {
    return getSession()?.openBlock(request) ?? null;
}

export function moveCaret(request: MoveCaretRequest): EditorResponse<MoveCaretResult> | null {
    return getSession()?.moveCaret(request) ?? null;
}

export function closeBlock(): EditorResponse<CommitResult> | null {
    return getSession()?.closeBlock() ?? null;
}

export function commit(request: CommitRequest): EditorResponse<CommitResult> | null {
    return getSession()?.commit(request) ?? null;
}

export function discard(): EditorResponse<void> | null {
    return getSession()?.discard() ?? null;
}


export function isActive(): boolean {
    return !!getSession()?.isActive();
}

export function hasUnsavedChanges(): boolean {
    return !!getSession()?.hasUnsavedChanges();
}

// ── P0: Bridge methods (direct EditorSession calls) ─────────────

export function syncInput(request: SyncInputRequest): EditorResponse<SyncInputResult> | null {
    return getSession()?.syncInput(request) ?? null;
}

export function applyCommand(request: ApplyCommandRequest): EditorResponse<ApplyCommandResult> | null {
    return getSession()?.applyCommand(request) ?? null;
}

export function setEditMode(enabled: boolean): EditorResponse<SetEditModeResult> | null {
    return getSession()?.setEditMode(enabled) ?? null;
}

export function readLegacySnapshot(displayZoom: number): LegacySnapshot | null {
    return getSession()?.readLegacySnapshot(displayZoom) ?? null;
}

export function paintCanvas(
    canvas: HTMLCanvasElement,
    displayZoom: number,
    draftText: string,
    caretIndex: number,
): boolean {
    return !!getSession()?.paintCanvas(canvas, displayZoom, draftText, caretIndex);
}

export function utf16ToCharIndex(text: string, utf16Offset: number): number {
    return getSession()?.utf16ToCharIndex(text, utf16Offset) ?? utf16Offset;
}

export function charToUtf16Offset(text: string, charIndex: number): number {
    return getSession()?.charToUtf16Offset(text, charIndex) ?? charIndex;
}

export function hasSessionChanges(): boolean {
    return !!getSession()?.hasSessionChanges();
}

// ── P1: Implemented methods ─────────────────────────────────────

export function insertText(text: string): EditorResponse<ApplyCommandResult> | null {
    return getSession()?.insertText(text) ?? null;
}

export function deleteText(direction: 'forward' | 'backward'): EditorResponse<ApplyCommandResult> | null {
    return getSession()?.deleteText(direction) ?? null;
}

export function applyFormat(action: EditorFormatAction): EditorResponse<CommitResult> | null {
    return getSession()?.applyFormat(action) ?? null;
}



// ── Region editor ───────────────────────────────────────────────

export function openRegion(request: {
    pageIndex: number;
    regionId: string;
    kind: string;
    originalText: string;
}): EditorResponse<import('./types').OpenBlockResult> | null {
    return getSession()?.openRegion(request) ?? null;
}

// ── Runtime / diagnostics ───────────────────────────────────────

export function setDisplayZoom(zoom: number): void {
    getSession()?.setDisplayZoom(zoom);
}

export function readDiagnostics(): unknown {
    return getSession()?.readDiagnostics() ?? null;
}

export async function saveSession(path: string, pageIndex: number): Promise<unknown> {
    return (await getSession()?.saveSession(path, pageIndex)) ?? null;
}

// ── Stubs for future features ───────────────────────────────────

export function setCaret(charIndex: number): EditorResponse | null {
    return getSession()?.setCaret(charIndex) ?? null;
}

export function setSelection(start: number, end: number): EditorResponse | null {
    return getSession()?.setSelection(start, end) ?? null;
}

export function selectAll(): EditorResponse | null {
    return getSession()?.selectAll() ?? null;
}

export function getSelection(): EditorResponse<TextSelection> | null {
    return getSession()?.getSelection() ?? null;
}

export function cut(): EditorResponse<string> | null {
    return getSession()?.cut() ?? null;
}

export function copy(): EditorResponse<string> | null {
    return getSession()?.copy() ?? null;
}

export function paste(text: string): EditorResponse | null {
    return getSession()?.paste(text) ?? null;
}

export function undo(): EditorResponse<SyncInputResult> | null {
    return getSession()?.undo() ?? null;
}

export function redo(): EditorResponse<SyncInputResult> | null {
    return getSession()?.redo() ?? null;
}

export function canUndo(): boolean {
    return !!getSession()?.canUndo();
}

export function canRedo(): boolean {
    return !!getSession()?.canRedo();
}

export function getTextContent(): EditorResponse<string> | null {
    return getSession()?.getTextContent() ?? null;
}

export function getTextLines(): EditorResponse<TextLine[]> | null {
    return getSession()?.getTextLines() ?? null;
}

export function getCharRects(start: number, end: number): EditorResponse<BoundingBox[]> | null {
    return getSession()?.getCharRects(start, end) ?? null;
}

export function clientToPage(
    clientX: number,
    clientY: number,
    referenceLeft: number,
    referenceTop: number,
    referenceWidth: number,
    referenceHeight: number,
    pageWidth: number,
    pageHeight: number,
): EditorResponse<PagePoint> | null {
    return getSession()?.clientToPage(
        clientX,
        clientY,
        referenceLeft,
        referenceTop,
        referenceWidth,
        referenceHeight,
        pageWidth,
        pageHeight,
    ) ?? null;
}

export function pageToClient(
    pageX: number,
    pageY: number,
    referenceLeft: number,
    referenceTop: number,
    referenceWidth: number,
    referenceHeight: number,
    pageWidth: number,
    pageHeight: number,
): EditorResponse<ClientPoint> | null {
    return getSession()?.pageToClient(
        pageX,
        pageY,
        referenceLeft,
        referenceTop,
        referenceWidth,
        referenceHeight,
        pageWidth,
        pageHeight,
    ) ?? null;
}

export function addTextBlock(x: number, y: number, maxWidth: number, text: string): EditorResponse<TextBlockInfo> | null {
    return getSession()?.addTextBlock(x, y, maxWidth, text) ?? null;
}

export function deleteTextBlock(blockId: string): EditorResponse | null {
    return getSession()?.deleteTextBlock(blockId) ?? null;
}

export function resizeTextBlock(blockId: string, maxWidth: number): EditorResponse | null {
    return getSession()?.resizeTextBlock(blockId, maxWidth) ?? null;
}

export function moveTextBlock(blockId: string, x: number, y: number): EditorResponse | null {
    return getSession()?.moveTextBlock(blockId, x, y) ?? null;
}

export function exportPatch(): EditorResponse<string> | null {
    return getSession()?.exportPatch() ?? null;
}

export function importPatch(patchJs: unknown): EditorResponse | null {
    return getSession()?.importPatch(patchJs) ?? null;
}
