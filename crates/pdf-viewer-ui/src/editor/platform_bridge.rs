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

use crate::app_context;

pub fn read_state() -> EditorHostRuntimeState {
    app_context::with_editor_host(Clone::clone)
}

pub fn reset_state() {
    app_context::with_editor_host_mut(|state| {
        *state = EditorHostRuntimeState::default();
    });
}

pub fn set_display_zoom(display_zoom: f32) {
    let display_zoom = if display_zoom.is_finite() && display_zoom > 0.0 {
        display_zoom
    } else {
        1.0
    };
    app_context::with_editor_host_mut(|state| {
        state.last_display_zoom = display_zoom;
    });
}

pub fn begin_commit() -> bool {
    app_context::with_editor_host_mut(|state| {
        if state.committing {
            false
        } else {
            state.committing = true;
            true
        }
    })
}

pub fn finish_commit() {
    app_context::with_editor_host_mut(|state| {
        state.committing = false;
    });
}
