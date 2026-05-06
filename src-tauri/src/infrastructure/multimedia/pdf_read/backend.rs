use crate::infrastructure::multimedia::pdf_read::types::{PagePreview, ReadDocumentMeta};
pub trait PdfReadBackend: Send + Sync {
fn open(&self, path: &str) -> Result<ReadDocumentMeta, String>;
fn get_page_preview(&self, path: &str, page_index: u16) -> Result<PagePreview, String>;
}
