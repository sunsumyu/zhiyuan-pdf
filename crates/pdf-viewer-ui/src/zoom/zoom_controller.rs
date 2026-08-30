//! Zoom controller — thin orchestrator for wheel zoom and preview host stepping.
//!
//! Sub-modules:
//! - `zoom_authority`: Core zoom state mutations (ADR-0001 single write entry)
//! - `zoom_anchor`: Anchor management (scroll/layout from pending anchors)
//! - `zoom_preview`: Preview host state (flags, transforms)
//! - `zoom_frame`: Animation frame stepping + committed frame queue
//!
//! This module re-exports everything from sub-modules so existing import
//! paths (`crate::zoom::zoom_controller::*`) continue to work.

pub use super::zoom_authority::*;
pub use super::zoom_anchor::*;
pub use super::zoom_preview::*;
pub use super::zoom_frame::*;

use crate::present::plan_builder::{
    build_frame_plan_result, AnchorViewportLayoutResult, FramePlanRequest, FramePlanResult,
};
use crate::present::present_store;
use crate::viewer::viewer_controller::set_zoom;
use pdf_viewer_core::render::zoom::animation::{
    WheelZoomRequest, WheelZoomResult,
};
use crate::zoom::zoom_store::HostZoomState;

use serde::{Deserialize, Serialize};
use crate::present::present_store::build_frame_plan_result as present_build_frame_plan_result;
use pdf_viewer_core::render::zoom_host::{
    resolve_preview_tick_decision, resolve_wheel_render_decision, PreviewTickDecision,
    PreviewTickDecisionRequest, WheelRenderDecision, WheelRenderDecisionRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelZoomHostRequest {
    pub wheel: WheelZoomRequest,
    pub frame: FramePlanRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelZoomHostResult {
    pub zoom: WheelZoomResult,
    pub render_decision: WheelRenderDecision,
    pub frame_plan: FramePlanResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewHostStepRequest {
    pub frame: FramePlanRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewHostStepResult {
    pub preview: pdf_viewer_core::render::zoom::animation::ZoomPreviewFrame,
    pub decision: PreviewTickDecision,
}

pub fn execute_wheel_zoom(request: WheelZoomHostRequest) -> WheelZoomHostResult {
    let zoom = resolve_wheel_zoom(&request.wheel);
    let zoom_state = read_zoom_state();
    let mut frame_request = request.frame;
    frame_request.display_zoom = zoom_state.visual_zoom;
    let frame_plan = present_build_frame_plan_result(&frame_request, false);
    let render_decision = resolve_wheel_render_decision(WheelRenderDecisionRequest {
        target_zoom: zoom_state.target_zoom,
        visual_zoom: zoom_state.visual_zoom,
        last_rendered_zoom: zoom_state.last_rendered_zoom,
        preview_active: zoom_state.preview_host.preview_active,
        allow_render_during_preview: frame_plan.allow_render_during_preview,
    });
    set_preview_active(true);
    set_wheel_render_pending(render_decision.defer_until_settled);

    WheelZoomHostResult {
        zoom,
        render_decision,
        frame_plan,
    }
}

pub fn step_preview_host(request: PreviewHostStepRequest) -> PreviewHostStepResult {
    let preview = step_zoom_frame_plan(&request.frame);
    let zoom_state = read_zoom_state();
    let decision = resolve_preview_tick_decision(PreviewTickDecisionRequest {
        settled: preview.settled,
        target_zoom: zoom_state.target_zoom,
        visual_zoom: zoom_state.visual_zoom,
        last_rendered_zoom: zoom_state.last_rendered_zoom,
        wheel_render_pending: zoom_state.preview_host.wheel_render_pending,
    });
    set_preview_active(decision.continue_preview);
    set_wheel_render_pending(decision.keep_wheel_render_pending);

    PreviewHostStepResult { preview, decision }
}

pub fn resolve_wheel_zoom(request: &WheelZoomRequest) -> WheelZoomResult {
    let (result, pending_anchor) = crate::zoom::zoom_store::ZOOM_STATE.with(|state| {
        let s = state.borrow();
        pdf_viewer_core::render::zoom_interaction::resolve_wheel_zoom_request(
            request,
            s.visual_layout.as_ref(),
            s.preview_transform.as_ref(),
        )
    });
    crate::zoom::zoom_store::ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();
        if s.visual_zoom <= 0.0 {
            s.visual_zoom = s.last_rendered_zoom.max(1.0);
        }
        s.target_zoom = result.target_zoom;
        s.last_animation_timestamp_ms = 0.0;
        s.pending_anchor = Some(pending_anchor);
    });
    set_zoom(result.target_zoom);
    result
}

pub fn tick_zoom_state(
    input: pdf_viewer_core::render::zoom_host::ZoomTickInput,
) -> pdf_viewer_core::render::zoom_host::ZoomTickOutput {
    crate::zoom::zoom_store::ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        pdf_viewer_core::render::zoom_host::tick_zoom_state_core(&mut state, &input)
    })
}

/// Clear preview host state with optional anchor clearing (orchestration).
///
/// This is the entry point for callers that need to clear preview state
/// and optionally the pending anchor. The preview module only handles
/// its own state; this function coordinates the cross-module operation.
pub fn clear_preview_host_with_anchor(do_clear_anchor: bool) {
    super::zoom_preview::clear_zoom_preview_host_state();
    if do_clear_anchor {
        super::zoom_anchor::clear_pending_anchor();
    }
}

/// Settle zoom to target and clear all preview/anchor state (orchestration).
pub fn settle_zoom_preview_at_target() {
    super::zoom_preview::clear_preview_settle_state();
    super::zoom_anchor::clear_pending_anchor();
}

/// Reset zoom preview host: mark rendered zoom + clear preview state.
pub fn reset_zoom_preview_host(target_zoom: f32) {
    super::zoom_authority::mark_rendered_zoom(target_zoom);
    super::zoom_preview::clear_zoom_preview_host_state();
}
