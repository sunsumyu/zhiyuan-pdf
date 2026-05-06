use std::cell::RefCell;

use crate::editor::session::render_scene_key as host_render_scene_key;
use crate::render::frame_cache::{
    reset_frame_cache as host_reset_frame_cache,
    resolve_viewport_refresh as host_resolve_viewport_refresh,
    store_frame_cache_entry as host_store_frame_cache_entry,
    touch_frame_cache_entry as host_touch_frame_cache_entry,
};
use crate::present::plan_builder::{
    build_frame_plan_result as host_build_frame_plan_result, FramePlanRequest, FramePlanResult,
};
use crate::render::scheduler::{
    schedule_render_frame as host_schedule_render_frame,
    RenderFrameEnvelope as HostRenderFrameEnvelope,
};
use crate::render::workflow::{
    build_render_frame_envelope, frame_plan_requires_render, frame_plans_share_render_work,
    settle_render_frame_inner as host_settle_render_frame_inner, RenderFrameEnvelope,
    RenderFrameTransition,
};
use crate::render::tile_cache::{
    FrameCacheStoreResult, HostFrameCacheState, HostPresentState,
};
use crate::viewer::session::HOST_VIEWER_SESSION;
use crate::viewport_refresh::{HostViewportRefreshState, ViewportRefreshDecision};
use crate::zoom::state::HOST_ZOOM_STATE;

thread_local! {
    pub static HOST_PRESENT_STATE: RefCell<HostPresentState> =
        RefCell::new(HostPresentState::default());
    pub static HOST_FRAME_CACHE_STATE: RefCell<HostFrameCacheState> =
        RefCell::new(HostFrameCacheState::default());
    pub static HOST_VIEWPORT_REFRESH_STATE: RefCell<HostViewportRefreshState> =
        RefCell::new(HostViewportRefreshState::default());
}

pub fn build_frame_plan_result(
    request: &FramePlanRequest,
    consume_anchor: bool,
) -> FramePlanResult {
    HOST_ZOOM_STATE.with(|zoom_state| {
        let mut zoom_state = zoom_state.borrow_mut();
        HOST_VIEWER_SESSION.with(|viewer_session| {
            let viewer_session = viewer_session.borrow();
            HOST_PRESENT_STATE.with(|present_state| {
                host_build_frame_plan_result(
                    request,
                    &mut zoom_state,
                    &viewer_session,
                    &present_state.borrow(),
                    &host_render_scene_key(),
                    consume_anchor,
                )
            })
        })
    })
}

pub fn resolve_viewport_refresh(request: &FramePlanRequest) -> ViewportRefreshDecision {
    let frame_plan = build_frame_plan_result(request, false);
    HOST_VIEWPORT_REFRESH_STATE.with(|state| {
        host_resolve_viewport_refresh(&state.borrow(), &frame_plan, request.timestamp_ms)
    })
}

pub fn touch_frame_cache_entry(is_detail: bool, key: &str) -> bool {
    HOST_FRAME_CACHE_STATE
        .with(|state| host_touch_frame_cache_entry(&mut state.borrow_mut(), is_detail, key))
}

pub fn store_frame_cache_entry(is_detail: bool, key: String) -> FrameCacheStoreResult {
    HOST_FRAME_CACHE_STATE
        .with(|state| host_store_frame_cache_entry(&mut state.borrow_mut(), is_detail, key))
}

pub fn reset_frame_cache() {
    HOST_FRAME_CACHE_STATE.with(|state| {
        host_reset_frame_cache(&mut state.borrow_mut());
    });
}

pub fn reset_present_runtime(reset_cache: bool, reset_refresh: bool) {
    HOST_PRESENT_STATE.with(|state| {
        *state.borrow_mut() = HostPresentState::default();
    });
    if reset_cache {
        HOST_FRAME_CACHE_STATE.with(|state| {
            *state.borrow_mut() = HostFrameCacheState::default();
        });
    }
    if reset_refresh {
        HOST_VIEWPORT_REFRESH_STATE.with(|state| {
            *state.borrow_mut() = HostViewportRefreshState::default();
        });
    }
}

pub fn schedule_render_frame_request(
    request: &FramePlanRequest,
) -> Option<RenderFrameEnvelope> {
    let frame_plan = build_frame_plan_result(request, false);
    let envelope: Option<HostRenderFrameEnvelope<FramePlanResult>> =
        host_schedule_render_frame(
            &frame_plan,
            frame_plan_requires_render,
            frame_plans_share_render_work,
            |plan| serde_json::to_value(plan).unwrap_or(serde_json::Value::Null),
            |plan_value| serde_json::from_value::<FramePlanResult>(plan_value.clone()).ok(),
        );
    envelope.map(|frame| build_render_frame_envelope(frame.frame_token, frame.frame_plan))
}

pub fn commit_render_frame(frame_token: u32, rendered_zoom: f32) -> bool {
    HOST_ZOOM_STATE.with(|zoom_state| {
        HOST_PRESENT_STATE.with(|present_state| {
            HOST_VIEWPORT_REFRESH_STATE.with(|refresh_state| {
                host_settle_render_frame_inner(
                    frame_token,
                    Some(rendered_zoom),
                    &mut zoom_state.borrow_mut(),
                    &mut present_state.borrow_mut(),
                    &mut refresh_state.borrow_mut(),
                )
                .accepted
            })
        })
    })
}

pub fn settle_render_frame(
    frame_token: u32,
    maybe_rendered_zoom: Option<f32>,
) -> RenderFrameTransition {
    HOST_ZOOM_STATE.with(|zoom_state| {
        HOST_PRESENT_STATE.with(|present_state| {
            HOST_VIEWPORT_REFRESH_STATE.with(|refresh_state| {
                host_settle_render_frame_inner(
                    frame_token,
                    maybe_rendered_zoom,
                    &mut zoom_state.borrow_mut(),
                    &mut present_state.borrow_mut(),
                    &mut refresh_state.borrow_mut(),
                )
            })
        })
    })
}

pub use crate::render::scheduler::is_render_frame_current;
