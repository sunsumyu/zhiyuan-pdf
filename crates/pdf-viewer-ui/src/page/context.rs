use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};

use crate::page::runtime::{
    init_page_context as host_init_page_context,
    update_page_viewport as host_update_page_viewport,
};
use crate::viewer::session::HOST_VIEWER_SESSION;

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

    let page_dimensions = host_init_page_context(
        vector_model,
        paint_plan,
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
    HOST_VIEWER_SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.current_zoom = zoom;
        if let Some((page_width, page_height)) = page_dimensions {
            session.page_width = page_width;
            session.page_height = page_height;
        }
    });
}

pub fn update_page_viewport_workflow(
    zoom: f32,
    dpr: f32,
    viewport_left: Option<f32>,
    viewport_top: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
) {
    host_update_page_viewport(
        zoom,
        dpr,
        viewport_left,
        viewport_top,
        viewport_width,
        viewport_height,
    );
    HOST_VIEWER_SESSION.with(|session| {
        session.borrow_mut().current_zoom = zoom;
    });
}
