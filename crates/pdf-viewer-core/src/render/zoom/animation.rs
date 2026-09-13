use serde::{Deserialize, Serialize};

use crate::render::plan_builder::{FramePlanRequest, FramePlanResult};
use crate::render::present_plan::preview_is_settled;
use crate::render::preview::{resolve_preview_present_plan, PreviewPresentPlan};
use crate::render::zoom_state::{HostZoomState, VisualLayoutState, ZoomAnimationStep};

/// Gap below which |visual_zoom - target_zoom| counts as settled. The UI-side
/// committed-frame path must use the same value so both sides agree on when
/// the animation has landed.
pub const ZOOM_SETTLED_THRESHOLD: f32 = 0.0008;

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

pub fn resolve_wheel_zoom_request(
    request: &WheelZoomRequest,
    _visual_layout: Option<&VisualLayoutState>,
) -> WheelZoomResult {
    let _content_width = sanitize_positive(request.content_width, 1.0);
    let _content_height = sanitize_positive(request.content_height, 1.0);
    let _page_width = sanitize_positive(request.page_width, 1.0);
    let _page_height = sanitize_positive(request.page_height, 1.0);
    let zoom_factor = 2.0_f32.powf(-request.delta_y / 800.0);
    let min_zoom = sanitize_positive(request.min_zoom, 0.1).max(0.1);
    let max_zoom = sanitize_positive(request.max_zoom, min_zoom).max(min_zoom);
    let next_zoom = clamp_zoom(request.target_zoom * zoom_factor, min_zoom, max_zoom);
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
    // Zoom always centers content — anchor fields are retained for the Wasm
    // contract (TS passes viewport_x/y for transform-origin reporting) but the
    // anchor page-point computation is no longer needed.
    let viewport_width = sanitize_non_negative(request.viewport_width, 0.0);
    let viewport_height = sanitize_non_negative(request.viewport_height, 0.0);
    let anchor_pdf_x = clamp_unit(viewport_x / viewport_width.max(1.0));
    let anchor_pdf_y = clamp_unit(viewport_y / viewport_height.max(1.0));
    WheelZoomResult {
        target_zoom: next_zoom,
        anchor_pdf_x,
        anchor_pdf_y,
        anchor_viewport_x: viewport_x,
        anchor_viewport_y: viewport_y,
        transform_origin_x: anchor_pdf_x * request.content_width,
        transform_origin_y: anchor_pdf_y * request.content_height,
    }
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
        return ZoomAnimationStep {
            visual_zoom: state.visual_zoom,
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

    let settled = diff.abs() < ZOOM_SETTLED_THRESHOLD;
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
    ZoomAnimationStep {
        visual_zoom: state.visual_zoom,
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
    state.last_animation_timestamp_ms = 0.0;
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
        1.0, // No CSS scaling — css_scale is always 1.0
    );
    ZoomPreviewFrame {
        settled: step.settled,
        visual_zoom: step.visual_zoom,
        rendered_base_zoom,
        preview_present,
        frame_plan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::plan_builder::compute_viewport_layout_result;
    use crate::render::zoom::state::HostZoomState;

    fn make_state(initial_zoom: f32) -> HostZoomState {
        HostZoomState {
            target_zoom: initial_zoom,
            visual_zoom: initial_zoom,
            last_rendered_zoom: initial_zoom,
            ..Default::default()
        }
    }

    #[test]
    fn tdd_full_raf_lifecycle_settles_and_stops() {
        let mut state = make_state(1.0);
        state.target_zoom = 1.5;
        state.last_animation_timestamp_ms = 0.0;

        let mut ticks = 0;
        let mut drawing_delay_active = false;
        let mut drawing_delay_started_at = 0.0_f64;
        const SETTLE_DRAWING_DELAY_MS: f64 = 50.0;
        let mut settle_fired = false;

        for i in 0..600 {
            let ts = 1000.0 + (i as f64) * 16.67;
            ticks += 1;
            let step = advance_zoom_animation_state(&mut state, Some(ts));

            if step.settled {
                if !drawing_delay_active {
                    drawing_delay_active = true;
                    drawing_delay_started_at = ts;
                } else if ts - drawing_delay_started_at >= SETTLE_DRAWING_DELAY_MS {
                    settle_fired = true;
                    break;
                }
            }
        }

        assert!(
            settle_fired,
            "loop should stop after settle + drawing delay; ran {} ticks",
            ticks
        );
        assert!(
            (state.visual_zoom - 1.5).abs() < 0.001,
            "visual_zoom must reach target: {}",
            state.visual_zoom
        );
        assert!(
            ticks >= 3,
            "drawing delay requires multiple settled ticks: {}",
            ticks
        );
    }

    #[test]
    fn tdd_second_gesture_after_settle_animates_again() {
        let mut state = make_state(1.0);
        state.target_zoom = 1.5;
        state.last_animation_timestamp_ms = 0.0;
        for i in 0..600 {
            let ts = 1000.0 + (i as f64) * 16.67;
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.settled {
                let final_zoom = state.visual_zoom;
                commit_rendered_zoom(&mut state, final_zoom);
                break;
            }
        }
        assert!(
            (state.visual_zoom - 1.5).abs() < 0.001,
            "first gesture must complete"
        );
        assert!(
            (state.last_rendered_zoom - 1.5).abs() < 0.001,
            "last_rendered must track settled zoom"
        );

        state.target_zoom = 2.25;
        state.last_animation_timestamp_ms = 0.0;
        let mut settled_second = false;
        for i in 0..600 {
            let ts = 2000.0 + (i as f64) * 16.67;
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.settled {
                settled_second = true;
                break;
            }
        }
        assert!(settled_second, "second gesture must also settle");
        assert!(
            (state.visual_zoom - 2.25).abs() < 0.001,
            "second gesture must reach new target"
        );
    }

    #[test]
    fn tdd_wheel_request_changes_target_zoom() {
        let state = make_state(1.0);
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

        let result = resolve_wheel_zoom_request(&request, state.visual_layout.as_ref());

        assert!(
            result.target_zoom > 1.0,
            "scroll-up should zoom in: {}",
            result.target_zoom
        );
        assert!(
            result.target_zoom < 2.0,
            "single scroll should not overshoot: {}",
            result.target_zoom
        );
    }

    #[test]
    fn tdd_rapid_wheel_events_accumulate() {
        let mut state = make_state(1.0);
        for i in 0..5 {
            let request = WheelZoomRequest {
                delta_y: -80.0,
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
            let result = resolve_wheel_zoom_request(&request, state.visual_layout.as_ref());
            state.target_zoom = result.target_zoom;
            state.last_animation_timestamp_ms = 0.0;
            let ts = 1000.0 + (i as f64) * 16.67;
            let _step = advance_zoom_animation_state(&mut state, Some(ts));
        }
        assert!(
            state.target_zoom > 1.3,
            "5 zoom-in events should produce target >> 1.0: {}",
            state.target_zoom
        );
        assert!(
            state.visual_zoom > 1.0,
            "visual_zoom should have advanced past 1.0: {}",
            state.visual_zoom
        );
    }

    #[test]
    fn tdd_animation_settles_after_enough_ticks() {
        let mut state = make_state(1.0);
        state.target_zoom = 1.5;
        state.last_animation_timestamp_ms = 0.0;
        let mut settled_at = None;
        for i in 0..300 {
            let ts = 1000.0 + (i as f64) * 16.67;
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.settled && settled_at.is_none() {
                settled_at = Some(i);
            }
        }
        assert!(
            settled_at.is_some(),
            "animation should settle within 300 ticks"
        );
        assert!(
            settled_at.unwrap() < 200,
            "animation should settle quickly: {}",
            settled_at.unwrap()
        );
        assert!(
            (state.visual_zoom - 1.5).abs() < 0.001,
            "visual_zoom should equal target: {}",
            state.visual_zoom
        );
    }

    #[test]
    fn tdd_zoom_out_works() {
        let state = make_state(2.0);
        let request = WheelZoomRequest {
            delta_y: 100.0,
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
        let result = resolve_wheel_zoom_request(&request, state.visual_layout.as_ref());
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

    #[test]
    fn tdd_settle_render_zoom_source_matches_resolved_target() {
        let mut state = make_state(1.0);
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
        let result = resolve_wheel_zoom_request(&request, state.visual_layout.as_ref());
        state.target_zoom = result.target_zoom;
        state.last_animation_timestamp_ms = 0.0;
        let session_current_zoom = state.target_zoom;

        for i in 0..600 {
            let ts = 1000.0 + (i as f64) * 16.67;
            let step = advance_zoom_animation_state(&mut state, Some(ts));
            if step.settled {
                let settled_zoom = state.visual_zoom;
                commit_rendered_zoom(&mut state, settled_zoom);
                break;
            }
        }
        assert!(
            (session_current_zoom - state.visual_zoom).abs() < 0.001,
            "session zoom ({}) must equal settled visual zoom ({})",
            session_current_zoom,
            state.visual_zoom
        );
        assert!(
            session_current_zoom > 1.05,
            "session zoom must move past pre-gesture value: {}",
            session_current_zoom
        );
    }

    #[test]
    fn viewport_layout_centers_at_all_zoom_levels() {
        // When display > viewport: content_left clamps to 0 (not negative),
        // host expands to display size.
        let result = compute_viewport_layout_result(1000.0, 1200.0, 800.0, 900.0);
        assert!(
            (result.content_left - 0.0).abs() < 0.001,
            "content_left must not go negative: {}",
            result.content_left
        );
        assert!(
            (result.content_top - 0.0).abs() < 0.001,
            "content_top must not go negative: {}",
            result.content_top
        );
        assert!((result.host_width - 1000.0).abs() < 0.001);
        assert!((result.host_height - 1200.0).abs() < 0.001);
    }

    #[test]
    fn viewport_layout_centers_when_display_smaller_than_viewport() {
        // When display < viewport, content is centered.
        let result = compute_viewport_layout_result(595.0, 842.0, 800.0, 900.0);
        assert!((result.content_left - (800.0 - 595.0) * 0.5).abs() < 0.001);
        assert!((result.content_top - (900.0 - 842.0) * 0.5).abs() < 0.001);
    }

    /// CRITICAL: Verify content_left continuity across the display==viewport
    /// boundary. content_left must change smoothly (not jump) as display_width
    /// crosses viewport_width.
    #[test]
    fn content_left_continuous_across_viewport_boundary() {
        let viewport_w = 800.0;
        let viewport_h = 600.0;

        // Sweep display_width from below to above viewport_width
        let mut prev_left: Option<f32> = None;
        for i in 0..100 {
            let display_w = 750.0 + (i as f32) * 1.0; // 750 to 850, crossing 800
            let display_h = 562.5 + (i as f32) * 0.75; // maintain aspect ratio
            let result =
                compute_viewport_layout_result(display_w, display_h, viewport_w, viewport_h);
            if let Some(prev) = prev_left {
                let delta = (result.content_left - prev).abs();
                assert!(
                    delta < 0.6, // max change per 1px step
                    "JUMP in content_left at display_w={}: prev={} curr={} delta={}",
                    display_w,
                    prev,
                    result.content_left,
                    delta
                );
            }
            prev_left = Some(result.content_left);
        }
    }
}
