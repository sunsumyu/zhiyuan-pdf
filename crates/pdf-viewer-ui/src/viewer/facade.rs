// ─────────────────────────────────────────────────────────────────────────────
// Viewer facade — frozen v1 API surface for the viewer session.
// (current document path / page count / current page / current zoom / page size)
//
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::host::command::{
    apply_zoom_selection as host_apply_zoom_selection,
    navigate_next_page as host_navigate_next_page,
    navigate_prev_page as host_navigate_prev_page,
};
use crate::viewer::runtime::{
    get_session as host_get_session,
    reset_session as host_reset_session,
    set_document as host_set_document,
    set_page as host_set_page,
    set_page_size as host_set_page_size,
    set_zoom as host_set_zoom,
};

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

// ─── Stable ──────────────────────────────────────────────────────────────────

/// Read the active viewer session (path, page count, current page, current zoom, page size).
#[wasm_bindgen(js_name = "viewerFacadeReadSession")]
pub fn facade_read_session() -> JsValue {
    to_value(&host_get_session()).unwrap_or(JsValue::NULL)
}

/// Reset the viewer session to defaults (no document loaded).
#[wasm_bindgen(js_name = "viewerFacadeResetSession")]
pub fn facade_reset_session() {
    host_reset_session();
}

/// Bind the viewer session to a freshly opened document.
#[wasm_bindgen(js_name = "viewerFacadeSetDocument")]
pub fn facade_set_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    host_set_document(path, page_count, initial_zoom);
}

/// Update the current page index (0-based).
#[wasm_bindgen(js_name = "viewerFacadeSetCurrentPage")]
pub fn facade_set_current_page(page_index: u16) {
    host_set_page(page_index);
}

/// Update the current zoom multiplier.
#[wasm_bindgen(js_name = "viewerFacadeSetCurrentZoom")]
pub fn facade_set_current_zoom(zoom: f32) {
    host_set_zoom(zoom);
}

/// Set the active page's intrinsic dimensions (PDF user-space units).
#[wasm_bindgen(js_name = "viewerFacadeSetPageSize")]
pub fn facade_set_page_size(page_width: f32, page_height: f32) {
    host_set_page_size(page_width, page_height);
}

/// Move to the previous page; returns the navigation result envelope.
#[wasm_bindgen(js_name = "viewerFacadeNavigatePrev")]
pub fn facade_navigate_prev() -> JsValue {
    to_value(&host_navigate_prev_page()).unwrap_or(JsValue::NULL)
}

/// Move to the next page; returns the navigation result envelope.
#[wasm_bindgen(js_name = "viewerFacadeNavigateNext")]
pub fn facade_navigate_next() -> JsValue {
    to_value(&host_navigate_next_page()).unwrap_or(JsValue::NULL)
}

/// Apply a zoom selection (e.g. fit-page / fit-width / explicit ratio).
#[wasm_bindgen(js_name = "viewerFacadeApplyZoomSelection")]
pub fn facade_apply_zoom_selection(zoom: f32) -> JsValue {
    to_value(&host_apply_zoom_selection(zoom)).unwrap_or(JsValue::NULL)
}

// ─── Stubs ───────────────────────────────────────────────────────────────────

/// Reserved: jump to a specific page with optional anchor (top/center/named-dest).
#[wasm_bindgen(js_name = "viewerFacadeGoToPage")]
pub fn facade_go_to_page(_page_index: u16, _anchor: Option<String>) -> JsValue {
    stub("viewer.goToPage")
}

/// Reserved: jump to a named destination defined in the document outline.
#[wasm_bindgen(js_name = "viewerFacadeGoToNamedDestination")]
pub fn facade_go_to_named_destination(_name: String) -> JsValue {
    stub("viewer.goToNamedDestination")
}

/// Reserved: enter / exit single-page presentation mode.
#[wasm_bindgen(js_name = "viewerFacadeSetPresentationMode")]
pub fn facade_set_presentation_mode(_enabled: bool) -> JsValue {
    stub("viewer.setPresentationMode")
}

/// Reserved: change page layout (single / continuous / facing / two-page).
#[wasm_bindgen(js_name = "viewerFacadeSetLayoutMode")]
pub fn facade_set_layout_mode(_mode: String) -> JsValue {
    stub("viewer.setLayoutMode")
}
