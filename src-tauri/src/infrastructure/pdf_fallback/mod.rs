//! Fallback PDF reading via the `pdf` crate (pdf-rs), used when lopdf yields
//! an unusable document (typically scanned/image-only PDFs).
//!
//! Distinct from `infrastructure::pdf::pdf_read`, which is the primary
//! lopdf-based content-parsing path.

pub mod backend;
pub mod classification;
pub mod scanned_backend;
pub mod types;
