use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelRenderDecisionRequest {
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    pub preview_active: bool,
    pub allow_render_during_preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelRenderDecision {
    pub request_render_now: bool,
    pub defer_until_settled: bool,
    pub skip_render: bool,
    pub delay_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTickDecisionRequest {
    pub settled: bool,
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    pub wheel_render_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTickDecision {
    pub continue_preview: bool,
    pub flush_committed_frame: bool,
    pub request_render_now: bool,
    pub keep_wheel_render_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderFollowUpDecision {
    pub schedule_latest_target: bool,
    pub target_zoom: f32,
}

fn needs_render(target_zoom: f32, rendered_zoom: f32) -> bool {
    (target_zoom - rendered_zoom).abs() >= 0.001
}

fn wheel_render_idle_ms(preview_active: bool, allow_render_during_preview: bool) -> u32 {
    if preview_active && !allow_render_during_preview {
        96
    } else if preview_active {
        72
    } else {
        64
    }
}

fn preview_is_active(preview_active: bool, target_zoom: f32, visual_zoom: f32) -> bool {
    preview_active || (target_zoom - visual_zoom).abs() >= 0.001
}

pub fn resolve_wheel_render_decision(
    request: WheelRenderDecisionRequest,
) -> WheelRenderDecision {
    if !needs_render(request.target_zoom, request.last_rendered_zoom) {
        return WheelRenderDecision {
            skip_render: true,
            delay_ms: wheel_render_idle_ms(
                request.preview_active,
                request.allow_render_during_preview,
            ),
            ..WheelRenderDecision::default()
        };
    }

    if preview_is_active(
        request.preview_active,
        request.target_zoom,
        request.visual_zoom,
    ) {
        if request.allow_render_during_preview {
            return WheelRenderDecision {
                request_render_now: true,
                delay_ms: wheel_render_idle_ms(
                    request.preview_active,
                    request.allow_render_during_preview,
                ),
                ..WheelRenderDecision::default()
            };
        }
        return WheelRenderDecision {
            defer_until_settled: true,
            delay_ms: wheel_render_idle_ms(
                request.preview_active,
                request.allow_render_during_preview,
            ),
            ..WheelRenderDecision::default()
        };
    }

    WheelRenderDecision {
        request_render_now: true,
        delay_ms: wheel_render_idle_ms(request.preview_active, request.allow_render_during_preview),
        ..WheelRenderDecision::default()
    }
}

pub fn resolve_preview_tick_decision(
    request: PreviewTickDecisionRequest,
) -> PreviewTickDecision {
    if request.settled {
        return PreviewTickDecision {
            continue_preview: false,
            flush_committed_frame: true,
            request_render_now: request.wheel_render_pending
                && needs_render(request.target_zoom, request.last_rendered_zoom),
            keep_wheel_render_pending: false,
        };
    }

    PreviewTickDecision {
        continue_preview: true,
        flush_committed_frame: false,
        request_render_now: false,
        keep_wheel_render_pending: request.wheel_render_pending,
    }
}

pub fn resolve_render_follow_up_decision(
    rendered_display_zoom: f32,
    current_target_zoom: f32,
) -> RenderFollowUpDecision {
    RenderFollowUpDecision {
        schedule_latest_target: needs_render(current_target_zoom, rendered_display_zoom),
        target_zoom: current_target_zoom,
    }
}
