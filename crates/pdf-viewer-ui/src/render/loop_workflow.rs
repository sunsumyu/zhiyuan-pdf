use crate::present::plan_builder::FramePlanRequest;
use crate::present::present_store::schedule_render_frame_request;
use crate::render::workflow::RenderFrameEnvelope;
use crate::viewer::viewer_controller::set_zoom;
use crate::zoom::host::{resolve_render_follow_up_decision, RenderFollowUpDecision};
use crate::zoom::zoom_controller::read_zoom_state;

pub fn resolve_follow_up(
    rendered_display_zoom: f32,
    current_target_zoom: f32,
) -> RenderFollowUpDecision {
    resolve_render_follow_up_decision(rendered_display_zoom, current_target_zoom)
}

pub fn schedule_follow_up(
    rendered_display_zoom: f32,
    request: &FramePlanRequest,
) -> Option<RenderFrameEnvelope> {
    let zoom_state = read_zoom_state();
    let decision = resolve_render_follow_up_decision(rendered_display_zoom, zoom_state.target_zoom);
    if !decision.schedule_latest_target {
        return None;
    }

    set_zoom(decision.target_zoom);
    schedule_render_frame_request(request)
}
