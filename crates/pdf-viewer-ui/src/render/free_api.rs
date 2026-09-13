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
use crate::render::host_runtime::{
    advance_render_loop_frame as inner_advance_render_loop_frame,
    queue_render_loop_frame as inner_queue_render_loop_frame,
};
use crate::render::layer::{
    resolve_layer_execution_plan as inner_resolve_layer_execution_plan,
    resolve_layer_present_decision as inner_resolve_layer_present_decision,
    resolve_render_execution_plan as inner_resolve_render_execution_plan,
};
use crate::render::progressive_workflow::{
    cancel_progressive_render as inner_cancel_progressive_render, render_page as inner_render_page,
    render_page_offscreen as inner_render_page_offscreen,
    start_progressive_render as inner_start_progressive_render,
    step_progressive_render as inner_step_progressive_render,
    step_progressive_render_offscreen as inner_step_progressive_render_offscreen,
};
use crate::render::workflow::RenderFrameEnvelope;
use crate::viewer::viewer_controller::set_zoom;
use crate::zoom::zoom_controller::read_zoom_state;
use crate::zoom::zoom_controller::step_zoom_frame_plan as inner_step_zoom_frame_plan;
use pdf_viewer_core::render::progressive::resolve_progressive_render_policy_request;
use pdf_viewer_core::render::zoom_host::resolve_render_follow_up_decision;
use pdf_viewer_core::render::zoom::{
    resolve_canvas_css_box as resolve_canvas_css_box_inner, CanvasCssBoxRequest,
};
use pdf_viewer_core::render::zoom_host::{
    is_immediate_mutation_frame as is_immediate_mutation_frame_inner,
    resolve_fit_to_width as resolve_fit_to_width_inner,
    resolve_layout_fallback as resolve_layout_fallback_inner, LayoutFallbackRequest, MAX_ZOOM,
    MIN_ZOOM,
};

// ─── Canvas CSS box ─────────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "resolveCanvasCssBox")]
pub fn resolve_canvas_css_box(request_js: JsValue) -> JsValue {
    let request: CanvasCssBoxRequest = from_value(request_js).unwrap_or_default();
    to_value(&resolve_canvas_css_box_inner(request)).unwrap_or(JsValue::NULL)
}

// ─── Frame plan ─────────────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "resolveFramePlan")]
pub fn resolve_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&build_frame_plan_result(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "takeFramePlan")]
pub fn take_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&build_frame_plan_result(&request)).unwrap_or(JsValue::NULL)
}

// ─── Schedule / commit / settle ─────────────────────────────────────────────

#[wasm_bindgen(js_name = "scheduleRenderFrame")]
pub fn schedule_render_frame(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    match schedule_render_frame_request(&request) {
        Some(frame) => to_value(&frame).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}

#[wasm_bindgen(js_name = "commitRenderResult")]
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

#[wasm_bindgen(js_name = "settleRenderFrame")]
pub fn settle_render_frame(frame_token: u32, rendered_zoom: f32) -> JsValue {
    to_value(&inner_settle_render_frame(frame_token, Some(rendered_zoom))).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "abortRenderFrame")]
pub fn abort_render_frame(frame_token: u32) -> JsValue {
    to_value(&inner_settle_render_frame(frame_token, None)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "isRenderFrameCurrent")]
pub fn is_render_frame_current(frame_token: u32) -> bool {
    inner_is_render_frame_current(frame_token)
}

