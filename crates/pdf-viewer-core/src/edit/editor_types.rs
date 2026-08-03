use serde::{Deserialize, Serialize};

// ── SessionState ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    /// View-only, editing disabled.
    Viewing,
    /// Edit mode activated, block highlights visible, no block open (transient).
    Editing,
    /// Actively editing a specific text block.
    EditingBlock,
    /// Saving to disk (async, guards against re-entry).
    Saving,
}

impl SessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionState::Viewing => "Viewing",
            SessionState::Editing => "Editing",
            SessionState::EditingBlock => "EditingBlock",
            SessionState::Saving => "Saving",
        }
    }
}

// ── EditorError ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EditorError {
    /// Called an API in an illegal state (e.g. open_block before begin).
    InvalidState { expected: String, actual: String },
    /// Target entity not found (e.g. block_id doesn't exist).
    NotFound { entity: String, id: String },
    /// Feature not yet implemented (P1-P3 stubs).
    NotImplemented { method: String },
    /// Unrecoverable internal error.
    Internal { message: String },
    /// IO error during save.
    IoError { message: String },
}

// ── EditorResponse<T> ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<EditorError>,
    /// Whether TS should trigger a re-render after this call.
    pub render: bool,
}

// ── Data Transfer Objects ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HitTestResult {
    pub block_id: Option<String>,
    pub page_x: f32,
    pub page_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenBlockResult {
    pub block_id: String,
    pub caret_index: u32,
    pub draft_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveCaretResult {
    pub caret_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitResult {
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotResult {
    pub state: SessionState,
    pub block_id: Option<String>,
    pub draft_text: Option<String>,
    pub caret_index: u32,
    pub has_unsaved_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextBlockInfo {
    pub id: String,
    pub bbox_left: f32,
    pub bbox_top: f32,
    pub bbox_right: f32,
    pub bbox_bottom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatState {
    pub bold: bool,
    pub italic: bool,
    pub font_size: Option<f32>,
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncInputResult {
    pub changed: bool,
    pub caret_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyCommandResult {
    pub changed: bool,
    pub caret_index: u32,
    pub draft_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEditModeResult {
    pub enabled: bool,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagePointDto {
    pub page_x: f32,
    pub page_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPointDto {
    pub client_x: f32,
    pub client_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSelection {
    pub start: u32,
    pub end: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextLineDto {
    pub text: String,
    pub bbox: crate::models::BoundingBox,
    pub char_count: u32,
}
