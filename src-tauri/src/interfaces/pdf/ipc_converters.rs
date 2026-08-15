//! Thin IPC adapters for the pdf interface commands.
//!
//! These were originally the home of orchestration logic; that logic now lives
//! in `application/pdf/edit_commands.rs`. This module exists only to satisfy
//! the re-export surface in `interfaces/pdf/mod.rs` and to convert between
//! IPC-layer types and application-layer types.

pub(crate) use crate::application::pdf::edit_commands::{
    delete_annotation_internal, ensure_document_loaded, update_text_comment,
};
