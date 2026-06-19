//! Application — top-level WASM composition root (Batch 2 §8).
//!
//! **Why this exists**
//!
//! Before this handle, opening or closing a document required the TS bridge to
//! call each Session's lifecycle methods individually (DocumentSession.open →
//! ViewerSession.setDocument → FindSession.clear → …).  If any call was missed,
//! stale state from the previous document leaked into the new one.
//!
//! `Application` provides a **single entry point** for document lifecycle
//! operations.  Internally it delegates to the existing Session stores and
//! controllers, ensuring every domain is reset/initialised atomically.
//!
//! **What stays in TS**
//!
//! DOM operations (clear vector host, sync zoom select, show empty state, etc.)
//! are **not** managed here — they remain the TS bridge's responsibility.
//! After calling `Application.open()` or `Application.close()`, the TS bridge
//! should trigger its DOM sync pass as before.
//!
//! **Aggregated state**
//!
//! `getState()` returns a snapshot of every domain's explicit state enum in one
//! call, so the TS bridge can drive conditional UI without N individual reads.

use serde::Serialize;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::document::host_pipeline::{
    close_document_pipeline, open_document_pipeline, CloseDocumentPipelineResult,
    OpenDocumentPipelineRequest, OpenDocumentPipelineResult,
};
use crate::editor::editor_store;
use crate::editor::editor_types::SessionState as EditorSessionState;
use crate::find::find_store;
use crate::find::find_store::FindSessionState;
use crate::presentation::page_turn::{read_snapshot as read_page_turn_snapshot, PageTurnSnapshot};
use crate::review::review_api::{read_review_state, ReviewSessionState};
use crate::viewer::viewer_store::{read_viewer_state, ViewerSessionState};
use crate::zoom::zoom_store::{read_session_state as read_zoom_session_state, ZoomSessionState};

// ── Aggregated state snapshot ───────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationState {
    pub viewer: ViewerSessionState,
    pub editor: EditorSessionState,
    pub find: FindSessionState,
    pub review: ReviewSessionState,
    pub zoom: ZoomSessionState,
    pub presentation: PageTurnSnapshot,
}

fn snapshot_state() -> ApplicationState {
    ApplicationState {
        viewer: read_viewer_state(),
        editor: editor_store::read_state(),
        find: find_store::read_find_state(),
        review: read_review_state(),
        zoom: read_zoom_session_state(),
        presentation: read_page_turn_snapshot(),
    }
}

// ── Cross-session reset helper ──────────────────────────────────
//
// `close_document_pipeline` → `reset_host_document_session` → `reset_session`
// → `reset_viewer_runtime` already resets:
//
//   viewer store, render state, present runtime, progressive render,
//   render loop, editor host runtime, editor mode, persistable patches,
//   host find session, comment/review session, zoom preview host.
//
// The ONE gap is the **find controller** (`find_store::CONTROLLER`) which
// tracks `is_open`, `last_result`, etc.  `clear_find_session()` (called by
// `reset_viewer_runtime`) only resets the *host* find session snapshot, not
// the controller itself.  We close that gap here.

fn reset_find_controller() {
    // close_find() sets is_open=false, clears last_result, and also calls
    // clear_find_session() internally — safe to call even if already cleared.
    let _ = find_store::close_find();
}

// ── Application handle ──────────────────────────────────────────

#[wasm_bindgen]
pub struct Application;

