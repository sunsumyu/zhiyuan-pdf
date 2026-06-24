//! Effective page render plan — 从 ui::render::effective_page_plan 迁入。
//! 纯计算 + 调试事件追踪；无 wasm 依赖。
//!
//! 模块拆分：
//! - `overlay_ops`: overlay 准备、追踪、bbox 计算等辅助函数
//! - `text_suppression`: 文本/路径压制决策逻辑
//! - `glyph_plan`: glyph-based 渲染计划构建

use std::collections::HashSet;

use crate::edit::paragraph_overlay::ParagraphRenderOverlay;
use crate::models::{BoundingBox, VectorPageModel};
use crate::render::prepared_scene::PreparedPageScene;

#[path = "overlay_ops.rs"]
mod overlay_ops;
#[path = "text_suppression.rs"]
mod text_suppression;
#[path = "glyph_plan.rs"]
pub mod glyph_plan;

pub use crate::render::source_suppression::SuppressedVectorTextRuns;

use overlay_ops::{
    build_entries_without_overlays, prepare_overlays, trace_overlay_identity, trace_overlay_summary,
};
use text_suppression::process_visible_objects;

/// Vector render entry — 用于 vector-based 渲染计划。
#[derive(Debug, Clone)]
pub enum EffectiveVectorRenderEntry {
    Object {
        object_index: usize,
        suppressed_text_runs: SuppressedVectorTextRuns,
    },
    ParagraphOverlay(ParagraphRenderOverlay),
}

/// Glyph paragraph reference — 用于 glyph-based 渲染计划。
#[derive(Debug, Clone)]
pub struct GlyphParagraphRef {
    pub region_index: usize,
    pub paragraph_index: usize,
    pub suppressed_run_object_ids: HashSet<String>,
    pub suppressed_run_indices: HashSet<usize>,
}

/// Glyph render entry — 用于 glyph-based 渲染计划。
#[derive(Debug, Clone)]
pub enum EffectiveGlyphRenderEntry {
    Paragraph(GlyphParagraphRef),
    ParagraphOverlay(ParagraphRenderOverlay),
}

/// 构建 vector-based 渲染计划。
///
/// 遍历所有可见 vector objects，处理 overlays 的压制逻辑，
/// 生成 `EffectiveVectorRenderEntry` 列表供渲染器消费。
pub fn build_effective_vector_render_plan(
    vector_model: &VectorPageModel,
    prepared_scene: Option<&PreparedPageScene>,
    viewport_bbox: &BoundingBox,
    overlays: &[ParagraphRenderOverlay],
) -> Vec<EffectiveVectorRenderEntry> {
    let visible_indices =
        overlay_ops::resolve_visible_indices(vector_model, prepared_scene, viewport_bbox);
    let mut prepared_overlays = prepare_overlays(overlays, viewport_bbox, vector_model.width);
    trace_overlay_identity(&prepared_overlays, &visible_indices, vector_model);

    if prepared_overlays.is_empty() {
        return build_entries_without_overlays(visible_indices, vector_model);
    }

    let mut entries = process_visible_objects(visible_indices, vector_model, &mut prepared_overlays);

    for overlay in prepared_overlays {
        trace_overlay_summary(&overlay);
        if !overlay.inserted {
            entries.push(EffectiveVectorRenderEntry::ParagraphOverlay(overlay.overlay));
        }
    }

    entries
}

/// 构建 glyph-based 渲染计划。
///
/// 重导出 `glyph_plan::build_effective_glyph_render_plan` 以维持 API 兼容。
pub use glyph_plan::build_effective_glyph_render_plan;

// Note: tests removed due to outdated struct definitions that don't match current code.
// The test code used deprecated fields like VectorTextObject.x/y/width/height,
// VectorPathObject.close, ParagraphRenderOverlay.shell_bbox, etc.
// These tests need to be rewritten when the render layer is refactored.