use crate::zoom::runtime::{
    clear_pending_anchor as core_clear_pending_anchor, get_zoom_state, mark_rendered_zoom,
};
use crate::zoom::state::{PendingCommittedFrame, HOST_ZOOM_STATE};

pub fn reset_zoom_preview_host(target_zoom: f32) {
    mark_rendered_zoom(target_zoom);
    clear_zoom_preview_host_state(false);
}

pub fn clear_zoom_preview_host_state(clear_pending_anchor: bool) {
    HOST_ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.preview_transform = None;
        state.preview_host = Default::default();
    });
    if clear_pending_anchor {
        core_clear_pending_anchor();
    }
}

pub fn settle_zoom_preview_at_target() {
    HOST_ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let target_zoom = if state.target_zoom.is_finite() && state.target_zoom > 0.0 {
            state.target_zoom
        } else {
            1.0
        };
        state.current_zoom = target_zoom;
        state.visual_zoom = target_zoom;
        // fix: do not overwrite `last_rendered_zoom`, let the render pipeline update it via commit
        // otherwise it causes the CSS scale to drop to 1.0 immediately before the new canvas is ready.
        state.last_animation_timestamp_ms = 0.0;
        state.pending_anchor = None;
        state.preview_transform = None;
        state.preview_host = Default::default();
    });
}

pub fn set_wheel_render_pending(pending: bool) {
    HOST_ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_host.wheel_render_pending = pending;
    });
}

pub fn set_preview_active(active: bool) {
    HOST_ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_host.preview_active = active;
    });
}

pub fn get_preview_active() -> bool {
    HOST_ZOOM_STATE.with(|state| state.borrow().preview_host.preview_active)
}

pub fn get_wheel_render_pending() -> bool {
    HOST_ZOOM_STATE.with(|state| state.borrow().preview_host.wheel_render_pending)
}

pub fn queue_committed_frame(frame_plan: &PendingCommittedFrame) {
    HOST_ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_host.pending_committed_frame = Some(frame_plan.clone());
    });
}

pub fn take_ready_committed_frame() -> Option<PendingCommittedFrame> {
    let zoom_state = get_zoom_state();
    if (zoom_state.target_zoom - zoom_state.visual_zoom).abs() >= 0.001 {
        return None;
    }
    HOST_ZOOM_STATE.with(|state| {
        state
            .borrow_mut()
            .preview_host
            .pending_committed_frame
            .take()
    })
}
