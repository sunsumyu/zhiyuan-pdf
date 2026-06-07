use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};

use crate::page::page_store::{init_page_context, update_page_viewport};
use crate::viewer::viewer_store;

pub fn init_page_context_from_models(
    vector_model: VectorPageModel,
    paint_plan: GlyphPaintPlan,
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) {
    // [Structural Flip] Coordinates are now pre-normalized to Y-Down in the backend.
    // Redundant UI-side flipping is removed to prevent double-inversion.

    let page_dimensions = init_page_context(
        vector_model,
        paint_plan,
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
    let (pw, ph) = page_dimensions.unzip();
    viewer_store::set_zoom_and_page_dimensions(zoom, pw, ph);
}

pub fn update_page_viewport_workflow(
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) {
    update_page_viewport(
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
    viewer_store::set_current_zoom(zoom);
}
