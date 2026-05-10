// ─────────────────────────────────────────────────────────────────────────────
// models — domain type hub.
//
// Each sub-module owns a single responsibility:
//   geometry    — BoundingBox
//   font        — FontHints, ResolvedFontFace, …
//   styled_run  — StyledRun, NativeTextModel, NativePageModel, …
//   layout      — LayoutRun, LayoutParagraph, SemanticRegion, enums, …
//   glyph       — GlyphPaintRun, GlyphPaintPlan, …
//   vector      — VectorPageModel, VectorRenderObject, …
//   document_runtime — PageState, ReadDocumentMeta, …
//   interaction — FieldProjection, InteractionTarget, …
//
// Everything is re-exported at this level so that existing `use models::Foo`
// paths continue to compile unchanged.
// ─────────────────────────────────────────────────────────────────────────────

pub mod geometry;
pub mod font;
pub mod styled_run;
pub mod layout;
pub mod glyph;
pub mod vector;
pub mod document_runtime;
pub mod interaction;

pub use geometry::*;
pub use font::*;
pub use styled_run::*;
pub use layout::*;
pub use glyph::*;
pub use vector::*;
pub use document_runtime::*;
pub use interaction::*;
