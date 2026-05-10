//! Structured error type for the PDF backend (architecture-review §3.3 / Phase 3).
//!
//! ## Why
//!
//! The codebase currently stringly-types every fallible path with
//! `Result<T, String>`. That is ergonomic at the Tauri boundary (Tauri can
//! serialize `String` directly to the JS error channel), but loses two
//! valuable properties internally:
//!
//!   1. **Pattern matching** — callers cannot distinguish "document missing"
//!      from "page out of range" without parsing the message text.
//!   2. **Source preservation** — `lopdf::Error`, `std::io::Error` etc. are
//!      flattened to `to_string()` and the underlying cause is lost.
//!
//! ## Strategy
//!
//! Introduce `PdfError` as a typed error enum, with `#[from]` conversions for
//! the upstream errors we care about. To stay 100% backwards-compatible with
//! the existing `Result<T, String>` surface, we also implement
//! `From<PdfError> for String`. This means callers can switch to
//! `Result<T, PdfError>` *gradually* — `?` will still convert into the
//! command-level `Result<T, String>` thanks to the `From` impl.
//!
//! Migration path for a command:
//!
//! ```ignore
//! // Before
//! pub async fn read_metadata(...) -> Result<Meta, String> {
//!     let doc = state.docs.pdf_documents.lock().unwrap()
//!         .get(&path)
//!         .cloned()
//!         .ok_or_else(|| format!("Document not found: {}", path))?;
//!     ...
//! }
//!
//! // After
//! pub async fn read_metadata(...) -> Result<Meta, String> {
//!     let doc = state.docs.pdf_documents.lock().unwrap()
//!         .get(&path)
//!         .cloned()
//!         .ok_or_else(|| PdfError::DocumentNotFound { path: path.clone() })?;
//!     // `?` works because `PdfError: Into<String>`.
//!     ...
//! }
//! ```

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PdfError {
    /// The named document is not in the in-memory cache.
    #[error("Document not found in cache: {path}")]
    DocumentNotFound { path: String },

    /// A page index is outside the document's range.
    #[error("Page {index} not found (document has {total} pages)")]
    PageOutOfRange { index: u16, total: u16 },

    /// A specific annotation could not be located on a page.
    #[error("Annotation {annot_id:?} not found on page {page}")]
    AnnotationNotFound {
        page: u32,
        annot_id: (u32, u16),
    },

    /// PDF parse / structure error (lopdf).
    #[error("PDF parse error: {0}")]
    LopdfError(#[from] lopdf::Error),

    /// Filesystem / disk I/O error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Tokio task join error (panicked or aborted).
    #[error("Async task error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    /// PDF save failed at the disk level.
    #[error("Disk save failure: {message}")]
    SaveFailed { message: String },

    /// Catch-all for legacy string errors during the migration period.
    #[error("{0}")]
    Other(String),
}

impl PdfError {
    /// Constructor for the `Other` catch-all variant.
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

/// Backwards-compatible bridge to the existing `Result<T, String>` surface.
///
/// Lets `?` inside a function returning `Result<T, String>` convert any
/// `PdfError` automatically, so the migration can proceed function-by-function
/// without a big-bang rewrite.
impl From<PdfError> for String {
    fn from(err: PdfError) -> Self {
        err.to_string()
    }
}

/// Convenience alias for backend operations that have not yet been migrated
/// off `Result<T, String>` but want to use the typed error inside.
pub type PdfResult<T> = std::result::Result<T, PdfError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_not_found_renders_path() {
        let err = PdfError::DocumentNotFound {
            path: "C:/x.pdf".into(),
        };
        let msg: String = err.into();
        assert!(msg.contains("C:/x.pdf"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn page_out_of_range_renders_indices() {
        let err = PdfError::PageOutOfRange {
            index: 5,
            total: 3,
        };
        let msg: String = err.into();
        assert!(msg.contains("5"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn other_passes_through() {
        let err = PdfError::other("custom message");
        let msg: String = err.into();
        assert_eq!(msg, "custom message");
    }
}
