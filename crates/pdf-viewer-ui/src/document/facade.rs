// ─────────────────────────────────────────────────────────────────────────────
// Document facade — frozen v1 API surface for document-level operations.
//
// Stability:
//   • Stable APIs in this file MUST NOT be renamed or have their signatures
//     changed. Add new fields as Optional only.
//   • Stub APIs are reserved js_names; implementations land later.
//
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::document::host_pipeline::{
    close_document_pipeline as host_close_pipeline,
    open_document_pipeline as host_open_pipeline,
    pick_document_pipeline as host_pick_pipeline,
    redo_document_pipeline as host_redo_pipeline,
    rotate_document_pipeline as host_rotate_pipeline,
    undo_document_pipeline as host_undo_pipeline,
    OpenDocumentPipelineRequest, PickDocumentPipelineRequest,
};
use crate::document::mutation_pipeline::request_document_refresh as host_request_refresh;
use crate::editor::replace_pipeline::{
    apply_region_text_replacements_tx as host_apply_region_replacements,
    RegionTextReplaceRequest,
};
use crate::editor::runtime::build_region_text_patch as host_build_region_patch;
use crate::host::command::{
    open_document_session as host_open_session,
    reset_host_document_session as host_reset_session,
    OpenDocumentSessionRequest,
};
use crate::document::patch_persistence::apply_document_patch as host_apply_patch;
use crate::present::plan_builder::FramePlanRequest;
use crate::viewer::runtime::note_document_mutation as host_note_mutation;

// ─── Shared stub helper (mirrors editor::facade::StubResult) ─────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StubResult {
    implemented: bool,
    error: String,
}

