use serde::{Deserialize, Serialize};

use crate::present::plan_builder::{FramePlanRequest, FramePlanResult};
use crate::present::present_store::build_frame_plan_result;
use crate::zoom::host::{
    resolve_preview_tick_decision, resolve_wheel_render_decision, PreviewTickDecision,
    PreviewTickDecisionRequest, WheelRenderDecision, WheelRenderDecisionRequest,
};
use crate::zoom::interaction::{WheelZoomRequest, WheelZoomResult};
use crate::zoom::preview_host::{set_preview_active, set_pending as set_wheel_render_pending};
use crate::zoom::request::resolve_wheel_zoom;
use crate::zoom::zoom_controller::{read_zoom_state, step_frame_plan as step_zoom_frame_plan};

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
    pub preview: crate::zoom::interaction::ZoomPreviewFrame,
    pub decision: PreviewTickDecision,
}

pub fn execute_wheel_zoom(request: WheelZoomHostRequest) -> WheelZoomHostResult {
    let zoom = resolve_wheel_zoom(&request.wheel);
    let zoom_state = read_zoom_state();
    let mut frame_request = request.frame;
    frame_request.display_zoom = zoom.target_zoom;
    let frame_plan = build_frame_plan_result(&frame_request, false);
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
