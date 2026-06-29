use crate::app_context;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::engine_state::LiveEditorParagraphState;
use crate::editor::session::history::LocalEditHistory;
use crate::ui_state_store::{
    current_paragraph_patch, current_patch_revision, patch_text as current_paragraph_patch_text,
};
use crate::viewer::viewer_store::current_document_revision;
use pdf_viewer_core::text::style_mapper::StyleMapper;
use serde::{Deserialize, Serialize};

// ActiveEditorTarget 数据结构已迁至 pdf_viewer_core::edit::active_target。
pub use pdf_viewer_core::edit::active_target::ActiveEditorTarget;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorModeState {
    pub text_edit_enabled: bool,
    pub active_paragraph_id: Option<String>,
    pub live_state: Option<LiveEditorParagraphState>,
    #[serde(skip)]
    pub history: LocalEditHistory,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEditorInputSyncResult {
    pub text_changed: bool,
    pub caret_changed: bool,
    pub scene_changed: bool,
    pub caret_index: usize,
    #[serde(default)]
    pub request_visibility_render: bool,
}

impl Default for EditorModeState {
    fn default() -> Self {
        Self {
            text_edit_enabled: false,
            active_paragraph_id: None,
            live_state: None,
            history: LocalEditHistory::new(),
        }
    }
}

pub fn with_editor_mode<R>(f: impl FnOnce(&EditorModeState) -> R) -> R {
    app_context::with_editor_mode(f)
}

pub fn with_editor_mode_mut<R>(f: impl FnOnce(&mut EditorModeState) -> R) -> R {
    app_context::with_editor_mode_mut(f)
}

pub fn reset_editor_mode() {
    with_editor_mode_mut(|mode| {
        *mode = EditorModeState::default();
    });
    crate::editor::editor_store::reset_session();
}

pub fn is_edit_enabled() -> bool {
    with_editor_mode(|mode| mode.text_edit_enabled)
}

pub fn set_edit_enabled(enabled: bool) {
    with_editor_mode_mut(|mode| {
        mode.text_edit_enabled = enabled;
        if !enabled {
            // 不变量保护：到这一步 live_state 必须已经被上层 commit 过了
            // （见 host_mode.rs 的 set_edit_mode / toggle_edit_mode）。
            // 若仍有 live_state，说明有路径绕过了 commit，记录 warning 便于定位。
            if mode.live_state.is_some() {
                crate::chain_trace!(
                    "session.warn",
                    "msg" => "set_text_edit_enabled(false) with live_state present — edit will be lost",
                );
            }
            mode.active_paragraph_id = None;
            mode.live_state = None;
            mode.history.clear();
        }
    });
}

pub fn paragraph_id() -> Option<String> {
    with_editor_mode(|mode| mode.active_paragraph_id.clone())
}

pub fn set_paragraph(paragraph_id: Option<String>) {
    with_editor_mode_mut(|mode| {
        mode.active_paragraph_id = paragraph_id;
        if mode.active_paragraph_id.is_none() {
            mode.live_state = None;
        }
    });
}

pub fn active_editor_state() -> Option<LiveEditorParagraphState> {
    with_editor_mode(|mode| mode.live_state.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_disabled() {
        reset_editor_mode();
        assert!(!is_edit_enabled());

        set_edit_enabled(true);
        assert!(is_edit_enabled());
    }
}

pub fn active_editor_target() -> Option<ActiveEditorTarget> {
    active_editor_state().map(|state| state.target)
}

pub fn open_paragraph_editor(paragraph_id: String, target: ActiveEditorTarget) -> bool {
    with_editor_mode_mut(|mode| {
        if !mode.text_edit_enabled {
            dbg_event(
                "session.open",
                "rejected-text-edit-disabled",
                vec![dbg_field("paragraphId", &paragraph_id)],
            );
            return false;
        }
        let initial_caret = target.initial_body_caret_index();
        let source_text = target.source_body_text().to_string();
        let maybe_text = current_paragraph_patch_text(&paragraph_id);
        let current_patch = current_paragraph_patch(&paragraph_id);
        let mut live_state = LiveEditorParagraphState::new(target);
        if let Some(text) = maybe_text.as_ref() {
            let _ = live_state.set_draft_text(text.clone());
        }
        if let Some(patch) = current_patch.as_ref() {
            if let Some(new_runs) = patch.new_runs.as_ref() {
                live_state.style_mapper = StyleMapper {
                    spans: new_runs
                        .iter()
                        .map(|run| pdf_viewer_core::text::style_mapper::StyleSpan {
                            text: run.text.clone(),
                            style: run.style.clone(),
                            is_decorative: false,
                        })
                        .collect(),
                };
                live_state.sync_target_control_style();
            }
            if let Some(align) = patch.align {
                let _ = live_state.set_alignment(align);
            }
            if let Some(new_marker_text) = patch.new_marker_text.as_deref() {
                live_state.restore_list_kind(new_marker_text);
            }
        }
        live_state.mark_session_clean();
        mode.history.clear();
        mode.active_paragraph_id = Some(paragraph_id);
        mode.live_state = Some(live_state);

        dbg_event(
            "session.open",
            "applied",
            vec![
                dbg_field(
                    "paragraphId",
                    mode.active_paragraph_id.as_deref().unwrap_or_default(),
                ),
                dbg_field("initialCaretIndex", initial_caret),
                dbg_field("sourceText", source_text),
                dbg_field("currentText", maybe_text.clone().unwrap_or_default()),
                dbg_field(
                    "liveColor",
                    mode.live_state
                        .as_ref()
                        .map(|state| state.target.color.as_str())
                        .unwrap_or_default(),
                ),
                dbg_field(
                    "liveTextDecoration",
                    mode.live_state
                        .as_ref()
                        .map(|state| state.target.text_decoration.as_str())
                        .unwrap_or_default(),
                ),
                dbg_field(
                    "liveUnderline",
                    mode.live_state
                        .as_ref()
                        .map(|state| state.is_underline_active())
                        .unwrap_or(false),
                ),
            ],
        );
        true
    })
}

pub fn close_active_editor() {
    with_editor_mode_mut(|mode| {
        mode.active_paragraph_id = None;
        mode.live_state = None;
        mode.history.clear();
    });
    crate::editor::editor_store::reset_session();
}

pub fn draft_text() -> Option<String> {
    active_editor_state().map(|state| state.current_text().to_string())
}

pub fn has_changes() -> bool {
    with_editor_mode(|mode| {
        mode.live_state
            .as_ref()
            .map(|state| state.has_session_changes())
            .unwrap_or(false)
    })
}

pub fn caret_index() -> usize {
    active_editor_state()
        .map(|state| state.caret_index)
        .unwrap_or(0)
}

pub fn set_caret(caret_index: usize) -> bool {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            dbg_event(
                "session.caret",
                "set-missed-no-live-state",
                vec![dbg_field("requestedCaretIndex", caret_index)],
            );
            return false;
        };
        let before = live_state.caret_index;
        let changed = live_state.set_caret_index(caret_index);
        dbg_event(
            "session.caret",
            "set",
            vec![
                dbg_field("paragraphId", live_state.paragraph_id()),
                dbg_field("requestedCaretIndex", caret_index),
                dbg_field("beforeCaretIndex", before),
                dbg_field("afterCaretIndex", live_state.caret_index),
                dbg_field("textCharCount", live_state.text_char_count()),
                dbg_field("changed", changed),
            ],
        );
        changed
    })
}

