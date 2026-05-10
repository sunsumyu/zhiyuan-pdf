//! FindSession — P2 struct-based WASM API for in-document search.
//!
//! Mirrors the P0 `EditorSession` / P1 `DocumentSession` pattern:
//!   - Zero-sized struct as handle.
//!   - `#[wasm_bindgen]` methods with camelCase `js_name`.
//!   - Thin delegation to `find::find_store` and `find::host_find_store`.
//!   - Inline `FindError` / `FindResponse<T>` for `NotImplemented` stubs.
//!
//! The legacy `find::facade::findFacade*` and `find::controller_facade::findController*`
//! functions remain for backward compatibility while the TS bridge migrates.

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::find::find_store as controller;
use crate::find::host_find_store::{
    clear_find_session, get_find_session, move_find_match, set_find_session, HostFindScope,
};

// ── FindError / FindResponse ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FindError {
    NotImplemented { method: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<FindError>,
}

fn err_not_implemented(method: &str) -> JsValue {
    let resp: FindResponse<()> = FindResponse {
        ok: false,
        data: None,
        error: Some(FindError::NotImplemented { method: method.into() }),
    };
    to_value(&resp).unwrap_or(JsValue::NULL)
}

fn parse_scope(scope: &str) -> HostFindScope {
    match scope {
        "document" => HostFindScope::Document,
        _ => HostFindScope::Page,
    }
}

fn parse_controller_scope(scope: &str) -> controller::FindScope {
    match scope {
        "document" => controller::FindScope::Document,
        _ => controller::FindScope::Page,
    }
}

// ── FindSession ─────────────────────────────────────────────────

#[wasm_bindgen]
pub struct FindSession;

#[wasm_bindgen]
impl FindSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        FindSession
    }

    // ── Session state (low-level) ───────────────────────────────

    /// Clear any active find session.
    #[wasm_bindgen(js_name = "clearSession")]
    pub fn clear_session(&self) {
        clear_find_session();
    }

    /// Read the current find session state.
    #[wasm_bindgen(js_name = "readSession")]
    pub fn read_session(&self) -> JsValue {
        to_value(&get_find_session()).unwrap_or(JsValue::NULL)
    }

    /// Set the active find session with query + scope + match pages.
    #[wasm_bindgen(js_name = "setSession")]
    pub fn set_session(
        &self,
        query: String,
        scope: String,
        match_pages_js: JsValue,
        preferred_active_page: Option<u16>,
    ) -> JsValue {
        let match_pages: Vec<u16> = from_value(match_pages_js).unwrap_or_default();
        to_value(&set_find_session(
            query,
            parse_scope(&scope),
            match_pages,
            preferred_active_page,
        ))
        .unwrap_or(JsValue::NULL)
    }

    /// Step the active match index by `step` (positive = forward, negative = backward).
    #[wasm_bindgen(js_name = "moveMatch")]
    pub fn move_match(&self, step: i32) -> JsValue {
        to_value(&move_find_match(step)).unwrap_or(JsValue::NULL)
    }

    // ── Toolbar / lifecycle (high-level) ────────────────────────

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

    /// Update the search result and recompute toolbar state.
    #[wasm_bindgen(js_name = "setResult")]
    pub fn set_result(&self, result_js: JsValue, scope: String, current_page: u16) -> JsValue {
        let result: controller::SearchResult = from_value(result_js).unwrap_or_default();
        to_value(&controller::set_search_result(
            result,
            parse_controller_scope(&scope),
            current_page,
        ))
        .unwrap_or(JsValue::NULL)
    }

    /// Clear the current search result (retains toolbar open state).
    #[wasm_bindgen(js_name = "clear")]
    pub fn clear(&self) -> JsValue {
        to_value(&controller::clear_search()).unwrap_or(JsValue::NULL)
    }

    /// Move the active match by `step` (toolbar-aware variant).
    #[wasm_bindgen(js_name = "moveActive")]
    pub fn move_active(&self, step: i32) -> JsValue {
        to_value(&controller::move_active(step)).unwrap_or(JsValue::NULL)
    }

    /// Notify the controller of a page change.
    #[wasm_bindgen(js_name = "setCurrentPage")]
    pub fn set_current_page(&self, page: u16) -> JsValue {
        to_value(&controller::set_current_page(page)).unwrap_or(JsValue::NULL)
    }

    /// Read the current toolbar UI state.
    #[wasm_bindgen(js_name = "getToolbarState")]
    pub fn get_toolbar_state(&self) -> JsValue {
        to_value(&controller::get_toolbar_state()).unwrap_or(JsValue::NULL)
    }

    /// Compute replace requests for the current scope (does not apply them).
    #[wasm_bindgen(js_name = "getReplaceRequests")]
    pub fn get_replace_requests(
        &self,
        replacement: String,
        replace_all: bool,
        scope: String,
    ) -> JsValue {
        to_value(&controller::get_replace_requests(
            &replacement,
            replace_all,
            parse_controller_scope(&scope),
        ))
        .unwrap_or(JsValue::NULL)
    }

    // ── Reserved stubs ──────────────────────────────────────────

    #[wasm_bindgen(js_name = "setOptions")]
    pub fn set_options(
        &self,
        _case_sensitive: bool,
        _whole_word: bool,
        _regex: bool,
    ) -> JsValue {
        err_not_implemented("find.setOptions")
    }

    #[wasm_bindgen(js_name = "replaceCurrent")]
    pub fn replace_current(&self, _replacement: String) -> JsValue {
        err_not_implemented("find.replaceCurrent")
    }

    #[wasm_bindgen(js_name = "replaceAll")]
    pub fn replace_all(&self, _replacement: String) -> JsValue {
        err_not_implemented("find.replaceAll")
    }

    #[wasm_bindgen(js_name = "highlightAll")]
    pub fn highlight_all(&self, _enabled: bool) -> JsValue {
        err_not_implemented("find.highlightAll")
    }
}

impl Default for FindSession {
    fn default() -> Self {
        Self::new()
    }
}
