pub mod models;
pub mod algorithms;
pub mod document;
pub mod geometry;
pub mod persistence;
pub mod render;
pub mod text;
pub mod typography;
pub mod utils;
#[path = "analysis/analyzer.rs"]
pub mod analyzer;
pub use geometry::bbox_utils;
pub use geometry::coordinate_transform;
pub use geometry::field_projection;
pub use geometry::layout_engine;
pub use geometry::reflow_engine;
pub use utils::sanitize;
pub use persistence::engine as persistence_engine;
pub use persistence::history_manager;
pub use persistence::models as persistence_models;
pub use persistence::state_manager;
pub use render::paint_plan;
pub use render::renderer;
pub use render::snapshot_paint_plan;
pub use text::editable_segments;
pub use text::glyph_layout;
pub use text::index_convert;
pub use text::list_semantics;
pub use text::semantic_axiom;
pub use text::style_preservation;
pub use utils::debug;
pub use typography::font_resolver;
pub use document::page_region_context;

pub fn get_core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
