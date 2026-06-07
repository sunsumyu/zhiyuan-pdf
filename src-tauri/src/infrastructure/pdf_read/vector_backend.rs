use crate::infrastructure::pdf_read::backend::PdfReadBackend;
use crate::infrastructure::pdf_read::types::{
    ClassificationReason, PagePreview, PdfDocumentKind, ReadDocumentMeta,
};
pub struct VectorReadBackend;
impl VectorReadBackend {
    pub fn new() -> Self {
        Self
    }
}
impl PdfReadBackend for VectorReadBackend {
    fn open(&self, path: &str) -> Result<ReadDocumentMeta, String> {
        Ok(ReadDocumentMeta {
            doc_id: path.to_string(),
            path: path.to_string(),
            page_count: 0,
            kind: PdfDocumentKind::Unknown,
            confidence: 0.0,
            allow_scan_preview_first_paint: false,
            classification_reason: ClassificationReason::Unknown,
        })
    }
    fn read_page_preview(&self, path: &str, page_index: u16) -> Result<PagePreview, String> {
        Ok(PagePreview {
            doc_id: path.to_string(),
            page_index,
            width: 0.0,
            height: 0.0,
            image_url: None,
            kind: PdfDocumentKind::Vector,
            ready: false,
        })
    }
}
