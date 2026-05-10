//! Annotation domain — PDF `/Annot` CRUD via `AnnotationManager`.
//!
//! Layered with `crate::comment::CommentManager`:
//! - This module owns the PDF-spec annotation surface (list, get, add,
//!   update, delete, flatten).
//! - `CommentManager` owns the review-session UX (panel state, scoped
//!   queries, overlay loading) on top of the same backend storage.

pub mod annotation_api;
pub mod annotation_types;
