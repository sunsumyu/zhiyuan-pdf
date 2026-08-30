//! Core zoom state mutations — the single write entry point (ADR-0001).
//!
//! All functions access `ZOOM_STATE` directly. Other zoom sub-modules
//! depend on this module for state reads/writes.

use crate::present::present_store::reset_present_runtime;
use crate::render::render_store::reset_render_state;
use pdf_viewer_core::render::zoom::animation::commit_rendered_zoom;
use crate::zoom::zoom_store::{
    reset_zoom_state, HostZoomState, VisualLayoutState, ZOOM_STATE,
};

pub fn reset_zoom_runtime(initial_zoom: f32) {
    reset_zoom_state(initial_zoom);
    reset_render_state();
    reset_present_runtime(true, false);
}

pub fn read_zoom_state() -> HostZoomState {
    ZOOM_STATE.with(|state| state.borrow().clone())
}

pub fn set_target_zoom(target_zoom: f32) {
    let zoom = if target_zoom.is_finite() && target_zoom > 0.0 {
        target_zoom
    } else {
        1.0
    };
    ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.target_zoom = zoom;
        state.last_animation_timestamp_ms = 0.0;
    });
}

/// 缩放权威的唯一写入入口（ADR-0001）。
///
/// ZOOM_STATE.target_zoom 是缩放事实的家；`VIEWER_SESSION` 快照中的
/// current_zoom 只是它的派生投影（read 时填充），因此这里不需要、也
/// 不允许任何向 session 存储的镜像写。
pub fn set_target_zoom_authoritative(target_zoom: f32) {
    set_target_zoom(target_zoom);
}

pub fn mark_rendered_zoom(rendered_zoom: f32) {
    ZOOM_STATE.with(|state| {
        commit_rendered_zoom(&mut state.borrow_mut(), rendered_zoom);
    });
}

/// Cancel any active drawing delay timer (called on wheel event).
pub fn cancel_drawing_delay() {
    ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.drawing_delay.active = false;
    });
}

pub fn set_visual_layout(display_zoom: f32, content_left: f32, content_top: f32) {
    ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.visual_layout = Some(VisualLayoutState {
            display_zoom: if display_zoom.is_finite() && display_zoom > 0.0 {
                display_zoom
            } else {
                1.0
            },
            content_left: if content_left.is_finite() {
                content_left.max(0.0)
            } else {
                0.0
            },
            content_top: if content_top.is_finite() {
                content_top.max(0.0)
            } else {
                0.0
            },
        });
    });
}
