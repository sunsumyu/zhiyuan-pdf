use serde::{Deserialize, Serialize};

use crate::present::plan::{preview_is_settled, quantize_cache_zoom, resolve_present_policy};
use crate::render::tile_cache::{
    build_base_cache_key, build_detail_cache_key, find_reusable_base_layer,
    find_reusable_detail_tile, HostPresentState,
};
use crate::utils::sanitize::{sanitize_non_negative, sanitize_positive};
use crate::viewer::session::HostViewerSession;
use crate::zoom::state::HostZoomState;

const PREVIEW_BASE_REFRESH_RATIO: f32 = 0.035;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderZoomRequest {
    pub display_zoom: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub device_pixel_ratio: f32,
    pub max_zoom: f32,
    pub max_canvas_dim: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderZoomResult {
    pub display_zoom: f32,
    pub render_zoom: f32,
    pub base_render_zoom: f32,
    pub css_scale: f32,
    pub use_viewport_tile: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FramePlanRequest {
    pub display_zoom: f32,
    pub render_reason: String,
    pub page_width: f32,
    pub page_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub device_pixel_ratio: f32,
    pub max_zoom: f32,
    pub max_canvas_dim: f32,
    pub timestamp_ms: f64,
    pub force_static_render_scale: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FramePlanResult {
    pub render_scene_key: String,
    pub render_reason: String,
    pub prepare_visible_layout: bool,
    pub display_zoom: f32,
    pub render_zoom: f32,
    pub base_render_zoom: f32,
    pub base_cache_zoom: f32,
    pub detail_cache_zoom: f32,
    pub base_cache_key: String,
    pub detail_cache_key: String,
    pub css_scale: f32,
    pub use_viewport_tile: bool,
    pub preview_settled: bool,
    pub allow_render_during_preview: bool,
    pub show_detail_overlay: bool,
    pub reuse_active_base_layer: bool,
    pub render_base_layer: bool,
    pub prefer_progressive_base: bool,
    pub reuse_active_detail_tile: bool,
    pub render_detail_layer: bool,
    pub prefer_progressive_detail: bool,
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub tile_left: f32,
    pub tile_top: f32,
    pub tile_width: f32,
    pub tile_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewportLayoutResult {
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewportTileResult {
    pub tile_left: f32,
    pub tile_top: f32,
    pub tile_width: f32,
    pub tile_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnchorViewportLayoutResult {
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
}

fn clamp_f32(value: f32, min_value: f32, max_value: f32) -> f32 {
    let min_value = if min_value.is_finite() {
        min_value
    } else {
        0.0
    };
    let max_value = if max_value.is_finite() && max_value >= min_value {
        max_value
    } else {
        min_value
    };
    if !value.is_finite() {
        return min_value;
    }
    if value < min_value {
        min_value
    } else if value > max_value {
        max_value
    } else {
        value
    }
}

fn centered_offset(content_size: f32, viewport_size: f32) -> f32 {
    ((viewport_size - content_size).max(0.0)) * 0.5
}

fn cache_zoom_ratio_delta(left: f32, right: f32) -> f32 {
    let safe_left = left.max(0.0001);
    let safe_right = right.max(0.0001);
    ((safe_left / safe_right) - 1.0).abs()
}

fn should_prepare_layout(render_reason: &str) -> bool {
    // Document mutations replace page content. Keep the old committed frame visible
    // until the new frame is ready, otherwise the host briefly shows old pixels in
    // a new layout during save/undo/redo/apply.
    render_reason.trim() != "documentMutation"
}

fn is_stable_document_frame(render_reason: &str) -> bool {
    matches!(
        render_reason.trim(),
        "editorVisibility" | "documentMutation"
    )
}

pub fn compute_viewport_layout_result(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> ViewportLayoutResult {
    let display_width = sanitize_positive(display_width, 1.0);
    let display_height = sanitize_positive(display_height, 1.0);
    let viewport_width = sanitize_non_negative(viewport_width, 0.0);
    let viewport_height = sanitize_non_negative(viewport_height, 0.0);
    ViewportLayoutResult {
        host_width: display_width.max(viewport_width),
        host_height: display_height.max(viewport_height),
        content_left: centered_offset(display_width, viewport_width),
        content_top: centered_offset(display_height, viewport_height),
    }
}

pub fn compute_viewport_tile_result(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
    scroll_left: f32,
    scroll_top: f32,
    content_left: f32,
    content_top: f32,
    overscan: f32,
) -> ViewportTileResult {
    let display_width = sanitize_positive(display_width, 1.0);
    let display_height = sanitize_positive(display_height, 1.0);
    let viewport_width = sanitize_positive(viewport_width, 1.0);
    let viewport_height = sanitize_positive(viewport_height, 1.0);
    let overscan = sanitize_non_negative(overscan, 0.0);
    let scroll_left = sanitize_non_negative(scroll_left, 0.0);
    let scroll_top = sanitize_non_negative(scroll_top, 0.0);
    let content_left = sanitize_non_negative(content_left, 0.0);
    let content_top = sanitize_non_negative(content_top, 0.0);
    let visible_left = clamp_f32(scroll_left - content_left, 0.0, display_width);
    let visible_top = clamp_f32(scroll_top - content_top, 0.0, display_height);
    let visible_right = clamp_f32(
        scroll_left + viewport_width - content_left,
        0.0,
        display_width,
    );
    let visible_bottom = clamp_f32(
        scroll_top + viewport_height - content_top,
        0.0,
        display_height,
    );
    let tile_left = (visible_left - overscan).max(0.0).floor();
    let tile_top = (visible_top - overscan).max(0.0).floor();
    let tile_right = (visible_right + overscan).min(display_width).ceil();
    let tile_bottom = (visible_bottom + overscan).min(display_height).ceil();
    ViewportTileResult {
        tile_left,
        tile_top,
        tile_width: (tile_right - tile_left).max(1.0),
        tile_height: (tile_bottom - tile_top).max(1.0),
    }
}

pub fn resolve_tile_overscan(viewport_width: f32, viewport_height: f32, display_zoom: f32) -> f32 {
    let viewport_extent = sanitize_positive(viewport_width.max(viewport_height), 1.0);
    let zoom = sanitize_positive(display_zoom, 1.0);
    let adaptive = if zoom >= 6.0 {
        viewport_extent * 1.15
    } else if zoom >= 3.0 {
        viewport_extent * 0.95
    } else if zoom >= 1.5 {
        viewport_extent * 0.8
    } else {
        viewport_extent * 0.65
    };
    adaptive.clamp(220.0, 960.0)
}

pub fn compute_anchor_viewport_layout_result(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
    anchor_page_x: f32,
    anchor_page_y: f32,
    page_width: f32,
    page_height: f32,
    viewport_x: f32,
    viewport_y: f32,
) -> AnchorViewportLayoutResult {
    let display_width = sanitize_positive(display_width, 1.0);
    let display_height = sanitize_positive(display_height, 1.0);
    let viewport_width = sanitize_positive(viewport_width, 1.0);
    let viewport_height = sanitize_positive(viewport_height, 1.0);
    let page_width = sanitize_positive(page_width, 1.0);
    let page_height = sanitize_positive(page_height, 1.0);
    let viewport_x = if viewport_x.is_finite() {
        viewport_x
    } else {
        0.0
    };
    let viewport_y = if viewport_y.is_finite() {
        viewport_y
    } else {
        0.0
    };
    let point_x = if page_width > 0.0 {
        clamp_f32(anchor_page_x, 0.0, page_width) * (display_width / page_width)
    } else {
        0.0
    };
    let point_y = if page_height > 0.0 {
        clamp_f32(anchor_page_y, 0.0, page_height) * (display_height / page_height)
    } else {
        0.0
    };
    let content_left = (viewport_x - point_x).max(0.0);
    let content_top = (viewport_y - point_y).max(0.0);
    let scroll_left = (content_left + point_x - viewport_x).max(0.0);
    let scroll_top = (content_top + point_y - viewport_y).max(0.0);
    AnchorViewportLayoutResult {
        host_width: (content_left + display_width)
            .max(scroll_left + viewport_width)
            .max(viewport_width),
        host_height: (content_top + display_height)
            .max(scroll_top + viewport_height)
            .max(viewport_height),
        content_left,
        content_top,
        scroll_left,
        scroll_top,
    }
}

pub fn compute_visible_content_rect(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
    scroll_left: f32,
    scroll_top: f32,
    content_left: f32,
    content_top: f32,
) -> (f32, f32, f32, f32) {
    let display_width = sanitize_positive(display_width, 1.0);
    let display_height = sanitize_positive(display_height, 1.0);
    let viewport_width = sanitize_positive(viewport_width, 1.0);
    let viewport_height = sanitize_positive(viewport_height, 1.0);
    let scroll_left = sanitize_non_negative(scroll_left, 0.0);
    let scroll_top = sanitize_non_negative(scroll_top, 0.0);
    let content_left = sanitize_non_negative(content_left, 0.0);
    let content_top = sanitize_non_negative(content_top, 0.0);
    let visible_left = clamp_f32(scroll_left - content_left, 0.0, display_width);
    let visible_top = clamp_f32(scroll_top - content_top, 0.0, display_height);
    let visible_right = clamp_f32(
        scroll_left + viewport_width - content_left,
        0.0,
        display_width,
    );
    let visible_bottom = clamp_f32(
        scroll_top + viewport_height - content_top,
        0.0,
        display_height,
    );
    (visible_left, visible_top, visible_right, visible_bottom)
}

pub fn resolve_render_zoom_result(request: &RenderZoomRequest) -> RenderZoomResult {
    let dpr = if request.device_pixel_ratio.is_finite() && request.device_pixel_ratio > 0.0 {
        request.device_pixel_ratio
    } else {
        1.0
    };
    let page_max = request.page_width.max(request.page_height).max(1.0);
    let max_canvas_dim = request.max_canvas_dim.max(1.0);
    let display_zoom = request.display_zoom.max(0.1).min(request.max_zoom.max(0.1));
    let safe_render_zoom = (max_canvas_dim / (page_max * dpr)).max(0.1);
    let use_viewport_tile = display_zoom > safe_render_zoom + 0.001;
    let render_zoom = if use_viewport_tile {
        display_zoom
    } else {
        display_zoom.min(safe_render_zoom)
    };
    let base_render_zoom = display_zoom.min(safe_render_zoom);
    let css_scale = if render_zoom > 0.0 && !use_viewport_tile {
        display_zoom / render_zoom
    } else {
        1.0
    };
    RenderZoomResult {
        display_zoom,
        render_zoom,
        base_render_zoom,
        css_scale,
        use_viewport_tile,
    }
}

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
