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
    InvalidInput { field: String, reason: String },
    /// The requested annotation id does not exist on the given page.
    NotFound { annotation_id: String },
    /// The requested method is reserved but not yet implemented.
    NotImplemented { method: String },
    /// Backend / IO failure when reading or writing annotations.
    IoError { message: String },
    /// Unrecoverable internal error.
    Internal { message: String },
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommentBoxRect {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

pub type PdfPageAnnotationBox = CommentBoxRect;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommentPercentFrame {
    pub left_percent: f32,
    pub top_percent: f32,
    pub width_percent: f32,
    pub height_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageCommentItem {
    pub id: String,
    pub page_index: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub color: [f32; 3],
    pub contents: String,
    pub box_rect: CommentBoxRect,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageCommentList {
    pub page_index: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub comments: Vec<PdfPageCommentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageAnnotationTarget {
    pub id: String,
    pub kind: String,
    pub page_index: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub label: String,
    pub box_rect: CommentBoxRect,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfPageAnnotationTargetResult {
    pub page_index: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub targets: Vec<PdfPageAnnotationTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentTargetOverlayMarker {
    pub id: String,
    pub kind: String,
    pub page_index: u16,
    pub label: String,
    pub title: String,
    pub frame: CommentPercentFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentTargetOverlayDisplay {
    pub targets: Vec<PdfCommentTargetOverlayMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewPageSummary {
    pub page_index: u16,
    pub total_comments: usize,
    pub filtered_comments: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewRequest {
    pub page_index: Option<u16>,
    #[serde(default)]
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewResult {
    pub total_comments: usize,
    pub filtered_comments: usize,
    pub pages_with_comments: usize,
    pub summaries: Vec<PdfCommentReviewPageSummary>,
    pub comments: Vec<PdfPageCommentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewSummaryChip {
    pub page_index: u16,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewCardAction {
    pub id: String,
    pub label: String,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewCard {
    pub id: String,
    pub page_index: u16,
    pub contents: String,
    pub page_label: String,
    pub location_label: String,
    pub helper_label: String,
    pub selected: bool,
    pub actions: Vec<PdfCommentReviewCardAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewPanel {
    pub meta_text: String,
    pub empty: bool,
    pub summary_chips: Vec<PdfCommentReviewSummaryChip>,
    pub cards: Vec<PdfCommentReviewCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentOverlayMarker {
    pub id: String,
    pub title: String,
    pub frame: CommentPercentFrame,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentOverlayDisplay {
    pub comments: Vec<PdfCommentOverlayMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfCommentReviewDisplay<TSession: Serialize> {
    pub session: TSession,
    pub review: PdfCommentReviewResult,
    pub panel: PdfCommentReviewPanel,
    pub overlay: PdfCommentOverlayDisplay,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfRegionCommentRequest {
    pub page_index: u16,
    pub region_id: String,
    pub kind: String,
    pub contents: String,
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfRegionCommentResult {
    pub added: bool,
    pub page_index: u16,
    pub region_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfDeleteAnnotationRequest {
    pub page_index: u16,
    pub annotation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfDeleteAnnotationResult {
    pub deleted: bool,
    pub page_index: u16,
    pub annotation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfUpdateCommentRequest {
    pub page_index: u16,
    pub annotation_id: String,
    pub contents: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PdfUpdateCommentResult {
    pub updated: bool,
    pub page_index: u16,
    pub annotation_id: String,
}
