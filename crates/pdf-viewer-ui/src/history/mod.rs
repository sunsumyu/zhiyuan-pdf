//! History domain — unified document undo/redo via `HistoryController`.
//!
//! Layered with `crate::document::DocumentSession`:
//! - `HistoryController` (this module) is the canonical Nutrient-style
//!   `instance.history` API: `undo`/`redo`/`canUndo`/`canRedo`/`clear`/`getState`.
//! - `DocumentSession.undo()` / `DocumentSession.redo()` remain as
//!   convenience wrappers; both layers delegate to the same backing
//!   `state_manager` patch stack — there is exactly one history.

pub mod history_api;
pub mod history_types;
