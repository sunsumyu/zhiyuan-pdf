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
    close_document_pipeline, open_document_pipeline, redo_document_pipeline,
    rotate_document_pipeline, undo_document_pipeline, OpenDocumentPipelineRequest,
};
use crate::document::mutation_pipeline::request_document_refresh;
use crate::document::patch_persistence::apply_document_patch;
use crate::editor::editor_controller::build_region_text_patch;
use crate::editor::orchestrator::replace_pipeline::{
    apply_region_text_replacements_tx, RegionTextReplaceRequest,
};
use crate::present::plan_builder::FramePlanRequest;

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
        to_value(&close_document_pipeline(
            default_page_width,
            default_page_height,
        ))
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

    // ── Dirty tracking (Nutrient borrowing #2) ─────────────────

    /// Whether the document has unsaved edits (patches pending persist).
    ///
    /// Equivalent to Nutrient's `instance.hasUnsavedChanges()`.
    #[wasm_bindgen(js_name = "hasUnsavedChanges")]
    pub fn has_unsaved_changes(&self) -> bool {
        use crate::ui_state_store::{has_visible_patches, read_patch_state};
        read_patch_state()
            .read()
            .map(|state| has_visible_patches(&state))
            .unwrap_or(false)
    }

    /// Number of persistable patches currently in memory.
    #[wasm_bindgen(js_name = "patchCount")]
    pub fn patch_count(&self) -> usize {
        use crate::ui_state_store::collect_persistable_patches;
        collect_persistable_patches().len()
    }

    /// Whether an undo step is available.
    #[wasm_bindgen(js_name = "canUndo")]
    pub fn can_undo(&self) -> bool {
        crate::ui_state_store::can_undo()
    }

    /// Whether a redo step is available.
    #[wasm_bindgen(js_name = "canRedo")]
    pub fn can_redo(&self) -> bool {
        crate::ui_state_store::can_redo()
    }

    // ── Document refresh ───────────────────────────────────────

    /// Refresh the document after a mutation (commit, undo, redo, etc.).
    ///
    /// Called by the JS bridge's `refreshDocument` flow to note the mutation
    /// in the viewer store and schedule a render frame for the updated content.
    #[wasm_bindgen(js_name = "requestRefresh")]
    pub fn request_refresh(&self, source: &str, frame_request_js: JsValue) -> JsValue {
        let frame_request: FramePlanRequest = from_value(frame_request_js).unwrap_or_default();
        let result = request_document_refresh(source, frame_request);
        to_value(&result).unwrap_or(JsValue::NULL)
    }

    // ── Patches ─────────────────────────────────────────────────

    /// Apply a region patch to the document in memory.
    #[wasm_bindgen(js_name = "applyPatch")]
    pub fn apply_patch(&self, patch_js: JsValue) {
        apply_document_patch(patch_js);
    }

    /// Build a region text patch.
    #[wasm_bindgen(js_name = "buildRegionPatch")]
    pub fn build_region_patch(
        &self,
        page_index: u16,
        region_id: String,
        kind: String,
        original_text: String,
        new_text: String,
    ) -> JsValue {
        let patch = build_region_text_patch(
            page_index,
            &region_id,
            &kind,
            &original_text,
            new_text,
        );
        to_value(&patch).unwrap_or(JsValue::NULL)
    }

    /// Apply region text replacements.
    #[wasm_bindgen(js_name = "applyRegionReplacements")]
    pub fn apply_region_replacements(
        &self,
        replacements_js: JsValue,
        frame_request_js: JsValue,
    ) -> JsValue {
        let replacements: Vec<RegionTextReplaceRequest> =
            from_value(replacements_js).unwrap_or_default();
        let frame_request: FramePlanRequest = from_value(frame_request_js).unwrap_or_default();
        let result = apply_region_text_replacements_tx(replacements, frame_request);
        to_value(&result).unwrap_or(JsValue::NULL)
    }
}

impl Default for DocumentSession {
    fn default() -> Self {
        Self::new()
    }
}
