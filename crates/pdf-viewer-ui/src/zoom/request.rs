use crate::viewer::viewer_controller::set_zoom;
use crate::zoom::interaction::{
    compute_anchor_scroll_result, resolve_wheel_zoom_request, AnchorScrollRequest,
    AnchorScrollResult, WheelZoomRequest, WheelZoomResult,
};
use crate::zoom::zoom_store;

pub fn resolve_wheel_zoom(request: &WheelZoomRequest) -> WheelZoomResult {
    let (result, pending_anchor) = zoom_store::with_zoom_state(|state| {
        resolve_wheel_zoom_request(
            request,
            state.visual_layout.as_ref(),
            state.preview_transform.as_ref(),
        )
    });
    zoom_store::with_zoom_state_mut(|state| {
        if state.visual_zoom <= 0.0 {
            state.visual_zoom = state.last_rendered_zoom.max(1.0);
        }
        state.target_zoom = result.target_zoom;
        state.last_animation_timestamp_ms = 0.0;
        state.pending_anchor = Some(pending_anchor);
    });
    set_zoom(result.target_zoom);
    result
}

pub fn resolve_anchor_scroll(request: &AnchorScrollRequest) -> AnchorScrollResult {
    compute_anchor_scroll_result(
        request.display_width,
        request.display_height,
        request.viewport_width,
        request.viewport_height,
        request.anchor_pdf_x,
        request.anchor_pdf_y,
        1.0,
        1.0,
        request.viewport_x,
        request.viewport_y,
    )
}
