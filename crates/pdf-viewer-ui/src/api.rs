//! WASM API umbrella — single discovery point for the public surface.
//!
//! Each domain lives in its own folder (`editor/`, `document/`, ...) so that
//! the API file sits next to the types, store, and pipeline it delegates to
//! (cohesion). This module **only re-exports** those handles so that a reader
//! who wants the bird's-eye view of "what does this WASM crate expose to JS?"
//! has one place to look.
//!
//! Logical centralisation, physical decentralisation. Do **not** add new types
//! here — define them in their domain folder and add a `pub use` line below.
//!
//! ## Public handles
//!
//! | Handle | Domain folder | Role |
//! |--------|---------------|------|
//! | [`Application`] | (top-level) | composition root / document lifecycle |
//! | [`AnnotationManager`] | `annotation/` | annotation CRUD |
//! | [`CommentManager`] | `comment/` | comment thread management |
//! | [`DocumentSession`] | `document/` | document lifecycle (open/save/undo) |
//! | [`EditorSession`] | `editor/` | text-editing session |
//! | [`FindSession`] | `find/` | search-in-document |
//! | [`ReviewSession`] | `review/` | review feed (accept/reject) |
//! | [`ViewerSession`] | `viewer/` | viewport / page navigation |

pub use crate::application::Application;
pub use crate::annotation::annotation_api::AnnotationManager;
pub use crate::comment::comment_api::CommentManager;
pub use crate::document::document_api::DocumentSession;
pub use crate::editor::editor_api::EditorSession;
pub use crate::find::find_api::FindSession;
pub use crate::review::review_api::ReviewSession;
pub use crate::viewer::viewer_api::ViewerSession;
