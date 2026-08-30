//! Preview host state — flags and transforms tracking preview render state.
//!
//! This module only manages preview-host-local state. Orchestration
//! (cross-module state mutations) lives in `zoom_controller.rs`.

use crate::zoom::zoom_store::ZOOM_STATE;

pub fn clear_preview_present() {
    ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_transform = None;
    });
}

pub fn clear_zoom_preview_host_state() {
    ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.preview_transform = None;
        s.preview_host = Default::default();
    });
}

pub fn clear_preview_settle_state() {
    ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();
        let target_zoom = if s.target_zoom.is_finite() && s.target_zoom > 0.0 {
            s.target_zoom
        } else {
            1.0
        };
        s.visual_zoom = target_zoom;
        s.recompute_css_scale();
        s.last_animation_timestamp_ms = 0.0;
        s.pending_anchor = None;
        s.preview_transform = None;
        s.preview_host = Default::default();
    });
}

pub fn set_wheel_render_pending(pending: bool) {
    ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_host.wheel_render_pending = pending;
    });
}

pub fn set_preview_active(active: bool) {
    ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_host.preview_active = active;
    });
}

pub fn set_cancel_pending_render(cancel: bool) {
    ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_host.cancel_pending_render = cancel;
    });
}

pub fn take_cancel_pending_render() -> bool {
    ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();
        let val = s.preview_host.cancel_pending_render;
        s.preview_host.cancel_pending_render = false;
        val
    })
}

pub fn is_preview_active() -> bool {
    ZOOM_STATE.with(|state| state.borrow().preview_host.preview_active)
}

pub fn is_wheel_render_pending() -> bool {
    ZOOM_STATE.with(|state| state.borrow().preview_host.wheel_render_pending)
}
