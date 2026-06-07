//! Render-domain free wasm exports for the TS bridge.
//!
//! The TS `render_wasm_api.ts` adapter calls these free functions via
//! `getWasmApi().schedule_render_frame?.(request)` etc.
//! Previously these lived in a deleted `wasm_api/render_api.rs`.
//! Re-created here to restore the render pipeline.

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::page::page_store::update_page_viewport as inner_update_page_viewport;
use crate::present::plan_builder::FramePlanRequest;
use crate::present::plan_builder::FramePlanResult;
use crate::present::present_store::{
    build_frame_plan_result, is_render_frame_current as inner_is_render_frame_current,
    reset_frame_cache as inner_reset_frame_cache,
    resolve_viewport_refresh as inner_resolve_viewport_refresh, schedule_render_frame_request,
    settle_render_frame as inner_settle_render_frame,
    store_frame_cache_entry as inner_store_frame_cache_entry,
    touch_frame_cache_entry as inner_touch_frame_cache_entry,
};
use crate::render::commit::commit_render_result as inner_commit_render_result;
use crate::render::facade::resolve_progressive_render_policy_request;
use crate::render::host_runtime::{
    advance_render_loop_frame as inner_advance_render_loop_frame,
    queue_render_loop_frame as inner_queue_render_loop_frame,
};
use crate::render::layer::{
    resolve_layer_execution_plan as inner_resolve_layer_execution_plan,
    resolve_layer_present_decision as inner_resolve_layer_present_decision,
    resolve_render_execution_plan as inner_resolve_render_execution_plan,
};
use crate::render::loop_workflow::schedule_render_follow_up_runtime;
use crate::render::progressive_workflow::{
    cancel_progressive_render as inner_cancel_progressive_render, render_page as inner_render_page,
    render_page_offscreen as inner_render_page_offscreen,
    start_progressive_render as inner_start_progressive_render,
    step_progressive_render as inner_step_progressive_render,
    step_progressive_render_offscreen as inner_step_progressive_render_offscreen,
};
use crate::render::workflow::RenderFrameEnvelope;
use crate::zoom::event::{
    execute_wheel_zoom as inner_handle_wheel_zoom_host,
    step_preview_host as inner_step_preview_host, PreviewHostStepRequest, WheelZoomHostRequest,
};
use crate::zoom::host::{
    resolve_preview_tick_decision as inner_resolve_preview_tick_decision,
    resolve_wheel_render_decision as inner_resolve_wheel_render_decision,
    PreviewTickDecisionRequest, WheelRenderDecisionRequest,
};
use crate::zoom::preview_host::{
    clear_zoom_preview_host_state as inner_clear_zoom_preview_host_state,
    is_wheel_render_pending as inner_get_wheel_render_pending,
    queue_committed_frame as inner_queue_committed_frame,
    set_wheel_render_pending as inner_set_wheel_render_pending,
    take_ready_committed_frame as inner_take_ready_committed_frame,
};
use crate::zoom::zoom_controller::step_zoom_frame_plan as inner_step_zoom_frame_plan;
use crate::zoom::zoom_store::PendingCommittedFrame;

// ─── Frame plan ─────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn resolve_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&build_frame_plan_result(&request, false)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn take_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&build_frame_plan_result(&request, true)).unwrap_or(JsValue::NULL)
}

// ─── Schedule / commit / settle ─────────────────────────────────────────────

