// ─────────────────────────────────────────────────────────────────────────────
// Zoom facade — frozen v1 API surface for zoom state / runtime.
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::zoom::runtime::{
    get_zoom_state as host_get_zoom_state,
    mark_rendered_zoom as host_mark_rendered_zoom,
    reset_zoom_runtime as host_reset_zoom_runtime,
    set_target_zoom as host_set_target_zoom,
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

/// Read the current zoom state (target / rendered / preview / settled).
#[wasm_bindgen(js_name = "zoomFacadeReadState")]
pub fn facade_read_state() -> JsValue {
    to_value(&host_get_zoom_state()).unwrap_or(JsValue::NULL)
}

/// Reset the zoom runtime to the supplied initial zoom value.
#[wasm_bindgen(js_name = "zoomFacadeReset")]
pub fn facade_reset(initial_zoom: f32) {
    host_reset_zoom_runtime(initial_zoom);
}

/// Set the target zoom multiplier (drives the zoom request pipeline).
#[wasm_bindgen(js_name = "zoomFacadeSetTarget")]
pub fn facade_set_target(target_zoom: f32) {
    host_set_target_zoom(target_zoom);
}

/// Mark the zoom that was actually rendered (the renderer reports back).
#[wasm_bindgen(js_name = "zoomFacadeMarkRendered")]
pub fn facade_mark_rendered(rendered_zoom: f32) {
    host_mark_rendered_zoom(rendered_zoom);
}

// ─── Stubs ───────────────────────────────────────────────────────────────────

/// Reserved: animate from current zoom to `target` over `duration_ms`.
#[wasm_bindgen(js_name = "zoomFacadeAnimateTo")]
pub fn facade_animate_to(_target: f32, _duration_ms: u32) -> JsValue {
    stub("zoom.animateTo")
}

/// Reserved: zoom to fit the page in the viewport.
#[wasm_bindgen(js_name = "zoomFacadeFitPage")]
pub fn facade_fit_page() -> JsValue {
    stub("zoom.fitPage")
}

/// Reserved: zoom to fit the page width.
#[wasm_bindgen(js_name = "zoomFacadeFitWidth")]
pub fn facade_fit_width() -> JsValue {
    stub("zoom.fitWidth")
}

/// Reserved: zoom to fit the actual size (1:1 device pixels).
#[wasm_bindgen(js_name = "zoomFacadeActualSize")]
pub fn facade_actual_size() -> JsValue {
    stub("zoom.actualSize")
}

/// Reserved: zoom centered at a specific viewport point.
#[wasm_bindgen(js_name = "zoomFacadeZoomAtPoint")]
pub fn facade_zoom_at_point(_target: f32, _client_x: f32, _client_y: f32) -> JsValue {
    stub("zoom.zoomAtPoint")
}
