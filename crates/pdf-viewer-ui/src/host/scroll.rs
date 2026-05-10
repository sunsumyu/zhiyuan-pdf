use crate::present::plan_builder::FramePlanRequest;
use crate::present::present_store::resolve_viewport_refresh;
use crate::viewer::viewer_controller::get_session;
use crate::viewport_refresh::ViewportRefreshDecision;
use crate::zoom::zoom_controller::get_zoom_state;

pub fn resolve_host_scroll_refresh(request: &FramePlanRequest) -> ViewportRefreshDecision {
    let session = get_session();
    if session.path.is_none() {
        return ViewportRefreshDecision::default();
    }

    let zoom_state = get_zoom_state();
    if (zoom_state.target_zoom - zoom_state.last_rendered_zoom).abs() > 0.001 {
        return ViewportRefreshDecision::default();
    }

    resolve_viewport_refresh(request)
}
