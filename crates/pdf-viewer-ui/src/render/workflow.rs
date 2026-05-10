// Re-export pure data structures and functions from core.
pub use pdf_viewer_core::render::workflow::*;

use crate::render::render_store::{
    settle_render_frame, RenderFrameTransition as HostRenderFrameTransition,
};
use crate::render::tile_cache::{
    clear_detail_tiles, remember_base_layer, remember_detail_tile, BaseLayerCacheEntry,
    DetailTileCacheEntry, HostPresentState,
};
use crate::viewport_refresh::{note_viewport_render_commit, HostViewportRefreshState};
use crate::zoom::interaction::commit_rendered_zoom;
use crate::zoom::zoom_store::HostZoomState;

use crate::present::plan_builder::FramePlanResult;

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

