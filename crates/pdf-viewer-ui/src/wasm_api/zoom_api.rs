//! Zoom, wheel, and preview WASM bindings.
//!
//! Extracted from `wasm_api/viewer.rs` as part of the P1 refactor:
//! splits the 65-function God File into focused per-domain modules.

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::host::command;
use crate::present::facade as present_facade;
use crate::present::plan_builder::{FramePlanRequest, RenderZoomRequest};
use crate::viewer::viewer_controller;
use crate::zoom::event::{PreviewHostStepRequest, WheelZoomHostRequest};
use crate::zoom::host::{PreviewTickDecisionRequest, WheelRenderDecisionRequest};
use crate::zoom::interaction::{
    resolve_zoom_limits_result, AnchorScrollRequest, WheelZoomRequest, ZoomLimitsRequest,
};
use crate::zoom::zoom_store::PendingCommittedFrame;
use crate::zoom::{event, host as zoom_host, preview_host, request, zoom_controller};

#[wasm_bindgen]
pub fn resolve_wheel_zoom(request_js: JsValue) -> JsValue {
    let request: WheelZoomRequest = from_value(request_js).unwrap_or_default();
    let result = request::resolve_wheel_zoom(&request);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn handle_wheel_zoom_host(request_js: JsValue) -> JsValue {
    let request: WheelZoomHostRequest = from_value(request_js).unwrap_or_default();
    to_value(&event::handle_wheel_zoom_host(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_anchor_scroll(request_js: JsValue) -> JsValue {
    let request: AnchorScrollRequest = from_value(request_js).unwrap_or_default();
    let result = request::resolve_anchor_scroll(&request);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_wheel_render_decision(request_js: JsValue) -> JsValue {
    let request: WheelRenderDecisionRequest = from_value(request_js).unwrap_or_default();
    to_value(&zoom_host::resolve_wheel_render_decision(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_preview_tick_decision(request_js: JsValue) -> JsValue {
    let request: PreviewTickDecisionRequest = from_value(request_js).unwrap_or_default();
    to_value(&zoom_host::resolve_preview_tick_decision(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_preview_host(request_js: JsValue) -> JsValue {
    let request: PreviewHostStepRequest = from_value(request_js).unwrap_or_default();
    to_value(&event::step_preview_host(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_zoom_limits(request_js: JsValue) -> JsValue {
    let request: ZoomLimitsRequest = from_value(request_js).unwrap_or_default();
    to_value(&resolve_zoom_limits_result(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_render_zoom(request_js: JsValue) -> JsValue {
    let request: RenderZoomRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&present_facade::resolve_render_zoom(&request))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reset_zoom_state(initial_zoom: f32) {
    viewer_controller::reset_zoom_view(initial_zoom);
}

#[wasm_bindgen]
pub fn get_zoom_state() -> JsValue {
    to_value(&zoom_controller::get_zoom_state()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn set_target_zoom(target_zoom: f32) {
    zoom_controller::set_target_zoom(target_zoom);
}

#[wasm_bindgen]
pub fn mark_rendered_zoom(rendered_zoom: f32) {
    zoom_controller::mark_rendered_zoom(rendered_zoom);
}

#[wasm_bindgen]
pub fn reset_zoom_preview_host(target_zoom: f32) {
    preview_host::reset_zoom_preview_host(target_zoom);
}

#[wasm_bindgen]
pub fn set_wheel_render_pending(pending: bool) {
    preview_host::set_wheel_render_pending(pending);
}

#[wasm_bindgen]
pub fn get_wheel_render_pending() -> bool {
    preview_host::get_wheel_render_pending()
}

#[wasm_bindgen]
pub fn queue_committed_frame(frame_js: JsValue) {
    let frame: PendingCommittedFrame = from_value(frame_js).unwrap_or_default();
    preview_host::queue_committed_frame(&frame);
}

#[wasm_bindgen]
pub fn take_ready_committed_frame() -> JsValue {
    to_value(&preview_host::take_ready_committed_frame()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_zoom_animation() -> JsValue {
    to_value(&zoom_controller::step_zoom_animation()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_zoom_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    let preview = zoom_controller::step_zoom_frame_plan(&request);
    to_value(&preview).unwrap_or(JsValue::NULL)
}

#[allow(dead_code)]
pub(crate) fn take_pending_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> JsValue {
    to_value(&zoom_controller::take_pending_anchor_scroll(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn clear_pending_anchor() {
    zoom_controller::clear_pending_anchor();
}

#[allow(dead_code)]
pub(crate) fn set_visual_layout(display_zoom: f32, content_left: f32, content_top: f32) {
    zoom_controller::set_visual_layout(display_zoom, content_left, content_top);
}

#[wasm_bindgen]
pub fn clear_preview_present() {
    zoom_controller::clear_preview_present();
}

#[wasm_bindgen]
pub fn clear_zoom_preview_host_state(clear_pending_anchor: bool) {
    preview_host::clear_zoom_preview_host_state(clear_pending_anchor);
}

#[allow(dead_code)]
pub(crate) fn peek_pending_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> JsValue {
    to_value(&zoom_controller::peek_pending_anchor_scroll(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[allow(dead_code)]
pub(crate) fn peek_pending_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> JsValue {
    to_value(&zoom_controller::peek_pending_anchor_layout(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[allow(dead_code)]
pub(crate) fn take_pending_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> JsValue {
    to_value(&zoom_controller::take_pending_anchor_layout(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn set_current_zoom(zoom: f32) {
    viewer_controller::set_zoom(zoom);
}

#[wasm_bindgen]
pub fn apply_zoom_selection(zoom: f32) -> JsValue {
    to_value(&command::apply_zoom_selection(zoom)).unwrap_or(JsValue::NULL)
}