pub fn set_selection(start: usize, end: usize) -> bool {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return false;
        };
        live_state.set_selection_range(start, end)
    })
}

pub fn clear_selection() -> bool {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return false;
        };
        live_state.clear_selection()
    })
}

pub fn active_editor_selection() -> Option<(usize, usize, String)> {
    with_editor_mode(|mode| {
        let live_state = mode.live_state.as_ref()?;
        let range = live_state.selection_range()?;
        let text = live_state.selection_text().unwrap_or_default();
        Some((range.0, range.1, text))
    })
}

pub fn sync_input(new_text: String, caret_index: usize) -> ActiveEditorInputSyncResult {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            dbg_event(
                "session.sync",
                "missed-no-live-state",
                vec![
                    dbg_field("requestedText", &new_text),
                    dbg_field("requestedCaretIndex", caret_index),
                ],
            );
            return ActiveEditorInputSyncResult::default();
        };

        let before_text = live_state.current_text().to_string();
        let before_caret = live_state.caret_index;
        if new_text != before_text {
            mode.history.push_snapshot(live_state);
        }
        let text_changed = live_state.set_draft_text(new_text.clone());

        let normalized_caret = caret_index.min(live_state.text_char_count());
        let caret_changed = live_state.set_caret_index(normalized_caret);

        dbg_event(
            "session.sync",
            "applied",
            vec![
                dbg_field("paragraphId", live_state.paragraph_id()),
                dbg_field("beforeText", before_text),
                dbg_field("beforeCaretIndex", before_caret),
                dbg_field("requestedText", &new_text),
                dbg_field("requestedCaretIndex", caret_index),
                dbg_field("normalizedCaretIndex", normalized_caret),
                dbg_field("afterText", live_state.current_text()),
                dbg_field("afterCaretIndex", live_state.caret_index),
                dbg_field("textChanged", text_changed),
                dbg_field("caretChanged", caret_changed),
            ],
        );

        ActiveEditorInputSyncResult {
            text_changed,
            caret_changed,
            scene_changed: text_changed,
            caret_index: live_state.normalized_caret_index(),
            request_visibility_render: text_changed,
        }
    })
}

pub fn undo_active_editor() -> Option<ActiveEditorInputSyncResult> {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return None;
        };
        let prev = mode.history.undo(live_state)?;
        *live_state = prev;

        Some(ActiveEditorInputSyncResult {
            text_changed: true,
            caret_changed: true,
            scene_changed: true,
            caret_index: live_state.normalized_caret_index(),
            request_visibility_render: true,
        })
    })
}

pub fn redo_active_editor() -> Option<ActiveEditorInputSyncResult> {
    with_editor_mode_mut(|mode| {
        let Some(live_state) = mode.live_state.as_mut() else {
            return None;
        };
        let next = mode.history.redo(live_state)?;
        *live_state = next;

        Some(ActiveEditorInputSyncResult {
            text_changed: true,
            caret_changed: true,
            scene_changed: true,
            caret_index: live_state.normalized_caret_index(),
            request_visibility_render: true,
        })
    })
}

pub fn can_undo() -> bool {
    with_editor_mode(|mode| mode.history.can_undo())
}

pub fn can_redo() -> bool {
    with_editor_mode(|mode| mode.history.can_redo())
}

pub fn render_scene_key() -> String {
    let document_revision = current_document_revision();
    let patch_revision = current_patch_revision();
    with_editor_mode(|mode| match mode.live_state.as_ref() {
        Some(live_state) if !live_state.paragraph_id().is_empty() => {
            format!(
                "doc:rev{}|patch:rev{}|edit:{}:rev{}",
                document_revision,
                patch_revision,
                live_state.paragraph_id(),
                live_state.scene_revision
            )
        }
        _ => format!(
            "doc:rev{}|patch:rev{}|edit:none:rev0",
            document_revision, patch_revision
        ),
    })
}
