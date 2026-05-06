use crate::editor::session::render_scene_key as host_render_scene_key;
use crate::present::plan_builder::{
    build_frame_plan_result as host_build_frame_plan_result, AnchorViewportLayoutResult,
    FramePlanRequest,
};
use crate::present::runtime::{
    reset_present_runtime as host_reset_present_runtime, HOST_PRESENT_STATE,
};
use crate::render::scheduler::reset_render_state as host_reset_render_state;
use crate::viewer::session::HOST_VIEWER_SESSION;
use crate::zoom::interaction::{
    advance_zoom_animation_state, build_zoom_preview_frame, commit_rendered_zoom,
    compute_anchor_scroll_result, compute_anchor_viewport_layout_result, AnchorScrollResult,
    ZoomPreviewFrame,
};
use crate::zoom::state::{
    reset_zoom_state as host_reset_zoom_state, HostZoomState, VisualLayoutState,
    HOST_ZOOM_STATE,
};

pub fn reset_zoom_runtime(initial_zoom: f32) {
    host_reset_zoom_state(initial_zoom);
    host_reset_render_state();
    host_reset_present_runtime(true, false);
}

pub fn get_zoom_state() -> HostZoomState {
    HOST_ZOOM_STATE.with(|state| state.borrow().clone())
}

pub fn set_target_zoom(target_zoom: f32) {
    let zoom = if target_zoom.is_finite() && target_zoom > 0.0 {
        target_zoom
    } else {
        1.0
    };
    HOST_ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.target_zoom = zoom;
        state.last_animation_timestamp_ms = 0.0;
    });
}

pub fn mark_rendered_zoom(rendered_zoom: f32) {
    HOST_ZOOM_STATE.with(|state| {
        commit_rendered_zoom(&mut state.borrow_mut(), rendered_zoom);
    });
}

pub fn step_zoom_animation() -> crate::zoom::state::ZoomAnimationStep {
    HOST_ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        advance_zoom_animation_state(&mut state, None)
    })
}

pub fn step_zoom_frame_plan(request: &FramePlanRequest) -> ZoomPreviewFrame {
    HOST_VIEWER_SESSION.with(|viewer_session| {
        let viewer_session = viewer_session.borrow();
        HOST_PRESENT_STATE.with(|present_state| {
            let present_state = present_state.borrow();
            HOST_ZOOM_STATE.with(|state| {
                let mut state = state.borrow_mut();
                build_zoom_preview_frame(request, &mut state, |frame_request, zoom_state| {
                    host_build_frame_plan_result(
                        frame_request,
                        zoom_state,
                        &viewer_session,
                        &present_state,
                        &host_render_scene_key(),
                        false,
                    )
                })
            })
        })
    })
}

pub fn take_pending_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorScrollResult> {
    HOST_ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let anchor = state.pending_anchor.take()?;
        Some(compute_anchor_scroll_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor.anchor_page_x,
            anchor.anchor_page_y,
            anchor.page_width,
            anchor.page_height,
            anchor.viewport_x,
            anchor.viewport_y,
        ))
    })
}

pub fn peek_pending_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorScrollResult> {
    HOST_ZOOM_STATE.with(|state| {
        let state = state.borrow();
        let anchor = state.pending_anchor.as_ref()?;
        Some(compute_anchor_scroll_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor.anchor_page_x,
            anchor.anchor_page_y,
            anchor.page_width,
            anchor.page_height,
            anchor.viewport_x,
            anchor.viewport_y,
        ))
    })
}

pub fn peek_pending_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorViewportLayoutResult> {
    HOST_ZOOM_STATE.with(|state| {
        let state = state.borrow();
        let anchor = state.pending_anchor.as_ref()?;
        Some(compute_anchor_viewport_layout_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor.anchor_page_x,
            anchor.anchor_page_y,
            anchor.page_width,
            anchor.page_height,
            anchor.viewport_x,
            anchor.viewport_y,
        ))
    })
}

pub fn take_pending_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorViewportLayoutResult> {
    HOST_ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let anchor = state.pending_anchor.take()?;
        Some(compute_anchor_viewport_layout_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor.anchor_page_x,
            anchor.anchor_page_y,
            anchor.page_width,
            anchor.page_height,
            anchor.viewport_x,
            anchor.viewport_y,
        ))
    })
}

pub fn clear_pending_anchor() {
    HOST_ZOOM_STATE.with(|state| {
        state.borrow_mut().pending_anchor = None;
    });
}

pub fn set_visual_layout(display_zoom: f32, content_left: f32, content_top: f32) {
    HOST_ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.visual_layout = Some(VisualLayoutState {
            display_zoom: if display_zoom.is_finite() && display_zoom > 0.0 {
                display_zoom
            } else {
                1.0
            },
            content_left: if content_left.is_finite() {
                content_left.max(0.0)
            } else {
                0.0
            },
            content_top: if content_top.is_finite() {
                content_top.max(0.0)
            } else {
                0.0
            },
        });
    });
}

pub fn clear_preview_present() {
    HOST_ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_transform = None;
    });
}