#[wasm_bindgen]
impl Application {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Application
    }

    /// Open a document, resetting all leftover state from any prior document.
    ///
    /// Equivalent to the TS-side `openTextPdfFlow` orchestration but performed
    /// atomically in Rust:
    ///
    /// 1. Reset find controller (the gap not covered by `open_document_pipeline`)
    /// 2. Delegate to `open_document_pipeline` which sets viewer + zoom + invokes Tauri
    ///
    /// Returns `OpenDocumentPipelineResult` (same shape DocumentSession.open returns).
    /// The TS bridge should then do its DOM sync (clearVectorHost, syncZoomSelect, …).
    #[wasm_bindgen(js_name = "open")]
    pub async fn open(&self, request_js: JsValue) -> Result<JsValue, JsValue> {
        // Reset stale state from a previous document that open_document_pipeline
        // doesn't cover.  open_document_session internally sets viewer+zoom but
        // doesn't reset find/editor.  The editor is stateless between documents
        // (Viewing state) but the find controller keeps is_open + last_result.
        reset_find_controller();

        let request: OpenDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
        let path_for_event = request.path.clone();
        let result: OpenDocumentPipelineResult = open_document_pipeline(request).await?;

        // Emit document.open event with the path as payload
        if result.opened {
            crate::events::emit(
                crate::events::event_names::DOCUMENT_OPEN,
                &JsValue::from_str(&path_for_event),
            );
        }

        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    /// Close the active document and reset **all** WASM session state.
    ///
    /// `close_document_pipeline` handles the heavy lifting (viewer, zoom, render,
    /// editor, patches, host find session, comment/review).  We additionally
    /// reset the find controller to close the gap.
    ///
    /// Returns `CloseDocumentPipelineResult`.
    /// The TS bridge should then do its DOM cleanup (clearVectorHost, showEmptyState, …).
    #[wasm_bindgen(js_name = "close")]
    pub fn close(&self, default_page_width: f32, default_page_height: f32) -> JsValue {
        let result: CloseDocumentPipelineResult =
            close_document_pipeline(default_page_width, default_page_height);
        reset_find_controller();

        // Emit document.close event
        crate::events::emit(
            crate::events::event_names::DOCUMENT_CLOSE,
            &JsValue::UNDEFINED,
        );

        to_value(&result).unwrap_or(JsValue::NULL)
    }

    /// Reset all WASM session state without opening or closing a document.
    ///
    /// Useful for a hard "return to empty state" without touching the Tauri
    /// backend.  Equivalent to `close` but without the Tauri side-effects.
    #[wasm_bindgen(js_name = "resetAll")]
    pub fn reset_all(&self) {
        use crate::viewer::viewer_controller::reset_session;
        reset_session(); // comprehensive reset (viewer, render, editor, patches, find host, review)
        reset_find_controller(); // close the find controller gap
        use crate::zoom::zoom_controller::reset_zoom_runtime;
        reset_zoom_runtime(1.0);
    }

    /// Aggregated state snapshot from every domain's explicit state enum.
    ///
    /// Returns `{ viewer, editor, find, review, zoom }` — one call instead of
    /// five individual Session.getState() calls.
    #[wasm_bindgen(js_name = "readState")]
    pub fn read_state(&self) -> JsValue {
        to_value(&snapshot_state()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "getState")]
    #[deprecated(since = "0.2.0", note = "Use readState instead")]
    pub fn get_state(&self) -> JsValue {
        self.read_state()
    }

    // ── Event system (Nutrient borrowing #1) ────────────────────

    /// Register an event listener. DOM-style: multiple listeners per event.
    ///
    /// ```js
    /// app.addEventListener("editor.stateChange", (state) => { ... });
    /// app.addEventListener("viewer.pageChange", (pageIndex) => { ... });
    /// ```
    ///
    /// See `events::event_names` for the full list of event names.
    #[wasm_bindgen(js_name = "addEventListener")]
    pub fn add_event_listener(&self, event: String, listener: js_sys::Function) {
        crate::events::add_listener(&event, listener);
    }

    /// Remove a previously registered listener (same function reference).
    ///
    /// Returns `true` if a listener was found and removed.
    #[wasm_bindgen(js_name = "removeEventListener")]
    pub fn remove_event_listener(&self, event: String, listener: &js_sys::Function) -> bool {
        crate::events::remove_listener(&event, listener)
    }

    /// Remove all event listeners (all events). Called internally by `resetAll`.
    #[wasm_bindgen(js_name = "removeAllEventListeners")]
    pub fn remove_all_event_listeners(&self) {
        crate::events::clear_all();
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
