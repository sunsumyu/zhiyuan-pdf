// ─────────────────────────────────────────────────────────────────────────────
// Annotation facade — frozen v1 API surface for non-comment annotations
// (highlights, ink, free-text, stamps, links).
//
// Note: comment-style annotations (PDF /Text and /Popup) live in
// `crate::comment::facade`. This module covers visual annotation primitives
// that overlay the page content.
//
// Status: ALL APIs in this module are STUB pending implementation. The
// js_names are FROZEN.
//
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

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

// ─── Highlight ───────────────────────────────────────────────────────────────

/// Reserved: add a highlight annotation over a region.
#[wasm_bindgen(js_name = "annotationFacadeAddHighlight")]
pub fn facade_add_highlight(_request_js: JsValue) -> JsValue {
    stub("annotation.addHighlight")
}

/// Reserved: delete a highlight annotation by id.
#[wasm_bindgen(js_name = "annotationFacadeDeleteHighlight")]
pub fn facade_delete_highlight(_request_js: JsValue) -> JsValue {
    stub("annotation.deleteHighlight")
}

/// Reserved: list all highlights on a page.
#[wasm_bindgen(js_name = "annotationFacadeListHighlights")]
pub fn facade_list_highlights(_path: String, _page_index: u16) -> JsValue {
    stub("annotation.listHighlights")
}

// ─── Ink / freehand ──────────────────────────────────────────────────────────

/// Reserved: add an ink stroke annotation (path + color + width).
#[wasm_bindgen(js_name = "annotationFacadeAddInk")]
pub fn facade_add_ink(_request_js: JsValue) -> JsValue {
    stub("annotation.addInk")
}

/// Reserved: delete an ink annotation.
#[wasm_bindgen(js_name = "annotationFacadeDeleteInk")]
pub fn facade_delete_ink(_path: String, _annotation_id: String) -> JsValue {
    stub("annotation.deleteInk")
}

// ─── Free-text ───────────────────────────────────────────────────────────────

/// Reserved: add a free-text (sticky note text) annotation.
#[wasm_bindgen(js_name = "annotationFacadeAddFreeText")]
pub fn facade_add_free_text(_request_js: JsValue) -> JsValue {
    stub("annotation.addFreeText")
}

/// Reserved: edit an existing free-text annotation.
#[wasm_bindgen(js_name = "annotationFacadeUpdateFreeText")]
pub fn facade_update_free_text(_request_js: JsValue) -> JsValue {
    stub("annotation.updateFreeText")
}

// ─── Stamp ───────────────────────────────────────────────────────────────────

/// Reserved: add a stamp annotation (built-in or custom image).
#[wasm_bindgen(js_name = "annotationFacadeAddStamp")]
pub fn facade_add_stamp(_request_js: JsValue) -> JsValue {
    stub("annotation.addStamp")
}

// ─── Link ────────────────────────────────────────────────────────────────────

/// Reserved: add a clickable link annotation (URI / page / named-dest).
#[wasm_bindgen(js_name = "annotationFacadeAddLink")]
pub fn facade_add_link(_request_js: JsValue) -> JsValue {
    stub("annotation.addLink")
}

/// Reserved: list all link annotations on a page.
#[wasm_bindgen(js_name = "annotationFacadeListLinks")]
pub fn facade_list_links(_path: String, _page_index: u16) -> JsValue {
    stub("annotation.listLinks")
}

// ─── General ─────────────────────────────────────────────────────────────────

/// Reserved: list all annotations of all types on a page.
#[wasm_bindgen(js_name = "annotationFacadeListAll")]
pub fn facade_list_all(_path: String, _page_index: u16) -> JsValue {
    stub("annotation.listAll")
}

/// Reserved: delete any annotation by id (regardless of type).
#[wasm_bindgen(js_name = "annotationFacadeDelete")]
pub fn facade_delete(_path: String, _annotation_id: String) -> JsValue {
    stub("annotation.delete")
}

/// Reserved: move an annotation to a new bounding box.
#[wasm_bindgen(js_name = "annotationFacadeMove")]
pub fn facade_move(_path: String, _annotation_id: String, _new_box_js: JsValue) -> JsValue {
    stub("annotation.move")
}

/// Reserved: change an annotation's color / opacity / appearance.
#[wasm_bindgen(js_name = "annotationFacadeRestyle")]
pub fn facade_restyle(_path: String, _annotation_id: String, _style_js: JsValue) -> JsValue {
    stub("annotation.restyle")
}
