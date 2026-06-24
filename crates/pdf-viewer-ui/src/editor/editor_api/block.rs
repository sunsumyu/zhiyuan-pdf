use wasm_bindgen::prelude::*;
use crate::editor::editor_types::*;
use super::EditorSession;

#[wasm_bindgen]
impl EditorSession {
    #[wasm_bindgen(js_name = "addTextBlock")]
    pub fn add_text_block(&self, _x: f32, _y: f32, _max_width: f32, _text: &str) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "addTextBlock".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "deleteTextBlock")]
    pub fn delete_text_block(&self, _block_id: &str) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "deleteTextBlock".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "resizeTextBlock")]
    pub fn resize_text_block(&self, _block_id: &str, _max_width: f32) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "resizeTextBlock".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "moveTextBlock")]
    pub fn move_text_block(&self, _block_id: &str, _x: f32, _y: f32) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "moveTextBlock".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "exportPatch")]
    pub fn export_patch(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "exportPatch".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "importPatch")]
    pub fn import_patch(&self, _patch_js: JsValue) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "importPatch".to_string(),
        })
    }
}
