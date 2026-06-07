pub use pdf_viewer_core::models::{ClassificationReason, PdfDocumentKind, ReadDocumentMeta};
use serde::{Deserialize, Serialize};

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
