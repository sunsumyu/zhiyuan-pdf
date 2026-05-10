//! RenderPipeline — P3 struct-based WASM API for progressive rendering,
//! page context, projection helpers, and page navigation.
//!
//! Mirrors the P0–P2 pattern: zero-sized struct + camelCase methods + thin
//! delegation. The flat `wasm_api::viewer` functions remain for backward
//! compatibility while TS migrates.

use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::host::command::{
    navigate_next_page,
    navigate_prev_page,
};
use crate::page::context::{
    init_page_context_from_models,
    update_page_viewport_workflow,
};
use crate::projection_workflow::{
    build_editable_segments,
    build_page_region_context,
    get_pagination_commands,
    project_page_rect,
    resolve_editor_projection,
    resolve_font_face,
    resolve_page_point,
};
use crate::render::commit::commit_render_result;
use crate::render::facade::{
    resolve_progressive_render_policy_request,
    ProgressiveRenderPolicyRequest,
};
use crate::render::progressive_workflow::{
    cancel_progressive_render,
    render_page,
    start_progressive_render,
    step_progressive_render,
};
use crate::viewer::viewer_controller::set_page;

// ── RenderPipeline ──────────────────────────────────────────────

#[wasm_bindgen]
pub struct RenderPipeline;

#[wasm_bindgen]
impl RenderPipeline {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        RenderPipeline
    }

    // ── Progressive render lifecycle ────────────────────────────

    /// Start a progressive render pass.
    #[wasm_bindgen(js_name = "startProgressive")]
    pub fn start_progressive(&self) -> JsValue {
        to_value(&start_progressive_render()).unwrap_or(JsValue::NULL)
    }

    /// Resolve the progressive render policy for a request.
    #[wasm_bindgen(js_name = "resolveProgressivePolicy")]
    pub fn resolve_progressive_policy(&self, request_js: JsValue) -> JsValue {
        let request: ProgressiveRenderPolicyRequest = from_value(request_js).unwrap_or_default();
        to_value(&resolve_progressive_render_policy_request(request))
            .unwrap_or(JsValue::NULL)
    }

    /// Step the progressive render with a per-frame budget.
    #[wasm_bindgen(js_name = "stepProgressive")]
    pub fn step_progressive(
        &self,
        canvas_id: String,
        image_cache: JsValue,
        budget_ms: f64,
        max_items: u32,
    ) -> JsValue {
        to_value(&step_progressive_render(
            canvas_id,
            image_cache,
            budget_ms,
            max_items,
        ))
        .unwrap_or(JsValue::NULL)
    }

    /// Cancel any in-flight progressive render.
    #[wasm_bindgen(js_name = "cancelProgressive")]
    pub fn cancel_progressive(&self) {
        cancel_progressive_render();
    }

    /// Render a page in one shot (non-progressive).
    #[wasm_bindgen(js_name = "renderPage")]
    pub fn render_page(&self, canvas_id: String, image_cache: JsValue) {
        render_page(canvas_id, image_cache);
    }

    /// Commit a completed render result back to the host.
    #[wasm_bindgen(js_name = "commitResult")]
    pub fn commit_result(
        &self,
        frame_token: u32,
        rendered_zoom: f32,
        page_width: f32,
        page_height: f32,
    ) -> JsValue {
        to_value(&commit_render_result(
            frame_token,
            rendered_zoom,
            page_width,
            page_height,
        ))
        .unwrap_or(JsValue::NULL)
    }

    // ── Page context ────────────────────────────────────────────

    /// Initialize page context from JSON-serialised models (entry point).
    #[wasm_bindgen(js_name = "initPageContext")]
    pub fn init_page_context(
        &self,
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
                crate::editor::debug_trace::record_editor_debug_event(
                    "wasm.init",
                    "json_error",
                    vec![
                        crate::editor::debug_trace::editor_debug_field("error", e.to_string()),
                        crate::editor::debug_trace::editor_debug_field(
                            "json_len",
                            vector_model_json.len(),
                        ),
                    ],
                );
                VectorPageModel::default()
            });
        crate::editor::debug_trace::record_editor_debug_event(
            "wasm.init",
            "model_parsed",
            vec![crate::editor::debug_trace::editor_debug_field(
                "object_count",
                vector_model.objects.len(),
            )],
        );
        let paint_plan: GlyphPaintPlan = serde_json::from_str(&glyph_plan_json).unwrap_or_else(|e| {
            crate::editor::debug_trace::record_editor_debug_event(
                "wasm.init",
                "glyph_plan_json_error",
                vec![
                    crate::editor::debug_trace::editor_debug_field("error", e.to_string()),
                    crate::editor::debug_trace::editor_debug_field(
                        "json_len",
                        glyph_plan_json.len(),
                    ),
                ],
            );
            GlyphPaintPlan::default()
        });
        init_page_context_from_models(
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

    /// Update the page viewport without re-initialising context.
    #[wasm_bindgen(js_name = "updatePageViewport")]
    pub fn update_page_viewport(
        &self,
        zoom: f32,
        dpr: f32,
        viewport_left: Option<f32>,
        viewport_top: Option<f32>,
        viewport_width: Option<f32>,
        viewport_height: Option<f32>,
    ) {
        update_page_viewport_workflow(
            zoom,
            dpr,
            viewport_left,
            viewport_top,
            viewport_width,
            viewport_height,
        );
    }

    /// Build the per-page region context (used by editor / hit-testing).
    #[wasm_bindgen(js_name = "buildPageRegionContext")]
    pub fn build_page_region_context(&self, page_model: JsValue) -> JsValue {
        build_page_region_context(page_model)
    }

    // ── Projection / coordinate helpers ─────────────────────────

    /// Resolve a font face from name + hints.
    #[wasm_bindgen(js_name = "resolveFontFace")]
    pub fn resolve_font_face(&self, font_name: String, hints: JsValue) -> JsValue {
        resolve_font_face(font_name, hints)
    }

    /// Build editable segments for a text model at the given page height.
    #[wasm_bindgen(js_name = "buildEditableSegments")]
    pub fn build_editable_segments(&self, text_model: JsValue, page_height: f32) -> JsValue {
        build_editable_segments(text_model, page_height)
    }

    /// Resolve the editor projection (text-block bounding box → DOM rect).
    #[wasm_bindgen(js_name = "resolveEditorProjection")]
    pub fn resolve_editor_projection(
        &self,
        box_rect_js: JsValue,
        zoom: f32,
        font_size: f32,
        page_height: f32,
    ) -> JsValue {
        resolve_editor_projection(box_rect_js, zoom, font_size, page_height)
    }

    /// Get pagination commands (prev/next available, page list, etc.).
    #[wasm_bindgen(js_name = "getPaginationCommands")]
    pub fn get_pagination_commands(
        &self,
        current_page: usize,
        total_pages: usize,
        path: String,
        zoom: f32,
    ) -> JsValue {
        get_pagination_commands(current_page, total_pages, path, zoom)
    }

    /// Project a page-space rect to a layer-space rect at the given zoom.
    #[wasm_bindgen(js_name = "projectPageRectToLayer")]
    pub fn project_page_rect_to_layer(&self, rect: JsValue, zoom: f32) -> JsValue {
        project_page_rect(rect, zoom)
    }

    /// Resolve a client-space point to a page-space point.
    #[wasm_bindgen(js_name = "resolvePagePoint")]
    pub fn resolve_page_point(
        &self,
        point: JsValue,
        reference_rect: JsValue,
        page_width: f32,
        page_height: f32,
    ) -> JsValue {
        resolve_page_point(point, reference_rect, page_width, page_height)
    }

    // ── Navigation ──────────────────────────────────────────────

    /// Set the active page index.
    #[wasm_bindgen(js_name = "setCurrentPage")]
    pub fn set_current_page(&self, page_index: u16) {
        set_page(page_index);
    }

    /// Navigate to the previous page.
    #[wasm_bindgen(js_name = "navigatePrev")]
    pub fn navigate_prev(&self) -> JsValue {
        to_value(&navigate_prev_page()).unwrap_or(JsValue::NULL)
    }

    /// Navigate to the next page.
    #[wasm_bindgen(js_name = "navigateNext")]
    pub fn navigate_next(&self) -> JsValue {
        to_value(&navigate_next_page()).unwrap_or(JsValue::NULL)
    }
}

impl Default for RenderPipeline {
    fn default() -> Self {
        Self::new()
    }
}
