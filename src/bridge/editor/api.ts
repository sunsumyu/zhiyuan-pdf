import { getWasmApi } from '../shared/wasm_loader';
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
} from './types';

// ── Singleton EditorSession WASM instance ───────────────────────

let _session: any = null;

function getSession(): any {
    if (!_session) {
        const api = getWasmApi() as any;
        if (typeof api?.EditorSession === 'function') {
            _session = new api.EditorSession();
        }
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

export function getSnapshot(displayZoom: number): EditorResponse<SnapshotResult> | null {
    return getSession()?.getSnapshot(displayZoom) ?? null;
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

export function getTextBlocks(pageIndex: number): EditorResponse<TextBlockInfo[]> | null {
    return getSession()?.getTextBlocks(pageIndex) ?? null;
}

export function getFormatState(): unknown {
    return getSession()?.getFormatState() ?? null;
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
