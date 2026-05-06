use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::host::command::{
    apply_zoom_selection as host_apply_zoom_selection,
    navigate_next_page as host_navigate_next_page,
    navigate_prev_page as host_navigate_prev_page,
};
use crate::host::layout::{
    sync_host_layout as host_sync_host_layout, SyncHostLayoutRequest,
};
use crate::render::layer::{
    resolve_layer_execution_plan as host_resolve_layer_execution_plan,
    resolve_layer_present_decision as host_resolve_layer_present_decision,
    resolve_render_execution_plan as host_resolve_render_execution_plan,
};
use crate::page::context::{
    init_page_context_from_models as host_init_page_context_from_models,
    update_page_viewport_workflow as host_update_page_viewport_workflow,
};
use crate::present::facade::{
    resolve_render_zoom as host_resolve_render_zoom_facade,
    resolve_frame_plan as host_resolve_frame_plan_facade,
    resolve_viewport_layout as host_resolve_viewport_layout_facade,
    resolve_viewport_tile as host_resolve_viewport_tile_facade,
    ViewportLayoutRequest, ViewportTileRequest,
};
use crate::present::runtime::{
    schedule_render_frame_request as host_schedule_render_frame_request,
    commit_render_frame as host_commit_render_frame_runtime,
    is_render_frame_current as host_is_render_frame_current_runtime,
    reset_frame_cache as host_reset_frame_cache_facade,
    settle_render_frame as host_settle_render_frame_runtime,
    store_frame_cache_entry as host_store_frame_cache_facade,
    touch_frame_cache_entry as host_touch_frame_cache_facade,
    resolve_viewport_refresh as host_resolve_viewport_refresh_facade,
};
use crate::present::plan_builder::{FramePlanRequest, RenderZoomRequest};
use crate::render::progressive_workflow::{
    cancel_progressive_render as host_cancel_progressive_render, render_page as host_render_page,
    start_progressive_render as host_start_progressive_render,
    step_progressive_render as host_step_progressive_render,
};
use crate::projection_workflow::{
    build_editable_segments as host_build_editable_segments,
    build_page_region_context as host_build_page_region_context,
    resolve_editor_projection as host_resolve_editor_projection,
    resolve_page_point as host_resolve_page_point,
    get_pagination_commands as host_get_pagination_commands,
    measure_dom_to_page_scale as host_measure_dom_to_page_scale,
    project_page_rect as host_project_page_rect,
    resolve_font_face as host_resolve_font_face,
};
use crate::render::commit::commit_render_result as host_commit_render_result;
use crate::render::facade::{
    resolve_progressive_render_policy_request as host_resolve_progressive_render_policy_request,
    ProgressiveRenderPolicyRequest,
};
use crate::render::host_runtime::{
    advance_render_loop_frame as host_advance_render_loop_frame,
    queue_render_loop_frame as host_queue_render_loop_frame,
};
use crate::render::loop_workflow::{
    resolve_render_follow_up_runtime as host_resolve_render_follow_up_runtime,
    schedule_render_follow_up_runtime as host_schedule_render_follow_up_runtime,
};
use crate::host::scroll::resolve_host_scroll_refresh as host_resolve_host_scroll_refresh;
use crate::viewer::runtime::{
    reset_zoom_view as host_reset_zoom_view, set_page as host_set_current_page,
    set_zoom as host_set_current_zoom,
};
use crate::zoom::event::{
    handle_wheel_zoom_host as host_handle_wheel_zoom_host,
    step_preview_host as host_step_preview_host, PreviewHostStepRequest,
    WheelZoomHostRequest,
};
use crate::zoom::host::{
    resolve_preview_tick_decision as host_resolve_preview_tick_decision,
    resolve_wheel_render_decision as host_resolve_wheel_render_decision,
    PreviewTickDecisionRequest, WheelRenderDecisionRequest,
};
use crate::zoom::interaction::{
    resolve_zoom_limits_result, AnchorScrollRequest, WheelZoomRequest, ZoomLimitsRequest,
};
use crate::zoom::preview_host::{
    clear_zoom_preview_host_state as host_clear_zoom_preview_host_state,
    get_wheel_render_pending as host_get_wheel_render_pending,
    queue_committed_frame as host_queue_committed_frame,
    reset_zoom_preview_host as host_reset_zoom_preview_host,
    set_wheel_render_pending as host_set_wheel_render_pending,
    take_ready_committed_frame as host_take_ready_committed_frame,
};
use crate::zoom::request::{
    resolve_anchor_scroll as host_resolve_anchor_scroll,
    resolve_wheel_zoom as host_resolve_wheel_zoom,
};
use crate::zoom::runtime::{
    clear_pending_anchor as host_clear_pending_anchor_runtime,
    clear_preview_present as host_clear_preview_present_runtime,
    get_zoom_state as host_get_zoom_state_runtime,
    mark_rendered_zoom as host_mark_rendered_zoom_runtime,
    peek_pending_anchor_layout as host_peek_pending_anchor_layout_runtime,
    peek_pending_anchor_scroll as host_peek_pending_anchor_scroll_runtime,
    set_target_zoom as host_set_target_zoom_runtime,
    set_visual_layout as host_set_visual_layout_runtime,
    step_zoom_animation as host_step_zoom_animation_runtime,
    step_zoom_frame_plan as host_step_zoom_frame_plan_runtime,
    take_pending_anchor_layout as host_take_pending_anchor_layout_runtime,
    take_pending_anchor_scroll as host_take_pending_anchor_scroll_runtime,
};
use crate::zoom::state::PendingCommittedFrame;

