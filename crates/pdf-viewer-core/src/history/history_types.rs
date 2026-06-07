//! HistoryController public types — P4 of the struct-based API refactor.
//!
//! Mirrors `edit::editor_types` / `document::document_types` /
//! `annotation::annotation_types` for the document history domain.
//!
//! Pure-data module (no `wasm_bindgen` / `JsValue`). The UI crate adds
//! thin helpers for JsValue serialization.

use serde::{Deserialize, Serialize};

// ── HistoryError ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum HistoryError {
    /// Attempted undo with empty history stack.
    NothingToUndo,
    /// Attempted redo with empty redo stack.
    NothingToRedo,
    /// Internal state corruption (lock poisoning, etc.).
    Internal { message: String },
}

// ── HistoryState ────────────────────────────────────────────────

/// Snapshot of the document history at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryState {
    /// Whether `undo()` would have an effect.
    pub can_undo: bool,
    /// Whether `redo()` would have an effect.
    pub can_redo: bool,
    /// Number of entries on the undo stack.
    pub undo_depth: u32,
    /// Number of entries on the redo stack.
    pub redo_depth: u32,
    /// Monotonic revision counter (bumps on every applied patch / undo / redo).
    pub revision: u64,
}

// ── HistoryStepResult ───────────────────────────────────────────

/// Result of a single `undo()` or `redo()` call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStepResult {
    /// Whether the operation actually moved a patch (false = no-op).
    pub changed: bool,
    /// The new history state after the operation.
    pub state: HistoryState,
}

// ── HistoryResponse<T> ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<HistoryError>,
}
