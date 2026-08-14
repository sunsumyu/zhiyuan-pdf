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
//   marker      — VisualMarker, VisualMarkerKind, GraphicType (统一视觉 marker 抽象)
//   document_runtime — PageState, ReadDocumentMeta, …
//   interaction — FieldProjection, InteractionTarget, …
//
// Everything is re-exported at this level so that existing `use models::Foo`
// paths continue to compile unchanged.
// ─────────────────────────────────────────────────────────────────────────────

pub mod document_runtime;
pub mod font;
pub mod geometry;
pub mod glyph;
pub mod interaction;
pub mod layout;
pub mod marker;
pub mod styled_run;
pub mod vector;

pub use document_runtime::*;
pub use font::*;
pub use geometry::*;
pub use glyph::*;
pub use interaction::*;
pub use layout::*;
pub use marker::*;
pub use styled_run::*;
pub use vector::*;
