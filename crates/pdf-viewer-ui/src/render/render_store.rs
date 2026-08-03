// Re-export pure data structures from core.
pub use pdf_viewer_core::render::scheduler::*;

use crate::app_context;

pub fn reset_render_state() {
    app_context::with_render_mut(|render| {
        *render = HostRenderState::default();
    });
}

pub fn is_frame_current(frame_token: u32) -> bool {
    if frame_token == 0 {
        return false;
    }
    app_context::with_render(|render| render.active_frame_token == frame_token)
}

pub fn schedule_render_frame<TPlan: Clone>(
    frame_plan: &TPlan,
    requires_render: impl Fn(&TPlan) -> bool,
    share_render_work: impl Fn(&TPlan, &TPlan) -> bool,
    to_value_plan: impl Fn(&TPlan) -> serde_json::Value,
    from_value_plan: impl Fn(&serde_json::Value) -> Option<TPlan>,
) -> Option<RenderFrameEnvelope<TPlan>> {
    if !requires_render(frame_plan) {
        crate::chain_trace!("schedule.skip", "reason" => "requires_render=false");
        return None;
    }

    app_context::with_render_mut(|state| {
        crate::chain_trace!(
            "schedule.enter",
            "inFlightToken" => state.in_flight_frame_token,
            "queuedToken" => state.queued_frame_token,
            "activeToken" => state.active_frame_token,
        );

        if let Some(in_flight_frame_plan) = state
            .in_flight_frame_plan
            .as_ref()
            .and_then(&from_value_plan)
        {
            if share_render_work(&in_flight_frame_plan, frame_plan) {
                crate::chain_trace!("schedule.skip", "reason" => "share-with-in-flight");
                return None;
            }
        }
        if let Some(queued_frame_plan) = state.queued_frame_plan.as_ref().and_then(&from_value_plan)
        {
            if share_render_work(&queued_frame_plan, frame_plan) {
                crate::chain_trace!("schedule.skip", "reason" => "share-with-queued");
                return None;
            }
        }

        if state.in_flight_frame_token == 0 {
            let token = allocate_render_frame_token(state);
            state.in_flight_frame_token = token;
            state.active_frame_token = token;
            state.in_flight_frame_plan = Some(to_value_plan(frame_plan));
            Some(RenderFrameEnvelope {
                frame_token: token,
                frame_plan: frame_plan.clone(),
            })
        } else {
            let token = allocate_render_frame_token(state);
            state.queued_frame_token = token;
            state.queued_frame_plan = Some(to_value_plan(frame_plan));
            state.active_frame_token = token;
            None
        }
    })
}

pub fn settle_render_frame<TPlan: Clone>(
    frame_token: u32,
    from_value_plan: impl Fn(&serde_json::Value) -> Option<TPlan>,
) -> RenderFrameTransition<TPlan> {
    if frame_token == 0 {
        return RenderFrameTransition::default();
    }

    app_context::with_render_mut(|state| {
        if state.in_flight_frame_token != frame_token {
            return RenderFrameTransition::default();
        }

        let accepted = state.active_frame_token == frame_token;
        let settled_frame_plan = state
            .in_flight_frame_plan
            .take()
            .and_then(|plan| from_value_plan(&plan));
        if accepted {
            state.committed_frame_token = frame_token;
        }

        let next_frame = if let Some(next_frame_plan_value) = state.queued_frame_plan.take() {
            let next_token = state.queued_frame_token.max(1);
            let next_frame_plan = from_value_plan(&next_frame_plan_value);
            state.queued_frame_token = 0;
            if let Some(next_frame_plan) = next_frame_plan {
                state.in_flight_frame_token = next_token;
                state.active_frame_token = next_token;
                state.in_flight_frame_plan = Some(next_frame_plan_value);
                Some(RenderFrameEnvelope {
                    frame_token: next_token,
                    frame_plan: next_frame_plan,
                })
            } else {
                state.in_flight_frame_token = 0;
                state.active_frame_token = 0;
                state.in_flight_frame_plan = None;
                None
            }
        } else {
            state.in_flight_frame_token = 0;
            state.queued_frame_token = 0;
            state.in_flight_frame_plan = None;
            state.queued_frame_plan = None;
            if state.active_frame_token == frame_token {
                state.active_frame_token = 0;
            }
            None
        };

        RenderFrameTransition {
            accepted,
            settled_frame_plan,
            next_frame,
        }
    })
}
