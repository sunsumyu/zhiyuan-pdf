use std::cell::RefCell;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorHostRuntimeState {
    pub committing: bool,
    pub last_display_zoom: f32,
}

impl Default for EditorHostRuntimeState {
    fn default() -> Self {
        Self {
            committing: false,
            last_display_zoom: 1.0,
        }
    }
}

thread_local! {
    pub static HOST_EDITOR_HOST_RUNTIME_STATE: RefCell<EditorHostRuntimeState> =
        RefCell::new(EditorHostRuntimeState::default());
}

pub fn get_state() -> EditorHostRuntimeState {
    HOST_EDITOR_HOST_RUNTIME_STATE.with(|state| state.borrow().clone())
}

pub fn reset_state() {
    HOST_EDITOR_HOST_RUNTIME_STATE.with(|state| {
        *state.borrow_mut() = EditorHostRuntimeState::default();
    });
}

pub fn set_display_zoom(display_zoom: f32) {
    let display_zoom = if display_zoom.is_finite() && display_zoom > 0.0 {
        display_zoom
    } else {
        1.0
    };
    HOST_EDITOR_HOST_RUNTIME_STATE.with(|state| {
        state.borrow_mut().last_display_zoom = display_zoom;
    });
}

pub fn begin_commit() -> bool {
    HOST_EDITOR_HOST_RUNTIME_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.committing {
            false
        } else {
            state.committing = true;
            true
        }
    })
}

pub fn finish_commit() {
    HOST_EDITOR_HOST_RUNTIME_STATE.with(|state| {
        state.borrow_mut().committing = false;
    });
}
