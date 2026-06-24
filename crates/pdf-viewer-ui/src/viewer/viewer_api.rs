//! ViewerSession — struct-based WASM API for viewer-state operations.
//!
//! Mirrors the established pattern (`EditorSession`, `DocumentSession`,
//! `FindSession`, `ReviewSession`, `CommentManager`):
//!   - Zero-sized struct as JS-visible handle.
//!   - `#[wasm_bindgen]` methods with camelCase `js_name`.
//!   - Thin delegation to `viewer_store` / `viewer_controller`.
//!   - All state lives in the wasm `VIEWER_SESSION` thread_local.
//!
//! Replaces the prior flat free-function exports
//! (`get_viewer_session`, `set_viewer_document`, `set_current_page`, …),
//! which remain for backward compatibility while the TS bridge migrates.

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::viewer::viewer_controller;
use crate::viewer::viewer_store;

#[wasm_bindgen]
pub struct ViewerSession;

#[wasm_bindgen]
impl ViewerSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        ViewerSession
    }

    /// Read the current viewer-session snapshot (path / pages / zoom / page dims).
    #[wasm_bindgen(js_name = "read")]
    pub fn read(&self) -> JsValue {
        let session = viewer_store::read_viewer_session();
        web_sys::console::log_1(&JsValue::from_str(&format!("[WASM-ViewerSession] read() is called. path={:?}, page_count={}", session.path, session.page_count)));
        to_value(&session).unwrap_or(JsValue::NULL)
    }

    /// Bind a freshly opened document into the viewer session.
    #[wasm_bindgen(js_name = "setDocument")]
    pub fn set_document(&self, path: String, page_count: u16, initial_zoom: f32) {
        viewer_store::set_viewer_document(Some(path), page_count, initial_zoom);
    }

    /// Reset the viewer session to its empty/default state.
    #[wasm_bindgen(js_name = "reset")]
    pub fn reset(&self) {
        viewer_store::reset_viewer_session();
    }

    /// Set the current page index.
    #[wasm_bindgen(js_name = "setCurrentPage")]
    pub fn set_current_page(&self, page_index: u16) {
        viewer_controller::set_page(page_index);
    }

    /// Set the current zoom factor.
    #[wasm_bindgen(js_name = "setCurrentZoom")]
    pub fn set_current_zoom(&self, zoom: f32) {
        viewer_controller::set_zoom(zoom);
    }

    /// Update the active page dimensions (for zoom / hit-testing math).
    #[wasm_bindgen(js_name = "setPageDimensions")]
    pub fn set_page_dimensions(&self, page_width: f32, page_height: f32) {
        viewer_store::set_page_dimensions(page_width, page_height);
    }

    /// Current session state (NoDocument / DocumentOpen).
    ///
    /// See `ViewerSessionState` in `viewer::viewer_store` for semantics.
    /// Derived from the live session's `path` field on every call.
    #[wasm_bindgen(js_name = "readState")]
    pub fn read_state(&self) -> JsValue {
        to_value(&viewer_store::read_viewer_state()).unwrap_or(JsValue::NULL)
    }

    /// Functional (Nutrient-style) atomic state update.
    ///
    /// ```js
    /// viewerSession.setState(s => ({ ...s, currentPage: 2, currentZoom: 1.5 }));
    /// ```
    ///
    /// The `updater` callback receives the current snapshot and must return
    /// the desired new state.  Only **mutable** fields are applied:
    ///
    ///   `currentPage`, `currentZoom`, `pageWidth`, `pageHeight`
    ///
    /// `path`, `pageCount`, and `documentRevision` are lifecycle-managed
    /// and silently ignored if the updater changes them.
    ///
    /// Returns the resulting snapshot after applying the update.
    #[wasm_bindgen(js_name = "setState")]
    pub fn set_state(&self, updater: &js_sys::Function) -> JsValue {
        use pdf_viewer_core::render::viewer_session::HostViewerSession;

        // 1. Read current snapshot
        let current = viewer_store::read_viewer_session();
        let current_js = to_value(&current).unwrap_or(JsValue::NULL);

        // 2. Call JS updater(currentState) → newState
        let new_js = match updater.call1(&JsValue::NULL, &current_js) {
            Ok(v) => v,
            Err(_) => return current_js,
        };

        // 3. Deserialize the returned object
        let updated: HostViewerSession = match from_value(new_js) {
            Ok(v) => v,
            Err(_) => return current_js,
        };

        // 4. Apply only mutable fields atomically
        viewer_store::update_mutable_fields(|s| {
            s.current_page = updated.current_page;
            s.current_zoom = if updated.current_zoom.is_finite() && updated.current_zoom > 0.0 {
                updated.current_zoom
            } else {
                s.current_zoom
            };
            s.page_width = if updated.page_width > 0.0 {
                updated.page_width
            } else {
                s.page_width
            };
            s.page_height = if updated.page_height > 0.0 {
                updated.page_height
            } else {
                s.page_height
            };
        });

        // 5. Return the resulting snapshot
        to_value(&viewer_store::read_viewer_session()).unwrap_or(JsValue::NULL)
    }
}

impl Default for ViewerSession {
    fn default() -> Self {
        Self::new()
    }
}
