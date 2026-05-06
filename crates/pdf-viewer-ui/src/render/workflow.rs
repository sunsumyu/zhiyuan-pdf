use serde::{Deserialize, Serialize};

use crate::render::progressive::{ProgressiveRenderStart, ProgressiveRenderStep};
use crate::render::scheduler::{
    settle_render_frame, RenderFrameTransition as HostRenderFrameTransition,
};
use crate::render::tile_cache::{
    clear_detail_tiles, remember_base_layer, remember_detail_tile, BaseLayerCacheEntry,
    DetailTileCacheEntry, HostPresentState,
};
use crate::viewport_refresh::{note_viewport_render_commit, HostViewportRefreshState};
use crate::zoom::interaction::commit_rendered_zoom;
use crate::zoom::state::HostZoomState;

use crate::present::plan_builder::FramePlanResult;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderFrameEnvelope {
    pub frame_token: u32,
    pub frame_plan: FramePlanResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderFrameTransition {
    pub accepted: bool,
    pub next_frame: Option<RenderFrameEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProgressiveRenderStartResult {
    pub started: bool,
    pub total_items: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProgressiveRenderStepResult {
    pub active: bool,
    pub completed: bool,
    pub processed_items: u32,
    pub remaining_items: u32,
}

pub fn build_render_frame_envelope(
    frame_token: u32,
    frame_plan: FramePlanResult,
) -> RenderFrameEnvelope {
    RenderFrameEnvelope {
        frame_token,
        frame_plan,
    }
}

pub fn frame_plan_requires_render(frame_plan: &FramePlanResult) -> bool {
    frame_plan.render_base_layer || frame_plan.render_detail_layer
}

pub fn frame_plan_needs_viewport_refresh(frame_plan: &FramePlanResult) -> bool {
    frame_plan.use_viewport_tile && frame_plan.render_detail_layer
}

pub fn frame_plans_share_render_work(left: &FramePlanResult, right: &FramePlanResult) -> bool {
    left.render_reason == right.render_reason
        && left.render_base_layer == right.render_base_layer
        && left.render_detail_layer == right.render_detail_layer
        && left.use_viewport_tile == right.use_viewport_tile
        && left.base_cache_key == right.base_cache_key
        && left.detail_cache_key == right.detail_cache_key
}

pub fn settle_render_frame_inner(
    frame_token: u32,
    maybe_rendered_zoom: Option<f32>,
    zoom_state: &mut HostZoomState,
    present_state: &mut HostPresentState,
    viewport_refresh_state: &mut HostViewportRefreshState,
) -> RenderFrameTransition {
    let transition: HostRenderFrameTransition<FramePlanResult> =
        settle_render_frame(frame_token, |plan_value| {
            serde_json::from_value::<FramePlanResult>(plan_value.clone()).ok()
        });
    let accepted = transition.accepted;
    let settled_frame_plan = transition.settled_frame_plan;
    let next_frame = transition
        .next_frame
        .map(|frame| build_render_frame_envelope(frame.frame_token, frame.frame_plan));

    if accepted {
        if let Some(rendered_zoom) = maybe_rendered_zoom {
            commit_rendered_zoom(zoom_state, rendered_zoom);
        }
        note_viewport_render_commit(viewport_refresh_state, js_sys::Date::now());
        if let Some(frame_plan) = settled_frame_plan {
            if frame_plan.render_base_layer || frame_plan.reuse_active_base_layer {
                remember_base_layer(
                    present_state,
                    BaseLayerCacheEntry {
                        key: frame_plan.base_cache_key.clone(),
                        cache_zoom: frame_plan.base_cache_zoom,
                        scene_key: frame_plan.render_scene_key.clone(),
                    },
                );
            }
            if frame_plan.show_detail_overlay && frame_plan.use_viewport_tile {
                remember_detail_tile(
                    present_state,
                    DetailTileCacheEntry {
                        key: frame_plan.detail_cache_key.clone(),
                        cache_zoom: frame_plan.detail_cache_zoom,
                        scene_key: frame_plan.render_scene_key.clone(),
                        tile_left: frame_plan.tile_left,
                        tile_top: frame_plan.tile_top,
                        tile_width: frame_plan.tile_width,
                        tile_height: frame_plan.tile_height,
                    },
                );
            } else if !frame_plan.use_viewport_tile {
                clear_detail_tiles(present_state);
            }
        }
    }

    RenderFrameTransition {
        accepted,
        next_frame,
    }
}

pub fn progressive_start_result(start: ProgressiveRenderStart) -> ProgressiveRenderStartResult {
    ProgressiveRenderStartResult {
        started: start.started,
        total_items: start.total_items.min(u32::MAX as usize) as u32,
    }
}

pub fn progressive_step_result(step: ProgressiveRenderStep) -> ProgressiveRenderStepResult {
    ProgressiveRenderStepResult {
        active: step.active,
        completed: step.completed,
        processed_items: step.processed_items.min(u32::MAX as usize) as u32,
        remaining_items: step.remaining_items.min(u32::MAX as usize) as u32,
    }
}
