//! UI-side helpers for `AnnotationResponse<T>` JsValue serialization.
//!
//! Mirrors `crate::editor::editor_types` / `crate::document::document_types`
//! helpers for the annotation domain.

use serde::Serialize;
use wasm_bindgen::prelude::*;

pub use pdf_viewer_core::annotation::annotation_types::{
    Annotation, AnnotationBBox, AnnotationError, AnnotationKind, AnnotationResponse,
};

fn response_to_js<T: Serialize>(resp: &AnnotationResponse<T>) -> JsValue {
    serde_wasm_bindgen::to_value(resp).unwrap_or(JsValue::NULL)
}

/// Construct a success response with payload.
pub fn ok_response<T: Serialize>(data: T) -> JsValue {
    let resp = AnnotationResponse {
        ok: true,
        data: Some(data),
        error: None,
    };
    response_to_js(&resp)
}

/// Construct a success response with no payload.
#[allow(dead_code)]
pub fn ok_empty() -> JsValue {
    let resp: AnnotationResponse<()> = AnnotationResponse {
        ok: true,
        data: None,
        error: None,
    };
    response_to_js(&resp)
}

/// Construct an error response.
pub fn err_response(error: AnnotationError) -> JsValue {
    let resp: AnnotationResponse<()> = AnnotationResponse {
        ok: false,
        data: None,
        error: Some(error),
    };
    response_to_js(&resp)
}
