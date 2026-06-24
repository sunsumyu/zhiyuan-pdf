use crate::editor::session::render_scene_key;
use crate::present::plan_builder::{
    build_frame_plan_result, AnchorViewportLayoutResult, FramePlanRequest,
};
use crate::present::present_store;
use crate::present::present_store::reset_present_runtime;
use crate::render::render_store::reset_render_state;
use crate::viewer::viewer_store;
use crate::zoom::interaction::{
    advance_zoom_animation_state, build_zoom_preview_frame, commit_rendered_zoom,
    compute_anchor_scroll_result, compute_anchor_viewport_layout_result, AnchorScrollResult,
    ZoomPreviewFrame,
};
use crate::zoom::zoom_store::{
    reset_zoom_state, read_zoom_state as store_read_zoom_state, with_zoom_state,
    with_zoom_state_mut, HostZoomState, VisualLayoutState,
};

pub fn reset_zoom_runtime(initial_zoom: f32) {
    reset_zoom_state(initial_zoom);
    reset_render_state();
    reset_present_runtime(true, false);
}

pub fn read_zoom_state() -> HostZoomState {
    store_read_zoom_state()
}

pub fn set_target_zoom(target_zoom: f32) {
    let zoom = if target_zoom.is_finite() && target_zoom > 0.0 {
        target_zoom
    } else {
        1.0
    };
    with_zoom_state_mut(|state| {
        state.target_zoom = zoom;
        state.last_animation_timestamp_ms = 0.0;
    });
}

pub fn mark_rendered_zoom(rendered_zoom: f32) {
    with_zoom_state_mut(|state| {
        commit_rendered_zoom(state, rendered_zoom);
    });
}

pub fn step_zoom_animation() -> crate::zoom::zoom_store::ZoomAnimationStep {
    with_zoom_state_mut(|state| {
        advance_zoom_animation_state(state, None)
    })
}

pub fn step_frame_plan(request: &FramePlanRequest) -> ZoomPreviewFrame {
    let viewer_session = viewer_store::read_viewer_session();
    let scene_key = render_scene_key();
    let present_snapshot = present_store::with_present_state(Clone::clone);
    with_zoom_state_mut(|state| {
        build_zoom_preview_frame(request, state, |frame_request, zoom_state| {
            build_frame_plan_result(
                frame_request,
                zoom_state,
                &viewer_session,
                &present_snapshot,
                &scene_key,
                false,
            )
        })
    })
}

pub fn take_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorScrollResult> {
    with_zoom_state_mut(|state| {
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

pub fn peek_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorScrollResult> {
    with_zoom_state(|state| {
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

pub fn peek_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorViewportLayoutResult> {
    with_zoom_state(|state| {
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

pub fn take_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorViewportLayoutResult> {
    with_zoom_state_mut(|state| {
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
    with_zoom_state_mut(|state| {
        state.pending_anchor = None;
    });
}

pub fn set_visual_layout(display_zoom: f32, content_left: f32, content_top: f32) {
    with_zoom_state_mut(|state| {
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
    with_zoom_state_mut(|state| {
        state.preview_transform = None;
    });
}
