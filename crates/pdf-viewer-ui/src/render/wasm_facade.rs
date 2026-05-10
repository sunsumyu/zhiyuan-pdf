// ─────────────────────────────────────────────────────────────────────────────
// Render facade — frozen v1 wasm API surface for the progressive render pipeline.
//
// Note: existing `wasm_api/viewer.rs` exposes raw render entrypoints under
// `start_progressive_render` / `step_progressive_render` / etc. This file
// re-exports them under canonical `renderFacade*` js_names so the frontend has
// a single, stable namespace for render operations.
//
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::render::progressive_workflow::{
    cancel_progressive_render,
    render_page,
    start_progressive_render,
    step_progressive_render,
};
use crate::render::commit::commit_render_result;
use crate::present::present_store::{
    is_render_frame_current,
    reset_frame_cache,
    settle_render_frame,
    store_frame_cache_entry,
    touch_frame_cache_entry,
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

// ─── Stable — progressive render lifecycle ───────────────────────────────────

#[wasm_bindgen(js_name = "renderFacadeStartProgressive")]
pub fn facade_start_progressive() -> JsValue {
    to_value(&start_progressive_render()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeStepProgressive")]
pub fn facade_step_progressive(
    canvas_id: String,
    image_cache: JsValue,
    budget_ms: f64,
    max_items: u32,
) -> JsValue {
    to_value(&step_progressive_render(canvas_id, image_cache, budget_ms, max_items))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeCancelProgressive")]
pub fn facade_cancel_progressive() {
    cancel_progressive_render();
}

#[wasm_bindgen(js_name = "renderFacadeRenderPage")]
pub fn facade_render_page(canvas_id: String, image_cache: JsValue) {
    render_page(canvas_id, image_cache);
}

// ─── Stable — frame commit / settle ──────────────────────────────────────────

#[wasm_bindgen(js_name = "renderFacadeCommitResult")]
pub fn facade_commit_result(
    frame_token: u32,
    rendered_zoom: f32,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    to_value(&commit_render_result(frame_token, rendered_zoom, page_width, page_height))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeAbortFrame")]
pub fn facade_abort_frame(frame_token: u32) -> JsValue {
    let transition = settle_render_frame(frame_token, None);
    to_value(&transition).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeIsFrameCurrent")]
pub fn facade_is_frame_current(frame_token: u32) -> bool {
    is_render_frame_current(frame_token)
}

// ─── Stable — frame cache ────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "renderFacadeTouchCache")]
pub fn facade_touch_cache(is_detail: bool, key: String) -> bool {
    touch_frame_cache_entry(is_detail, &key)
}

#[wasm_bindgen(js_name = "renderFacadeStoreCache")]
pub fn facade_store_cache(is_detail: bool, key: String) -> JsValue {
    to_value(&store_frame_cache_entry(is_detail, key)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeResetCache")]
pub fn facade_reset_cache() {
    reset_frame_cache();
}

// ─── Stubs ───────────────────────────────────────────────────────────────────

/// Reserved: render the current page off-screen and return a PNG buffer.
#[wasm_bindgen(js_name = "renderFacadeSnapshotPng")]
pub fn facade_snapshot_png(_dpi: f32) -> JsValue {
    stub("render.snapshotPng")
}

/// Reserved: pre-warm the frame cache for an upcoming page.
#[wasm_bindgen(js_name = "renderFacadePrewarmCache")]
pub fn facade_prewarm_cache(_page_index: u16) -> JsValue {
    stub("render.prewarmCache")
}

/// Reserved: configure rendering quality presets (draft / normal / high).
#[wasm_bindgen(js_name = "renderFacadeSetQuality")]
pub fn facade_set_quality(_preset: String) -> JsValue {
    stub("render.setQuality")
}

/// Reserved: enable/disable debug overlay (tile boundaries, frame tokens).
#[wasm_bindgen(js_name = "renderFacadeSetDebugOverlay")]
pub fn facade_set_debug_overlay(_enabled: bool) -> JsValue {
    stub("render.setDebugOverlay")
}
