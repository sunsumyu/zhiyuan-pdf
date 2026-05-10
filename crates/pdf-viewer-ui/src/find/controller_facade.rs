// ─────────────────────────────────────────────────────────────────────────────
// Find Controller facade — WASM bindings for the find controller.
// Moves search orchestration logic from TS into Rust.
// ─────────────────────────────────────────────────────────────────────────────

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use super::find_store as controller;

#[wasm_bindgen(js_name = "findControllerOpen")]
pub fn facade_open(current_page: u16, page_count: u16, path: String) -> JsValue {
    to_value(&controller::open_find(current_page, page_count, path)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "findControllerClose")]
pub fn facade_close() -> JsValue {
    to_value(&controller::close_find()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "findControllerToggle")]
pub fn facade_toggle(current_page: u16, page_count: u16, path: String) -> JsValue {
    to_value(&controller::toggle_find(current_page, page_count, path)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "findControllerSetResult")]
pub fn facade_set_result(result_js: JsValue, scope_js: String, current_page: u16) -> JsValue {
    let result: controller::SearchResult = from_value(result_js).unwrap_or_default();
    let scope = match scope_js.as_str() {
        "document" => controller::FindScope::Document,
        _ => controller::FindScope::Page,
    };
    to_value(&controller::set_search_result(result, scope, current_page)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "findControllerClear")]
pub fn facade_clear() -> JsValue {
    to_value(&controller::clear_search()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "findControllerMoveActive")]
pub fn facade_move_active(step: i32) -> JsValue {
    to_value(&controller::move_active(step)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "findControllerSetCurrentPage")]
pub fn facade_set_current_page(page: u16) -> JsValue {
    to_value(&controller::set_current_page(page)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "findControllerGetToolbarState")]
pub fn facade_get_toolbar_state() -> JsValue {
    to_value(&controller::get_toolbar_state()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "findControllerGetReplaceRequests")]
pub fn facade_get_replace_requests(replacement: String, replace_all: bool, scope_js: String) -> JsValue {
    let scope = match scope_js.as_str() {
        "document" => controller::FindScope::Document,
        _ => controller::FindScope::Page,
    };
    to_value(&controller::get_replace_requests(&replacement, replace_all, scope)).unwrap_or(JsValue::NULL)
}
