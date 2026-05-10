use serde::{Deserialize, Serialize};

use crate::render::present_plan::preview_is_settled;
use crate::render::plan_builder::{
    AnchorViewportLayoutResult, FramePlanRequest, FramePlanResult,
};
use crate::render::preview::{resolve_preview_present_plan, PreviewPresentPlan};
use crate::render::zoom_state::{
    HostZoomState, PreviewTransformState, VisualLayoutState, ZoomAnchorState,
    ZoomAnimationStep,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelZoomRequest {
    pub delta_y: f32,
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub anchor_page_x: Option<f32>,
    pub anchor_page_y: Option<f32>,
    pub page_ratio_x: Option<f32>,
    pub page_ratio_y: Option<f32>,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub content_width: f32,
    pub content_height: f32,
    pub target_zoom: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelZoomResult {
    pub target_zoom: f32,
    pub anchor_pdf_x: f32,
    pub anchor_pdf_y: f32,
    pub anchor_viewport_x: f32,
    pub anchor_viewport_y: f32,
    pub transform_origin_x: f32,
    pub transform_origin_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnchorScrollRequest {
    pub display_width: f32,
    pub display_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub anchor_pdf_x: f32,
    pub anchor_pdf_y: f32,
    pub viewport_x: f32,
    pub viewport_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnchorScrollResult {
    pub scroll_left: f32,
    pub scroll_top: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomLimitsRequest {
    pub page_width: f32,
    pub page_height: f32,
    pub device_pixel_ratio: f32,
    pub max_zoom: f32,
    pub max_canvas_dim: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomLimitsResult {
    pub safe_max_zoom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomPreviewFrame {
    pub settled: bool,
    pub visual_zoom: f32,
    pub rendered_base_zoom: f32,
    pub css_scale: f32,
    pub preview_present: PreviewPresentPlan,
    pub frame_plan: FramePlanResult,
}

pub fn clamp_zoom(value: f32, min_zoom: f32, max_zoom: f32) -> f32 {
    if !value.is_finite() {
        return min_zoom.max(1.0);
    }
    value.max(min_zoom).min(max_zoom)
}

pub use crate::utils::sanitize::{sanitize_non_negative, sanitize_positive};

pub fn clamp_f32(value: f32, min_value: f32, max_value: f32) -> f32 {
    let min_value = if min_value.is_finite() {
        min_value
    } else {
        0.0
    };
    let max_value = if max_value.is_finite() && max_value >= min_value {
        max_value
    } else {
        min_value
    };
    if !value.is_finite() {
        return min_value;
    }
    if value < min_value {
        min_value
    } else if value > max_value {
        max_value
    } else {
        value
    }
}

pub fn clamp_unit(value: f32) -> f32 {
    clamp_f32(value, 0.0, 1.0)
}

pub fn centered_offset(content_size: f32, viewport_size: f32) -> f32 {
    ((viewport_size - content_size).max(0.0)) * 0.5
}

pub fn compute_anchor_scroll_result(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
    anchor_page_x: f32,
    anchor_page_y: f32,
    page_width: f32,
    page_height: f32,
    viewport_x: f32,
    viewport_y: f32,
) -> AnchorScrollResult {
    let display_width = sanitize_positive(display_width, 1.0);
    let display_height = sanitize_positive(display_height, 1.0);
    let viewport_width = sanitize_non_negative(viewport_width, 0.0);
    let viewport_height = sanitize_non_negative(viewport_height, 0.0);
    let page_width = sanitize_positive(page_width, 1.0);
    let page_height = sanitize_positive(page_height, 1.0);
    let viewport_x = if viewport_x.is_finite() {
        viewport_x
    } else {
        0.0
    };
    let viewport_y = if viewport_y.is_finite() {
        viewport_y
    } else {
        0.0
    };
    let offset_x = centered_offset(display_width, viewport_width);
    let offset_y = centered_offset(display_height, viewport_height);
    let viewport_content_x = viewport_x - offset_x;
    let viewport_content_y = viewport_y - offset_y;
    let anchor_display_x = if page_width > 0.0 {
        clamp_f32(anchor_page_x, 0.0, page_width) * (display_width / page_width)
    } else {
        0.0
    };
    let anchor_display_y = if page_height > 0.0 {
        clamp_f32(anchor_page_y, 0.0, page_height) * (display_height / page_height)
    } else {
        0.0
    };
    AnchorScrollResult {
        scroll_left: (anchor_display_x - viewport_content_x).max(0.0),
        scroll_top: (anchor_display_y - viewport_content_y).max(0.0),
    }
}

pub fn resolve_anchor_from_visible_preview_state(
    layout: &VisualLayoutState,
    preview_transform: Option<&PreviewTransformState>,
    scroll_left: f32,
    scroll_top: f32,
    viewport_x: f32,
    viewport_y: f32,
    page_width: f32,
    page_height: f32,
) -> (f32, f32) {
    let display_zoom = sanitize_positive(layout.display_zoom, 1.0);
    let content_left = sanitize_non_negative(layout.content_left, 0.0);
    let content_top = sanitize_non_negative(layout.content_top, 0.0);
    let translate_x = preview_transform
        .map(|transform| transform.translate_x)
        .filter(|value| value.is_finite())
        .unwrap_or(0.0);
    let translate_y = preview_transform
        .map(|transform| transform.translate_y)
        .filter(|value| value.is_finite())
        .unwrap_or(0.0);
    let css_scale = preview_transform
        .map(|transform| transform.css_scale)
        .filter(|value| value.is_finite() && *value > 0.0001)
        .unwrap_or(1.0);
    let visible_content_x =
        ((scroll_left + viewport_x) - content_left - translate_x) / css_scale.max(0.0001);
    let visible_content_y =
        ((scroll_top + viewport_y) - content_top - translate_y) / css_scale.max(0.0001);
    let anchor_page_x = clamp_f32(visible_content_x / display_zoom, 0.0, page_width);
    let anchor_page_y = clamp_f32(visible_content_y / display_zoom, 0.0, page_height);
    (anchor_page_x, anchor_page_y)
}

pub fn compute_anchor_viewport_layout_result(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
    anchor_page_x: f32,
    anchor_page_y: f32,
    page_width: f32,
    page_height: f32,
    viewport_x: f32,
    viewport_y: f32,
) -> AnchorViewportLayoutResult {
    let display_width = sanitize_positive(display_width, 1.0);
    let display_height = sanitize_positive(display_height, 1.0);
    let viewport_width = sanitize_positive(viewport_width, 1.0);
    let viewport_height = sanitize_positive(viewport_height, 1.0);
    let page_width = sanitize_positive(page_width, 1.0);
    let page_height = sanitize_positive(page_height, 1.0);
    let viewport_x = if viewport_x.is_finite() {
        viewport_x
    } else {
        0.0
    };
    let viewport_y = if viewport_y.is_finite() {
        viewport_y
    } else {
        0.0
    };
    let point_x = if page_width > 0.0 {
        clamp_f32(anchor_page_x, 0.0, page_width) * (display_width / page_width)
    } else {
        0.0
    };
    let point_y = if page_height > 0.0 {
        clamp_f32(anchor_page_y, 0.0, page_height) * (display_height / page_height)
    } else {
        0.0
    };
    let content_left = (viewport_x - point_x).max(0.0);
    let content_top = (viewport_y - point_y).max(0.0);
    let scroll_left = (content_left + point_x - viewport_x).max(0.0);
    let scroll_top = (content_top + point_y - viewport_y).max(0.0);
    AnchorViewportLayoutResult {
        host_width: (content_left + display_width)
            .max(scroll_left + viewport_width)
            .max(viewport_width),
        host_height: (content_top + display_height)
            .max(scroll_top + viewport_height)
            .max(viewport_height),
        content_left,
        content_top,
        scroll_left,
        scroll_top,
    }
}

pub fn resolve_wheel_zoom_request(
    request: &WheelZoomRequest,
    visual_layout: Option<&VisualLayoutState>,
    preview_transform: Option<&PreviewTransformState>,
) -> (WheelZoomResult, ZoomAnchorState) {
    let content_width = sanitize_positive(request.content_width, 1.0);
    let content_height = sanitize_positive(request.content_height, 1.0);
    let page_width = sanitize_positive(request.page_width, 1.0);
    let page_height = sanitize_positive(request.page_height, 1.0);
    let zoom_factor = 2.0_f32.powf(-request.delta_y / 800.0);
    let min_zoom = sanitize_positive(request.min_zoom, 0.1).max(0.1);
    let max_zoom = sanitize_positive(request.max_zoom, min_zoom).max(min_zoom);
    let next_zoom = clamp_zoom(request.target_zoom * zoom_factor, min_zoom, max_zoom);
    let viewport_width = sanitize_non_negative(request.viewport_width, 0.0);
    let viewport_height = sanitize_non_negative(request.viewport_height, 0.0);
    let scroll_left = sanitize_non_negative(request.scroll_left, 0.0);
    let scroll_top = sanitize_non_negative(request.scroll_top, 0.0);
    let viewport_x = if request.viewport_x.is_finite() {
        request.viewport_x
    } else {
        0.0
    };
    let viewport_y = if request.viewport_y.is_finite() {
        request.viewport_y
    } else {
        0.0
    };
    let offset_x = centered_offset(content_width, viewport_width);
    let offset_y = centered_offset(content_height, viewport_height);
    let viewport_content_x = viewport_x - offset_x;
    let viewport_content_y = viewport_y - offset_y;
    let layout_anchor = visual_layout.map(|layout| {
        resolve_anchor_from_visible_preview_state(
            layout,
            preview_transform,
            scroll_left,
            scroll_top,
            viewport_x,
            viewport_y,
            page_width,
            page_height,
        )
    });
    let fallback_anchor_page_x = layout_anchor
        .map(|(x, _)| x)
        .unwrap_or(clamp_unit((scroll_left + viewport_content_x) / content_width) * page_width);
    let fallback_anchor_page_y = layout_anchor
        .map(|(_, y)| y)
        .unwrap_or(clamp_unit((scroll_top + viewport_content_y) / content_height) * page_height);
    let anchor_page_x = request
        .anchor_page_x
        .filter(|value: &f32| value.is_finite())
        .or_else(|| {
            request
                .page_ratio_x
                .filter(|value: &f32| value.is_finite())
                .map(|ratio: f32| clamp_unit(ratio) * page_width)
        })
        .unwrap_or(fallback_anchor_page_x)
        .max(0.0)
        .min(page_width);
    let anchor_page_y = request
        .anchor_page_y
        .filter(|value: &f32| value.is_finite())
        .or_else(|| {
            request
                .page_ratio_y
                .filter(|value: &f32| value.is_finite())
                .map(|ratio: f32| clamp_unit(ratio) * page_height)
        })
        .unwrap_or(fallback_anchor_page_y)
        .max(0.0)
        .min(page_height);
    let anchor_pdf_x = clamp_unit(anchor_page_x / page_width);
    let anchor_pdf_y = clamp_unit(anchor_page_y / page_height);
    let result = WheelZoomResult {
        target_zoom: next_zoom,
        anchor_pdf_x,
        anchor_pdf_y,
        anchor_viewport_x: viewport_x,
        anchor_viewport_y: viewport_y,
        transform_origin_x: anchor_pdf_x * content_width,
        transform_origin_y: anchor_pdf_y * content_height,
    };
    let pending_anchor = ZoomAnchorState {
        anchor_page_x,
        anchor_page_y,
        page_width,
        page_height,
        viewport_x: result.anchor_viewport_x,
        viewport_y: result.anchor_viewport_y,
    };
    (result, pending_anchor)
}

pub fn resolve_zoom_limits_result(request: &ZoomLimitsRequest) -> ZoomLimitsResult {
    let dpr = if request.device_pixel_ratio.is_finite() && request.device_pixel_ratio > 0.0 {
        request.device_pixel_ratio
    } else {
        1.0
    };
    let page_max = request.page_width.max(request.page_height).max(1.0);
    let max_canvas_dim = request.max_canvas_dim.max(1.0);
    let requested_max_zoom = request.max_zoom.max(0.1);
    let safe_max_zoom = (max_canvas_dim / (page_max * dpr))
        .min(requested_max_zoom)
        .max(0.1);
    ZoomLimitsResult { safe_max_zoom }
}

pub fn advance_zoom_animation_state(
    state: &mut HostZoomState,
    timestamp_ms: Option<f64>,
) -> ZoomAnimationStep {
    let target_zoom = sanitize_positive(state.target_zoom, 1.0);
    let visual_zoom = sanitize_positive(state.visual_zoom, target_zoom);
    state.target_zoom = target_zoom;
    state.visual_zoom = visual_zoom;
    if preview_is_settled(target_zoom, visual_zoom) {
        state.visual_zoom = target_zoom;
        state.last_animation_timestamp_ms = 0.0;
        let base_zoom = if state.last_rendered_zoom > 0.0 {
            state.last_rendered_zoom
        } else {
            1.0
        };
        return ZoomAnimationStep {
            visual_zoom: state.visual_zoom,
            css_scale: state.visual_zoom / base_zoom,
            settled: true,
        };
    }
    let diff = target_zoom - visual_zoom;

    let timestamp_ms = timestamp_ms.filter(|value| value.is_finite() && *value > 0.0);
    let dt = if let Some(timestamp_ms) = timestamp_ms {
        let dt = if state.last_animation_timestamp_ms > 0.0 {
            ((timestamp_ms - state.last_animation_timestamp_ms) / 1000.0) as f32
        } else {
            1.0 / 60.0
        };
        state.last_animation_timestamp_ms = timestamp_ms;
        clamp_f32(dt, 1.0 / 240.0, 1.0 / 24.0)
    } else {
        state.last_animation_timestamp_ms = 0.0;
        1.0 / 60.0
    };

    let settled = diff.abs() < 0.0008;
    if settled {
        state.visual_zoom = target_zoom;
    } else {
        let response = if diff.abs() > 1.5 {
            18.0
        } else if diff.abs() > 0.5 {
            15.0
        } else if diff.abs() > 0.15 {
            12.0
        } else {
            9.0
        };
        let alpha = 1.0 - (-response * dt).exp();
        state.visual_zoom += diff * alpha;
    }
    let base_zoom = if state.last_rendered_zoom > 0.0 {
        state.last_rendered_zoom
    } else {
        1.0
    };
    ZoomAnimationStep {
        visual_zoom: state.visual_zoom,
        css_scale: state.visual_zoom / base_zoom,
        settled: settled || (state.target_zoom - state.visual_zoom).abs() < 0.001,
    }
}

pub fn commit_rendered_zoom(state: &mut HostZoomState, rendered_zoom: f32) {
    let zoom = if rendered_zoom.is_finite() && rendered_zoom > 0.0 {
        rendered_zoom
    } else {
        1.0
    };
    state.current_zoom = zoom;
    state.last_rendered_zoom = zoom;
    state.visual_zoom = sanitize_positive(state.visual_zoom, state.target_zoom);
    if preview_is_settled(state.target_zoom, state.visual_zoom) {
        state.visual_zoom = state.target_zoom;
    }
    state.last_animation_timestamp_ms = 0.0;
    state.preview_transform = None;
}

pub fn build_zoom_preview_frame<F>(
    request: &FramePlanRequest,
    state: &mut HostZoomState,
    build_frame_plan: F,
) -> ZoomPreviewFrame
where
    F: Fn(&FramePlanRequest, &mut HostZoomState) -> FramePlanResult,
{
    let step = advance_zoom_animation_state(state, Some(request.timestamp_ms));
    let rendered_base_zoom = if state.last_rendered_zoom > 0.0 {
        state.last_rendered_zoom
    } else {
        1.0
    };
    let mut frame_request = request.clone();
    frame_request.display_zoom = step.visual_zoom.max(0.1);
    let frame_plan = build_frame_plan(&frame_request, state);
    let current_layout = state.visual_layout.as_ref();
    let preview_present = resolve_preview_present_plan(
        current_layout
            .map(|layout| layout.content_left)
            .unwrap_or(frame_plan.content_left),
        current_layout
            .map(|layout| layout.content_top)
            .unwrap_or(frame_plan.content_top),
        request.scroll_left.max(0.0),
        request.scroll_top.max(0.0),
        frame_plan.content_left,
        frame_plan.content_top,
        frame_plan.scroll_left,
        frame_plan.scroll_top,
        step.css_scale,
    );
    if step.settled {
        state.preview_transform = None;
    } else {
        state.preview_transform = Some(PreviewTransformState {
            translate_x: preview_present.translate_x,
            translate_y: preview_present.translate_y,
            css_scale: preview_present.css_scale,
        });
    }
    ZoomPreviewFrame {
        settled: step.settled,
        visual_zoom: step.visual_zoom,
        rendered_base_zoom,
        css_scale: step.css_scale,
        preview_present,
        frame_plan,
    }
}

#[cfg(test)]
mod tests {
    use super::compute_anchor_viewport_layout_result;

    #[test]
    fn anchor_layout_preserves_cursor_point_when_page_is_centered() {
        let page_width = 595.0;
        let page_height = 842.0;
        let display_width = 595.0;
        let display_height = 842.0;
        let viewport_width = 800.0;
        let viewport_height = 900.0;
        let anchor_page_x = 320.0;
        let anchor_page_y = 450.0;
        let viewport_x = 420.0;
        let viewport_y = 500.0;

        let result = compute_anchor_viewport_layout_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor_page_x,
            anchor_page_y,
            page_width,
            page_height,
            viewport_x,
            viewport_y,
        );

        let displayed_anchor_x = result.content_left + (anchor_page_x / page_width) * display_width;
        let displayed_anchor_y =
            result.content_top + (anchor_page_y / page_height) * display_height;
        assert!((displayed_anchor_x - viewport_x).abs() < 0.001);
        assert!((displayed_anchor_y - viewport_y).abs() < 0.001);
    }
}
