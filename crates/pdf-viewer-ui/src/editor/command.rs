use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::document_edit_ops::{delete_backward, delete_forward, insert_text};
use crate::editor::engine_state::LiveEditorParagraphState;
use crate::editor::mode::read_state;
use crate::editor::navigation::execute_navigation;
use crate::editor::session::{
    caret_index, set_caret, sync_input,
    ActiveEditorInputSyncResult,
};

#[derive(Debug, Clone, Copy)]
pub enum EditorInputCommand<'a> {
    Navigation(&'a str),
    InsertText(&'a str),
    DeleteBackward,
    DeleteForward,
}

fn command_name(command: EditorInputCommand<'_>) -> String {
    match command {
        EditorInputCommand::Navigation(key) => format!("navigation:{key}"),
        EditorInputCommand::InsertText(text) => format!("insert:{}", text),
        EditorInputCommand::DeleteBackward => "backspace".to_string(),
        EditorInputCommand::DeleteForward => "delete".to_string(),
    }
}

fn effective_editor_state(
    host_text: Option<String>,
    host_caret_index: Option<usize>,
) -> Option<LiveEditorParagraphState> {
    let state = read_state()?;
    let before_caret = state.caret_index;
    let before_text = state.current_text().to_string();
    // The host textarea is an IME/event adapter, not the source of truth. Keeping
    // command state inside Rust prevents DOM text/caret snapshots from collapsing
    // PDF geometry gaps or overwriting the active editor session.
    dbg_event(
        "command.effective-state",
        "resolved",
        vec![
            dbg_field("paragraphId", state.paragraph_id()),
            dbg_field("storedText", before_text),
            dbg_field("storedCaretIndex", before_caret),
            dbg_field(
                "hostTextIgnored",
                host_text.unwrap_or_else(|| "none".to_string()),
            ),
            dbg_field(
                "hostCaretIndex",
                host_caret_index
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
            dbg_field("effectiveText", state.current_text().to_string()),
            dbg_field("effectiveCaretIndex", state.normalized_caret_index()),
        ],
    );
    Some(state)
}

pub fn apply_editor_input_command(command: EditorInputCommand<'_>) -> ActiveEditorInputSyncResult {
    apply_host_input(command, None, None)
}

pub fn apply_host_input(
    command: EditorInputCommand<'_>,
    host_text: Option<String>,
    host_caret_index: Option<usize>,
) -> ActiveEditorInputSyncResult {
    dbg_event(
        "command.apply",
        "input",
        vec![
            dbg_field("command", command_name(command)),
            dbg_field(
                "hostText",
                host_text.clone().unwrap_or_else(|| "none".to_string()),
            ),
            dbg_field(
                "hostCaretIndex",
                host_caret_index
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
        ],
    );
    match command {
        EditorInputCommand::Navigation(key) => {
            if let Some(state) = effective_editor_state(host_text.clone(), host_caret_index) {
                let _ = sync_input(
                    state.current_text().to_string(),
                    state.normalized_caret_index(),
                );
            }
            let next_caret = execute_navigation(key);
            ActiveEditorInputSyncResult {
                caret_changed: next_caret.is_some(),
                caret_index: next_caret.unwrap_or_else(caret_index),
                ..Default::default()
            }
        }
        EditorInputCommand::InsertText(inserted_text) => {
            let Some(active_state) = effective_editor_state(host_text, host_caret_index) else {
                return ActiveEditorInputSyncResult::default();
            };
            let mutation = insert_text(&active_state, inserted_text);
            sync_input(mutation.text, mutation.caret_index)
        }
        EditorInputCommand::DeleteBackward => {
            let Some(active_state) = effective_editor_state(host_text, host_caret_index) else {
                return ActiveEditorInputSyncResult::default();
            };
            let before_caret = active_state.normalized_caret_index();
            let before_text = active_state.current_text().to_string();
            let before_len = before_text.chars().count();
            let mutation = delete_backward(&active_state);
            let removed_char: String = before_text
                .chars()
                .nth(before_caret.saturating_sub(1))
                .map(|c| c.to_string())
                .unwrap_or_default();
            crate::chain_trace!("cmd.backspace",
                "beforeCaret" => before_caret,
                "beforeLen" => before_len,
                "removedChar" => removed_char,
                "afterCaret" => mutation.caret_index,
                "afterLen" => mutation.text.chars().count()
            );
            sync_input(mutation.text, mutation.caret_index)
        }
        EditorInputCommand::DeleteForward => {
            let Some(active_state) = effective_editor_state(host_text, host_caret_index) else {
                return ActiveEditorInputSyncResult::default();
            };
            let current_caret = active_state.normalized_caret_index();
            let mutation = delete_forward(&active_state);
            if mutation.caret_index == active_state.normalized_caret_index()
                && mutation.text == active_state.current_text()
            {
                let _ = set_caret(current_caret);
                return ActiveEditorInputSyncResult {
                    caret_index: mutation.caret_index,
                    ..Default::default()
                };
            }
            sync_input(mutation.text, mutation.caret_index)
        }
    }
}
