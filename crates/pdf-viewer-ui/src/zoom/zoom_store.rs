use std::cell::RefCell;

// Re-export pure data structures from core.
pub use pdf_viewer_core::render::zoom_state::*;

thread_local! {
    pub static ZOOM_STATE: RefCell<HostZoomState> =
        RefCell::new(HostZoomState::default());
}

pub fn get_zoom_state() -> HostZoomState {
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
