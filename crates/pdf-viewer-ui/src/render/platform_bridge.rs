use crate::render::workflow::RenderFrameEnvelope;

#[derive(Default)]
pub struct HostRenderLoopState {
    pub active: bool,
    pub pending_frame: Option<RenderFrameEnvelope>,
}

use crate::app_context;

pub fn queue_frame(frame: Option<RenderFrameEnvelope>) -> Option<RenderFrameEnvelope> {
    app_context::with_render_loop_mut(|state| {
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

pub fn advance_frame(
    next_frame: Option<RenderFrameEnvelope>,
) -> Option<RenderFrameEnvelope> {
    app_context::with_render_loop_mut(|state| {
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

pub fn reset_runtime() {
    app_context::with_render_loop_mut(|state| {
        *state = HostRenderLoopState::default();
    });
}
