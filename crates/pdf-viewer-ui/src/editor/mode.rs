use crate::editor::engine_state::LiveEditorParagraphState;
use crate::editor::session::{
    active_editor_state, active_editor_target, close_active_editor as session_close_active_editor,
    is_edit_enabled as session_is_edit_enabled, paragraph_id,
    reset_editor_mode as session_reset_editor_mode, set_edit_enabled as session_set_edit_enabled,
    set_paragraph as session_set_paragraph, ActiveEditorTarget,
};
pub fn read_paragraph() -> Option<String> {
    paragraph_id()
}

pub fn read_target() -> Option<ActiveEditorTarget> {
    active_editor_target()
}

pub fn read_state() -> Option<LiveEditorParagraphState> {
    active_editor_state()
}

pub fn is_edit_enabled() -> bool {
    session_is_edit_enabled()
}

pub fn set_edit_enabled(enabled: bool) {
    session_set_edit_enabled(enabled);
}

pub fn set_paragraph(paragraph_id: Option<String>) {
    session_set_paragraph(paragraph_id);
}

pub fn close_active_editor() -> bool {
    let changed = paragraph_id().is_some();
    session_close_active_editor();
    changed
}

pub fn reset_editor_mode() {
    session_reset_editor_mode();
}
