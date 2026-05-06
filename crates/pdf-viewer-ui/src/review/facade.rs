// ─────────────────────────────────────────────────────────────────────────────
// Review facade — frozen v1 API surface for change-review (accept/reject patches).
//
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::document::review::{
    accept_all_review_changes as host_accept_all,
    accept_review_change as host_accept,
    get_review_feed as host_get_feed,
    reject_all_review_changes as host_reject_all,
    reject_review_change as host_reject,
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

/// Read the current review feed (all pending change patches).
#[wasm_bindgen(js_name = "reviewFacadeReadFeed")]
pub fn facade_read_feed() -> JsValue {
    to_value(&host_get_feed()).unwrap_or(JsValue::NULL)
}

/// Accept a single change identified by `patch_key`.
#[wasm_bindgen(js_name = "reviewFacadeAccept")]
pub fn facade_accept(patch_key: String) -> JsValue {
    to_value(&host_accept(&patch_key)).unwrap_or(JsValue::NULL)
}

/// Reject (revert) a single change identified by `patch_key`.
#[wasm_bindgen(js_name = "reviewFacadeReject")]
pub fn facade_reject(patch_key: String) -> JsValue {
    to_value(&host_reject(&patch_key)).unwrap_or(JsValue::NULL)
}

/// Accept all pending changes.
#[wasm_bindgen(js_name = "reviewFacadeAcceptAll")]
pub fn facade_accept_all() -> JsValue {
    to_value(&host_accept_all()).unwrap_or(JsValue::NULL)
}

/// Reject all pending changes.
#[wasm_bindgen(js_name = "reviewFacadeRejectAll")]
pub fn facade_reject_all() -> JsValue {
    to_value(&host_reject_all()).unwrap_or(JsValue::NULL)
}

// ─── Stubs ───────────────────────────────────────────────────────────────────

/// Reserved: export the review feed as a structured summary report.
#[wasm_bindgen(js_name = "reviewFacadeExportReport")]
pub fn facade_export_report(_format: String) -> JsValue {
    stub("review.exportReport")
}

/// Reserved: filter the review feed (by author / page / kind / status).
#[wasm_bindgen(js_name = "reviewFacadeReadFilteredFeed")]
pub fn facade_read_filtered_feed(_filter_js: JsValue) -> JsValue {
    stub("review.readFilteredFeed")
}
