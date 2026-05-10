//! PDF interface layer — Tauri command handlers grouped by domain.
//!
//! Originally a single 1009-line `interfaces/pdf.rs`. Split per Single
//! Responsibility per file (architecture-review §2.3 / Phase 1).
//!
//! Public surface preserved: every command remains accessible as
//! `crate::interfaces::pdf::<command_name>` thanks to the glob re-exports
//! below, so `lib.rs invoke_handler!` and external `use` paths are unchanged.

pub mod annotation;
pub mod comment;
pub mod document;
pub mod helpers;
pub mod page;
pub mod render;
pub mod replace;
pub mod search;
pub mod system;

// ── Command re-exports (flat surface for `crate::interfaces::pdf::*`) ───────

pub use annotation::*;
pub use comment::*;
pub use document::*;
pub use page::*;
pub use render::*;
pub use replace::*;
pub use search::*;
pub use system::*;

// ── Helper re-exports (used by `crate::application::pdf::*` modules) ─────

pub(crate) use helpers::{
    apply_highlight_annotation, apply_text_comment, delete_annotation_internal,
    ensure_document_loaded, update_text_comment,
};
