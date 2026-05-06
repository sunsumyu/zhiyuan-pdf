use std::cell::RefCell;

#[derive(Debug, Clone)]
pub struct HostRenderState<TPlan> {
    pub next_frame_token: u32,
    pub active_frame_token: u32,
    pub committed_frame_token: u32,
    pub in_flight_frame_token: u32,
    pub queued_frame_token: u32,
    pub in_flight_frame_plan: Option<TPlan>,
    pub queued_frame_plan: Option<TPlan>,
}

impl<TPlan> Default for HostRenderState<TPlan> {
    fn default() -> Self {
        Self {
            next_frame_token: 1,
            active_frame_token: 0,
            committed_frame_token: 0,
            in_flight_frame_token: 0,
            queued_frame_token: 0,
            in_flight_frame_plan: None,
            queued_frame_plan: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderFrameEnvelope<TPlan> {
    pub frame_token: u32,
    pub frame_plan: TPlan,
}

#[derive(Debug, Clone)]
pub struct RenderFrameTransition<TPlan> {
    pub accepted: bool,
    pub settled_frame_plan: Option<TPlan>,
    pub next_frame: Option<RenderFrameEnvelope<TPlan>>,
}

impl<TPlan> Default for RenderFrameTransition<TPlan> {
    fn default() -> Self {
        Self {
            accepted: false,
            settled_frame_plan: None,
            next_frame: None,
        }
    }
}

thread_local! {
    pub static HOST_RENDER_STATE: RefCell<HostRenderState<serde_json::Value>> =
        RefCell::new(HostRenderState::default());
}

pub fn reset_render_state() {
    HOST_RENDER_STATE.with(|state| {
        *state.borrow_mut() = HostRenderState::default();
    });
}

pub fn is_render_frame_current(frame_token: u32) -> bool {
    if frame_token == 0 {
        return false;
    }
    HOST_RENDER_STATE.with(|state| state.borrow().active_frame_token == frame_token)
}

pub fn schedule_render_frame<TPlan: Clone>(
    frame_plan: &TPlan,
    requires_render: impl Fn(&TPlan) -> bool,
    share_render_work: impl Fn(&TPlan, &TPlan) -> bool,
    to_value_plan: impl Fn(&TPlan) -> serde_json::Value,
    from_value_plan: impl Fn(&serde_json::Value) -> Option<TPlan>,
) -> Option<RenderFrameEnvelope<TPlan>> {
    if !requires_render(frame_plan) {
        return None;
    }

    HOST_RENDER_STATE.with(|state| {
        let mut state = state.borrow_mut();

        if let Some(in_flight_frame_plan) = state
            .in_flight_frame_plan
            .as_ref()
            .and_then(&from_value_plan)
        {
            if share_render_work(&in_flight_frame_plan, frame_plan) {
                return None;
            }
        }
        if let Some(queued_frame_plan) = state.queued_frame_plan.as_ref().and_then(&from_value_plan)
        {
            if share_render_work(&queued_frame_plan, frame_plan) {
                return None;
            }
        }

        if state.in_flight_frame_token == 0 {
            let token = allocate_render_frame_token(&mut state);
            state.in_flight_frame_token = token;
            state.active_frame_token = token;
            state.in_flight_frame_plan = Some(to_value_plan(frame_plan));
            Some(RenderFrameEnvelope {
                frame_token: token,
                frame_plan: frame_plan.clone(),
            })
        } else {
            let token = allocate_render_frame_token(&mut state);
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

    HOST_RENDER_STATE.with(|state| {
        let mut state = state.borrow_mut();
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

fn allocate_render_frame_token<TPlan>(state: &mut HostRenderState<TPlan>) -> u32 {
    let token = state.next_frame_token.max(1);
    state.next_frame_token = token.wrapping_add(1).max(1);
    token
}
