//! Effective page render plan — 从 ui::render::effective_page_plan 迁入。
//! 纯计算 + 调试事件追踪；无 wasm 依赖。
//!
//! 拆分（closeout #06，范式同 canvas/）：类型定义住 mod.rs，逻辑按职责分布：
//! - `overlay_predicates`: overlay 判定与预置（viewport 命中、来源抑制集合）
//! - `object_summary`:      矢量对象 bbox 与调试摘要
//! - `trace`:               overlay 身份与摘要的调试事件追踪
//! - `vector_plan`:         build_effective_vector_render_plan（覆盖层抑制 + 可见对象计划）
//! - `glyph_plan`:          build_effective_glyph_render_plan（字形回退计划）
//! - `tests`:               原内联测试模块（原样搬移）

mod glyph_plan;
mod object_summary;
mod overlay_predicates;
#[cfg(test)]
mod tests;
mod trace;
mod vector_plan;

use std::collections::HashSet;

use crate::edit::paragraph_overlay::ParagraphRenderOverlay;
use crate::edit::replacement_region::ParagraphReplacementRegion;
use crate::models::BoundingBox;

pub use crate::render::source_suppression::SuppressedVectorTextRuns;
pub use glyph_plan::build_effective_glyph_render_plan;
pub use vector_plan::build_effective_vector_render_plan;

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum EffectiveVectorRenderEntry {
    Object {
        object_index: usize,
        suppressed_text_runs: SuppressedVectorTextRuns,
    },
    ParagraphOverlay(ParagraphRenderOverlay),
}

#[derive(Debug, Clone)]
pub struct GlyphParagraphRef {
    pub region_index: usize,
    pub paragraph_index: usize,
    pub suppressed_run_object_ids: HashSet<String>,
    pub suppressed_run_indices: HashSet<usize>,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum EffectiveGlyphRenderEntry {
    Paragraph(GlyphParagraphRef),
    ParagraphOverlay(ParagraphRenderOverlay),
}

struct PreparedOverlay {
    overlay: ParagraphRenderOverlay,
    replacement_region: ParagraphReplacementRegion,
    object_ids: HashSet<String>,
    object_indices: HashSet<usize>,
    path_suppression_bbox: BoundingBox,
    inserted: bool,
    suppressed_text_object_count: usize,
    suppressed_text_run_count: usize,
    object_intersect_count: usize,
    text_intersect_count: usize,
    path_intersect_count: usize,
    image_intersect_count: usize,
    thin_horizontal_path_count: usize,
    suppressed_path_count: usize,
    first_path_summary: Option<String>,
    object_summary_1: Option<String>,
    object_summary_2: Option<String>,
    object_summary_3: Option<String>,
}