#[wasm_bindgen]
pub fn start_progressive_render() -> JsValue {
    to_value(&host_start_progressive_render()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_progressive_render_policy(request_js: JsValue) -> JsValue {
    let request: ProgressiveRenderPolicyRequest = from_value(request_js).unwrap_or_default();
    let policy = host_resolve_progressive_render_policy_request(request);
    to_value(&policy).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_progressive_render(
    canvas_id: String,
    image_cache: JsValue,
    budget_ms: f64,
    max_items: u32,
) -> JsValue {
    to_value(&host_step_progressive_render(
        canvas_id,
        image_cache,
        budget_ms,
        max_items,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn cancel_progressive_render() {
    host_cancel_progressive_render();
}

#[wasm_bindgen]
pub fn render_page(canvas_id: String, image_cache: JsValue) {
    host_render_page(canvas_id, image_cache);
}

#[wasm_bindgen]
pub fn commit_render_result(
    frame_token: u32,
    rendered_zoom: f32,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    to_value(&host_commit_render_result(
        frame_token,
        rendered_zoom,
        page_width,
        page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_font_face(font_name: String, hints: JsValue) -> JsValue {
    host_resolve_font_face(font_name, hints)
}

#[wasm_bindgen]
pub fn build_editable_segments(text_model: JsValue, page_height: f32) -> JsValue {
    host_build_editable_segments(text_model, page_height)
}

#[wasm_bindgen]
pub fn resolve_editor_projection(
    box_rect_js: JsValue,
    zoom: f32,
    font_size: f32,
    page_height: f32,
) -> JsValue {
    host_resolve_editor_projection(box_rect_js, zoom, font_size, page_height)
}

#[wasm_bindgen]
pub fn get_pagination_commands(
    current_page: usize,
    total_pages: usize,
    path: String,
    zoom: f32,
) -> JsValue {
    host_get_pagination_commands(current_page, total_pages, path, zoom)
}

#[wasm_bindgen]
pub fn build_page_region_context(page_model: JsValue) -> JsValue {
    host_build_page_region_context(page_model)
}

#[wasm_bindgen]
pub fn project_page_rect_to_layer_rect(rect: JsValue, zoom: f32) -> JsValue {
    host_project_page_rect(rect, zoom)
}

pub(crate) fn measure_dom_to_page_scale(
    reference_rect: JsValue,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    host_measure_dom_to_page_scale(reference_rect, page_width, page_height)
}

#[wasm_bindgen]
pub fn resolve_page_point(
    point: JsValue,
    reference_rect: JsValue,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    host_resolve_page_point(point, reference_rect, page_width, page_height)
}

#[wasm_bindgen]
pub fn init_page_context(
    vector_model_json: String,
    glyph_plan_json: String,
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) {
    let vector_model: VectorPageModel =
        serde_json::from_str(&vector_model_json).unwrap_or_else(|e| {
            crate::editor::debug_trace::record_editor_debug_event("wasm.init", "json_error", vec![
                crate::editor::debug_trace::editor_debug_field("error", e.to_string()),
                crate::editor::debug_trace::editor_debug_field("json_len", vector_model_json.len()),
            ]);
            VectorPageModel::default()
        });
    crate::editor::debug_trace::record_editor_debug_event("wasm.init", "model_parsed", vec![
        crate::editor::debug_trace::editor_debug_field("object_count", vector_model.objects.len()),
    ]);
    let paint_plan: GlyphPaintPlan = serde_json::from_str(&glyph_plan_json).unwrap_or_else(|e| {
        crate::editor::debug_trace::record_editor_debug_event("wasm.init", "glyph_plan_json_error", vec![
            crate::editor::debug_trace::editor_debug_field("error", e.to_string()),
            crate::editor::debug_trace::editor_debug_field("json_len", glyph_plan_json.len()),
        ]);
        GlyphPaintPlan::default()
    });
    host_init_page_context_from_models(
        vector_model,
        paint_plan,
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
}

#[wasm_bindgen]
pub fn update_page_viewport(
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) {
    host_update_page_viewport_workflow(
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
}

#[wasm_bindgen]
pub fn resolve_wheel_zoom(request_js: JsValue) -> JsValue {
    let request: WheelZoomRequest = from_value(request_js).unwrap_or_default();
    let result = host_resolve_wheel_zoom(&request);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn handle_wheel_zoom_host(request_js: JsValue) -> JsValue {
    let request: WheelZoomHostRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_handle_wheel_zoom_host(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_anchor_scroll(request_js: JsValue) -> JsValue {
    let request: AnchorScrollRequest = from_value(request_js).unwrap_or_default();
    let result = host_resolve_anchor_scroll(&request);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_wheel_render_decision(request_js: JsValue) -> JsValue {
    let request: WheelRenderDecisionRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_resolve_wheel_render_decision(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_preview_tick_decision(request_js: JsValue) -> JsValue {
    let request: PreviewTickDecisionRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_resolve_preview_tick_decision(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_preview_host(request_js: JsValue) -> JsValue {
    let request: PreviewHostStepRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_step_preview_host(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_render_follow_up(
    rendered_display_zoom: f32,
    current_target_zoom: f32,
) -> JsValue {
    to_value(&host_resolve_render_follow_up_runtime(
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
    to_value(&host_schedule_render_follow_up_runtime(
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
    to_value(&host_resolve_layer_execution_plan(
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
    to_value(&host_resolve_render_execution_plan(
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
    to_value(&host_resolve_layer_present_decision(
        use_detail_layer,
        &frame_plan,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_zoom_limits(request_js: JsValue) -> JsValue {
    let request: ZoomLimitsRequest = from_value(request_js).unwrap_or_default();
    to_value(&resolve_zoom_limits_result(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_render_zoom(request_js: JsValue) -> JsValue {
    let request: RenderZoomRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&host_resolve_render_zoom_facade(&request))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&host_resolve_frame_plan_facade(&request, false))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_viewport_refresh(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_resolve_viewport_refresh_facade(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_host_scroll_refresh(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_resolve_host_scroll_refresh(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn touch_frame_cache_entry(is_detail: bool, key: String) -> bool {
    host_touch_frame_cache_facade(is_detail, &key)
}

#[wasm_bindgen]
pub fn store_frame_cache_entry(is_detail: bool, key: String) -> JsValue {
    let result = host_store_frame_cache_facade(is_detail, key);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reset_frame_cache() {
    host_reset_frame_cache_facade();
}

#[wasm_bindgen]
pub fn take_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&host_resolve_frame_plan_facade(&request, true))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn begin_render_frame(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_schedule_render_frame_request(&request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn schedule_render_frame(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    let envelope = host_schedule_render_frame_request(&request);
    to_value(&envelope).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn is_render_frame_current(frame_token: u32) -> bool {
    host_is_render_frame_current_runtime(frame_token)
}

#[wasm_bindgen]
pub fn commit_render_frame(frame_token: u32, rendered_zoom: f32) -> bool {
    host_commit_render_frame_runtime(frame_token, rendered_zoom)
}

#[wasm_bindgen]
pub fn settle_render_frame(frame_token: u32, rendered_zoom: f32) -> JsValue {
    let transition = host_settle_render_frame_runtime(frame_token, Some(rendered_zoom));
    to_value(&transition).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn queue_render_loop_frame(frame_js: JsValue) -> JsValue {
    let frame = if frame_js.is_null() || frame_js.is_undefined() {
        None
    } else {
        from_value(frame_js).ok()
    };
    to_value(&host_queue_render_loop_frame(frame)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn advance_render_loop_frame(frame_js: JsValue) -> JsValue {
    let frame = if frame_js.is_null() || frame_js.is_undefined() {
        None
    } else {
        from_value(frame_js).ok()
    };
    to_value(&host_advance_render_loop_frame(frame)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn abort_render_frame(frame_token: u32) -> JsValue {
    let transition = host_settle_render_frame_runtime(frame_token, None);
    to_value(&transition).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_viewport_layout(request_js: JsValue) -> JsValue {
    let request: ViewportLayoutRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&host_resolve_viewport_layout_facade(&request))
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
    to_value(&host_sync_host_layout(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_viewport_tile(request_js: JsValue) -> JsValue {
    let request: ViewportTileRequest = from_value(request_js).unwrap_or_default();
    serde_wasm_bindgen::to_value(&host_resolve_viewport_tile_facade(&request))
        .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reset_zoom_state(initial_zoom: f32) {
    host_reset_zoom_view(initial_zoom);
}

#[wasm_bindgen]
pub fn get_zoom_state() -> JsValue {
    to_value(&host_get_zoom_state_runtime()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn set_target_zoom(target_zoom: f32) {
    host_set_target_zoom_runtime(target_zoom);
}

#[wasm_bindgen]
pub fn mark_rendered_zoom(rendered_zoom: f32) {
    host_mark_rendered_zoom_runtime(rendered_zoom);
}

#[wasm_bindgen]
pub fn reset_zoom_preview_host(target_zoom: f32) {
    host_reset_zoom_preview_host(target_zoom);
}

#[wasm_bindgen]
pub fn set_wheel_render_pending(pending: bool) {
    host_set_wheel_render_pending(pending);
}

#[wasm_bindgen]
pub fn get_wheel_render_pending() -> bool {
    host_get_wheel_render_pending()
}

#[wasm_bindgen]
pub fn queue_committed_frame(frame_js: JsValue) {
    let frame: PendingCommittedFrame = from_value(frame_js).unwrap_or_default();
    host_queue_committed_frame(&frame);
}

#[wasm_bindgen]
pub fn take_ready_committed_frame() -> JsValue {
    to_value(&host_take_ready_committed_frame()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_zoom_animation() -> JsValue {
    to_value(&host_step_zoom_animation_runtime()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_zoom_frame_plan(request_js: JsValue) -> JsValue {
    let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    let preview = host_step_zoom_frame_plan_runtime(&request);
    to_value(&preview).unwrap_or(JsValue::NULL)
}

pub(crate) fn take_pending_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> JsValue {
    to_value(&host_take_pending_anchor_scroll_runtime(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn clear_pending_anchor() {
    host_clear_pending_anchor_runtime();
}

pub(crate) fn set_visual_layout(display_zoom: f32, content_left: f32, content_top: f32) {
    host_set_visual_layout_runtime(display_zoom, content_left, content_top);
}

#[wasm_bindgen]
pub fn clear_preview_present() {
    host_clear_preview_present_runtime();
}

#[wasm_bindgen]
pub fn clear_zoom_preview_host_state(clear_pending_anchor: bool) {
    host_clear_zoom_preview_host_state(clear_pending_anchor);
}

pub(crate) fn peek_pending_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> JsValue {
    to_value(&host_peek_pending_anchor_scroll_runtime(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
    ))
    .unwrap_or(JsValue::NULL)
}

pub(crate) fn peek_pending_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> JsValue {
    to_value(&host_peek_pending_anchor_layout_runtime(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
    ))
    .unwrap_or(JsValue::NULL)
}

pub(crate) fn take_pending_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> JsValue {
    to_value(&host_take_pending_anchor_layout_runtime(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn set_current_page(page_index: u16) {
    host_set_current_page(page_index);
}

#[wasm_bindgen]
pub fn set_current_zoom(zoom: f32) {
    host_set_current_zoom(zoom);
}

#[wasm_bindgen]
pub fn navigate_prev_page() -> JsValue {
    to_value(&host_navigate_prev_page()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn navigate_next_page() -> JsValue {
    to_value(&host_navigate_next_page()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn apply_zoom_selection(zoom: f32) -> JsValue {
    to_value(&host_apply_zoom_selection(zoom)).unwrap_or(JsValue::NULL)
}


