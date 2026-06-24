use wasm_bindgen::prelude::*;
use crate::editor::editor_types::*;
use crate::guard_state;
use super::{EditorSession, build_frame_request};

#[wasm_bindgen]
impl EditorSession {
    // ── P1: Real implementations ────────────────────────────────

    /// Insert text at the current caret position.
    #[wasm_bindgen(js_name = "insertText")]
    pub fn insert_text(&self, text: &str) -> JsValue {
        guard_state!(SessionState::EditingBlock, "insert_text");

        use crate::editor::command::EditorInputCommand;
        use crate::editor::host_snapshot::resolve_snapshot;
        use crate::editor::orchestrator::render_transaction::apply_input_tx;

        let frame_request = build_frame_request();
        let result = apply_input_tx(EditorInputCommand::InsertText(text), None, None, frame_request);
        let snapshot = resolve_snapshot(1.0);

        ok_response(
            ApplyCommandResult {
                changed: result.text_changed || result.scene_changed,
                caret_index: result.caret_index as u32,
                draft_text: snapshot.draft_text,
            },
            result.render_frame.is_some(),
        )
    }

    /// Delete text in the given direction ("forward" or "backward").
    #[wasm_bindgen(js_name = "deleteText")]
    pub fn delete_text(&self, direction: &str) -> JsValue {
        guard_state!(SessionState::EditingBlock, "delete_text");

        use crate::editor::command::EditorInputCommand;
        use crate::editor::host_snapshot::resolve_snapshot;
        use crate::editor::orchestrator::render_transaction::apply_input_tx;

        let command = match direction {
            "forward" => EditorInputCommand::DeleteForward,
            "backward" => EditorInputCommand::DeleteBackward,
            other => {
                return err_response(EditorError::Internal {
                    message: format!("unknown delete direction: {other}"),
                });
            }
        };

        let frame_request = build_frame_request();
        let result = apply_input_tx(command, None, None, frame_request);
        let snapshot = resolve_snapshot(1.0);

        ok_response(
            ApplyCommandResult {
                changed: result.text_changed || result.scene_changed,
                caret_index: result.caret_index as u32,
                draft_text: snapshot.draft_text,
            },
            result.render_frame.is_some(),
        )
    }

    // ── Stubs for future features ────────────────────────────────

    #[wasm_bindgen(js_name = "setCaret")]
    pub fn set_caret(&self, char_index: u32) -> JsValue {
        guard_state!(SessionState::EditingBlock, "setCaret");
        use crate::editor::session::set_caret;
        let changed = set_caret(char_index as usize);
        ok_empty(changed)
    }

    #[wasm_bindgen(js_name = "setSelection")]
    pub fn set_selection(&self, start: u32, end: u32) -> JsValue {
        guard_state!(SessionState::EditingBlock, "setSelection");
        use crate::editor::session::set_selection;
        let changed = set_selection(start as usize, end as usize);
        ok_empty(changed)
    }

    #[wasm_bindgen(js_name = "selectAll")]
    pub fn select_all(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "selectAll");
        use crate::editor::session::{active_editor_state, set_selection};
        let len = active_editor_state()
            .map(|state| state.text_char_count())
            .unwrap_or(0);
        let changed = set_selection(0, len);
        ok_empty(changed)
    }

    #[wasm_bindgen(js_name = "getSelection")]
    pub fn read_selection(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "getSelection");
        use crate::editor::session::active_editor_selection;
        match active_editor_selection() {
            Some((start, end, text)) => ok_response(
                TextSelection {
                    start: start as u32,
                    end: end as u32,
                    text,
                },
                false,
            ),
            None => ok_empty(false),
        }
    }

    #[wasm_bindgen(js_name = "cut")]
    pub fn cut(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "cut".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "copy")]
    pub fn copy(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "copy".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "paste")]
    pub fn paste(&self, _text: &str) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "paste".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "getTextContent")]
    pub fn read_text_content(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "getTextContent".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "getTextLines")]
    pub fn read_text_lines(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "getTextLines".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "getCharRects")]
    pub fn read_char_rects(&self, _start: u32, _end: u32) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "getCharRects".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "clientToPage")]
    pub fn client_to_page(
        &self,
        _client_x: f32,
        _client_y: f32,
        _reference_left: f32,
        _reference_top: f32,
        _reference_width: f32,
        _reference_height: f32,
        _page_width: f32,
        _page_height: f32,
    ) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "clientToPage".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "pageToClient")]
    pub fn page_to_client(
        &self,
        _page_x: f32,
        _page_y: f32,
        _reference_left: f32,
        _reference_top: f32,
        _reference_width: f32,
        _reference_height: f32,
        _page_width: f32,
        _page_height: f32,
    ) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "pageToClient".to_string(),
        })
    }
}
