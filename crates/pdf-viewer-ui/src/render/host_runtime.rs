use std::cell::RefCell;

use crate::render::workflow::RenderFrameEnvelope;

#[derive(Default)]
pub struct HostRenderLoopState {
    pub active: bool,
    pub pending_frame: Option<RenderFrameEnvelope>,
}

thread_local! {
    pub static RENDER_LOOP_STATE: RefCell<HostRenderLoopState> =
        RefCell::new(HostRenderLoopState::default());
}

pub fn queue_render_loop_frame(frame: Option<RenderFrameEnvelope>) -> Option<RenderFrameEnvelope> {
    RENDER_LOOP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(frame) = frame {
            state.pending_frame = Some(frame);
        }
        if state.active {
            return None;
        }
        let next = state.pending_frame.take();
        if next.is_some() {
            state.active = true;
        }
        next
    })
}

pub fn advance_render_loop_frame(
    next_frame: Option<RenderFrameEnvelope>,
) -> Option<RenderFrameEnvelope> {
    RENDER_LOOP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(frame) = next_frame {
            return Some(frame);
        }
        if let Some(frame) = state.pending_frame.take() {
            return Some(frame);
        }
        state.active = false;
        None
    })
}

pub fn reset_render_loop_runtime() {
    RENDER_LOOP_STATE.with(|state| {
        *state.borrow_mut() = HostRenderLoopState::default();
    });
}