fn stub(api: &str) -> JsValue {
    let result = StubResult {
        implemented: false,
        error: format!("{} is reserved but not yet implemented", api),
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

// ─────────────────────────────────────────────────────────────────────────────
// Document facade — STABLE API
// ─────────────────────────────────────────────────────────────────────────────

/// Open a PDF through the host pipeline.
#[wasm_bindgen(js_name = "documentFacadeOpen")]
pub async fn facade_open(request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: OpenDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
    let result = host_open_pipeline(request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

/// Open a PDF after presenting the system file picker.
#[wasm_bindgen(js_name = "documentFacadePick")]
pub async fn facade_pick(request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: PickDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
    host_pick_pipeline(request).await
}

/// Close the active document and reset host session to default page size.
#[wasm_bindgen(js_name = "documentFacadeClose")]
pub fn facade_close(default_page_width: f32, default_page_height: f32) -> JsValue {
    to_value(&host_close_pipeline(default_page_width, default_page_height))
        .unwrap_or(JsValue::NULL)
}

/// Document-level undo (one history step).
#[wasm_bindgen(js_name = "documentFacadeUndo")]
pub fn facade_undo() -> JsValue {
    to_value(&host_undo_pipeline()).unwrap_or(JsValue::NULL)
}

/// Document-level redo.
#[wasm_bindgen(js_name = "documentFacadeRedo")]
pub fn facade_redo() -> JsValue {
    to_value(&host_redo_pipeline()).unwrap_or(JsValue::NULL)
}

/// Rotate the current page by `delta * 90` degrees.
#[wasm_bindgen(js_name = "documentFacadeRotate")]
pub async fn facade_rotate(delta: i32) -> Result<JsValue, JsValue> {
    let result = host_rotate_pipeline(delta).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

/// Request a refresh / re-render after an external mutation.
#[wasm_bindgen(js_name = "documentFacadeRequestRefresh")]
pub fn facade_request_refresh(reason: String, frame_request_js: JsValue) -> JsValue {
    let frame_request: FramePlanRequest = from_value(frame_request_js).unwrap_or_default();
    to_value(&host_request_refresh(&reason, frame_request)).unwrap_or(JsValue::NULL)
}

/// Bump the document revision counter (e.g. after a sidecar mutation).
#[wasm_bindgen(js_name = "documentFacadeBumpRevision")]
pub fn facade_bump_revision(reason: String) -> u64 {
    host_note_mutation(&reason)
}

/// Initialize the host session for a freshly loaded document.
#[wasm_bindgen(js_name = "documentFacadeOpenSession")]
pub fn facade_open_session(request_js: JsValue) -> JsValue {
    let request: OpenDocumentSessionRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_open_session(request)).unwrap_or(JsValue::NULL)
}

/// Reset the host session to default page dimensions.
#[wasm_bindgen(js_name = "documentFacadeResetSession")]
pub fn facade_reset_session(default_page_width: f32, default_page_height: f32) -> JsValue {
    to_value(&host_reset_session(default_page_width, default_page_height))
        .unwrap_or(JsValue::NULL)
}

/// Apply a persistable region patch (used by the patch queue / undo stack).
#[wasm_bindgen(js_name = "documentFacadeApplyPatch")]
pub fn facade_apply_patch(patch_js: JsValue) {
    host_apply_patch(patch_js);
}

/// Build a region patch object (does not apply it).
#[wasm_bindgen(js_name = "documentFacadeBuildRegionPatch")]
pub fn facade_build_region_patch(
    page_index: u16,
    region_id: String,
    kind: String,
    original_text: String,
    new_text: String,
) -> JsValue {
    let patch = host_build_region_patch(page_index, &region_id, &kind, &original_text, new_text);
    to_value(&patch).unwrap_or(JsValue::NULL)
}

/// Apply a batch of region text replacements transactionally.
#[wasm_bindgen(js_name = "documentFacadeApplyRegionReplacements")]
pub fn facade_apply_region_replacements(
    replacements_js: JsValue,
    frame_request_js: JsValue,
) -> JsValue {
    let replacements: Vec<RegionTextReplaceRequest> =
        from_value(replacements_js).unwrap_or_default();
    let frame_request: FramePlanRequest = from_value(frame_request_js).unwrap_or_default();
    to_value(&host_apply_region_replacements(replacements, frame_request))
        .unwrap_or(JsValue::NULL)
}

// ─────────────────────────────────────────────────────────────────────────────
// Document facade — STUB API (reserved js_names, frozen)
// ─────────────────────────────────────────────────────────────────────────────

/// Reserved: insert a blank page at `index` (or copy from `source_path`).
#[wasm_bindgen(js_name = "documentFacadeInsertPage")]
pub fn facade_insert_page(_index: u16, _source_path: Option<String>) -> JsValue {
    stub("document.insertPage")
}

/// Reserved: remove page at `index`.
#[wasm_bindgen(js_name = "documentFacadeRemovePage")]
pub fn facade_remove_page(_index: u16) -> JsValue {
    stub("document.removePage")
}

/// Reserved: reorder pages by moving `from` to `to`.
#[wasm_bindgen(js_name = "documentFacadeMovePage")]
pub fn facade_move_page(_from: u16, _to: u16) -> JsValue {
    stub("document.movePage")
}

/// Reserved: rotate a single page (vs. document-wide rotate).
#[wasm_bindgen(js_name = "documentFacadeRotatePage")]
pub fn facade_rotate_page(_index: u16, _delta: i32) -> JsValue {
    stub("document.rotatePage")
}

/// Reserved: read document metadata (title/author/subject/keywords/dates).
#[wasm_bindgen(js_name = "documentFacadeReadMetadata")]
pub fn facade_read_metadata() -> JsValue {
    stub("document.readMetadata")
}

/// Reserved: write document metadata.
#[wasm_bindgen(js_name = "documentFacadeSetMetadata")]
pub fn facade_set_metadata(_metadata_js: JsValue) -> JsValue {
    stub("document.setMetadata")
}

/// Reserved: export selected pages to PDF / image / text.
#[wasm_bindgen(js_name = "documentFacadeExportPages")]
pub fn facade_export_pages(
    _page_indices_js: JsValue,
    _format: String,
    _output_path: String,
) -> JsValue {
    stub("document.exportPages")
}

/// Reserved: encrypt document with a password.
#[wasm_bindgen(js_name = "documentFacadeSetPassword")]
pub fn facade_set_password(_owner: String, _user: String) -> JsValue {
    stub("document.setPassword")
}

/// Reserved: remove document encryption (requires owner credentials in state).
#[wasm_bindgen(js_name = "documentFacadeRemovePassword")]
pub fn facade_remove_password() -> JsValue {
    stub("document.removePassword")
}

/// Reserved: read document outline (table of contents) tree.
#[wasm_bindgen(js_name = "documentFacadeReadOutline")]
pub fn facade_read_outline() -> JsValue {
    stub("document.readOutline")
}

/// Reserved: write document outline.
#[wasm_bindgen(js_name = "documentFacadeSetOutline")]
pub fn facade_set_outline(_outline_js: JsValue) -> JsValue {
    stub("document.setOutline")
}

/// Reserved: PDF/A-style flatten — convert annotations + form fields to content.
#[wasm_bindgen(js_name = "documentFacadeFlatten")]
pub fn facade_flatten() -> JsValue {
    stub("document.flatten")
}

/// Reserved: list embedded form fields.
#[wasm_bindgen(js_name = "documentFacadeReadFormFields")]
pub fn facade_read_form_fields() -> JsValue {
    stub("document.readFormFields")
}

/// Reserved: fill a form field by name.
#[wasm_bindgen(js_name = "documentFacadeFillFormField")]
pub fn facade_fill_form_field(_field_name: String, _value: String) -> JsValue {
    stub("document.fillFormField")
}

/// Reserved: enumerate digital signatures.
#[wasm_bindgen(js_name = "documentFacadeReadSignatures")]
pub fn facade_read_signatures() -> JsValue {
    stub("document.readSignatures")
}

/// Reserved: list embedded attachments / file specs.
#[wasm_bindgen(js_name = "documentFacadeReadAttachments")]
pub fn facade_read_attachments() -> JsValue {
    stub("document.readAttachments")
}
