//! UI-side helpers for `HistoryResponse<T>` JsValue serialization.
//!
//! Mirrors `crate::editor::editor_types` / `crate::document::document_types`
//! / `crate::annotation::annotation_types` helpers for the history domain.

use serde::Serialize;
use wasm_bindgen::prelude::*;

pub use pdf_viewer_core::history::history_types::{
    HistoryError, HistoryResponse, HistoryState, HistoryStepResult,
};

fn response_to_js<T: Serialize>(resp: &HistoryResponse<T>) -> JsValue {
    serde_wasm_bindgen::to_value(resp).unwrap_or(JsValue::NULL)
}

/// Construct a success response with payload.
pub fn ok_response<T: Serialize>(data: T) -> JsValue {
    let resp = HistoryResponse {
        ok: true,
        data: Some(data),
        error: None,
    };
    response_to_js(&resp)
}

/// Construct an error response.
#[allow(dead_code)]
pub fn err_response(error: HistoryError) -> JsValue {
    let resp: HistoryResponse<()> = HistoryResponse {
        ok: false,
        data: None,
        error: Some(error),
    };
    response_to_js(&resp)
}
