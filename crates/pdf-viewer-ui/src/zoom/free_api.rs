//! Zoom-domain free wasm exports retained for the TS bridge.
//!
//! 这些函数原位于 `wasm_api/zoom_api.rs`（已删除）。新代码请走 `ZoomController`。

use serde_wasm_bindgen::from_value;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::viewer::viewer_controller;
use crate::zoom::interaction::WheelZoomRequest;
use crate::zoom::{request, zoom_controller};

#[wasm_bindgen(js_name = "resolveWheelZoom")]
pub fn resolve_wheel_zoom(request_js: JsValue) -> JsValue {
    let request: WheelZoomRequest = from_value(request_js).unwrap_or_default();
    let result = request::resolve_wheel_zoom(&request);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resetZoomState")]
pub fn reset_zoom_state(initial_zoom: f32) {
    viewer_controller::reset_zoom_view(initial_zoom);
}

#[wasm_bindgen(js_name = "readZoomState")]
pub fn read_zoom_state() -> JsValue {
    to_value(&zoom_controller::read_zoom_state()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "getZoomState")]
#[deprecated(since = "0.2.0", note = "Use read_zoom_state instead")]
pub fn get_zoom_state() -> JsValue {
    read_zoom_state()
}

#[wasm_bindgen(js_name = "setTargetZoom")]
pub fn set_target_zoom(target_zoom: f32) {
    zoom_controller::set_target_zoom(target_zoom);
}

#[wasm_bindgen(js_name = "markRenderedZoom")]
pub fn mark_rendered_zoom(rendered_zoom: f32) {
    zoom_controller::mark_rendered_zoom(rendered_zoom);
}

#[wasm_bindgen(js_name = "clearPendingAnchor")]
pub fn clear_pending_anchor() {
    zoom_controller::clear_pending_anchor();
}

#[wasm_bindgen(js_name = "applyZoomSelection")]
pub fn apply_zoom_selection(zoom: f32) -> JsValue {
    let result = crate::host::command::apply_zoom_selection(zoom);
    to_value(&result).unwrap_or(JsValue::NULL)
}
