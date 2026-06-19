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

#[cfg(test)]
mod tests {
    use super::{
        build_effective_glyph_render_plan, build_effective_vector_render_plan,
        EffectiveGlyphRenderEntry, EffectiveVectorRenderEntry,
    };
    use crate::edit::active_target::ActiveEditorTarget;
    use crate::edit::paragraph_overlay::{ParagraphRenderOverlay, ParagraphRenderOverlayOwner};
    use crate::models::{
        BoundingBox, EditorControlStyle, GlyphPaintParagraph, GlyphPaintPlan, GlyphPaintRegion,
        GlyphPaintRun, LayoutMode, LayoutParagraph, LayoutRole, LayoutRun, ParagraphEditContext,
        StyledRun, VectorPageModel, VectorPathObject, VectorPathSegment, VectorRenderObject,
        VectorTextObject,
    };

    fn horizontal_stroked_path(id: &str, y: f32) -> VectorRenderObject {
        horizontal_stroked_path_between(id, 80.0, 340.0, y)
    }

    fn horizontal_stroked_path_between(
        id: &str,
        left: f32,
        right: f32,
        y: f32,
    ) -> VectorRenderObject {
        VectorRenderObject::Path(VectorPathObject {
            id: id.to_string(),
            segments: vec![
                VectorPathSegment {
                    command: "move".to_string(),
                    points: vec![left, y],
                },
                VectorPathSegment {
                    command: "line".to_string(),
                    points: vec![right, y],
                },
            ],
            stroke_color: Some("#000000".to_string()),
            stroke_width: 1.0,
            fill_color: None,
            close: false,
        })
    }

    fn sample_text_object(id: &str, x: f32, y: f32, text: &str) -> VectorRenderObject {
        VectorRenderObject::Text(VectorTextObject {
            id: id.to_string(),
            x,
            y,
            width: 100.0,
            height: 12.0,
            runs: vec![StyledRun {
                text: text.to_string(),
                tx: x,
                ty: y,
                width: 100.0,
                font_name: "Helvetica".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                char_spacing: 0.0,
                horizontal_scaling: 100.0,
                render_mode: 0,
                object_id: None,
                z_index: 0,
                char_origins: vec![],
                char_widths: vec![],
            }],
            z_index: 0,
        })
    }

    fn sample_overlay(paragraph_id: &str, replaces_source: bool) -> ParagraphRenderOverlay {
        ParagraphRenderOverlay {
            target: ActiveEditorTarget {
                paragraph_id: paragraph_id.to_string(),
                ..Default::default()
            },
            source_text: "original".to_string(),
            draft_text: "edited".to_string(),
            replaces_source,
            owner: ParagraphRenderOverlayOwner::ActiveEditorShell,
            source_object_indices: vec![],
            shell_bbox: None,
        }
    }

    #[test]
    fn test_build_effective_vector_render_plan_empty() {
        let model = VectorPageModel {
            width: 400.0,
            height: 600.0,
            objects: vec![sample_text_object("t1", 50.0, 50.0, "Hello")],
        };
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 400.0,
            bottom: 600.0,
        };
        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[]);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_build_effective_vector_render_plan_with_overlay() {
        let model = VectorPageModel {
            width: 400.0,
            height: 600.0,
            objects: vec![sample_text_object("t1", 50.0, 50.0, "Hello")],
        };
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 400.0,
            bottom: 600.0,
        };
        let overlay = sample_overlay("p1", true);
        let entries = build_effective_vector_render_plan(&model, None, &viewport, &[overlay]);
        // Overlay 应被插入，且 source object 应被压制或部分压制
        assert!(entries.len() >= 1);
    }

    #[test]
    fn test_build_effective_glyph_render_plan_empty() {
        let plan = GlyphPaintPlan {
            regions: vec![],
        };
        let viewport = BoundingBox {
            left: 0.0,
            top: 0.0,
            right: 400.0,
            bottom: 600.0,
        };
        let entries = build_effective_glyph_render_plan(&plan, &viewport, &[]);
        assert!(entries.is_empty());
    }
}