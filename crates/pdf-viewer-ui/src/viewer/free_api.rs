//! Viewer-domain free wasm exports retained for the TS bridge.
//!
//! 这些函数原位于 `wasm_api/viewer.rs`（已删除）。新代码请走 `ViewerSession` /
//! `RenderPipeline`。

use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};
use wasm_bindgen::prelude::*;

use crate::page::context;
use crate::viewer::viewer_controller;

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
pub fn set_current_page(page_index: u16) {
    viewer_controller::set_page(page_index);
}
