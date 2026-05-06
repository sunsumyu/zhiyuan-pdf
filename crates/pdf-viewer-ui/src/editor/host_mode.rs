use serde::{Deserialize, Serialize};

use crate::editor::mode::{is_text_edit_mode_enabled, set_text_edit_mode_enabled};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToggleEditorModeResult {
    pub enabled: bool,
    pub changed: bool,
}

pub fn toggle_text_edit_mode() -> ToggleEditorModeResult {
    let next_enabled = !is_text_edit_mode_enabled();
    set_text_edit_mode_enabled(next_enabled);
    ToggleEditorModeResult {
        enabled: next_enabled,
        changed: true,
    }
}

pub fn set_text_edit_mode(enabled: bool) -> ToggleEditorModeResult {
    let previous = is_text_edit_mode_enabled();
    set_text_edit_mode_enabled(enabled);
    ToggleEditorModeResult {
        enabled,
        changed: previous != enabled,
    }
}
