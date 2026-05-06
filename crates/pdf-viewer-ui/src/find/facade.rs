// ─────────────────────────────────────────────────────────────────────────────
// Find facade — frozen v1 API surface for in-document search.
//
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::viewer::find::{
    clear_find_session as host_clear,
    get_find_session as host_get,
    move_find_match as host_move,
    set_find_session as host_set,
    HostFindScope,
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

/// Clear any active find session.
#[wasm_bindgen(js_name = "findFacadeClearSession")]
pub fn facade_clear_session() {
    host_clear();
}

/// Read the current find session state.
#[wasm_bindgen(js_name = "findFacadeReadSession")]
pub fn facade_read_session() -> JsValue {
    to_value(&host_get()).unwrap_or(JsValue::NULL)
}

/// Set the active find session with query + scope + match pages.
#[wasm_bindgen(js_name = "findFacadeSetSession")]
pub fn facade_set_session(
    query: String,
    scope: String,
    match_pages_js: JsValue,
    preferred_active_page: Option<u16>,
) -> JsValue {
    let match_pages: Vec<u16> = from_value(match_pages_js).unwrap_or_default();
    let scope = match scope.as_str() {
        "document" => HostFindScope::Document,
        _ => HostFindScope::Page,
    };
    to_value(&host_set(query, scope, match_pages, preferred_active_page))
        .unwrap_or(JsValue::NULL)
}

/// Step the active match index by `step` (positive = forward, negative = backward).
#[wasm_bindgen(js_name = "findFacadeMoveMatch")]
pub fn facade_move_match(step: i32) -> JsValue {
    to_value(&host_move(step)).unwrap_or(JsValue::NULL)
}

// ─── Stubs ───────────────────────────────────────────────────────────────────

/// Reserved: configure search options (case-sensitive / regex / whole-word).
#[wasm_bindgen(js_name = "findFacadeSetOptions")]
pub fn facade_set_options(
    _case_sensitive: bool,
    _whole_word: bool,
    _regex: bool,
) -> JsValue {
    stub("find.setOptions")
}

/// Reserved: replace the currently active match.
#[wasm_bindgen(js_name = "findFacadeReplaceCurrent")]
pub fn facade_replace_current(_replacement: String) -> JsValue {
    stub("find.replaceCurrent")
}

/// Reserved: replace all matches in the current scope.
#[wasm_bindgen(js_name = "findFacadeReplaceAll")]
pub fn facade_replace_all(_replacement: String) -> JsValue {
    stub("find.replaceAll")
}

/// Reserved: highlight all matches without changing the active match.
#[wasm_bindgen(js_name = "findFacadeHighlightAll")]
pub fn facade_highlight_all(_enabled: bool) -> JsValue {
    stub("find.highlightAll")
}
