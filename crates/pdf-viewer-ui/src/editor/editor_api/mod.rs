//! Editor API — WASM bindings for text editing operations.
//!
//! Split (closeout #06, same paradigm as canvas/):
//! - `types`: Request DTOs + EditorSession struct definition
//! - `session`: EditorSession impl block (all methods)
//! - `helpers`: Internal helper functions

mod types;
mod session;
mod helpers;

pub use types::EditorSession;
