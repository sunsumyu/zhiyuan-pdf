//! DocumentSession public types.
//!
//! Mirrors `edit::editor_types` for the document domain (P1 of the
//! `docs/editor-api-architecture-proposal.md` struct-based API refactor).
//!
//! Pure-data module (no wasm_bindgen / JsValue). The UI crate adds thin
//! helpers for JsValue serialization.

use serde::{Deserialize, Serialize};

// ── DocumentError ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DocumentError {
    /// Input payload failed to deserialize or had illegal values.
    InvalidInput { field: String, reason: String },
    /// The requested method is reserved but not yet implemented.
    NotImplemented { method: String },
    /// IO failure when reading / writing document bytes.
    IoError { message: String },
    /// Unrecoverable internal error.
    Internal { message: String },
}

// ── DocumentResponse<T> ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DocumentError>,
}
