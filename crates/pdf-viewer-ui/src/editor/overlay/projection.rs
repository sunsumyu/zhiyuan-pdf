use crate::editor::bridge::ParagraphInteractionTarget;
use crate::editor::mode::get_active_editor_state;
use crate::editor::editor_controller::collect_paragraph_targets;
use pdf_viewer_core::geometry::coordinate_transform::PdfToPageViewTransform;
use serde::{Deserialize, Serialize};

const EDITOR_Y_BUFFER: f32 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedParagraphInteractionTarget {
    pub paragraph_id: String,
    pub region_id: String,
    pub page_index: u16,
    pub text: String,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: String,
    pub font_style: String,
    pub color: String,
    #[serde(default)]
    pub text_decoration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedEditorShell {
    pub paragraph_id: String,
    pub region_id: String,
    pub page_index: u16,
    pub text: String,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub font_family: String,
    pub font_size_px: f32,
    pub font_weight: String,
    pub font_style: String,
    pub color: String,
    #[serde(default)]
    pub text_decoration: String,
    #[serde(default)]
    pub initial_caret_index: usize,
}

// const WIDTH_BUFFER: f32 = 6.0; // 已移除硬编码缓冲

pub fn project_paragraph_interaction_targets(
    display_zoom: f32,
    page_height: f32,
) -> Vec<ProjectedParagraphInteractionTarget> {
    let zoom = sanitize_display_zoom(display_zoom);
    let transform = PdfToPageViewTransform::new(page_height);
    let targets_value = collect_paragraph_targets();
    let targets: Vec<ParagraphInteractionTarget> =
        serde_wasm_bindgen::from_value(targets_value).unwrap_or_default();
    targets
        .into_iter()
        .map(|target| {
            // BBox 约定: top < bottom (内部已统一为 Y-Down, y向下)
            // transform.point 现在不进行 flips，仅由 bbox 直接映射。
            let visual_top_left = transform.point(target.bbox.left, target.bbox.top);
            let visual_bottom_right = transform.point(target.bbox.right, target.bbox.bottom);

            ProjectedParagraphInteractionTarget {
                paragraph_id: target.paragraph_id,
                region_id: target.region_id,
                page_index: target.page_index,
                text: target.text,
                left: (visual_top_left.x * zoom),
                top: (visual_top_left.y * zoom),
                width: ((visual_bottom_right.x - visual_top_left.x) * zoom).max(0.0),
                height: ((visual_bottom_right.y - visual_top_left.y) * zoom).max(0.0),
                font_family: target.font_family,
                font_size: target.font_size,
                font_weight: target.font_weight,
                font_style: target.font_style,
                color: target.color,
                text_decoration: target.text_decoration,
            }
        })
        .collect()
}

pub fn project_active_editor_shell(
    display_zoom: f32,
    page_height: f32,
) -> Option<ProjectedEditorShell> {
    let zoom = sanitize_display_zoom(display_zoom);
    let transform = PdfToPageViewTransform::new(page_height);
    let active_state = get_active_editor_state()?;
    let caret_index = active_state.normalized_caret_index();
    let draft_text = active_state.current_text().to_string();
    let target = active_state.target;

    // 同上：bbox_top 为视觉顶部, bbox_bottom 为视觉底部 (Y-Down 统一语义)
    let visual_top_left = transform.point(target.bbox_left, target.bbox_top);
    let visual_bottom_right = transform.point(target.bbox_right, target.bbox_bottom);

    Some(ProjectedEditorShell {
        paragraph_id: target.paragraph_id,
        region_id: target.region_id,
        page_index: target.page_index,
        text: draft_text,
        left: (visual_top_left.x * zoom),
        top: (visual_top_left.y * zoom) - (EDITOR_Y_BUFFER * zoom),
        width: ((visual_bottom_right.x - visual_top_left.x) * zoom).max(0.0),
        // Keep the projected shell bounds aligned with the actual overlay
        // canvas, which adds a symmetric Y buffer for caret painting.
        height: (((visual_bottom_right.y - visual_top_left.y) + (EDITOR_Y_BUFFER * 2.0)) * zoom)
            .max(0.0),
        font_family: target.font_family,
        font_size_px: (target.font_size * zoom),
        font_weight: target.font_weight,
        font_style: target.font_style,
        color: target.color,
        text_decoration: target.text_decoration,
        initial_caret_index: caret_index,
    })
}

fn sanitize_display_zoom(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}
