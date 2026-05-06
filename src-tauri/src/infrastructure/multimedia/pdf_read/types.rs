use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PdfDocumentKind {
    Unknown,
    Scanned,
    Mixed,
    Vector,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClassificationReason {
    FullPageImageNoText,
    FullPageImageWithOcrLayer,
    TextOperatorsDominant,
    FontResourcesDominant,
    LowConfidenceFallback,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadDocumentMeta {
pub doc_id: String,
pub path: String,
pub page_count: usize,
pub kind: PdfDocumentKind,
pub confidence: f32,
pub allow_scan_preview_first_paint: bool,
pub classification_reason: ClassificationReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagePreview {
pub doc_id: String,
pub page_index: u16,
pub width: f32,
pub height: f32,
pub image_url: Option<String>,
pub kind: PdfDocumentKind,
pub ready: bool,
}
