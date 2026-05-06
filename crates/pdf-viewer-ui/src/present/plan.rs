use serde::{Deserialize, Serialize};

const PREVIEW_SETTLED_EPSILON: f32 = 0.001;
const PREVIEW_BASE_LAYER_REUSE_RATIO: f32 = 0.28;
const PREVIEW_DETAIL_LAYER_REUSE_RATIO: f32 = 0.18;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresentPolicy {
    pub preview_settled: bool,
    pub snap_visual_zoom: bool,
    pub allow_render_during_preview: bool,
    pub show_detail_overlay: bool,
    pub reuse_active_base_layer: bool,
    pub render_base_layer: bool,
    pub prefer_progressive_base: bool,
    pub reuse_active_detail_tile: bool,
    pub render_detail_layer: bool,
    pub prefer_progressive_detail: bool,
}

pub fn resolve_present_policy(
    target_zoom: f32,
    visual_zoom: f32,
    use_viewport_tile: bool,
    has_pending_anchor: bool,
    has_reusable_base_layer: bool,
    has_reusable_detail_tile: bool,
) -> PresentPolicy {
    let preview_settled = (target_zoom - visual_zoom).abs() < PREVIEW_SETTLED_EPSILON;
    let preview_active = !preview_settled;
    let snap_visual_zoom = has_pending_anchor && !preview_settled;
    let reuse_active_base_layer = has_reusable_base_layer || preview_active;
    let render_base_layer = !reuse_active_base_layer && preview_settled;
    let reuse_active_detail_tile = use_viewport_tile && has_reusable_detail_tile;
    // When the base layer is being CSS-scaled during active preview, allow the
    // detail layer to refine the visible viewport in the background instead of
    // waiting for the zoom animation to fully settle.
    let render_detail_layer = use_viewport_tile && !has_reusable_detail_tile;
    let allow_render_during_preview = preview_active && (render_detail_layer || render_base_layer);
    let show_detail_overlay =
        use_viewport_tile && (has_reusable_detail_tile || render_detail_layer || preview_settled);
    PresentPolicy {
        preview_settled,
        snap_visual_zoom,
        allow_render_during_preview,
        show_detail_overlay,
        reuse_active_base_layer,
        render_base_layer,
        prefer_progressive_base: render_base_layer && preview_active,
        reuse_active_detail_tile,
        render_detail_layer,
        prefer_progressive_detail: render_detail_layer,
    }
}

pub fn preview_is_settled(target_zoom: f32, visual_zoom: f32) -> bool {
    (target_zoom - visual_zoom).abs() < PREVIEW_SETTLED_EPSILON
}

pub fn preview_base_layer_reuse_ratio() -> f32 {
    PREVIEW_BASE_LAYER_REUSE_RATIO
}

pub fn preview_detail_layer_reuse_ratio() -> f32 {
    PREVIEW_DETAIL_LAYER_REUSE_RATIO
}

pub fn quantize_cache_zoom(zoom: f32, use_viewport_tile: bool) -> f32 {
    let safe_zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let step = if use_viewport_tile {
        if safe_zoom >= 8.0 {
            0.30
        } else if safe_zoom >= 4.0 {
            0.18
        } else if safe_zoom >= 2.0 {
            0.10
        } else {
            0.05
        }
    } else if safe_zoom >= 3.0 {
        0.12
    } else if safe_zoom >= 1.5 {
        0.06
    } else {
        0.03
    };

    (safe_zoom / step).round() * step
}
