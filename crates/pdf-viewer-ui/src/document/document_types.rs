//! UI-side helpers for `DocumentResponse<T>` JsValue serialization.
//!
//! Mirrors `crate::editor::editor_types` helpers for the document domain.

use serde::Serialize;
use wasm_bindgen::prelude::*;

pub use pdf_viewer_core::document::document_types::{DocumentError, DocumentResponse};

fn response_to_js<T: Serialize>(resp: &DocumentResponse<T>) -> JsValue {
    serde_wasm_bindgen::to_value(resp).unwrap_or(JsValue::NULL)
}

/// Construct a success response with payload.
pub fn ok_response<T: Serialize>(data: T) -> JsValue {
    let resp = DocumentResponse {
        ok: true,
        data: Some(data),
        error: None,
    };
    response_to_js(&resp)
}

/// Construct a success response with no payload.
#[allow(dead_code)]
pub fn ok_empty() -> JsValue {
    let resp: DocumentResponse<()> = DocumentResponse {
        ok: true,
        data: None,
        error: None,
    };
    response_to_js(&resp)
}

/// Construct an error response.
pub fn err_response(error: DocumentError) -> JsValue {
    let resp: DocumentResponse<()> = DocumentResponse {
        ok: false,
        data: None,
        error: Some(error),
    };
    response_to_js(&resp)
}

/// Deserialize a `JsValue` into `T`, or return a structured document error response.
pub fn parse_request<T: serde::de::DeserializeOwned>(
    js: JsValue,
    method: &str,
) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(js).map_err(|e| {
        err_response(DocumentError::InvalidInput {
            field: method.to_string(),
            reason: format!("failed to parse request: {e}"),
        })
    })
}
