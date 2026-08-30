//! Zoom-domain free wasm exports retained for the TS bridge.
//!
//! 这些函数原位于 `wasm_api/zoom_api.rs`（已删除）。新代码请走 `ZoomController`。

use serde_wasm_bindgen::from_value;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::viewer::viewer_controller;
use pdf_viewer_core::render::zoom::animation::WheelZoomRequest;
use crate::zoom::zoom_controller;

#[wasm_bindgen(js_name = "resolveWheelZoom")]
pub fn resolve_wheel_zoom(request_js: JsValue) -> JsValue {
    let request: WheelZoomRequest = from_value(request_js).unwrap_or_default();
    let result = zoom_controller::resolve_wheel_zoom(&request);
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

#[wasm_bindgen(js_name = "clearPreviewPresent")]
pub fn clear_preview_present() {
    zoom_controller::clear_preview_present();
}

#[wasm_bindgen(js_name = "cancelDrawingDelay")]
pub fn cancel_drawing_delay() {
    zoom_controller::cancel_drawing_delay();
}

#[wasm_bindgen(js_name = "takeCancelPendingRender")]
pub fn take_cancel_pending_render() -> bool {
    zoom_controller::take_cancel_pending_render()
}

#[wasm_bindgen(js_name = "syncHostLayout")]
pub fn sync_host_layout_wasm(request_js: JsValue) -> JsValue {
    let request: crate::host::layout::SyncHostLayoutRequest = from_value(request_js).unwrap_or_default();
    let result = crate::host::layout::sync_host_layout(request);
    to_value(&result).unwrap_or(JsValue::NULL)
}

/// Single-read snapshot of all zoom state fields the TS bridge needs.
/// Avoids multiple WASM round-trips that could see stale intermediate values.
///
/// `currentZoom` 是权威派生投影：恒等于 targetZoom（ADR-0001）。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoomSnapshot {
    pub current_zoom: f32,
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    pub css_scale: f32,
    pub preview_active: bool,
    pub wheel_render_pending: bool,
}

#[wasm_bindgen(js_name = "readZoomSnapshot")]
pub fn read_zoom_snapshot() -> JsValue {
    let state = zoom_controller::read_zoom_state();
    let snapshot = ZoomSnapshot {
        current_zoom: state.target_zoom,
        target_zoom: state.target_zoom,
        visual_zoom: state.visual_zoom,
        last_rendered_zoom: state.last_rendered_zoom,
        css_scale: state.css_scale,
        preview_active: zoom_controller::is_preview_active(),
        wheel_render_pending: zoom_controller::is_wheel_render_pending(),
    };
    to_value(&snapshot).unwrap_or(JsValue::NULL)
}

/// Zoom state machine: one call per RAF frame.
/// TS passes DOM reads, receives DomOps + AsyncOps to execute.
#[wasm_bindgen(js_name = "tickZoomState")]
pub fn tick_zoom_state(input_js: JsValue) -> JsValue {
    let input: pdf_viewer_core::render::zoom_host::ZoomTickInput =
        from_value(input_js).unwrap_or_default();
    let output = zoom_controller::tick_zoom_state(input);
    to_value(&output).unwrap_or(JsValue::NULL)
}

// ─── RAF loop API (new) ───────────────────────────────────────────

/// Start the Rust-driven zoom RAF loop.
#[wasm_bindgen(js_name = "startZoomRafLoop")]
pub fn start_zoom_raf_loop() {
    crate::zoom::raf_loop::start_zoom_raf_loop();
}

/// Stop the Rust-driven zoom RAF loop.
#[wasm_bindgen(js_name = "stopZoomRafLoop")]
pub fn stop_zoom_raf_loop() {
    crate::zoom::raf_loop::stop_zoom_raf_loop();
}

/// Check if the RAF loop is running.
#[wasm_bindgen(js_name = "isZoomRafLoopRunning")]
pub fn is_zoom_raf_loop_running() -> bool {
    crate::zoom::raf_loop::is_raf_loop_running()
}

/// Handle a complete wheel event. TS passes raw DOM values.
#[wasm_bindgen(js_name = "onWheelEvent")]
pub fn on_wheel_event(input_js: JsValue) -> JsValue {
    let input: crate::zoom::raf_loop::WheelEventInput =
        from_value(input_js).unwrap_or_default();
    let output = crate::zoom::raf_loop::on_wheel_event(input);
    // The loop self-stops after settle; every real wheel gesture must
    // guarantee it is ticking again, otherwise zoom state changes are
    // never rendered.
    crate::zoom::raf_loop::ensure_raf_loop_after_wheel();
    to_value(&output).unwrap_or(JsValue::NULL)
}

/// Push a committed frame from the render pipeline.
#[wasm_bindgen(js_name = "commitRenderedFrameToQueue")]
pub fn commit_rendered_frame_to_queue(frame_js: JsValue) {
    if let Ok(frame) = from_value::<crate::zoom::raf_loop::CommittedFrame>(frame_js) {
        crate::zoom::raf_loop::commit_rendered_frame(frame);
    }
}

