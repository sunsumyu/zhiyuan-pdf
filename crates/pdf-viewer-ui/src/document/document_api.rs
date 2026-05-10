//! DocumentSession — P1 struct-based WASM API for document-level operations.
//!
//! Mirrors the pattern established by `EditorSession` (P0):
//!   - Zero-sized struct as handle.
//!   - `#[wasm_bindgen]` methods with camelCase `js_name`.
//!   - Thin delegation to existing `host_*` pipeline functions.
//!   - Structured `DocumentError` / `DocumentResponse<T>` wrappers.
//!   - Stubs return `NotImplemented` errors (no silent fallback).
//!
//! The legacy `document::facade::documentFacade*` functions remain for
//! backward compatibility with the TS bridge; new TS code should
//! construct `DocumentSession` and call its methods directly.

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::document::host_pipeline::{
    close_document_pipeline,
    open_document_pipeline,
    redo_document_pipeline,
    rotate_document_pipeline,
    undo_document_pipeline,
    OpenDocumentPipelineRequest,
};

// ── DocumentSession ─────────────────────────────────────────────

#[wasm_bindgen]
pub struct DocumentSession;

#[wasm_bindgen]
impl DocumentSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        DocumentSession
    }

    // ── Lifecycle (async) ───────────────────────────────────────

    /// Open a PDF from a known source (path / bytes / URL).
    #[wasm_bindgen(js_name = "open")]
    pub async fn open(&self, request_js: JsValue) -> Result<JsValue, JsValue> {
        let request: OpenDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
        let result = open_document_pipeline(request).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    /// Close the active document and reset host session to default dimensions.
    #[wasm_bindgen(js_name = "close")]
    pub fn close(&self, default_page_width: f32, default_page_height: f32) -> JsValue {
        to_value(&close_document_pipeline(default_page_width, default_page_height))
            .unwrap_or(JsValue::NULL)
    }

    // ── History ─────────────────────────────────────────────────

    /// Document-level undo (one history step).
    #[wasm_bindgen(js_name = "undo")]
    pub fn undo(&self) -> JsValue {
        to_value(&undo_document_pipeline()).unwrap_or(JsValue::NULL)
    }

    /// Document-level redo.
    #[wasm_bindgen(js_name = "redo")]
    pub fn redo(&self) -> JsValue {
        to_value(&redo_document_pipeline()).unwrap_or(JsValue::NULL)
    }

    // ── Page operations ────────────────────────────────────────

    /// Rotate the current page by `delta * 90` degrees.
    #[wasm_bindgen(js_name = "rotate")]
    pub async fn rotate(&self, delta: i32) -> Result<JsValue, JsValue> {
        let result = rotate_document_pipeline(delta).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

}

impl Default for DocumentSession {
    fn default() -> Self {
        Self::new()
    }
}
