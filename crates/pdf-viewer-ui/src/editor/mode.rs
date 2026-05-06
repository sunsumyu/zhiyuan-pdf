use crate::editor::engine_state::LiveEditorParagraphState;
use crate::editor::session::{
    active_edit_paragraph_id as host_active_edit_paragraph_id,
    active_editor_state as host_active_editor_state,
    active_editor_target as host_active_editor_target,
    close_active_editor as host_close_active_editor,
    is_text_edit_enabled as host_is_text_edit_enabled,
    reset_editor_mode as host_reset_editor_mode,
    set_active_edit_paragraph as host_set_active_edit_paragraph,
    set_text_edit_enabled as host_set_text_edit_enabled, ActiveEditorTarget,
};
pub fn get_active_edit_paragraph() -> Option<String> {
    host_active_edit_paragraph_id()
}

pub fn get_active_editor_target() -> Option<ActiveEditorTarget> {
    host_active_editor_target()
}

pub fn get_active_editor_state() -> Option<LiveEditorParagraphState> {
    host_active_editor_state()
}

pub fn is_text_edit_mode_enabled() -> bool {
    host_is_text_edit_enabled()
}

pub fn set_text_edit_mode_enabled(enabled: bool) {
    host_set_text_edit_enabled(enabled);
}

pub fn set_active_edit_paragraph(paragraph_id: Option<String>) {
    host_set_active_edit_paragraph(paragraph_id);
}

pub fn close_active_editor() -> bool {
    let changed = host_active_edit_paragraph_id().is_some();
    host_close_active_editor();
    changed
}

pub fn reset_editor_mode() {
    host_reset_editor_mode();
}
