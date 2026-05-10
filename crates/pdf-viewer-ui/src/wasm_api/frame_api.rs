//! Frame plan, present-layer decision, frame cache, render-frame lifecycle,
//! and viewport layout WASM bindings.
//!
//! Extracted from `wasm_api/viewer.rs` as part of the P1 refactor.

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::host::layout::SyncHostLayoutRequest;
use crate::host::{layout, scroll};
use crate::present::facade::{ViewportLayoutRequest, ViewportTileRequest};
use crate::present::plan_builder::FramePlanRequest;
use crate::present::{facade as present_facade, present_store};
use crate::render::{host_runtime, layer, loop_workflow};

#[wasm_bindgen]
pub fn resolve_render_follow_up(
    rendered_display_zoom: f32,
    current_target_zoom: f32,
) -> JsValue {
    to_value(&loop_workflow::resolve_render_follow_up_runtime(
        rendered_display_zoom,
        current_target_zoom,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn schedule_render_follow_up(
    rendered_display_zoom: f32,
    request_js: JsValue,
) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&loop_workflow::schedule_render_follow_up_runtime(
        rendered_display_zoom,
        &request,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_layer_execution_plan(
    bundle_changed: bool,
    frame_plan_js: JsValue,
) -> JsValue {
    let frame_plan: crate::present::plan_builder::FramePlanResult =
        from_value(frame_plan_js).unwrap_or_default();
    to_value(&layer::resolve_layer_execution_plan(
        bundle_changed,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_render_execution_plan(
    bundle_changed: bool,
    frame_plan_js: JsValue,
) -> JsValue {
    let frame_plan: crate::present::plan_builder::FramePlanResult =
        from_value(frame_plan_js).unwrap_or_default();
    to_value(&layer::resolve_render_execution_plan(
        bundle_changed,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_layer_present_decision(
    use_detail_layer: bool,
    frame_plan_js: JsValue,
) -> JsValue {
    let frame_plan: crate::present::plan_builder::FramePlanResult =
        from_value(frame_plan_js).unwrap_or_default();
    to_value(&layer::resolve_layer_present_decision(
        use_detail_layer,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&present_facade::resolve_frame_plan(&request, false))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_viewport_refresh(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&present_store::resolve_viewport_refresh(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_host_scroll_refresh(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&scroll::resolve_host_scroll_refresh(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn touch_frame_cache_entry(is_detail: bool, key: String) -> bool {
    present_store::touch_frame_cache_entry(is_detail, &key)
}

#[wasm_bindgen]
pub fn store_frame_cache_entry(is_detail: bool, key: String) -> JsValue {
    let result = present_store::store_frame_cache_entry(is_detail, key);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reset_frame_cache() {
    present_store::reset_frame_cache();
}

#[wasm_bindgen]
pub fn take_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&present_facade::resolve_frame_plan(&request, true))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn begin_render_frame(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&present_store::schedule_render_frame_request(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn schedule_render_frame(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    let envelope = present_store::schedule_render_frame_request(&request);
    to_value(&envelope).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn is_render_frame_current(frame_token: u32) -> bool {
    present_store::is_render_frame_current(frame_token)
}

#[wasm_bindgen]
pub fn commit_render_frame(frame_token: u32, rendered_zoom: f32) -> bool {
    present_store::commit_render_frame(frame_token, rendered_zoom)
}

#[wasm_bindgen]
pub fn settle_render_frame(frame_token: u32, rendered_zoom: f32) -> JsValue {
    let transition = present_store::settle_render_frame(frame_token, Some(rendered_zoom));
    to_value(&transition).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn queue_render_loop_frame(frame_js: JsValue) -> JsValue {
    let frame = if frame_js.is_null() || frame_js.is_undefined() {
        None
    } else {
        from_value(frame_js).ok()
    };
    to_value(&host_runtime::queue_render_loop_frame(frame)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn advance_render_loop_frame(frame_js: JsValue) -> JsValue {
    let frame = if frame_js.is_null() || frame_js.is_undefined() {
        None
    } else {
        from_value(frame_js).ok()
    };
    to_value(&host_runtime::advance_render_loop_frame(frame)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn abort_render_frame(frame_token: u32) -> JsValue {
    let transition = present_store::settle_render_frame(frame_token, None);
    to_value(&transition).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_viewport_layout(request_js: JsValue) -> JsValue {
    let request: ViewportLayoutRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&present_facade::resolve_viewport_layout(&request))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn sync_host_layout(request_js: JsValue) -> JsValue {
    let request: SyncHostLayoutRequest = from_value(request_js.clone()).unwrap_or_default();
    log::info!(
        "[PAGE-SIZE] wasm_sync_host_layout called. Width={}, Height={}",
        request.page_width,
        request.page_height
    );
    let request: SyncHostLayoutRequest = from_value(request_js).unwrap_or_default();
    to_value(&layout::sync_host_layout(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_viewport_tile(request_js: JsValue) -> JsValue {
    let request: ViewportTileRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&present_facade::resolve_viewport_tile(&request))
        .unwrap_or(JsValue::NULL)
}
