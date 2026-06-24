//! FindSession — P2 struct-based WASM API for in-document search.
//!
//! Mirrors the P0 `EditorSession` / P1 `DocumentSession` pattern:
//!   - Zero-sized struct as handle.
//!   - `#[wasm_bindgen]` methods with camelCase `js_name`.
//!   - Thin delegation to `find::find_store`.
//!
//! The legacy `find::facade::findFacade*` and `find::controller_facade::findController*`
//! functions remain for backward compatibility while the TS bridge migrates.

use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::find::find_store as controller;

// ── FindSession ─────────────────────────────────────────────────

#[wasm_bindgen]
pub struct FindSession;

#[wasm_bindgen]
impl FindSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        FindSession
    }

    // ── Toolbar / lifecycle ─────────────────────────────────

    /// Open the find toolbar.
    #[wasm_bindgen(js_name = "open")]
    pub fn open(&self, current_page: u16, page_count: u16, path: String) -> JsValue {
        to_value(&controller::open_find(current_page, page_count, path)).unwrap_or(JsValue::NULL)
    }

    /// Close the find toolbar.
    #[wasm_bindgen(js_name = "close")]
    pub fn close(&self) -> JsValue {
        to_value(&controller::close_find()).unwrap_or(JsValue::NULL)
    }

    /// Toggle the find toolbar open/closed.
    #[wasm_bindgen(js_name = "toggle")]
    pub fn toggle(&self, current_page: u16, page_count: u16, path: String) -> JsValue {
        to_value(&controller::toggle_find(current_page, page_count, path)).unwrap_or(JsValue::NULL)
    }

    /// Clear the current search result (retains toolbar open state).
    #[wasm_bindgen(js_name = "clear")]
    pub fn clear(&self) -> JsValue {
        to_value(&controller::clear_search()).unwrap_or(JsValue::NULL)
    }

    /// Notify the controller of a page change.
    #[wasm_bindgen(js_name = "setCurrentPage")]
    pub fn set_current_page(&self, page: u16) -> JsValue {
        to_value(&controller::set_current_page(page)).unwrap_or(JsValue::NULL)
    }

    /// Current session state (Closed / Open / Searching / Active).
    ///
    /// See `FindSessionState` in `find::find_store` for semantics. The
    /// value is derived from live controller data on every call — it is
    /// always in sync with the controller state, never stale.
    #[wasm_bindgen(js_name = "readState")]
    pub fn read_state(&self) -> JsValue {
        to_value(&controller::read_find_state()).unwrap_or(JsValue::NULL)
    }
}

impl Default for FindSession {
    fn default() -> Self {
        Self::new()
    }
}
