use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::presentation::page_turn::{
    admit_page_asset, can_prefetch, decide_adjacent_prefetch, is_latest_turn,
    mark_page_visible, request_page_turn, reset_state,
};
use crate::presentation::render_queue::resolve_queue_action;

#[wasm_bindgen]
pub struct PagePresentationRuntime;

#[wasm_bindgen]
impl PagePresentationRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new() -> PagePresentationRuntime {
        PagePresentationRuntime
    }

    #[wasm_bindgen(js_name = "requestPageTurn")]
    pub fn request_page_turn(&self, target_page: u16, reason: String, now_ms: f64) -> JsValue {
        to_value(&request_page_turn(target_page, reason, now_ms)).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "readPageTurn")]
    pub fn read_page_turn(&self) -> JsValue {
        to_value(&crate::presentation::page_turn::read_snapshot()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "isLatestPageTurn")]
    pub fn is_latest_page_turn(&self, page_turn_id: u32, page_index: u16) -> bool {
        is_latest_turn(page_turn_id, page_index)
    }

    #[wasm_bindgen(js_name = "markPageVisible")]
    pub fn mark_page_visible(&self, page_index: u16, surface: String) -> JsValue {
        to_value(&mark_page_visible(page_index, surface)).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "canPrefetch")]
    pub fn can_prefetch(&self, page_index: u16) -> bool {
        can_prefetch(page_index)
    }

    #[wasm_bindgen(js_name = "admitPageAsset")]
    pub fn admit_page_asset(&self, page_index: u16, role: String, asset_kind: String) -> JsValue {
        to_value(&admit_page_asset(page_index, role, asset_kind)).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "decideAdjacentPrefetch")]
    pub fn decide_adjacent_prefetch(&self, anchor_page: u16, page_count: u16) -> JsValue {
        to_value(&decide_adjacent_prefetch(anchor_page, page_count)).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "resolveRenderQueueAction")]
    pub fn resolve_render_queue_action(
        &self,
        source: String,
        executing: bool,
        now_ms: f64,
        last_commit_ms: f64,
    ) -> JsValue {
        to_value(&resolve_queue_action(
            source,
            executing,
            now_ms,
            last_commit_ms,
        ))
        .unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "reset")]
    pub fn reset(&self) {
        reset_state();
    }
}

impl Default for PagePresentationRuntime {
    fn default() -> Self {
        Self::new()
    }
}
