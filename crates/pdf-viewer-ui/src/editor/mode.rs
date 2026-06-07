use crate::editor::engine_state::LiveEditorParagraphState;
use crate::editor::session::{
    active_edit_paragraph_id, active_editor_state, active_editor_target,
    close_active_editor as session_close_active_editor, is_text_edit_enabled,
    reset_editor_mode as session_reset_editor_mode,
    set_active_edit_paragraph as session_set_active_edit_paragraph, set_text_edit_enabled,
    ActiveEditorTarget,
};
pub fn read_active_edit_paragraph() -> Option<String> {
    active_edit_paragraph_id()
}

pub fn read_active_editor_target() -> Option<ActiveEditorTarget> {
    active_editor_target()
}

pub fn read_active_editor_state() -> Option<LiveEditorParagraphState> {
    active_editor_state()
}

pub fn is_text_edit_mode_enabled() -> bool {
    is_text_edit_enabled()
}

pub fn set_text_edit_mode_enabled(enabled: bool) {
    set_text_edit_enabled(enabled);
}

pub fn set_active_edit_paragraph(paragraph_id: Option<String>) {
    session_set_active_edit_paragraph(paragraph_id);
}

pub fn close_active_editor() -> bool {
    let changed = active_edit_paragraph_id().is_some();
    session_close_active_editor();
    changed
}

pub fn reset_editor_mode() {
    session_reset_editor_mode();
}
