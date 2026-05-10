//! Annotation public types — P3 of the struct-based API refactor.
//!
//! Mirrors `edit::editor_types` / `document::document_types` for the
//! annotation domain (PDF `/Annot` objects).
//!
//! Pure-data module (no `wasm_bindgen` / `JsValue`). The UI crate adds
//! thin helpers for `JsValue` serialization.
//!
//! Scope: PDF spec annotation kinds (text note, highlight, ink, stamp,
//! signature placeholder, link, etc.). The `comment` domain (review
//! session, panel UX) is layered on top of this in `CommentManager`.

use serde::{Deserialize, Serialize};

// ── AnnotationKind ──────────────────────────────────────────────

/// PDF annotation subtype (per PDF 1.7 §12.5.6).
///
/// Only the subset relevant to this viewer is enumerated; unknown kinds
/// fall through to `Other`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnnotationKind {
    /// Sticky note (`/Text`).
    TextNote,
    /// Highlighted region (`/Highlight`).
    Highlight,
    /// Underline (`/Underline`).
    Underline,
    /// Squiggly underline (`/Squiggly`).
    Squiggly,
    /// Strike-through (`/StrikeOut`).
    Strikeout,
    /// Free-form ink stroke (`/Ink`).
    Ink,
    /// Image stamp (`/Stamp`).
    Stamp,
    /// Hyperlink (`/Link`).
    Link,
    /// Digital signature placeholder (`/Widget` with `/FT /Sig`).
    Signature,
    /// Free-floating text label (`/FreeText`).
    FreeText,
    /// Any other / unsupported annotation subtype.
    Other,
}

// ── Annotation ──────────────────────────────────────────────────

/// A PDF `/Annot` object as exposed to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    /// Stable identifier (PDF object number or synthetic UUID).
    pub id: String,
    /// 0-based page index this annotation belongs to.
    pub page_index: u16,
    /// Annotation subtype.
    pub kind: AnnotationKind,
    /// Bounding box in page coordinates (PDF user-space, origin = bottom-left).
    pub bbox: AnnotationBBox,
    /// Optional textual contents (`/Contents` entry).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
    /// Optional author / creator string (`/T` entry).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Optional creation timestamp (`/CreationDate`, ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Optional last-modified timestamp (`/M`, ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
}

/// Annotation bounding box in PDF page coordinates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationBBox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

// ── AnnotationError ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AnnotationError {
    /// Input payload failed to deserialize or had illegal values.
    InvalidInput {
        field: String,
        reason: String,
    },
    /// The requested annotation id does not exist on the given page.
    NotFound {
        annotation_id: String,
    },
    /// The requested method is reserved but not yet implemented.
    NotImplemented {
        method: String,
    },
    /// Backend / IO failure when reading or writing annotations.
    IoError {
        message: String,
    },
    /// Unrecoverable internal error.
    Internal {
        message: String,
    },
}

// ── AnnotationResponse<T> ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AnnotationError>,
}
