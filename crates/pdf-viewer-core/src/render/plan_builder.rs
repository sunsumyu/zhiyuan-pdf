use serde::{Deserialize, Serialize};

use crate::common::sanitize::{sanitize_non_negative, sanitize_positive};

pub const PREVIEW_BASE_REFRESH_RATIO: f32 = 0.035;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderZoomRequest {
    pub display_zoom: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub device_pixel_ratio: f32,
    pub max_zoom: f32,
    pub max_canvas_dim: f32,
    /// ADR-0012: a mid-gesture refresh only needs the visible viewport.
    /// A full-page render costs O(page × zoom²) of main-thread bitmap
    /// realloc + copy per wheel step; the viewport tile is
    /// O(viewport × dpr²) and renders at native resolution. The settle render
    /// (display_zoom == target_zoom) leaves this false and keeps the
    /// full-page path unchanged.
    #[serde(default)]
    pub prefer_viewport_tile: bool,
    /// ADR-0016: upper bound on the full-page base bitmap, in device pixels.
    /// The canvas-dim guard (`max_canvas_dim`) is a *memory* limit (10240px);
    /// it let a 5.28× settle render allocate a 3140×4427 ≈ 13.9M px bitmap,
    /// blocking the main thread 190–264ms. This budget is a *cost* limit:
    /// once `page × display_zoom² × dpr²` exceeds it, the base is clamped and
    /// the visible viewport is covered by the native-resolution detail tile.
    /// `0.0` disables the budget (legacy behaviour). Callers derive it from
    /// the viewport so it adapts to screen size and DPR.
    #[serde(default)]
    pub max_render_pixels: f32,
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

pub fn clamp_f32(value: f32, min_value: f32, max_value: f32) -> f32 {
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

pub fn centered_offset(content_size: f32, viewport_size: f32) -> f32 {
    ((viewport_size - content_size).max(0.0)) * 0.5
}

pub fn cache_zoom_ratio_delta(left: f32, right: f32) -> f32 {
    let safe_left = left.max(0.0001);
    let safe_right = right.max(0.0001);
    ((safe_left / safe_right) - 1.0).abs()
}

pub fn should_prepare_layout(render_reason: &str) -> bool {
    // Document mutations replace page content. Keep the old committed frame visible
    // until the new frame is ready, otherwise the host briefly shows old pixels in
    // a new layout during save/undo/redo/apply.
    render_reason.trim() != "documentMutation"
}

pub fn is_stable_document_frame(render_reason: &str) -> bool {
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
    // ADR-0016: the canvas-dim guard is a MEMORY limit — it still permits a
    // 13.9M px full-page bitmap at 5.28×, which costs 190–264ms of main-thread
    // realloc + copy on the settle render. The pixel budget is the COST limit:
    // the full-page base may not exceed `max_render_pixels`, so beyond it the
    // base is clamped (then CSS-scaled) and the viewport is covered by the
    // native-resolution detail tile — the same architecture already proven
    // above 9.7×. A non-positive budget disables the cap (legacy behaviour).
    let budget_zoom = if request.max_render_pixels.is_finite() && request.max_render_pixels > 0.0 {
        let page_px = request.page_width.max(1.0) * request.page_height.max(1.0) * dpr * dpr;
        (request.max_render_pixels / page_px).sqrt().max(0.1)
    } else {
        f32::INFINITY
    };
    let effective_cap = safe_render_zoom.min(budget_zoom);
    // ADR-0012: a zoom-gesture refresh must use the viewport tile on COST
    // grounds long before either cap — the user can only see the viewport, so
    // re-rendering the whole page per wheel step starves the main thread
    // (measured 100–126ms blocks).
    let use_viewport_tile =
        display_zoom > effective_cap + 0.001 || request.prefer_viewport_tile;
    let render_zoom = if use_viewport_tile {
        display_zoom
    } else {
        display_zoom.min(effective_cap)
    };
    let base_render_zoom = display_zoom.min(effective_cap);
    // ADR-0004 (revised): this ratio compensates canvas-dimension clamping —
    // when display_zoom exceeds the 10240px bitmap limit the canvas renders at
    // render_zoom and the DOM box must scale it back up (display/render).
    // Pinning to 1.0 collapses the canvas box and makes zoom jump violently.
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

#[cfg(test)]
mod budget_tests {
    use super::*;

    fn req(display_zoom: f32, max_render_pixels: f32) -> RenderZoomRequest {
        RenderZoomRequest {
            display_zoom,
            page_width: 595.0,
            page_height: 842.0,
            device_pixel_ratio: 1.0,
            max_zoom: 30.0,
            max_canvas_dim: 10240.0,
            prefer_viewport_tile: false,
            max_render_pixels,
        }
    }

    /// With the budget disabled the memory guard still allows a 5.28× settle
    /// render to allocate the full 3140×4427 ≈ 13.9M px bitmap — the 190–264ms
    /// freeze. This pins the legacy behaviour the budget must replace.
    #[test]
    fn disabled_budget_keeps_full_page_base_at_5x() {
        let result = resolve_render_zoom_result(&req(5.28, 0.0));
        assert!(!result.use_viewport_tile);
        assert!((result.base_render_zoom - 5.28).abs() < 0.001);
    }

    /// A 3M px budget flips the 5.28× settle render to the viewport-tile path
    /// and clamps the base so its bitmap never exceeds the budget.
    #[test]
    fn budget_flips_settle_render_and_clamps_base() {
        let result = resolve_render_zoom_result(&req(5.28, 3_000_000.0));
        assert!(result.use_viewport_tile, "5.28× must exceed the pixel budget");
        assert!(
            (result.render_zoom - 5.28).abs() < 0.001,
            "the detail tile still renders at native display zoom"
        );
        // base bitmap = page × base_render_zoom² must fit the budget.
        let base_px = 595.0 * 842.0 * result.base_render_zoom * result.base_render_zoom;
        assert!(
            base_px <= 3_000_000.0 + 1.0,
            "base bitmap {base_px} exceeds the budget"
        );
        assert!(
            (result.css_scale - 1.0).abs() < 0.001,
            "the viewport-tile path must not CSS-scale the base"
        );
    }

    /// Below the budget the settle render is byte-for-byte the legacy path.
    #[test]
    fn within_budget_settle_is_unchanged() {
        let result = resolve_render_zoom_result(&req(2.0, 3_000_000.0));
        assert!(!result.use_viewport_tile, "2.0× is under the 2.45× budget");
        assert!((result.base_render_zoom - 2.0).abs() < 0.001);
        assert!((result.render_zoom - 2.0).abs() < 0.001);
        assert!((result.css_scale - 1.0).abs() < 0.001);
    }

    /// The cap is the stricter of the memory guard and the pixel budget.
    #[test]
    fn budget_never_loosens_the_memory_guard() {
        // Tiny budget → budget binds; huge budget → memory guard binds.
        let tight = resolve_render_zoom_result(&req(20.0, 100_000.0));
        let loose = resolve_render_zoom_result(&req(20.0, 1_000_000_000.0));
        assert!(tight.base_render_zoom <= loose.base_render_zoom);
        // safe_render_zoom = 10240/842 ≈ 12.16 when the memory guard binds.
        assert!((loose.base_render_zoom - 10240.0 / 842.0).abs() < 0.01);
    }
}
