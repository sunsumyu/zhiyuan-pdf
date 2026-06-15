use serde::Serialize;
use wasm_bindgen::prelude::*;

// Re-export pure data structures from core.
pub use pdf_viewer_core::edit::editor_types::*;

fn response_to_js<T: Serialize>(resp: &EditorResponse<T>) -> JsValue {
    serde_wasm_bindgen::to_value(resp).unwrap_or(JsValue::NULL)
}

/// Construct a success response.
pub fn ok_response<T: Serialize>(data: T, render: bool) -> JsValue {
    let resp = EditorResponse {
        ok: true,
        data: Some(data),
        error: None,
        render,
    };
    response_to_js(&resp)
}

/// Construct a success response with no payload.
pub fn ok_empty(render: bool) -> JsValue {
    let resp: EditorResponse<()> = EditorResponse {
        ok: true,
        data: None,
        error: None,
        render,
    };
    response_to_js(&resp)
}

/// Construct an error response.
pub fn err_response(error: EditorError) -> JsValue {
    let resp: EditorResponse<()> = EditorResponse {
        ok: false,
        data: None,
        error: Some(error),
        render: false,
    };
    response_to_js(&resp)
}

/// Deserialize a `JsValue` into `T`, or return an `err_response` JsValue for the caller to return.
/// Eliminates the repeated `serde_wasm_bindgen::from_value` + `err_response` boilerplate across API methods.
pub fn parse_request<T: serde::de::DeserializeOwned>(js: JsValue, method: &str) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(js).map_err(|e| {
        err_response(EditorError::Internal {
            message: format!("failed to parse {method} request: {e}"),
        })
    })
}
