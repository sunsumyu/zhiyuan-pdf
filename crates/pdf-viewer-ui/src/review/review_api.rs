//! ReviewSession — P2 struct-based WASM API for change-review (accept/reject patches).
//!
//! Mirrors the P0 `EditorSession` / P1 `DocumentSession` / P2 `FindSession` pattern.
//! Delegates to `crate::document::review`. The legacy `review::facade::reviewFacade*`
//! functions remain for backward compatibility.

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::document::review::{
    accept_all_review_changes,
    accept_review_change,
    get_review_feed,
    reject_all_review_changes,
    reject_review_change,
};

// ── ReviewError / ReviewResponse ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ReviewError {
    NotImplemented { method: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ReviewError>,
}

fn err_not_implemented(method: &str) -> JsValue {
    let resp: ReviewResponse<()> = ReviewResponse {
        ok: false,
        data: None,
        error: Some(ReviewError::NotImplemented { method: method.into() }),
    };
    to_value(&resp).unwrap_or(JsValue::NULL)
}

// ── ReviewSession ───────────────────────────────────────────────

#[wasm_bindgen]
pub struct ReviewSession;

#[wasm_bindgen]
impl ReviewSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        ReviewSession
    }

    /// Read the current review feed (all pending change patches).
    #[wasm_bindgen(js_name = "readFeed")]
    pub fn read_feed(&self) -> JsValue {
        to_value(&get_review_feed()).unwrap_or(JsValue::NULL)
    }

    /// Accept a single change identified by `patch_key`.
    #[wasm_bindgen(js_name = "accept")]
    pub fn accept(&self, patch_key: String) -> JsValue {
        to_value(&accept_review_change(&patch_key)).unwrap_or(JsValue::NULL)
    }

    /// Reject (revert) a single change identified by `patch_key`.
    #[wasm_bindgen(js_name = "reject")]
    pub fn reject(&self, patch_key: String) -> JsValue {
        to_value(&reject_review_change(&patch_key)).unwrap_or(JsValue::NULL)
    }

    /// Accept all pending changes.
    #[wasm_bindgen(js_name = "acceptAll")]
    pub fn accept_all(&self) -> JsValue {
        to_value(&accept_all_review_changes()).unwrap_or(JsValue::NULL)
    }

    /// Reject all pending changes.
    #[wasm_bindgen(js_name = "rejectAll")]
    pub fn reject_all(&self) -> JsValue {
        to_value(&reject_all_review_changes()).unwrap_or(JsValue::NULL)
    }

    // ── Reserved stubs ──────────────────────────────────────────

    #[wasm_bindgen(js_name = "exportReport")]
    pub fn export_report(&self, _format: String) -> JsValue {
        err_not_implemented("review.exportReport")
    }

    #[wasm_bindgen(js_name = "readFilteredFeed")]
    pub fn read_filtered_feed(&self, _filter_js: JsValue) -> JsValue {
        err_not_implemented("review.readFilteredFeed")
    }
}

impl Default for ReviewSession {
    fn default() -> Self {
        Self::new()
    }
}
