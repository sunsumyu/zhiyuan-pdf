use crate::editor::session::render_scene_key;
use crate::present::plan_builder::{
    build_frame_plan_result as inner_build_plan_result, FramePlanRequest, FramePlanResult,
};
use crate::render::frame_cache::{
    reset_frame_cache as inner_reset_frame_cache,
    resolve_viewport_refresh as inner_resolve_viewport_refresh,
    store_frame_cache_entry as inner_store_cache_entry,
    touch_frame_cache_entry as inner_touch_cache_entry,
};
use crate::render::render_store::{
    schedule_render_frame, RenderFrameEnvelope as HostRenderFrameEnvelope,
};
use crate::render::tile_cache::{FrameCacheStoreResult, HostFrameCacheState, HostPresentState};
use crate::render::workflow::{
    build_render_frame_envelope, frame_plan_requires_render, frame_plans_share_render_work,
    settle_render_frame_inner, RenderFrameEnvelope, RenderFrameTransition,
};
use crate::viewer::viewer_store;
use crate::viewport_refresh::{HostViewportRefreshState, ViewportRefreshDecision};
use crate::zoom::zoom_store;

use crate::app_context;

pub fn with_present_state<R>(f: impl FnOnce(&HostPresentState) -> R) -> R {
    app_context::with_present(f)
}

pub fn build_plan_result(
    request: &FramePlanRequest,
    consume_anchor: bool,
) -> FramePlanResult {
    let viewer_session = viewer_store::read_viewer_session();
    let scene_key = render_scene_key();
    let present_snapshot = app_context::with_present(Clone::clone);
    zoom_store::with_zoom_state_mut(|zoom_state| {
        inner_build_plan_result(
            request,
            zoom_state,
            &viewer_session,
            &present_snapshot,
            &scene_key,
            consume_anchor,
        )
    })
}

pub fn resolve_viewport_refresh(request: &FramePlanRequest) -> ViewportRefreshDecision {
    let frame_plan = build_plan_result(request, false);
    app_context::with_viewport_refresh(|viewport_refresh| {
        inner_resolve_viewport_refresh(viewport_refresh, &frame_plan, request.timestamp_ms)
    })
}

pub fn touch_cache_entry(is_detail: bool, key: &str) -> bool {
    app_context::with_frame_cache_mut(|frame_cache| {
        inner_touch_cache_entry(frame_cache, is_detail, key)
    })
}

pub fn store_cache_entry(is_detail: bool, key: String) -> FrameCacheStoreResult {
    app_context::with_frame_cache_mut(|frame_cache| {
        inner_store_cache_entry(frame_cache, is_detail, key)
    })
}

pub fn reset_frame_cache() {
    app_context::with_frame_cache_mut(|frame_cache| {
        inner_reset_frame_cache(frame_cache);
    });
}

pub fn reset_present_runtime(reset_cache: bool, reset_refresh: bool) {
    app_context::with_present_runtime_mut(|present, frame_cache, viewport_refresh| {
        *present = HostPresentState::default();
        if reset_cache {
            *frame_cache = HostFrameCacheState::default();
        }
        if reset_refresh {
            *viewport_refresh = HostViewportRefreshState::default();
        }
    });
}

pub fn schedule_request(request: &FramePlanRequest) -> Option<RenderFrameEnvelope> {
    let frame_plan = build_plan_result(request, false);
    eprintln!(
        "[DEBUG-RENDER] schedule_request: render_reason={}, render_base_layer={}, render_detail_layer={}, display_zoom={}, base_cache_key={}",
        frame_plan.render_reason,
        frame_plan.render_base_layer,
        frame_plan.render_detail_layer,
        frame_plan.display_zoom,
        frame_plan.base_cache_key
    );
    // Editor-driven renders carry a fresh scene_revision per keystroke, so any
    // pending in-flight frame is stale and will never be committed by JS (the
    // active token is overwritten before progressive completes). To avoid
    // queueing forever and deadlocking subsequent schedules, settle the stale
    // in-flight token here before scheduling. This is safe because the new
    // frame supersedes the old visually.
    if frame_plan.render_reason == "editorVisibility" {
        let stale_token = app_context::with_render(|s| {
            if s.in_flight_frame_token != 0 && s.active_frame_token != s.in_flight_frame_token {
                s.in_flight_frame_token
            } else {
                0
            }
        });
        if stale_token != 0 {
            crate::chain_trace!(
                "schedule.evict-stale-in-flight",
                "token" => stale_token,
            );
            let _ = settle_render_frame(stale_token, None);
        }
    }
    let envelope: Option<HostRenderFrameEnvelope<FramePlanResult>> = schedule_render_frame(
        &frame_plan,
        frame_plan_requires_render,
        frame_plans_share_render_work,
        |plan| serde_json::to_value(plan).unwrap_or(serde_json::Value::Null),
        |plan_value| serde_json::from_value::<FramePlanResult>(plan_value.clone()).ok(),
    );
    envelope.map(|frame| build_render_frame_envelope(frame.frame_token, frame.frame_plan))
}

pub fn commit_render_frame(frame_token: u32, rendered_zoom: f32) -> bool {
    let transition = crate::render::render_store::settle_render_frame(frame_token, |plan_value| {
        serde_json::from_value::<FramePlanResult>(plan_value.clone()).ok()
    });
    let accepted = transition.accepted;
    zoom_store::with_zoom_state_mut(|zoom_state| {
        app_context::with_present_and_viewport_refresh_mut(|present, viewport_refresh| {
            settle_render_frame_inner(
                transition,
                Some(rendered_zoom),
                zoom_state,
                present,
                viewport_refresh,
            );
        })
    });
    accepted
}

pub fn settle_render_frame(
    frame_token: u32,
    maybe_rendered_zoom: Option<f32>,
) -> RenderFrameTransition {
    let transition = crate::render::render_store::settle_render_frame(frame_token, |plan_value| {
        serde_json::from_value::<FramePlanResult>(plan_value.clone()).ok()
    });
    zoom_store::with_zoom_state_mut(|zoom_state| {
        app_context::with_present_and_viewport_refresh_mut(|present, viewport_refresh| {
            settle_render_frame_inner(
                transition,
                maybe_rendered_zoom,
                zoom_state,
                present,
                viewport_refresh,
            )
        })
    })
}

pub use crate::render::render_store::is_frame_current as is_render_frame_current;

// Frame cache entry helpers (re-exported with different names for free_api compatibility)
pub fn touch_frame_cache_entry(is_detail: bool, key: &str) -> bool {
    touch_cache_entry(is_detail, key)
}

pub fn store_frame_cache_entry(is_detail: bool, key: String) -> FrameCacheStoreResult {
    store_cache_entry(is_detail, key)
}

/// Alias used by render_transaction and other callers that imported `schedule_render_frame_request`.
pub fn schedule_render_frame_request(request: &FramePlanRequest) -> Option<RenderFrameEnvelope> {
    schedule_request(request)
}

/// Alias used by free_api that imported `build_frame_plan_result`.
pub fn build_frame_plan_result(request: &FramePlanRequest, consume_anchor: bool) -> FramePlanResult {
    build_plan_result(request, consume_anchor)
}
