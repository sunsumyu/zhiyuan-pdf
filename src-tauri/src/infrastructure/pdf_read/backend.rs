use crate::infrastructure::pdf_read::types::{PagePreview, ReadDocumentMeta};
pub trait PdfReadBackend: Send + Sync {
    fn open(&self, path: &str) -> Result<ReadDocumentMeta, String>;
    fn read_page_preview(&self, path: &str, page_index: u16) -> Result<PagePreview, String>;
}
