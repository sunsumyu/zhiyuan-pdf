// ── Response types from EditorSession WASM API ─────────────────

export type SessionState = 'viewing' | 'editing' | 'editingBlock' | 'saving';

export type EditorError = {
    type: 'invalidState' | 'notFound' | 'notImplemented' | 'internal' | 'ioError';
    expected?: string;
    actual?: string;
    entity?: string;
    id?: string;
    method?: string;
    message?: string;
};

export type EditorResponse<T = unknown> = {
    ok: boolean;
    data?: T | null;
    error?: EditorError | null;
    render: boolean;
};

// ── Data payloads ───────────────────────────────────────────────

export type HitTestResult = {
    blockId: string | null;
    pageX: number;
    pageY: number;
};

export type OpenBlockResult = {
    blockId: string;
    caretIndex: number;
    draftText: string;
};

export type MoveCaretResult = {
    caretIndex: number;
};

export type CommitResult = {
    changed: boolean;
};

export type SnapshotResult = {
    state: SessionState;
    blockId: string | null;
    draftText: string | null;
    caretIndex: number;
    hasUnsavedChanges: boolean;
};

export type TextBlockInfo = {
    id: string;
    bboxLeft: number;
    bboxTop: number;
    bboxRight: number;
    bboxBottom: number;
};

// ── Request payloads (TS → WASM) ────────────────────────────────

export type HitTestRequest = {
    clientX: number;
    clientY: number;
    referenceLeft: number;
    referenceTop: number;
    referenceWidth: number;
    referenceHeight: number;
    pageWidth: number;
    pageHeight: number;
};

export type OpenBlockRequest = {
    blockId: string;
    clientX: number;
    clientY: number;
    referenceLeft: number;
    referenceTop: number;
    referenceWidth: number;
    referenceHeight: number;
    pageWidth: number;
    pageHeight: number;
};

export type MoveCaretRequest = {
    clientX: number;
    clientY: number;
    referenceLeft: number;
    referenceTop: number;
    referenceWidth: number;
    referenceHeight: number;
    pageWidth: number;
    pageHeight: number;
};

export type CommitRequest = {
    draftText: string;
    caretIndex: number;
};

export type SyncInputRequest = {
    text: string;
    caretIndex: number;
};

export type ApplyCommandRequest = {
    command: string;
    insertedText: string | null;
};

// ── Result types for bridge methods ─────────────────────────────

export type SyncInputResult = {
    changed: boolean;
    caretIndex: number;
};

export type ApplyCommandResult = {
    changed: boolean;
    caretIndex: number;
    draftText: string | null;
};

export type SetEditModeResult = {
    enabled: boolean;
    changed: boolean;
};

// ── Legacy types re-exported for backward compat ────────────────
// These map to the old EditorHostSnapshot from Rust facade.

export type LegacyActiveTarget = {
    paragraphId: string;
    regionId: string;
    pageIndex: number;
    text: string;
    left: number;
    top: number;
    width: number;
    height: number;
    initialCaretIndex: number;
    fontFamily: string;
    fontSizePx: number;
    fontWeight: string;
    fontStyle: string;
    color: string;
    textDecoration: string;
};

export type LegacyInteractionTarget = {
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

export type LegacySnapshot = {
    enabled: boolean;
    activeTarget: LegacyActiveTarget | null;
    draftText: string | null;
    caretIndex: number;
    targets: LegacyInteractionTarget[];
    hasPersistablePatches: boolean;
};

export type EditorFormatAction =
    | { type: 'toggleBold' }
    | { type: 'toggleItalic' }
    | { type: 'toggleUnderline' }
    | { type: 'increaseFontSize' }
    | { type: 'decreaseFontSize' }
    | { type: 'setParagraphMode'; mode: string }
    | { type: 'setColor'; color: string }
    | { type: 'setFontFamily'; fontFamily: string }
    | { type: 'setFontSize'; fontSize: number }
    | { type: 'setCharSpacing'; charSpacing: number }
    | { type: 'setLineHeight'; lineHeight: number }
    | { type: 'setAlignment'; alignment: string }
    | { type: 'setListKind'; listKind: string };
