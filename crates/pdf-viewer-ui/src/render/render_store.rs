use std::cell::RefCell;

// Re-export pure data structures from core.
pub use pdf_viewer_core::render::scheduler::*;

use pdf_viewer_core::render::plan_builder::FramePlanResult;

// The frame-plan store is typed (FramePlanResult, not serde_json::Value):
// plans are cloned in and out directly. The former to_value/from_value
// round-trip cloned the whole JSON tree on every share-check and settle, and
// forced stringly-typed plan reads (`.get("render_zoom")`) at the call sites.
thread_local! {
    pub static RENDER_STATE: RefCell<HostRenderState<FramePlanResult>> =
        RefCell::new(HostRenderState::default());
}

pub fn reset_render_state() {
    RENDER_STATE.with(|state| {
        *state.borrow_mut() = HostRenderState::default();
    });
}

pub fn is_render_frame_current(frame_token: u32) -> bool {
    if frame_token == 0 {
        return false;
    }
    RENDER_STATE.with(|state| state.borrow().active_frame_token == frame_token)
}

pub fn schedule_render_frame(
    frame_plan: &FramePlanResult,
    requires_render: impl Fn(&FramePlanResult) -> bool,
    share_render_work: impl Fn(&FramePlanResult, &FramePlanResult) -> bool,
) -> Option<RenderFrameEnvelope<FramePlanResult>> {
    if !requires_render(frame_plan) {
        crate::chain_trace!("schedule.skip", "reason" => "requires_render=false");
        return None;
    }

    RENDER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        crate::chain_trace!(
            "schedule.enter",
            "inFlightToken" => state.in_flight_frame_token,
            "queuedToken" => state.queued_frame_token,
            "activeToken" => state.active_frame_token,
        );

        if let Some(in_flight_frame_plan) = state.in_flight_frame_plan.as_ref() {
            if share_render_work(in_flight_frame_plan, frame_plan) {
                crate::chain_trace!("schedule.skip", "reason" => "share-with-in-flight");
                return None;
            }
        }
        if let Some(queued_frame_plan) = state.queued_frame_plan.as_ref() {
            if share_render_work(queued_frame_plan, frame_plan) {
                crate::chain_trace!("schedule.skip", "reason" => "share-with-queued");
                return None;
            }
        }

        if state.in_flight_frame_token == 0 {
            let token = allocate_render_frame_token(&mut state);
            state.in_flight_frame_token = token;
            state.active_frame_token = token;
            state.in_flight_frame_plan = Some(frame_plan.clone());
            Some(RenderFrameEnvelope {
                frame_token: token,
                frame_plan: frame_plan.clone(),
            })
        } else {
            let token = allocate_render_frame_token(&mut state);
            state.queued_frame_token = token;
            state.queued_frame_plan = Some(frame_plan.clone());
            // Do NOT update active_frame_token here — the in-flight frame
            // is still rendering and must remain "current" until it settles.
            // Updating it prematurely causes the render loop to treat the
            // in-flight frame as stale and abort it, leaving the queue stuck.
            None
        }
    })
}

pub fn settle_render_frame(frame_token: u32) -> RenderFrameTransition<FramePlanResult> {
    if frame_token == 0 {
        return RenderFrameTransition::default();
    }

    RENDER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.in_flight_frame_token != frame_token {
            return RenderFrameTransition::default();
        }

        let accepted = state.active_frame_token == frame_token;
        let settled_frame_plan = state.in_flight_frame_plan.take();
        if accepted {
            state.committed_frame_token = frame_token;
        }

        let next_frame = if let Some(next_frame_plan) = state.queued_frame_plan.take() {
            let next_token = state.queued_frame_token.max(1);
            state.queued_frame_token = 0;
            state.in_flight_frame_token = next_token;
            state.active_frame_token = next_token;
            state.in_flight_frame_plan = Some(next_frame_plan.clone());
            Some(RenderFrameEnvelope {
                frame_token: next_token,
                frame_plan: next_frame_plan,
            })
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

/// Drop the queued frame without promoting or committing it. Used when a
/// superseding render makes the queued frame stale: unlike
/// [`settle_render_frame`], which only accepts the in-flight token, this
/// clears the queue slot directly and leaves the in-flight frame untouched.
pub fn drop_queued_render_frame(frame_token: u32) -> bool {
    if frame_token == 0 {
        return false;
    }
    RENDER_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.queued_frame_token == frame_token {
            state.queued_frame_token = 0;
            state.queued_frame_plan = None;
            true
        } else {
            false
        }
    })
}
