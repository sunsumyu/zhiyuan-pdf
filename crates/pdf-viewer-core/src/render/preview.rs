use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPresentPlan {
    pub translate_x: f32,
    pub translate_y: f32,
    pub css_scale: f32,
}

pub fn resolve_preview_present_plan(
    current_content_left: f32,
    current_content_top: f32,
    current_scroll_left: f32,
    current_scroll_top: f32,
    next_content_left: f32,
    next_content_top: f32,
    next_scroll_left: f32,
    next_scroll_top: f32,
    css_scale: f32,
) -> PreviewPresentPlan {
    let current_visible_left = current_content_left - current_scroll_left;
    let current_visible_top = current_content_top - current_scroll_top;
    let next_visible_left = next_content_left - next_scroll_left;
    let next_visible_top = next_content_top - next_scroll_top;

    PreviewPresentPlan {
        translate_x: next_visible_left - current_visible_left,
        translate_y: next_visible_top - current_visible_top,
        css_scale,
    }
}
