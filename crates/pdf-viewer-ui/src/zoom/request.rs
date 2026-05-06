use crate::viewer::runtime::set_zoom;
use crate::zoom::interaction::{
    compute_anchor_scroll_result, resolve_wheel_zoom_request, AnchorScrollRequest,
    AnchorScrollResult, WheelZoomRequest, WheelZoomResult,
};
use crate::zoom::state::HOST_ZOOM_STATE;

pub fn resolve_wheel_zoom(request: &WheelZoomRequest) -> WheelZoomResult {
    let (result, pending_anchor) = HOST_ZOOM_STATE.with(|state| {
        let state = state.borrow();
        resolve_wheel_zoom_request(
            request,
            state.visual_layout.as_ref(),
            state.preview_transform.as_ref(),
        )
    });
    HOST_ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
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
