use super::{build_frame_request, EditorSession};
use crate::editor::editor_types::*;
use crate::guard_state;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl EditorSession {
    /// Apply a format action to the active block.
    #[wasm_bindgen(js_name = "applyFormat")]
    pub fn apply_format(&self, action_js: JsValue) -> JsValue {
        guard_state!(SessionState::EditingBlock, "apply_format");

        use crate::editor::editor_controller::EditorFormatAction;
        use crate::editor::orchestrator::render_transaction::apply_format_tx;

        let action: EditorFormatAction = match serde_wasm_bindgen::from_value(action_js) {
            Ok(a) => a,
            Err(e) => {
                return err_response(EditorError::Internal {
                    message: format!("failed to parse format action: {e}"),
                });
            }
        };

        let frame_request = build_frame_request();
        let result = apply_format_tx(action, frame_request);

        ok_response(
            CommitResult {
                changed: result.changed,
            },
            result.render_frame.is_some(),
        )
    }

    /// Read the format state of the active editor.
    #[wasm_bindgen(js_name = "readFormatState")]
    pub fn read_format_state(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "read_format_state");

        use crate::editor::editor_controller::format_state;
        let state = format_state();
        serde_wasm_bindgen::to_value(&state).unwrap_or(JsValue::NULL)
    }
}
