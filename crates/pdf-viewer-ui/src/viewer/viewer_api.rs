//! ViewerSession — struct-based WASM API for viewer-state operations.
//!
//! Mirrors the established pattern (`EditorSession`, `DocumentSession`,
//! `FindSession`, `ReviewSession`, `CommentManager`):
//!   - Zero-sized struct as JS-visible handle.
//!   - `#[wasm_bindgen]` methods with camelCase `js_name`.
//!   - Thin delegation to `viewer_store` / `viewer_controller`.
//!   - All state lives in the wasm `HOST_VIEWER_SESSION` thread_local.
//!
//! Replaces the prior flat free-function exports
//! (`get_viewer_session`, `set_viewer_document`, `set_current_page`, …),
//! which remain for backward compatibility while the TS bridge migrates.

use serde_wasm_bindgen::to_value;
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
        to_value(&viewer_store::get_viewer_session()).unwrap_or(JsValue::NULL)
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
}

impl Default for ViewerSession {
    fn default() -> Self {
        Self::new()
    }
}
