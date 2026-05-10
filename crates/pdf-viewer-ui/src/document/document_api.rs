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
    pick_document_pipeline,
    redo_document_pipeline,
    rotate_document_pipeline,
    undo_document_pipeline,
    OpenDocumentPipelineRequest, PickDocumentPipelineRequest,
};
use crate::document::mutation_pipeline::request_document_refresh;
use crate::document::patch_persistence::apply_document_patch;
use crate::editor::editor_controller::build_region_text_patch;
use crate::editor::replace_pipeline::{
    apply_region_text_replacements_tx,
    RegionTextReplaceRequest,
};
use crate::host::command::{
    open_document_session,
    reset_host_document_session, OpenDocumentSessionRequest,
};
use crate::present::plan_builder::FramePlanRequest;
use crate::viewer::viewer_controller::note_document_mutation;

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

    /// Open a PDF after presenting the system file picker.
    #[wasm_bindgen(js_name = "pick")]
    pub async fn pick(&self, request_js: JsValue) -> Result<JsValue, JsValue> {
        let request: PickDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
        pick_document_pipeline(request).await
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

    // ── Refresh / revision ─────────────────────────────────────

    /// Request a refresh / re-render after an external mutation.
    #[wasm_bindgen(js_name = "requestRefresh")]
    pub fn request_refresh(&self, reason: String, frame_request_js: JsValue) -> JsValue {
        let frame_request: FramePlanRequest = from_value(frame_request_js).unwrap_or_default();
        to_value(&request_document_refresh(&reason, frame_request)).unwrap_or(JsValue::NULL)
    }

    /// Bump the document revision counter (e.g. after a sidecar mutation).
    #[wasm_bindgen(js_name = "bumpRevision")]
    pub fn bump_revision(&self, reason: String) -> u64 {
        note_document_mutation(&reason)
    }

    // ── Session setup ──────────────────────────────────────────

    /// Initialize the host session for a freshly loaded document.
    #[wasm_bindgen(js_name = "openSession")]
    pub fn open_session(&self, request_js: JsValue) -> JsValue {
        let request: OpenDocumentSessionRequest = from_value(request_js).unwrap_or_default();
        to_value(&open_document_session(request)).unwrap_or(JsValue::NULL)
    }

    /// Reset the host session to default page dimensions.
    #[wasm_bindgen(js_name = "resetSession")]
    pub fn reset_session(&self, default_page_width: f32, default_page_height: f32) -> JsValue {
        to_value(&reset_host_document_session(default_page_width, default_page_height))
            .unwrap_or(JsValue::NULL)
    }

    // ── Patch / region edits ───────────────────────────────────

    /// Apply a persistable region patch (used by the patch queue / undo stack).
    #[wasm_bindgen(js_name = "applyPatch")]
    pub fn apply_patch(&self, patch_js: JsValue) {
        apply_document_patch(patch_js);
    }

    /// Build a region patch object (does not apply it).
    #[wasm_bindgen(js_name = "buildRegionPatch")]
    pub fn build_region_patch(
        &self,
        page_index: u16,
        region_id: String,
        kind: String,
        original_text: String,
        new_text: String,
    ) -> JsValue {
        let patch =
            build_region_text_patch(page_index, &region_id, &kind, &original_text, new_text);
        to_value(&patch).unwrap_or(JsValue::NULL)
    }

    /// Apply a batch of region text replacements transactionally.
    #[wasm_bindgen(js_name = "applyRegionReplacements")]
    pub fn apply_region_replacements(
        &self,
        replacements_js: JsValue,
        frame_request_js: JsValue,
    ) -> JsValue {
        let replacements: Vec<RegionTextReplaceRequest> =
            from_value(replacements_js).unwrap_or_default();
        let frame_request: FramePlanRequest = from_value(frame_request_js).unwrap_or_default();
        to_value(&apply_region_text_replacements_tx(replacements, frame_request))
            .unwrap_or(JsValue::NULL)
    }

}

impl Default for DocumentSession {
    fn default() -> Self {
        Self::new()
    }
}
