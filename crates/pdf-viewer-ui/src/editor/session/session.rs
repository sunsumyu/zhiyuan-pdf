use std::cell::RefCell;

use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::engine_state::LiveEditorParagraphState;
use crate::editor::paragraph_scene::ParagraphEditorScene;
use crate::state_manager::{
    current_paragraph_patch_text, current_paragraph_patch, current_patch_revision,
};
use crate::style_mapper::StyleMapper;
use crate::viewer::session::current_document_revision;
use pdf_viewer_core::models::EditorSession;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEditorTarget {
    pub paragraph_id: String,
    pub region_id: String,
    pub page_index: u16,
    pub text: String,
    pub bbox_left: f32,
    pub bbox_top: f32,
    pub bbox_right: f32,
    pub bbox_bottom: f32,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: String,
    pub font_style: String,
    pub color: String,
    #[serde(default)]
    pub text_decoration: String,
    #[serde(default)]
    pub initial_caret_index: usize,
    pub editor_session: EditorSession,
    #[serde(default)]
    pub scene: ParagraphEditorScene,
}

impl Default for ActiveEditorTarget {
    fn default() -> Self {
        Self {
            paragraph_id: String::new(),
            region_id: String::new(),
            page_index: 0,
            text: String::new(),
            bbox_left: 0.0,
            bbox_top: 0.0,
            bbox_right: 0.0,
            bbox_bottom: 0.0,
            font_family: String::new(),
            font_size: 0.0,
            font_weight: String::new(),
            font_style: String::new(),
            color: String::new(),
            text_decoration: String::new(),
            initial_caret_index: 0,
            editor_session: EditorSession {
                anchor_bbox: pdf_viewer_core::models::BoundingBox::default(),
                paragraph: pdf_viewer_core::models::LayoutParagraph::default(),
            },
            scene: ParagraphEditorScene::default(),
        }
    }
}

impl ActiveEditorTarget {
    pub fn source_body_text(&self) -> &str {
        self.scene.document_plan.source_body_text()
    }

    pub fn initial_body_caret_index(&self) -> usize {
        self.scene.document_plan.body_initial_caret
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorModeState {
    pub text_edit_enabled: bool,
    pub active_paragraph_id: Option<String>,
    pub live_state: Option<LiveEditorParagraphState>,
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
        }
    }
}

thread_local! {
    pub static HOST_EDITOR_MODE: RefCell<EditorModeState> =
        RefCell::new(EditorModeState::default());
}

pub fn reset_editor_mode() {
    HOST_EDITOR_MODE.with(|mode| {
        *mode.borrow_mut() = EditorModeState::default();
    });
}

pub fn is_text_edit_enabled() -> bool {
    HOST_EDITOR_MODE.with(|mode| mode.borrow().text_edit_enabled)
}

pub fn set_text_edit_enabled(enabled: bool) {
    HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        mode.text_edit_enabled = enabled;
        if !enabled {
            // 不变量保护：到这一步 live_state 必须已经被上层 commit 过了
            // （见 host_mode.rs 的 set_text_edit_mode / toggle_text_edit_mode）。
            // 若仍有 live_state，说明有路径绕过了 commit，记录 warning 便于定位。
            if mode.live_state.is_some() {
                web_sys::console::log_1(
                    &"[SESSION-WARN] set_text_edit_enabled(false) called with live_state still present \
                       — this is a bug, edit will be lost. See docs/edit-save-architecture.md §4.1"
                        .into(),
                );
            }
            mode.active_paragraph_id = None;
            mode.live_state = None;
        }
    });
}

pub fn active_edit_paragraph_id() -> Option<String> {
    HOST_EDITOR_MODE.with(|mode| mode.borrow().active_paragraph_id.clone())
}

pub fn set_active_edit_paragraph(paragraph_id: Option<String>) {
    HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        mode.active_paragraph_id = paragraph_id;
        if mode.active_paragraph_id.is_none() {
            mode.live_state = None;
        }
    });
}

pub fn active_editor_state() -> Option<LiveEditorParagraphState> {
    HOST_EDITOR_MODE.with(|mode| mode.borrow().live_state.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_edit_mode_starts_disabled_until_toolbar_enables_it() {
        reset_editor_mode();
        assert!(!is_text_edit_enabled());

        set_text_edit_enabled(true);
        assert!(is_text_edit_enabled());
    }
}

pub fn active_editor_target() -> Option<ActiveEditorTarget> {
    active_editor_state().map(|state| state.target)
}

pub fn open_paragraph_editor(paragraph_id: String, target: ActiveEditorTarget) -> bool {
    HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
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
        let current_text = current_paragraph_patch_text(&paragraph_id);
        let current_patch = current_paragraph_patch(&paragraph_id);
        let mut live_state = LiveEditorParagraphState::new(target);
        if let Some(current_text) = current_text.as_ref() {
            let _ = live_state.set_draft_text(current_text.clone());
        }
        if let Some(patch) = current_patch.as_ref() {
            if let Some(new_runs) = patch.new_runs.as_ref() {
                live_state.style_mapper = StyleMapper {
                    spans: new_runs
                        .iter()
                        .map(|run| crate::style_mapper::StyleSpan {
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
                live_state.restore_list_kind_from_marker_text(new_marker_text);
            }
        }
        live_state.mark_session_clean();
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
                dbg_field(
                    "currentText",
                    current_text.unwrap_or_else(|| "source".to_string()),
                ),
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
    HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        mode.active_paragraph_id = None;
        mode.live_state = None;
    });
}

pub fn active_editor_draft_text() -> Option<String> {
    active_editor_state().map(|state| state.current_text().to_string())
}

pub fn active_editor_has_session_changes() -> bool {
    HOST_EDITOR_MODE.with(|mode| {
        mode.borrow()
            .live_state
            .as_ref()
            .map(|state| state.has_session_changes())
            .unwrap_or(false)
    })
}

pub fn active_editor_caret_index() -> usize {
    active_editor_state()
        .map(|state| state.caret_index)
        .unwrap_or(0)
}

pub fn set_active_editor_caret_index(caret_index: usize) -> bool {
    HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
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

pub fn sync_active_editor_input(
    new_text: String,
    caret_index: usize,
) -> ActiveEditorInputSyncResult {
    HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
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

pub fn render_scene_key() -> String {
    let document_revision = current_document_revision();
    let patch_revision = current_patch_revision();
    HOST_EDITOR_MODE.with(|mode| {
        let mode = mode.borrow();
        match mode.live_state.as_ref() {
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
        }
    })
}
