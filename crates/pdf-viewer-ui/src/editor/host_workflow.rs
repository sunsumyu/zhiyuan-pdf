use wasm_bindgen::prelude::JsValue;

use crate::editor::activation::{
    move_caret_to_client_point as activate_caret_from_client_point,
    save_editor_session as save_editor_session_activation, MoveCaretToClientPointRequest,
    SaveEditorSessionResult,
};

pub use crate::editor::activation::activate_editor_from_client_point as open_editor_at_client_point;

pub fn move_caret_to_client_point(request: MoveCaretToClientPointRequest) -> Option<usize> {
    activate_caret_from_client_point(request)
}

pub async fn save_editor_session(path: String, page_index: u16) -> SaveEditorSessionResult {
    save_editor_session_activation(path, page_index).await
}

pub fn read_paragraph_shell_bbox(paragraph_id: &str) -> Option<JsValue> {
    let bbox = crate::editor::runtime::find_paragraph_shell_bbox(paragraph_id)?;
    serde_wasm_bindgen::to_value(&bbox).ok()
}
