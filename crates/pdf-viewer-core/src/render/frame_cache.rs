use crate::render::tile_cache::{
    clear_frame_cache_keys, store_frame_cache_key, touch_frame_cache_key,
    FrameCacheStoreResult, HostFrameCacheState,
};
use crate::render::viewport_refresh::{
    resolve_viewport_refresh_decision, HostViewportRefreshState, ViewportRefreshDecision,
};

use crate::render::plan_builder::FramePlanResult;

pub fn resolve_viewport_refresh(
    refresh_state: &HostViewportRefreshState,
    frame_plan: &FramePlanResult,
    timestamp_ms: f64,
) -> ViewportRefreshDecision {
    resolve_viewport_refresh_decision(
        refresh_state,
        frame_plan.use_viewport_tile,
        frame_plan.use_viewport_tile && frame_plan.render_detail_layer,
        timestamp_ms,
    )
}

pub fn touch_frame_cache_entry(
    frame_cache_state: &mut HostFrameCacheState,
    is_detail: bool,
    key: &str,
) -> bool {
    if key.is_empty() {
        return false;
    }
    touch_frame_cache_key(frame_cache_state, is_detail, key)
}

pub fn store_frame_cache_entry(
    frame_cache_state: &mut HostFrameCacheState,
    is_detail: bool,
    key: String,
) -> FrameCacheStoreResult {
    if key.is_empty() {
        return FrameCacheStoreResult::default();
    }
    store_frame_cache_key(frame_cache_state, is_detail, key)
}

pub fn reset_frame_cache(frame_cache_state: &mut HostFrameCacheState) {
    clear_frame_cache_keys(frame_cache_state);
}
