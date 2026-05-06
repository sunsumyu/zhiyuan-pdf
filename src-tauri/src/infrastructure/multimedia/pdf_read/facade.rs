use crate::infrastructure::multimedia::pdf_read::backend::PdfReadBackend;
use crate::infrastructure::multimedia::pdf_read::scanned_backend::ScannedReadBackend;
use crate::infrastructure::multimedia::pdf_read::types::{PagePreview, ReadDocumentMeta};
use crate::infrastructure::multimedia::pdf_read::vector_backend::VectorReadBackend;
pub struct PdfReadFacade {
    scanned: ScannedReadBackend,
    vector: VectorReadBackend,
}
impl PdfReadFacade {
pub fn new() -> Self {
        Self {
            scanned: ScannedReadBackend::new(),
            vector: VectorReadBackend::new(),
        }
    }
pub fn open(&self, path: &str) -> Result<ReadDocumentMeta, String> {
        match self.scanned.open(path) {
            Ok(meta) => Ok(meta),
            Err(_) => self.vector.open(path),
        }
    }
pub fn probe_kind_fast(&self, path: &str) -> Result<ReadDocumentMeta, String> {
        self.scanned.open(path)
    }
pub fn get_page_preview(&self, path: &str, page_index: u16) -> Result<PagePreview, String> {
        let preview = self.scanned.get_page_preview(path, page_index)?;
        if preview.ready {
            Ok(preview)
        } else {
            self.vector.get_page_preview(path, page_index)
        }
    }
}
