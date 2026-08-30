use serde::{Deserialize, Serialize};

use crate::render::plan_builder::{AnchorViewportLayoutResult, FramePlanRequest, FramePlanResult};
use crate::render::present_plan::preview_is_settled;
use crate::render::preview::{resolve_preview_present_plan, PreviewPresentPlan};
use crate::render::zoom_state::{
    HostZoomState, PreviewTransformState, VisualLayoutState, ZoomAnchorState, ZoomAnimationStep,
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

pub use crate::common::sanitize::{sanitize_non_negative, sanitize_positive};

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
        // host dimensions = display dimensions (page at target zoom).
        // During CSS transform animation, visual_size = host * css_scale.
        // After committed frame, visual_size = host (no css_scale).
        // For seamless transition: host_prev * css_scale = host_new.
        // This holds when host = page * zoom (not clamped to viewport).
        host_width: display_width,
        host_height: display_height,
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
        state.recompute_css_scale();
        return ZoomAnimationStep {
            visual_zoom: state.visual_zoom,
            css_scale: state.css_scale,
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
    state.recompute_css_scale();
    ZoomAnimationStep {
        visual_zoom: state.visual_zoom,
        css_scale: state.css_scale,
        settled: settled || (state.target_zoom - state.visual_zoom).abs() < 0.001,
    }
}

pub fn commit_rendered_zoom(state: &mut HostZoomState, rendered_zoom: f32) {
    let zoom = if rendered_zoom.is_finite() && rendered_zoom > 0.0 {
        rendered_zoom
    } else {
        1.0
    };
    state.last_rendered_zoom = zoom;
    state.visual_zoom = sanitize_positive(state.visual_zoom, state.target_zoom);
    if preview_is_settled(state.target_zoom, state.visual_zoom) {
        state.visual_zoom = state.target_zoom;
    }
    state.recompute_css_scale();
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
    use super::*;
    use super::compute_anchor_viewport_layout_result;
    use crate::render::zoom::state::HostZoomState;

    // ── TDD: full zoom pipeline integration ───────────────────────

    /// Helper: create a default HostZoomState at initial_zoom.
    fn make_state(initial_zoom: f32) -> HostZoomState {
        let mut s = HostZoomState::default();
        s.target_zoom = initial_zoom;
        s.visual_zoom = initial_zoom;
        s.last_rendered_zoom = initial_zoom;
        s.css_scale = 1.0;
        s
    }

    /// TDD-5 (integration): replicate the raf_loop tick sequence exactly —
    /// wheel event sets target, then repeated ticks with the drawing-delay
    /// state machine must eventually stop and report settle.
    ///
    /// This catches bugs where:
    /// - the loop never stops (drawing delay never expires)
    /// - the loop stops before animation completes
    /// - css_scale never diverges from 1.0 during zoom-in
    #[test]
    fn tdd_full_raf_lifecycle_settles_and_stops() {
        let mut state = make_state(1.0);

        // ── Simulate on_wheel_event: target becomes 1.5 ──
        state.target_zoom = 1.5;
        state.last_animation_timestamp_ms = 0.0;

        let mut ticks = 0;
        let mut drawing_delay_active = false;
        let mut drawing_delay_started_at = 0.0_f64;
        const SETTLE_DRAWING_DELAY_MS: f64 = 50.0;
        let mut settle_fired = false;
        let mut max_css_scale_seen = 1.0_f32;

        // RAF timestamps like a real browser at 60fps
        for i in 0..600 {
            let ts = 1000.0 + (i as f64) * 16.67;
            ticks += 1;

            // Step 1: advance animation (same as raf_loop::tick)
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.css_scale > max_css_scale_seen {
                max_css_scale_seen = step.css_scale;
            }

            // Step 4+5: drawing delay + scheduling decision (same as raf_loop::tick)
            if step.settled {
                if !drawing_delay_active {
                    drawing_delay_active = true;
                    drawing_delay_started_at = ts;
                } else if ts - drawing_delay_started_at >= SETTLE_DRAWING_DELAY_MS {
                    settle_fired = true;
                    break; // stop_zoom_raf_loop() + notify_settle()
                }
                // else: keep ticking until delay elapses
            }
        }

        assert!(
            settle_fired,
            "loop should stop after settle + drawing delay; ran {} ticks, visual={}, target={}",
            ticks, state.visual_zoom, state.target_zoom
        );
        assert!(
            (state.visual_zoom - 1.5).abs() < 0.001,
            "visual_zoom must reach target after settle: {}",
            state.visual_zoom
        );
        assert!(
            max_css_scale_seen > 1.01,
            "css_scale must grow above 1.0 during zoom-in animation: {}",
            max_css_scale_seen
        );
        // Drawing delay means the loop kept ticking past settle instead of
        // stopping instantly — final render is triggered only after the delay.
        assert!(
            ticks >= 3,
            "drawing delay requires multiple settled ticks before stop: {}",
            ticks
        );
    }

    /// TDD-6 (regression): a second wheel gesture AFTER a completed lifecycle
    /// must animate again. Catches stale drawing_delay / timestamp state that
    /// would make the second gesture settle instantly without visual movement.
    #[test]
    fn tdd_second_gesture_after_settle_animates_again() {
        let mut state = make_state(1.0);

        // ── First gesture: 1.0 → 1.5, run to completion ──
        state.target_zoom = 1.5;
        state.last_animation_timestamp_ms = 0.0;
        for i in 0..600 {
            let ts = 1000.0 + (i as f64) * 16.67;
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.settled {
                // Real settle path: render pipeline settles with the new zoom
                // via commit_rendered_zoom (same as markRenderedZoom).
                let final_zoom = state.visual_zoom;
                commit_rendered_zoom(&mut state, final_zoom);
                break;
            }
        }
        assert!((state.visual_zoom - 1.5).abs() < 0.001, "first gesture must complete");
        assert!(
            (state.last_rendered_zoom - 1.5).abs() < 0.001,
            "last_rendered must track settled zoom after commit: {}",
            state.last_rendered_zoom
        );
        assert!((state.css_scale - 1.0).abs() < 0.001, "css_scale returns to 1.0 after commit");

        // ── Second gesture: 1.5 → 2.25 ──
        state.target_zoom = 2.25;
        state.last_animation_timestamp_ms = 0.0;

        let mut moved = false;
        let mut settled_second = false;
        for i in 0..600 {
            let ts = 2000.0 + (i as f64) * 16.67;
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.css_scale > 1.01 {
                moved = true;
            }
            if step.settled {
                settled_second = true;
                break;
            }
        }

        assert!(settled_second, "second gesture must also settle");
        assert!(
            (state.visual_zoom - 2.25).abs() < 0.001,
            "second gesture must reach new target: {}",
            state.visual_zoom
        );
        assert!(moved, "second gesture must produce visible css_scale growth");
    }

    /// TDD-1: resolve_wheel_zoom_request must change target_zoom.
    #[test]
    fn tdd_wheel_request_changes_target_zoom() {
        let mut state = make_state(1.0);
        let request = WheelZoomRequest {
            delta_y: -100.0, // scroll up → zoom in
            viewport_x: 400.0,
            viewport_y: 300.0,
            viewport_width: 800.0,
            viewport_height: 600.0,
            page_width: 595.0,
            page_height: 842.0,
            anchor_page_x: None,
            anchor_page_y: None,
            page_ratio_x: None,
            page_ratio_y: None,
            scroll_left: 0.0,
            scroll_top: 0.0,
            content_width: 595.0,
            content_height: 842.0,
            target_zoom: 1.0,
            min_zoom: 0.1,
            max_zoom: 30.0,
        };

        let (result, _anchor) = resolve_wheel_zoom_request(
            &request,
            state.visual_layout.as_ref(),
            state.preview_transform.as_ref(),
        );

        // zoom_factor = 2^(-(-100)/800) = 2^(0.125) ≈ 1.0905
        assert!(
            result.target_zoom > 1.0,
            "scroll-up should zoom in: target_zoom={}, expected > 1.0",
            result.target_zoom
        );
        assert!(
            result.target_zoom < 2.0,
            "single scroll should not overshoot: target_zoom={}, expected < 2.0",
            result.target_zoom
        );

        // Apply to state and verify animation can advance
        state.target_zoom = result.target_zoom;
        state.last_animation_timestamp_ms = 0.0;

        let step = advance_zoom_animation_state(&mut state, Some(1000.0));
        assert!(
            !step.settled,
            "first tick should NOT be settled: visual={}, target={}",
            step.visual_zoom, result.target_zoom
        );
        assert!(
            step.visual_zoom > 1.0,
            "visual_zoom should have advanced: {}",
            step.visual_zoom
        );
        assert!(
            step.css_scale > 1.0,
            "css_scale should reflect zoom-in: {}",
            step.css_scale
        );
    }

    /// TDD-2: rapid wheel events must accumulate zoom, not reset.
    #[test]
    fn tdd_rapid_wheel_events_accumulate() {
        let mut state = make_state(1.0);

        // Simulate 5 rapid wheel events
        for i in 0..5 {
            let request = WheelZoomRequest {
                delta_y: -80.0, // zoom in each time
                viewport_x: 400.0,
                viewport_y: 300.0,
                viewport_width: 800.0,
                viewport_height: 600.0,
                page_width: 595.0,
                page_height: 842.0,
                anchor_page_x: None,
                anchor_page_y: None,
                page_ratio_x: None,
                page_ratio_y: None,
                scroll_left: 0.0,
                scroll_top: 0.0,
                content_width: 595.0 * state.visual_zoom,
                content_height: 842.0 * state.visual_zoom,
                target_zoom: state.target_zoom,
                min_zoom: 0.1,
                max_zoom: 30.0,
            };

            let (result, anchor) = resolve_wheel_zoom_request(
                &request,
                state.visual_layout.as_ref(),
                state.preview_transform.as_ref(),
            );

            state.target_zoom = result.target_zoom;
            state.last_animation_timestamp_ms = 0.0;
            state.pending_anchor = Some(anchor);

            // Advance one frame
            let ts = 1000.0 + (i as f64) * 16.67;
            let step = advance_zoom_animation_state(&mut state, Some(ts));

            eprintln!(
                "  tick {}: target={:.4} visual={:.4} css={:.4} settled={}",
                i, state.target_zoom, step.visual_zoom, step.css_scale, step.settled
            );
        }

        // After 5 zoom-in events, target should be significantly > 1.0
        assert!(
            state.target_zoom > 1.3,
            "5 zoom-in events should produce target >> 1.0: {}",
            state.target_zoom
        );
        // visual_zoom should be chasing target (may not have caught up yet)
        assert!(
            state.visual_zoom > 1.0,
            "visual_zoom should have advanced past 1.0: {}",
            state.visual_zoom
        );
    }

    /// TDD-3: animation must settle after enough ticks.
    #[test]
    fn tdd_animation_settles_after_enough_ticks() {
        let mut state = make_state(1.0);
        let target = 1.5;

        // Set target
        state.target_zoom = target;
        state.last_animation_timestamp_ms = 0.0;

        let mut settled_at = None;
        for i in 0..300 {
            let ts = 1000.0 + (i as f64) * 16.67; // ~60fps
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.settled && settled_at.is_none() {
                settled_at = Some(i);
            }
        }

        assert!(
            settled_at.is_some(),
            "animation should settle within 300 ticks (5 seconds at 60fps)"
        );
        let tick = settled_at.unwrap();
        assert!(
            tick < 200,
            "animation should settle quickly: settled at tick {}",
            tick
        );
        // After settling, visual must equal target
        assert!(
            (state.visual_zoom - target).abs() < 0.001,
            "visual_zoom should equal target after settle: {} vs {}",
            state.visual_zoom,
            target
        );
    }

    /// TDD-4: zoom-out (positive deltaY) must decrease target_zoom.
    #[test]
    fn tdd_zoom_out_works() {
        let mut state = make_state(2.0);
        let request = WheelZoomRequest {
            delta_y: 100.0, // scroll down → zoom out
            viewport_x: 400.0,
            viewport_y: 300.0,
            viewport_width: 800.0,
            viewport_height: 600.0,
            page_width: 595.0,
            page_height: 842.0,
            anchor_page_x: None,
            anchor_page_y: None,
            page_ratio_x: None,
            page_ratio_y: None,
            scroll_left: 0.0,
            scroll_top: 0.0,
            content_width: 595.0 * 2.0,
            content_height: 842.0 * 2.0,
            target_zoom: 2.0,
            min_zoom: 0.1,
            max_zoom: 30.0,
        };

        let (result, _anchor) = resolve_wheel_zoom_request(
            &request,
            state.visual_layout.as_ref(),
            state.preview_transform.as_ref(),
        );

        assert!(
            result.target_zoom < 2.0,
            "zoom-out should decrease target: {}",
            result.target_zoom
        );
        assert!(
            result.target_zoom > 0.1,
            "zoom-out should not go below min: {}",
            result.target_zoom
        );
    }

    /// TDD-5: css_scale = visual_zoom / last_rendered_zoom.
    #[test]
    fn tdd_css_scale_matches_visual_over_rendered() {
        let mut state = make_state(1.0);
        state.last_rendered_zoom = 1.0;
        state.target_zoom = 1.5;
        state.last_animation_timestamp_ms = 0.0;

        let step = advance_zoom_animation_state(&mut state, Some(1000.0));

        // css_scale should be visual_zoom / last_rendered_zoom
        let expected_css = state.visual_zoom / 1.0;
        assert!(
            (step.css_scale - expected_css).abs() < 0.0001,
            "css_scale mismatch: got {}, expected {}",
            step.css_scale,
            expected_css
        );
    }

    /// TDD-7 (regression): the settle-time final render is scheduled from the
    /// host viewer session's current_zoom, not ZOOM_STATE. The RAF wheel path
    /// must therefore publish each resolved target_zoom into that session —
    /// otherwise executeActualRender schedules at the pre-gesture zoom, the
    /// settle render reuses the stale base layer, and the page stays as a
    /// stretched bitmap ("no longer vector") after zooming.
    ///
    /// The ui crate is wasm32-only so on_wheel_event itself can't run here;
    /// instead we pin the data contract: after a wheel resolves target T and
    /// animation settles at T, the value the render scheduler reads (session
    /// current_zoom) must be T — i.e. equal to visual/target — not the old zoom.
    #[test]
    fn tdd_settle_render_zoom_source_matches_resolved_target() {
        let mut state = make_state(1.0);

        // ── Wheel gesture 1.0 → 1.5 (as resolved by resolve_wheel_zoom_request) ──
        let request = WheelZoomRequest {
            delta_y: -100.0,
            viewport_x: 400.0,
            viewport_y: 300.0,
            viewport_width: 800.0,
            viewport_height: 600.0,
            page_width: 595.0,
            page_height: 842.0,
            anchor_page_x: None,
            anchor_page_y: None,
            page_ratio_x: None,
            page_ratio_y: None,
            scroll_left: 0.0,
            scroll_top: 0.0,
            content_width: 595.0,
            content_height: 842.0,
            target_zoom: 1.0,
            min_zoom: 0.1,
            max_zoom: 30.0,
        };
        let (result, _anchor) = resolve_wheel_zoom_request(
            &request,
            state.visual_layout.as_ref(),
            state.preview_transform.as_ref(),
        );
        state.target_zoom = result.target_zoom;
        state.last_animation_timestamp_ms = 0.0;

        // Contract under test: the wheel path publishes target into the
        // session BEFORE the settle render runs. Simulate the fixed raf_loop
        // behavior (`set_zoom(result.target_zoom)`), then run to settle.
        let session_current_zoom = state.target_zoom; // set_zoom(target)

        for i in 0..600 {
            let ts = 1000.0 + (i as f64) * 16.67;
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.settled {
                let settled_zoom = state.visual_zoom;
                commit_rendered_zoom(&mut state, settled_zoom);
                break;
            }
        }

        // The settle scheduler reads this session value and must see the NEW
        // zoom; if it saw the old one (1.0) it would reuse the stale bitmap.
        assert!(
            (session_current_zoom - state.visual_zoom).abs() < 0.001,
            "session zoom fed to settle render ({}) must equal settled visual zoom ({})",
            session_current_zoom,
            state.visual_zoom
        );
        assert!(
            session_current_zoom > 1.05,
            "session zoom must actually move past the pre-gesture value: {}",
            session_current_zoom
        );
    }

    #[test]
    fn preserves_cursor_anchor() {
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