#[wasm_bindgen(js_name = "scheduleRenderFollowUp")]
pub fn schedule_render_follow_up(rendered_display_zoom: f32, request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    let zoom_state = read_zoom_state();
    let decision = resolve_render_follow_up_decision(
        rendered_display_zoom,
        zoom_state.target_zoom,
        zoom_state.visual_zoom,
    );
    if !decision.schedule_latest_target {
        return JsValue::NULL;
    }
    set_zoom(decision.target_zoom);
    match schedule_render_frame_request(&request) {
        Some(frame) => to_value(&frame).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}

// ─── Render loop management ─────────────────────────────────────────────────

#[wasm_bindgen(js_name = "queueRenderLoopFrame")]
pub fn queue_render_loop_frame(frame_js: JsValue) -> JsValue {
    let frame: Option<RenderFrameEnvelope> = from_value(frame_js).ok();
    match inner_queue_render_loop_frame(frame) {
        Some(f) => to_value(&f).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}

#[wasm_bindgen(js_name = "advanceRenderLoopFrame")]
pub fn advance_render_loop_frame(frame_js: JsValue) -> JsValue {
    let frame: Option<RenderFrameEnvelope> = from_value(frame_js).ok();
    match inner_advance_render_loop_frame(frame) {
        Some(f) => to_value(&f).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    }
}

// ─── Zoom preview ───────────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "stepZoomFramePlan")]
pub fn step_zoom_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_step_zoom_frame_plan(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resolveViewportRefresh")]
pub fn resolve_viewport_refresh(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_resolve_viewport_refresh(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resolveHostScrollRefresh")]
pub fn resolve_host_scroll_refresh(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&inner_resolve_viewport_refresh(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resolveLayoutFallback")]
pub fn resolve_layout_fallback(request_js: JsValue) -> JsValue {
    let request: LayoutFallbackRequest = from_value(request_js).unwrap_or_default();
    to_value(&resolve_layout_fallback_inner(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resolveFitToWidth")]
pub fn resolve_fit_to_width(viewport_width: f32, page_width: f32) -> JsValue {
    to_value(&resolve_fit_to_width_inner(viewport_width, page_width)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "MIN_ZOOM")]
pub fn min_zoom() -> f32 {
    MIN_ZOOM
}

#[wasm_bindgen(js_name = "MAX_ZOOM")]
pub fn max_zoom() -> f32 {
    MAX_ZOOM
}

#[wasm_bindgen(js_name = "isImmediateMutationFrame")]
pub fn is_immediate_mutation_frame(render_reason: &str) -> bool {
    is_immediate_mutation_frame_inner(render_reason)
}

// ─── Layer execution plan ───────────────────────────────────────────────────

#[wasm_bindgen(js_name = "resolveRenderExecutionPlan")]
pub fn resolve_render_execution_plan(bundle_changed: bool, frame_plan_js: JsValue) -> JsValue {
    let frame_plan: FramePlanResult = from_value(frame_plan_js).unwrap_or_default();
    to_value(&inner_resolve_render_execution_plan(
        bundle_changed,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resolveLayerExecutionPlan")]
pub fn resolve_layer_execution_plan(bundle_changed: bool, frame_plan_js: JsValue) -> JsValue {
    let frame_plan: FramePlanResult = from_value(frame_plan_js).unwrap_or_default();
    to_value(&inner_resolve_layer_execution_plan(
        bundle_changed,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resolveLayerPresentDecision")]
pub fn resolve_layer_present_decision(use_detail_layer: bool, frame_plan_js: JsValue) -> JsValue {
    let frame_plan: FramePlanResult = from_value(frame_plan_js).unwrap_or_default();
    to_value(&inner_resolve_layer_present_decision(
        use_detail_layer,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

// ─── Page context / viewport ────────────────────────────────────────────────

#[wasm_bindgen(js_name = "updatePageViewport")]
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

#[wasm_bindgen(js_name = "renderPage")]
pub fn render_page(canvas_id: String, image_cache: JsValue) {
    inner_render_page(canvas_id, image_cache);
}

#[wasm_bindgen(js_name = "renderPageOffscreen")]
pub fn render_page_offscreen(canvas_js: JsValue, image_cache: JsValue, dpr: f32) {
    inner_render_page_offscreen(canvas_js, image_cache, dpr);
}

#[wasm_bindgen(js_name = "startProgressiveRender")]
pub fn start_progressive_render() -> JsValue {
    to_value(&inner_start_progressive_render()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "stepProgressiveRender")]
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

#[wasm_bindgen(js_name = "stepProgressiveRenderOffscreen")]
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

#[wasm_bindgen(js_name = "cancelProgressiveRender")]
pub fn cancel_progressive_render() {
    inner_cancel_progressive_render();
}

#[wasm_bindgen(js_name = "resolveProgressiveRenderPolicy")]
pub fn resolve_progressive_render_policy(request_js: JsValue) -> JsValue {
    let request = from_value(request_js).unwrap_or_default();
    to_value(&resolve_progressive_render_policy_request(request)).unwrap_or(JsValue::NULL)
}

// ─── Frame cache ────────────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "touchFrameCacheEntry")]
pub fn touch_frame_cache_entry(use_viewport_tile: bool, cache_key: String) -> JsValue {
    let result = inner_touch_frame_cache_entry(use_viewport_tile, &cache_key);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "storeFrameCacheEntry")]
pub fn store_frame_cache_entry(use_viewport_tile: bool, cache_key: String) -> JsValue {
    to_value(&inner_store_frame_cache_entry(use_viewport_tile, cache_key)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resetFrameCache")]
pub fn reset_frame_cache() {
    inner_reset_frame_cache();
}
