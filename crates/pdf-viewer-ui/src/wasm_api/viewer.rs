//! Viewer WASM bindings: progressive rendering, page context, projection,
//! and page-level navigation.
//!
//! Zoom/wheel bindings moved to `zoom_api`.
//! Frame plan/cache/lifecycle bindings moved to `frame_api`.

use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::host::command;
use crate::page::context;
use crate::projection_workflow;
use crate::render::commit;
use crate::render::facade::ProgressiveRenderPolicyRequest;
use crate::render::{facade as render_facade, progressive_workflow};
use crate::viewer::viewer_controller;

#[wasm_bindgen]
pub fn start_progressive_render() -> JsValue {
    to_value(&progressive_workflow::start_progressive_render()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_progressive_render_policy(request_js: JsValue) -> JsValue {
    let request: ProgressiveRenderPolicyRequest = from_value(request_js).unwrap_or_default();
    let policy = render_facade::resolve_progressive_render_policy_request(request);
    to_value(&policy).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn step_progressive_render(
    canvas_id: String,
    image_cache: JsValue,
    budget_ms: f64,
    max_items: u32,
) -> JsValue {
    to_value(&progressive_workflow::step_progressive_render(
        canvas_id,
        image_cache,
        budget_ms,
        max_items,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn cancel_progressive_render() {
    progressive_workflow::cancel_progressive_render();
}

#[wasm_bindgen]
pub fn render_page(canvas_id: String, image_cache: JsValue) {
    progressive_workflow::render_page(canvas_id, image_cache);
}

#[wasm_bindgen]
pub fn commit_render_result(
    frame_token: u32,
    rendered_zoom: f32,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    to_value(&commit::commit_render_result(
        frame_token,
        rendered_zoom,
        page_width,
        page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn resolve_font_face(font_name: String, hints: JsValue) -> JsValue {
    projection_workflow::resolve_font_face(font_name, hints)
}

#[wasm_bindgen]
pub fn build_editable_segments(text_model: JsValue, page_height: f32) -> JsValue {
    projection_workflow::build_editable_segments(text_model, page_height)
}

#[wasm_bindgen]
pub fn resolve_editor_projection(
    box_rect_js: JsValue,
    zoom: f32,
    font_size: f32,
    page_height: f32,
) -> JsValue {
    projection_workflow::resolve_editor_projection(box_rect_js, zoom, font_size, page_height)
}

#[wasm_bindgen]
pub fn get_pagination_commands(
    current_page: usize,
    total_pages: usize,
    path: String,
    zoom: f32,
) -> JsValue {
    projection_workflow::get_pagination_commands(current_page, total_pages, path, zoom)
}

#[wasm_bindgen]
pub fn build_page_region_context(page_model: JsValue) -> JsValue {
    projection_workflow::build_page_region_context(page_model)
}

#[wasm_bindgen]
pub fn project_page_rect_to_layer_rect(rect: JsValue, zoom: f32) -> JsValue {
    projection_workflow::project_page_rect(rect, zoom)
}

#[allow(dead_code)]
pub(crate) fn measure_dom_to_page_scale(
    reference_rect: JsValue,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    projection_workflow::measure_dom_to_page_scale(reference_rect, page_width, page_height)
}

#[wasm_bindgen]
pub fn resolve_page_point(
    point: JsValue,
    reference_rect: JsValue,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    projection_workflow::resolve_page_point(point, reference_rect, page_width, page_height)
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
    context::init_page_context_from_models(
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
    context::update_page_viewport_workflow(
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
}

#[wasm_bindgen]
pub fn set_current_page(page_index: u16) {
    viewer_controller::set_page(page_index);
}

#[wasm_bindgen]
pub fn navigate_prev_page() -> JsValue {
    to_value(&command::navigate_prev_page()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn navigate_next_page() -> JsValue {
    to_value(&command::navigate_next_page()).unwrap_or(JsValue::NULL)
}
