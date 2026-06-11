// Re-export pure data structures and functions from core.
pub use pdf_viewer_core::render::plan_builder::*;

use crate::present::plan::{preview_is_settled, quantize_cache_zoom, resolve_present_policy};
use crate::render::tile_cache::{
    build_base_cache_key, build_detail_cache_key, find_reusable_base_layer,
    find_reusable_detail_tile, HostPresentState,
};
use crate::common::sanitize::sanitize_positive;
use crate::viewer::viewer_store::HostViewerSession;
use crate::zoom::zoom_store::HostZoomState;

fn resolve_anchor_layout_from_zoom_state(
    zoom_state: &mut HostZoomState,
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
    consume_anchor: bool,
) -> Option<AnchorViewportLayoutResult> {
    if consume_anchor {
        zoom_state.pending_anchor.take().map(|anchor| {
            compute_anchor_viewport_layout_result(
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
            )
        })
    } else {
        zoom_state.pending_anchor.as_ref().map(|anchor| {
            compute_anchor_viewport_layout_result(
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
            )
        })
    }
}

pub fn build_frame_plan_result(
    request: &FramePlanRequest,
    zoom_state: &mut HostZoomState,
    viewer_session: &HostViewerSession,
    present_state: &HostPresentState,
    render_scene_key: &str,
    consume_anchor: bool,
) -> FramePlanResult {
    log::info!(
        "[PAGE-SIZE] plan_builder: build_frame_plan_result called. Width={}, Height={}",
        request.page_width,
        request.page_height
    );
    let render_reason = if request.render_reason.trim().is_empty() {
        "default".to_string()
    } else {
        request.render_reason.clone()
    };
    let stable_document_frame = is_stable_document_frame(&render_reason);
    let render = resolve_render_zoom_result(&RenderZoomRequest {
        display_zoom: request.display_zoom,
        page_width: request.page_width,
        page_height: request.page_height,
        device_pixel_ratio: request.device_pixel_ratio,
        max_zoom: request.max_zoom,
        max_canvas_dim: request.max_canvas_dim,
    });
    let display_width = request.page_width.max(1.0) * render.display_zoom.max(0.1);
    let display_height = request.page_height.max(1.0) * render.display_zoom.max(0.1);
    let viewport_width = request.viewport_width.max(0.0);
    let viewport_height = request.viewport_height.max(0.0);

    let (host_width, host_height, content_left, content_top, scroll_left, scroll_top) =
        if let Some(anchor_layout) = resolve_anchor_layout_from_zoom_state(
            zoom_state,
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            consume_anchor,
        ) {
            (
                anchor_layout.host_width,
                anchor_layout.host_height,
                anchor_layout.content_left,
                anchor_layout.content_top,
                anchor_layout.scroll_left,
                anchor_layout.scroll_top,
            )
        } else {
            let layout = compute_viewport_layout_result(
                display_width,
                display_height,
                viewport_width,
                viewport_height,
            );
            (
                layout.host_width,
                layout.host_height,
                layout.content_left,
                layout.content_top,
                request.scroll_left.max(0.0),
                request.scroll_top.max(0.0),
            )
        };

    let target_zoom = sanitize_positive(zoom_state.target_zoom, render.display_zoom);
    let visual_zoom = sanitize_positive(zoom_state.visual_zoom, render.display_zoom);
    let preview_settled = preview_is_settled(target_zoom, visual_zoom);
    let base_cache_zoom = quantize_cache_zoom(render.base_render_zoom, false);
    let detail_cache_zoom = quantize_cache_zoom(render.render_zoom, render.use_viewport_tile);
    let (visible_left, visible_top, visible_right, visible_bottom) = compute_visible_content_rect(
        display_width,
        display_height,
        viewport_width,
        viewport_height,
        scroll_left,
        scroll_top,
        content_left,
        content_top,
    );

    let reusable_detail_tile = if render.use_viewport_tile && !stable_document_frame {
        find_reusable_detail_tile(
            present_state,
            render_scene_key,
            detail_cache_zoom,
            visible_left,
            visible_top,
            visible_right,
            visible_bottom,
            preview_settled,
        )
    } else {
        None
    };

    let reusable_base_layer = find_reusable_base_layer(
        present_state,
        render_scene_key,
        base_cache_zoom,
        preview_settled,
    );

    let requires_preview_base_refresh = !preview_settled
        && !render.use_viewport_tile
        && reusable_base_layer
            .as_ref()
            .map(|layer| {
                cache_zoom_ratio_delta(layer.cache_zoom, base_cache_zoom)
                    > PREVIEW_BASE_REFRESH_RATIO
            })
            .unwrap_or(true);

    let effective_base_cache_zoom = if requires_preview_base_refresh {
        base_cache_zoom
    } else {
        reusable_base_layer
            .as_ref()
            .map(|layer| layer.cache_zoom)
            .unwrap_or(base_cache_zoom)
    };
    let effective_detail_cache_zoom = reusable_detail_tile
        .as_ref()
        .map(|tile| tile.cache_zoom)
        .unwrap_or(detail_cache_zoom);

    let tile = if let Some(active_tile) = reusable_detail_tile.as_ref() {
        ViewportTileResult {
            tile_left: active_tile.tile_left,
            tile_top: active_tile.tile_top,
            tile_width: active_tile.tile_width,
            tile_height: active_tile.tile_height,
        }
    } else if render.use_viewport_tile {
        compute_viewport_tile_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            scroll_left,
            scroll_top,
            content_left,
            content_top,
            resolve_tile_overscan(viewport_width, viewport_height, render.display_zoom),
        )
    } else {
        ViewportTileResult {
            tile_left: 0.0,
            tile_top: 0.0,
            tile_width: display_width.max(1.0),
            tile_height: display_height.max(1.0),
        }
    };

    let present_policy = resolve_present_policy(
        target_zoom,
        visual_zoom,
        render.use_viewport_tile,
        zoom_state.pending_anchor.is_some(),
        reusable_base_layer.is_some(),
        reusable_detail_tile.is_some(),
    );

    let render_base_layer = if stable_document_frame {
        true
    } else {
        present_policy.render_base_layer || requires_preview_base_refresh
    };
    let allow_render_during_preview = if stable_document_frame {
        false
    } else {
        present_policy.allow_render_during_preview || requires_preview_base_refresh
    };
    let prefer_progressive_base = if stable_document_frame {
        false
    } else {
        present_policy.prefer_progressive_base || requires_preview_base_refresh
    };
    let show_detail_overlay = if stable_document_frame {
        false
    } else {
        present_policy.show_detail_overlay
    };
    let reuse_active_base_layer = if stable_document_frame {
        false
    } else {
        present_policy.reuse_active_base_layer
    };
    let reuse_active_detail_tile = if stable_document_frame {
        false
    } else {
        present_policy.reuse_active_detail_tile
    };
    let render_detail_layer = if stable_document_frame {
        false
    } else {
        present_policy.render_detail_layer
    };
    let prefer_progressive_detail = if stable_document_frame {
        false
    } else {
        present_policy.prefer_progressive_detail
    };

    let session_path = viewer_session
        .path
        .clone()
        .unwrap_or_else(|| "__unknown__".to_string());
    let session_page = viewer_session.current_page;
    let base_cache_key = if requires_preview_base_refresh {
        build_base_cache_key(
            &session_path,
            session_page,
            render_scene_key,
            effective_base_cache_zoom,
            request.device_pixel_ratio,
        )
    } else {
        reusable_base_layer
            .as_ref()
            .map(|layer| layer.key.clone())
            .unwrap_or_else(|| {
                build_base_cache_key(
                    &session_path,
                    session_page,
                    render_scene_key,
                    effective_base_cache_zoom,
                    request.device_pixel_ratio,
                )
            })
    };
    let detail_cache_key = reusable_detail_tile
        .as_ref()
        .map(|tile| tile.key.clone())
        .unwrap_or_else(|| {
            build_detail_cache_key(
                &session_path,
                session_page,
                render_scene_key,
                effective_detail_cache_zoom,
                request.device_pixel_ratio,
                tile.tile_left,
                tile.tile_top,
                tile.tile_width,
                tile.tile_height,
            )
        });

    FramePlanResult {
        render_scene_key: render_scene_key.to_string(),
        prepare_visible_layout: should_prepare_layout(&render_reason),
        render_reason,
        display_zoom: render.display_zoom,
        render_zoom: render.render_zoom,
        base_render_zoom: render.base_render_zoom,
        base_cache_zoom: effective_base_cache_zoom,
        detail_cache_zoom: effective_detail_cache_zoom,
        base_cache_key,
        detail_cache_key,
        css_scale: render.css_scale,
        use_viewport_tile: render.use_viewport_tile,
        preview_settled: present_policy.preview_settled,
        allow_render_during_preview,
        show_detail_overlay,
        reuse_active_base_layer,
        render_base_layer,
        prefer_progressive_base,
        reuse_active_detail_tile,
        render_detail_layer,
        prefer_progressive_detail,
        host_width,
        host_height,
        content_left,
        content_top,
        scroll_left,
        scroll_top,
        tile_left: tile.tile_left,
        tile_top: tile.tile_top,
        tile_width: tile.tile_width,
        tile_height: tile.tile_height,
    }
}
