//! Animation frame stepping and committed frame queue management.

use crate::editor::session::render_scene_key;
use crate::present::plan_builder::{build_frame_plan_result, FramePlanRequest};
use crate::present::present_store;
use pdf_viewer_core::render::zoom::animation::{advance_zoom_animation_state, build_zoom_preview_frame, ZoomPreviewFrame};
use crate::zoom::zoom_store::{PendingCommittedFrame, ZOOM_STATE};
use crate::viewer::viewer_store;
use super::zoom_authority::read_zoom_state;

pub fn step_zoom_animation() -> crate::zoom::zoom_store::ZoomAnimationStep {
    ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        advance_zoom_animation_state(&mut state, None)
    })
}

pub fn step_zoom_frame_plan(request: &FramePlanRequest) -> ZoomPreviewFrame {
    let viewer_session = viewer_store::read_viewer_session();
    present_store::with_present_state(|present_state| {
        ZOOM_STATE.with(|state| {
            let mut state = state.borrow_mut();
            build_zoom_preview_frame(request, &mut state, |frame_request, zoom_state| {
                build_frame_plan_result(
                    frame_request,
                    zoom_state,
                    &viewer_session,
                    present_state,
                    &render_scene_key(),
                    false,
                )
            })
        })
    })
}

pub fn queue_committed_frame(frame_plan: &PendingCommittedFrame) {
    ZOOM_STATE.with(|state| {
        state.borrow_mut().preview_host.pending_committed_frame = Some(frame_plan.clone());
    });
}

pub fn take_ready_committed_frame() -> Option<PendingCommittedFrame> {
    let zoom_state = read_zoom_state();
    if (zoom_state.target_zoom - zoom_state.visual_zoom).abs() >= 0.001 {
        return None;
    }
    ZOOM_STATE.with(|state| state.borrow_mut().preview_host.pending_committed_frame.take())
}
