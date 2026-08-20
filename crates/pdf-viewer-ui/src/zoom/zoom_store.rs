use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// Re-export pure data structures from core.
pub use pdf_viewer_core::render::zoom_state::*;

// ─── ZoomSessionState (Batch 2 sec 4) ───────────────────────────
//
// Explicit enum for the Zoom domain state machine. Since ZoomController
// (the WASM session handle) was deleted as TS-dead, this enum is
// exposed only to Rust callers. A future re-created ZoomSession could
// surface it via getState().
//
// Derived from HostZoomState fields on demand:
//
//   Idle        current_zoom == target_zoom, no preview
//   Animating   current_zoom != target_zoom (zoom-to-fit / pinch release)
//   Previewing  preview_transform is Some (live pinch/scroll gesture)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ZoomSessionState {
    Idle,
    Animating,
    Previewing,
}

impl ZoomSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZoomSessionState::Idle => "Idle",
            ZoomSessionState::Animating => "Animating",
            ZoomSessionState::Previewing => "Previewing",
        }
    }
}

/// Snapshot of the current zoom state.
pub fn read_zoom_session_state() -> ZoomSessionState {
    ZOOM_STATE.with(|state| {
        let s = state.borrow();
        if s.preview_transform.is_some() {
            ZoomSessionState::Previewing
        } else if (s.current_zoom - s.target_zoom).abs() > f32::EPSILON {
            ZoomSessionState::Animating
        } else {
            ZoomSessionState::Idle
        }
    })
}

thread_local! {
    pub static ZOOM_STATE: RefCell<HostZoomState> =
        RefCell::new(HostZoomState::default());
}

pub fn read_zoom_state() -> HostZoomState {
    ZOOM_STATE.with(|state| state.borrow().clone())
}

pub fn with_zoom_state<R>(f: impl FnOnce(&HostZoomState) -> R) -> R {
    ZOOM_STATE.with(|state| f(&state.borrow()))
}

pub fn with_zoom_state_mut<R>(f: impl FnOnce(&mut HostZoomState) -> R) -> R {
    ZOOM_STATE.with(|state| f(&mut state.borrow_mut()))
}

pub fn reset_zoom_state(initial_zoom: f32) {
    let zoom = sanitize_zoom(initial_zoom);
    ZOOM_STATE.with(|state| {
        *state.borrow_mut() = HostZoomState {
            current_zoom: zoom,
            target_zoom: zoom,
            visual_zoom: zoom,
            last_rendered_zoom: zoom,
            css_scale: 1.0,
            last_animation_timestamp_ms: 0.0,
            pending_anchor: None,
            visual_layout: None,
            preview_transform: None,
            preview_host: PreviewHostState::default(),
        };
    });
}

fn sanitize_zoom(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}
