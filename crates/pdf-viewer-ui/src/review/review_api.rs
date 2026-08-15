//! ReviewSession — P2 struct-based WASM API for change-review (accept/reject patches).
//!
//! Mirrors the P0 `EditorSession` / P1 `DocumentSession` / P2 `FindSession` pattern.
//! Delegates to `crate::ui_state_store`. The legacy `review::facade::reviewFacade*`
//! functions remain for backward compatibility.

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use pdf_viewer_core::persistence::review_types::ReviewFeedResult;

use crate::ui_state_store::{
    accept_all_review_changes, accept_review_change, collect_review_changes,
    current_patch_revision, reject_all_review_changes, reject_review_change,
};

fn read_review_feed() -> ReviewFeedResult {
    let changes = collect_review_changes();
    ReviewFeedResult {
        revision: current_patch_revision(),
        pending_count: changes.len(),
        changes,
    }
}

// ── ReviewSessionState (Batch 2 sec 4) ──────────────────────────
//
// Explicit enum for the Review session, complementing EditorSession's
// SessionState and FindSession's FindSessionState. Like FindSessionState,
// ReviewSessionState is **derived** from existing data — specifically
// `collect_review_changes().len()` — rather than stored redundantly.
//
// Semantics
//
//   Idle        no pending review changes
//   HasPending  one or more patches awaiting accept/reject

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewSessionState {
    Idle,
    HasPending,
}

impl ReviewSessionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewSessionState::Idle => "Idle",
            ReviewSessionState::HasPending => "HasPending",
        }
    }

    fn derive(pending_count: usize) -> Self {
        if pending_count == 0 {
            ReviewSessionState::Idle
        } else {
            ReviewSessionState::HasPending
        }
    }
}

/// Snapshot of the current review session state.
pub fn read_review_state() -> ReviewSessionState {
    ReviewSessionState::derive(collect_review_changes().len())
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
        to_value(&read_review_feed()).unwrap_or(JsValue::NULL)
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

    /// Current session state (Idle / HasPending).
    ///
    /// See `ReviewSessionState` for semantics. Derived from the live
    /// review change count on every call.
    #[wasm_bindgen(js_name = "readState")]
    pub fn read_state(&self) -> JsValue {
        to_value(&read_review_state()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "getState")]
    #[deprecated(since = "0.2.0", note = "Use readState instead")]
    pub fn get_state(&self) -> JsValue {
        self.read_state()
    }
}

impl Default for ReviewSession {
    fn default() -> Self {
        Self::new()
    }
}