#[wasm_bindgen]
pub fn schedule_render_frame(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    match schedule_render_frame_request(&request) {
        Some(frame) => to_value(&frame).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}

#[wasm_bindgen]
pub fn commit_render_result(
    frame_token: u32,
    rendered_zoom: f32,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    to_value(&inner_commit_render_result(
        frame_token,
        rendered_zoom,
        page_width,
        page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn settle_render_frame(frame_token: u32, rendered_zoom: f32) -> JsValue {
    to_value(&inner_settle_render_frame(frame_token, Some(rendered_zoom))).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn abort_render_frame(frame_token: u32) -> JsValue {
    to_value(&inner_settle_render_frame(frame_token, None)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn is_render_frame_current(frame_token: u32) -> bool {
    inner_is_render_frame_current(frame_token)
}

#[wasm_bindgen]
pub fn schedule_render_follow_up(rendered_display_zoom: f32, request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    match schedule_render_follow_up_runtime(rendered_display_zoom, &request) {
        Some(frame) => to_value(&frame).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}

// ─── Render loop management ─────────────────────────────────────────────────

#[wasm_bindgen]
pub fn queue_render_loop_frame(frame_js: JsValue) -> JsValue {
    let frame: Option<RenderFrameEnvelope> = from_value(frame_js).ok();
    match inner_queue_render_loop_frame(frame) {
        Some(f) => to_value(&f).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}

#[wasm_bindgen]
pub fn advance_render_loop_frame(frame_js: JsValue) -> JsValue {
    let frame: Option<RenderFrameEnvelope> = from_value(frame_js).ok();
    match inner_advance_render_loop_frame(frame) {
        Some(f) => to_value(&f).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}

// ─── Zoom preview ───────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn step_zoom_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_step_zoom_frame_plan(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_viewport_refresh(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_resolve_viewport_refresh(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_host_scroll_refresh(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_resolve_viewport_refresh(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn clear_zoom_preview_host_state(clear_anchor: bool) {
    inner_clear_zoom_preview_host_state(clear_anchor);
}

#[wasm_bindgen]
pub fn resolve_wheel_render_decision(request_js: JsValue) -> JsValue {
    let request: WheelRenderDecisionRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_resolve_wheel_render_decision(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_preview_tick_decision(request_js: JsValue) -> JsValue {
    let request: PreviewTickDecisionRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_resolve_preview_tick_decision(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn handle_wheel_zoom_host(request_js: JsValue) -> JsValue {
    let request: WheelZoomHostRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_handle_wheel_zoom_host(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_preview_host(request_js: JsValue) -> JsValue {
    let request: PreviewHostStepRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_step_preview_host(request)).unwrap_or(JsValue::NULL)
}

// ─── Layer execution plan ───────────────────────────────────────────────────

#[wasm_bindgen]
pub fn resolve_render_execution_plan(bundle_changed: bool, frame_plan_js: JsValue) -> JsValue {
    let frame_plan: FramePlanResult = from_value(frame_plan_js).unwrap_or_default();
    to_value(&inner_resolve_render_execution_plan(
        bundle_changed,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_layer_execution_plan(bundle_changed: bool, frame_plan_js: JsValue) -> JsValue {
    let frame_plan: FramePlanResult = from_value(frame_plan_js).unwrap_or_default();
    to_value(&inner_resolve_layer_execution_plan(
        bundle_changed,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_layer_present_decision(use_detail_layer: bool, frame_plan_js: JsValue) -> JsValue {
    let frame_plan: FramePlanResult = from_value(frame_plan_js).unwrap_or_default();
    to_value(&inner_resolve_layer_present_decision(
        use_detail_layer,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

// ─── Page context / viewport ────────────────────────────────────────────────

#[wasm_bindgen]
pub fn update_page_viewport(
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) {
    inner_update_page_viewport(
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
}

// ─── Canvas rendering ───────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn render_page(canvas_id: String, image_cache: JsValue) {
    inner_render_page(canvas_id, image_cache);
}

#[wasm_bindgen]
pub fn render_page_offscreen(canvas_js: JsValue, image_cache: JsValue, dpr: f32) {
    inner_render_page_offscreen(canvas_js, image_cache, dpr);
}

#[wasm_bindgen]
pub fn start_progressive_render() -> JsValue {
    to_value(&inner_start_progressive_render()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_progressive_render(
    canvas_id: String,
    image_cache: JsValue,
    budget_ms: f64,
    max_items: u32,
) -> JsValue {
    to_value(&inner_step_progressive_render(
        canvas_id,
        image_cache,
        budget_ms,
        max_items,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_progressive_render_offscreen(
    canvas_js: JsValue,
    image_cache: JsValue,
    budget_ms: f64,
    max_items: u32,
    dpr: f32,
) -> JsValue {
    to_value(&inner_step_progressive_render_offscreen(
        canvas_js,
        image_cache,
        budget_ms,
        max_items,
        dpr,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn cancel_progressive_render() {
    inner_cancel_progressive_render();
}

#[wasm_bindgen]
pub fn resolve_progressive_render_policy(request_js: JsValue) -> JsValue {
    let request = from_value(request_js).unwrap_or_default();
    to_value(&resolve_progressive_render_policy_request(request)).unwrap_or(JsValue::NULL)
}

// ─── Frame cache ────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn touch_frame_cache_entry(use_viewport_tile: bool, cache_key: String) -> JsValue {
    let result = inner_touch_frame_cache_entry(use_viewport_tile, &cache_key);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn store_frame_cache_entry(use_viewport_tile: bool, cache_key: String) -> JsValue {
    to_value(&inner_store_frame_cache_entry(use_viewport_tile, cache_key)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reset_frame_cache() {
    inner_reset_frame_cache();
}

// ─── Wheel / preview host ───────────────────────────────────────────────────

#[wasm_bindgen]
pub fn set_wheel_render_pending(pending: bool) {
    inner_set_wheel_render_pending(pending);
}

#[wasm_bindgen]
pub fn get_wheel_render_pending() -> bool {
    inner_get_wheel_render_pending()
}

#[wasm_bindgen]
pub fn queue_committed_frame(frame_js: JsValue) {
    if let Ok(frame) = from_value::<PendingCommittedFrame>(frame_js) {
        inner_queue_committed_frame(&frame);
    }
}

#[wasm_bindgen]
pub fn take_ready_committed_frame() -> JsValue {
    match inner_take_ready_committed_frame() {
        Some(frame) => to_value(&frame).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}
