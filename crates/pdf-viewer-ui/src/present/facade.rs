use crate::present::plan_builder::{
    compute_viewport_layout_result, compute_viewport_tile_result, resolve_render_zoom_result,
    FramePlanRequest, RenderZoomRequest,
};
use crate::present::present_store::build_frame_plan_result;

// Re-export pure DTO types from core.
pub use pdf_viewer_core::render::facade_types::*;

pub fn resolve_render_zoom(request: &RenderZoomRequest) -> serde_json::Value {
    serde_json::to_value(resolve_render_zoom_result(request)).unwrap_or(serde_json::Value::Null)
}

pub fn resolve_frame_plan(
    request: &FramePlanRequest,
    consume_anchor: bool,
) -> serde_json::Value {
    serde_json::to_value(build_frame_plan_result(request, consume_anchor))
        .unwrap_or(serde_json::Value::Null)
}

pub fn resolve_viewport_layout(request: &ViewportLayoutRequest) -> serde_json::Value {
    serde_json::to_value(compute_viewport_layout_result(
        request.display_width,
        request.display_height,
        request.viewport_width,
        request.viewport_height,
    ))
    .unwrap_or(serde_json::Value::Null)
}

pub fn resolve_viewport_tile(request: &ViewportTileRequest) -> serde_json::Value {
    serde_json::to_value(compute_viewport_tile_result(
        request.display_width,
        request.display_height,
        request.viewport_width,
        request.viewport_height,
        request.scroll_left,
        request.scroll_top,
        request.content_left,
        request.content_top,
        request.overscan,
    ))
    .unwrap_or(serde_json::Value::Null)
}
