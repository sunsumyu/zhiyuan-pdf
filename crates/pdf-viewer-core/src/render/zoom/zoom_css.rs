//! CSS transform computation, canvas box geometry, and layout helpers.
//!
//! Pure logic — no DOM, no WASM, no thread_local.

use serde::{Deserialize, Serialize};

/// Mode for the CSS transform applied to the vector container.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CssTransformMode {
    /// No transform needed — cssScale ≈ 1.0 and no translate.
    Idle,
    /// Preview animation in progress — CSS stretch while bitmap catches up.
    Preview,
    /// Frame committed but preview still running — keep CSS scale.
    Committed,
}

/// Result of `resolve_css_transform`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssTransformDecision {
    pub css_scale: f32,
    pub mode: CssTransformMode,
}

const CSS_SCALE_IDLE_EPSILON: f32 = 0.001;

/// Decide the CSS transform for the vector container.
///
/// Replaces the three independent cssScale computations that were in TS:
///   1. `applyVisualZoomPreview` (line 247): `previewZoom / lastRenderedZoom`
///   2. `restorePendingAnchor` (line 452): `targetZoom / renderedZoom` + 0.001 threshold
///   3. `applyCommittedFrame` (line 181): mode choice based on `wheelZoomRafId`
pub fn resolve_css_transform(request: CssTransformRequest) -> CssTransformDecision {
    let base = if request.last_rendered_zoom > 0.0 {
        request.last_rendered_zoom
    } else {
        1.0
    };
    let css_scale = request.preview_zoom / base;
    let mode = if request.preview_active {
        CssTransformMode::Preview
    } else if (css_scale - 1.0).abs() < CSS_SCALE_IDLE_EPSILON {
        CssTransformMode::Idle
    } else {
        CssTransformMode::Committed
    };
    CssTransformDecision { css_scale, mode }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CssTransformRequest {
    pub preview_zoom: f32,
    pub last_rendered_zoom: f32,
    /// Whether the RAF preview loop is active (replaces TS's `wheelZoomRafId !== null`).
    pub preview_active: bool,
}

// ─── Wheel request params ───────────────────────────────────────────────────
//
// Single-owner: ALL wheel-zoom parameter computation lives here.
// TS采集 DOM 事件原始值后传入，Rust 计算派生值并返回。

/// Result of `resolve_wheel_request_params`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WheelRequestParams {
    pub content_width: f32,
    pub content_height: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

/// Compute derived wheel-zoom parameters from page dimensions and current zoom.
///
/// Replaces the TS-side `contentWidth = pageWidth * currentDisplayZoom` and
/// hardcoded `minZoom: 0.1` in `bindWheelZoom`.
pub fn resolve_wheel_request_params(
    request: WheelRequestParamsRequest,
) -> WheelRequestParams {
    let current_display_zoom = if request.current_display_zoom > 0.0 {
        request.current_display_zoom
    } else {
        1.0
    };
    let page_width = if request.page_width > 0.0 { request.page_width } else { 1.0 };
    let page_height = if request.page_height > 0.0 { request.page_height } else { 1.0 };
    let min_zoom = 0.1_f32;
    let max_zoom = request.max_zoom.max(min_zoom);
    WheelRequestParams {
        content_width: page_width * current_display_zoom,
        content_height: page_height * current_display_zoom,
        min_zoom,
        max_zoom,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelRequestParamsRequest {
    pub page_width: f32,
    pub page_height: f32,
    pub current_display_zoom: f32,
    pub max_zoom: f32,
}

// ─── Layout fallback ────────────────────────────────────────────────────────
//
// Single-owner: ALL layout fallback computation lives here.
// When `syncHostLayout` returns None/missing fields, TS should not recompute
// domain values — it should call this function.

/// Complete fallback layout values when `syncHostLayout` returns partial data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutFallback {
    pub dom_width: f32,
    pub dom_height: f32,
    pub display_width: f32,
    pub display_height: f32,
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
    pub css_scale: f32,
}

/// Compute fallback layout dimensions when the WASM `syncHostLayout` call
/// fails or returns incomplete data.
///
/// Replaces the TS-side fallback formulas in `pdf_layout_sync.ts:66-74`.
pub fn resolve_layout_fallback(request: LayoutFallbackRequest) -> LayoutFallback {
    let page_width = if request.page_width > 0.0 { request.page_width } else { 1.0 };
    let page_height = if request.page_height > 0.0 { request.page_height } else { 1.0 };
    let rendered_zoom = if request.rendered_zoom > 0.0 {
        request.rendered_zoom
    } else {
        request.display_zoom
    };
    let display_zoom = if request.display_zoom > 0.0 { request.display_zoom } else { 1.0 };
    let dom_width = page_width * rendered_zoom;
    let dom_height = page_height * rendered_zoom;
    let display_width = page_width * display_zoom;
    let display_height = page_height * display_zoom;
    let css_scale = display_zoom / rendered_zoom;
    LayoutFallback {
        dom_width,
        dom_height,
        display_width,
        display_height,
        host_width: display_width,
        host_height: display_height,
        content_left: 0.0,
        content_top: 0.0,
        css_scale,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LayoutFallbackRequest {
    pub page_width: f32,
    pub page_height: f32,
    pub display_zoom: f32,
    pub rendered_zoom: f32,
}

// ─── Zoom limits constants ──────────────────────────────────────────────────
//
// Single-owner: These constants are defined in Rust and exported to TS.
// TS must NOT define its own MIN_ZOOM / MAX_ZOOM.

pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 30.0;

// ─── Fit-to-width computation ───────────────────────────────────────────────
//
// Single-owner: ALL fit-to-width logic lives here.
// TS采集 viewport/page 尺寸后传入，Rust 计算缩放值并返回。

/// Result of `resolve_fit_to_width`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FitToWidthResult {
    /// The computed fit-to-width zoom value, clamped to [MIN_ZOOM, MAX_ZOOM].
    pub fit_zoom: f32,
    /// Whether the page is wider than the viewport (i.e., fit-to-width applies).
    pub should_fit: bool,
}

/// Compute the fit-to-width zoom level for a document.
///
/// Replaces the TS-side `clampZoom(vpWidth / pageWidth)` in `pdf_runtime.ts`.
/// Returns `should_fit: false` when the page already fits the viewport.
pub fn resolve_fit_to_width(
    viewport_width: f32,
    page_width: f32,
) -> FitToWidthResult {
    let vp = if viewport_width > 0.0 { viewport_width } else { 1.0 };
    let pw = if page_width > 0.0 { page_width } else { 1.0 };
    if pw <= vp {
        return FitToWidthResult {
            fit_zoom: 1.0,
            should_fit: false,
        };
    }
    let raw = vp / pw;
    let fit_zoom = raw.max(MIN_ZOOM).min(MAX_ZOOM);
    FitToWidthResult {
        fit_zoom,
        should_fit: true,
    }
}

// ─── CSS transform string ─────────────────────────────────────────────────
//
// Single-owner: ALL CSS transform string computation lives here.
// TS must NOT format transform strings directly.

/// Compute the full CSS transform string for the vector container.
///
/// Replaces the TS-side `applyZoomTransform()` string formatting.
/// Returns an empty string when no transform is needed (scale ≈ 1.0 and no translate).
pub fn resolve_css_transform_string(
    translate_x: f32,
    translate_y: f32,
    css_scale: f32,
) -> String {
    let has_translate = translate_x.abs() >= 0.01 || translate_y.abs() >= 0.01;
    let has_scale = (css_scale - 1.0).abs() >= 0.001;
    if !has_scale && !has_translate {
        String::new()
    } else if !has_translate {
        format!("scale({})", css_scale)
    } else if !has_scale {
        format!("translate3d({:.2}px, {:.2}px, 0)", translate_x, translate_y)
    } else {
        format!(
            "translate3d({:.2}px, {:.2}px, 0) scale({})",
            translate_x, translate_y, css_scale
        )
    }
}

// ─── Canvas CSS box computation ───────────────────────────────────────────
//
// Single-owner: ALL canvas CSS box logic lives here.
// TS must NOT compute domWidth/domHeight/baseScale directly.

/// Compute the canvas CSS box dimensions from display/zoom values.
///
/// Replaces the TS-side formula in `vector_canvas_host.ts`:
/// `domWidth = (displayWidth / displayZoom) * baseRenderZoom`
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CanvasCssBox {
    pub dom_width: f32,
    pub dom_height: f32,
    pub base_scale: f32,
}

pub fn resolve_canvas_css_box(
    display_zoom: f32,
    base_render_zoom: f32,
    display_width: f32,
    display_height: f32,
) -> CanvasCssBox {
    let safe_display_zoom = if display_zoom > 0.0001 { display_zoom } else { 1.0 };
    let safe_base_render_zoom = if base_render_zoom > 0.0001 { base_render_zoom } else { 0.1 };

    let dom_width = (display_width / safe_display_zoom) * safe_base_render_zoom;
    let dom_height = (display_height / safe_display_zoom) * safe_base_render_zoom;
    let base_scale = safe_base_render_zoom / safe_display_zoom;

    CanvasCssBox {
        dom_width,
        dom_height,
        base_scale,
    }
}

// ─── Immediate mutation check ─────────────────────────────────────────────
//
// Single-owner: ALL render-reason classification lives here.
// TS must NOT check renderReason strings directly.

/// Check if a render reason indicates an immediate (non-preview) mutation.
///
/// Replaces the TS-side `isImmediateMutationFrame()` in `zoom_controller.ts`.
pub fn is_immediate_mutation_frame(render_reason: &str) -> bool {
    render_reason == "editorVisibility" || render_reason == "documentMutation"
}

// ─── Settled transition transform ──────────────────────────────────────────
//
// Single-owner: ALL preview→settled transition logic lives here.
// When the preview settles and a committed frame arrives, the DOM transitions
// from "frozen scroll + CSS translate" to "correct scroll + no CSS translate".
// This function computes the CSS translate that makes this transition seamless.

/// Result of `resolve_settled_transform`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettledTransformResult {
    /// CSS translateX to apply AFTER syncLayoutBox, before scroll update.
    pub translate_x: f32,
    /// CSS translateY to apply AFTER syncLayoutBox, before scroll update.
    pub translate_y: f32,
    /// The settled scroll_left (from anchor at settled zoom).
    pub scroll_left: f32,
    /// The settled scroll_top (from anchor at settled zoom).
    pub scroll_top: f32,
}

/// Compute the CSS translate needed for a seamless preview→settled transition.
///
/// During preview, the DOM scroll is frozen and a CSS translate compensates.
/// When the preview settles, we need to:
///   1. Update container dimensions (syncLayoutBox)
///   2. Apply a CSS translate that keeps the visual position stable
///   3. Update the scroll to the anchor-computed values
///   4. Clear the CSS translate
///
/// Steps 2-3-4 must be atomic (same DOM frame). This function computes the
/// translate for step 2, given the current preview state and the settled values.
///
/// # Arguments
/// * `preview_content_left` — container `left` during preview (from `visual_layout`)
/// * `preview_content_top` — container `top` during preview (from `visual_layout`)
/// * `preview_scroll_left` — DOM scrollLeft during preview (frozen)
/// * `preview_scroll_top` — DOM scrollTop during preview (frozen)
/// * `preview_css_scale` — CSS scale during preview (`visual_zoom / last_rendered_zoom`)
/// * `settled_content_left` — new container `left` at settled zoom (from anchor layout)
/// * `settled_content_top` — new container `top` at settled zoom (from anchor layout)
/// * `settled_scroll_left` — target scrollLeft at settled zoom (from anchor)
/// * `settled_scroll_top` — target scrollTop at settled zoom (from anchor)
pub fn resolve_settled_transform(
    preview_content_left: f32,
    preview_content_top: f32,
    preview_scroll_left: f32,
    preview_scroll_top: f32,
    preview_css_scale: f32,
    settled_content_left: f32,
    settled_content_top: f32,
    settled_scroll_left: f32,
    settled_scroll_top: f32,
) -> SettledTransformResult {
    let safe_scale = if preview_css_scale > 0.0001 {
        preview_css_scale
    } else {
        1.0
    };

    // Visual position during preview (CSS-scaled):
    //   visual = (scroll + offset - content_left) / css_scale
    let preview_visual_left =
        (preview_scroll_left - preview_content_left) / safe_scale;
    let preview_visual_top =
        (preview_scroll_top - preview_content_top) / safe_scale;

    // Natural position after syncLayoutBox (no CSS transform):
    //   natural = settled_scroll - settled_content_left
    let settled_natural_left = settled_scroll_left - settled_content_left;
    let settled_natural_top = settled_scroll_top - settled_content_top;

    // CSS translate needed to bridge the gap:
    //   translate = preview_visual - settled_natural
    let translate_x = preview_visual_left - settled_natural_left;
    let translate_y = preview_visual_top - settled_natural_top;

    SettledTransformResult {
        translate_x,
        translate_y,
        scroll_left: settled_scroll_left,
        scroll_top: settled_scroll_top,
    }
}

/// Compute the CSS translate for preview compensation between ticks.
///
/// Returns the translate needed to keep the visible content position stable
/// when the container layout changes between preview ticks.
pub fn compute_preview_translate(
    current_content_left: f32,
    current_content_top: f32,
    current_scroll_left: f32,
    current_scroll_top: f32,
    next_content_left: f32,
    next_content_top: f32,
    next_scroll_left: f32,
    next_scroll_top: f32,
) -> (f32, f32) {
    let current_visible_left = current_content_left - current_scroll_left;
    let current_visible_top = current_content_top - current_scroll_top;
    let next_visible_left = next_content_left - next_scroll_left;
    let next_visible_top = next_content_top - next_scroll_top;
    (
        next_visible_left - current_visible_left,
        next_visible_top - current_visible_top,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── resolve_css_transform ────────────────────────────────────────────

    #[test]
    fn css_transform_idle_when_scale_near_one_and_not_previewing() {
        let d = resolve_css_transform(CssTransformRequest {
            preview_zoom: 1.0,
            last_rendered_zoom: 1.0,
            preview_active: false,
        });
        assert!((d.css_scale - 1.0).abs() < 0.001);
        assert_eq!(d.mode, CssTransformMode::Idle);
    }

    #[test]
    fn css_transform_preview_when_preview_active() {
        let d = resolve_css_transform(CssTransformRequest {
            preview_zoom: 1.5,
            last_rendered_zoom: 1.0,
            preview_active: true,
        });
        assert!((d.css_scale - 1.5).abs() < 0.001);
        assert_eq!(d.mode, CssTransformMode::Preview);
    }

    #[test]
    fn css_transform_committed_when_scale_far_from_one() {
        let d = resolve_css_transform(CssTransformRequest {
            preview_zoom: 1.5,
            last_rendered_zoom: 1.0,
            preview_active: false,
        });
        assert!((d.css_scale - 1.5).abs() < 0.001);
        assert_eq!(d.mode, CssTransformMode::Committed);
    }

    #[test]
    fn css_transform_uses_last_rendered_as_base() {
        let d = resolve_css_transform(CssTransformRequest {
            preview_zoom: 2.0,
            last_rendered_zoom: 1.5,
            preview_active: false,
        });
        // css_scale = 2.0 / 1.5 = 1.333...
        assert!((d.css_scale - (2.0 / 1.5)).abs() < 0.001);
    }

    #[test]
    fn css_transform_fallback_base_to_one_when_zero() {
        let d = resolve_css_transform(CssTransformRequest {
            preview_zoom: 1.5,
            last_rendered_zoom: 0.0,
            preview_active: false,
        });
        assert!((d.css_scale - 1.5).abs() < 0.001);
    }

    // ─── resolve_wheel_request_params ─────────────────────────────────────

    #[test]
    fn wheel_params_content_size_scales_by_display_zoom() {
        let p = resolve_wheel_request_params(WheelRequestParamsRequest {
            page_width: 595.0,
            page_height: 842.0,
            current_display_zoom: 2.0,
            max_zoom: 30.0,
        });
        assert!((p.content_width - 1190.0).abs() < 0.01);
        assert!((p.content_height - 1684.0).abs() < 0.01);
    }

    #[test]
    fn wheel_params_min_zoom_is_always_0_1() {
        let p = resolve_wheel_request_params(WheelRequestParamsRequest {
            page_width: 100.0,
            page_height: 100.0,
            current_display_zoom: 1.0,
            max_zoom: 30.0,
        });
        assert!((p.min_zoom - 0.1).abs() < 0.001);
    }

    #[test]
    fn wheel_params_max_zoom_floor_is_min_zoom() {
        let p = resolve_wheel_request_params(WheelRequestParamsRequest {
            page_width: 100.0,
            page_height: 100.0,
            current_display_zoom: 1.0,
            max_zoom: 0.01, // below min_zoom
        });
        assert!(p.max_zoom >= p.min_zoom);
    }

    // ─── resolve_layout_fallback ──────────────────────────────────────────

    #[test]
    fn layout_fallback_dom_dimensions_use_rendered_zoom() {
        let f = resolve_layout_fallback(LayoutFallbackRequest {
            page_width: 595.0,
            page_height: 842.0,
            display_zoom: 2.0,
            rendered_zoom: 1.5,
        });
        assert!((f.dom_width - 595.0 * 1.5).abs() < 0.01);
        assert!((f.dom_height - 842.0 * 1.5).abs() < 0.01);
    }

    #[test]
    fn layout_fallback_display_dimensions_use_display_zoom() {
        let f = resolve_layout_fallback(LayoutFallbackRequest {
            page_width: 595.0,
            page_height: 842.0,
            display_zoom: 2.0,
            rendered_zoom: 1.5,
        });
        assert!((f.display_width - 595.0 * 2.0).abs() < 0.01);
        assert!((f.display_height - 842.0 * 2.0).abs() < 0.01);
    }

    #[test]
    fn layout_fallback_css_scale_is_display_over_rendered() {
        let f = resolve_layout_fallback(LayoutFallbackRequest {
            page_width: 595.0,
            page_height: 842.0,
            display_zoom: 2.0,
            rendered_zoom: 1.5,
        });
        assert!((f.css_scale - (2.0 / 1.5)).abs() < 0.001);
    }

    #[test]
    fn layout_fallback_zero_rendered_uses_display() {
        let f = resolve_layout_fallback(LayoutFallbackRequest {
            page_width: 595.0,
            page_height: 842.0,
            display_zoom: 2.0,
            rendered_zoom: 0.0,
        });
        // rendered_zoom=0 → fallback to display_zoom → css_scale=1.0
        assert!((f.css_scale - 1.0).abs() < 0.001);
    }

    // ─── resolve_fit_to_width ────────────────────────────────────────────

    #[test]
    fn fit_to_width_page_wider_than_viewport() {
        let r = resolve_fit_to_width(800.0, 595.0 * 2.0);
        // 800 / 1190 = 0.672...
        assert!(r.should_fit);
        assert!((r.fit_zoom - (800.0 / 1190.0)).abs() < 0.001);
    }

    #[test]
    fn fit_to_width_page_fits_in_viewport() {
        let r = resolve_fit_to_width(1200.0, 595.0);
        assert!(!r.should_fit);
    }

    #[test]
    fn fit_to_width_clamps_to_min_zoom() {
        let r = resolve_fit_to_width(100.0, 100000.0);
        // 100 / 100000 = 0.001, clamped to 0.1
        assert!(r.should_fit);
        assert!((r.fit_zoom - MIN_ZOOM).abs() < 0.001);
    }

    #[test]
    fn fit_to_width_clamps_to_max_zoom() {
        let r = resolve_fit_to_width(100000.0, 100.0);
        // Page fits, so should_fit=false
        assert!(!r.should_fit);
    }

    #[test]
    fn fit_to_width_zero_page_width() {
        let r = resolve_fit_to_width(800.0, 0.0);
        // page_width=0 → treated as 1.0 → fits
        assert!(!r.should_fit);
    }

    // ─── resolve_css_transform_string ──────────────────────────────────

    #[test]
    fn css_transform_string_empty_when_no_scale_no_translate() {
        let s = resolve_css_transform_string(0.0, 0.0, 1.0);
        assert!(s.is_empty());
    }

    #[test]
    fn css_transform_string_scale_only() {
        let s = resolve_css_transform_string(0.0, 0.0, 1.5);
        assert_eq!(s, "scale(1.5)");
    }

    #[test]
    fn css_transform_string_translate_and_scale() {
        let s = resolve_css_transform_string(5.0, 3.0, 1.2);
        assert!(s.contains("translate3d"));
        assert!(s.contains("scale(1.2)"));
    }

    #[test]
    fn css_transform_string_translate_only() {
        let s = resolve_css_transform_string(5.0, 3.0, 1.0);
        assert!(s.contains("translate3d"));
        assert!(!s.contains("scale"));
    }

    // ─── resolve_canvas_css_box ────────────────────────────────────────

    #[test]
    fn canvas_css_box_normalizes_display_zoom() {
        let b = resolve_canvas_css_box(2.0, 1.5, 595.0, 842.0);
        // dom_width = (595 / 2.0) * 1.5 = 446.25
        assert!((b.dom_width - 446.25).abs() < 0.1);
        // dom_height = (842 / 2.0) * 1.5 = 631.5
        assert!((b.dom_height - 631.5).abs() < 0.1);
        // base_scale = 1.5 / 2.0 = 0.75
        assert!((b.base_scale - 0.75).abs() < 0.001);
    }

    #[test]
    fn canvas_css_box_zero_display_zoom_uses_one() {
        let b = resolve_canvas_css_box(0.0, 1.0, 595.0, 842.0);
        // display_zoom=0 → treated as 1.0
        assert!((b.dom_width - 595.0).abs() < 0.1);
    }

    // ─── is_immediate_mutation_frame ───────────────────────────────────

    #[test]
    fn immediate_mutation_editor_visibility() {
        assert!(is_immediate_mutation_frame("editorVisibility"));
    }

    #[test]
    fn immediate_mutation_document_mutation() {
        assert!(is_immediate_mutation_frame("documentMutation"));
    }

    #[test]
    fn immediate_mutation_zoom_is_not() {
        assert!(!is_immediate_mutation_frame("zoom"));
    }

    #[test]
    fn immediate_mutation_default_is_not() {
        assert!(!is_immediate_mutation_frame("default"));
    }

    // ─── resolve_settled_transform ─────────────────────────────────────

    #[test]
    fn settled_transform_no_change_when_state_matches_and_scale_one() {
        // When css_scale=1.0 and all states match, translate should be 0.
        let r = resolve_settled_transform(
            100.0, 50.0, // preview content_left, content_top
            200.0, 100.0, // preview scroll_left, scroll_top
            1.0,          // preview css_scale
            100.0, 50.0,  // settled content_left, content_top
            200.0, 100.0, // settled scroll_left, scroll_top
        );
        assert!(r.translate_x.abs() < 0.01);
        assert!(r.translate_y.abs() < 0.01);
        assert!((r.scroll_left - 200.0).abs() < 0.01);
        assert!((r.scroll_top - 100.0).abs() < 0.01);
    }

    #[test]
    fn settled_transform_compensates_for_content_left_shift() {
        // Preview: content_left=50, scroll=200, css_scale=2.0
        //   visual_left = (200 - 50) / 2.0 = 75
        // Settled: content_left=100, scroll=200
        //   natural_left = 200 - 100 = 100
        // translate_x = 75 - 100 = -25
        let r = resolve_settled_transform(
            50.0, 0.0,
            200.0, 0.0,
            2.0,
            100.0, 0.0,
            200.0, 0.0,
        );
        assert!((r.translate_x - (-25.0)).abs() < 0.01);
        assert!(r.translate_y.abs() < 0.01);
    }

    #[test]
    fn settled_transform_compensates_for_scroll_and_scale() {
        // Preview: content_left=0, scroll=100, css_scale=1.0
        //   visual_left = (100 - 0) / 1.0 = 100
        // Settled: content_left=50, scroll=200
        //   natural_left = 200 - 50 = 150
        // translate_x = 100 - 150 = -50
        let r = resolve_settled_transform(
            0.0, 0.0,
            100.0, 0.0,
            1.0,
            50.0, 0.0,
            200.0, 0.0,
        );
        assert!((r.translate_x - (-50.0)).abs() < 0.01);
    }

    #[test]
    fn settled_transform_handles_zero_scale() {
        // Zero scale should be treated as 1.0 (safe_div).
        let r = resolve_settled_transform(
            0.0, 0.0,
            100.0, 50.0,
            0.0, // zero → treated as 1.0
            0.0, 0.0,
            100.0, 50.0,
        );
        // Same state → translate should be 0
        assert!(r.translate_x.abs() < 0.01);
        assert!(r.translate_y.abs() < 0.01);
    }

    #[test]
    fn settled_transform_both_axes() {
        // Test X and Y simultaneously.
        // Preview: content=(20, 30), scroll=(150, 250), scale=2.0
        //   visual_x = (150 - 20) / 2.0 = 65
        //   visual_y = (250 - 30) / 2.0 = 110
        // Settled: content=(80, 90), scroll=(200, 300)
        //   natural_x = 200 - 80 = 120
        //   natural_y = 300 - 90 = 210
        //   translate_x = 65 - 120 = -55
        //   translate_y = 110 - 210 = -100
        let r = resolve_settled_transform(
            20.0, 30.0,
            150.0, 250.0,
            2.0,
            80.0, 90.0,
            200.0, 300.0,
        );
        assert!((r.translate_x - (-55.0)).abs() < 0.01);
        assert!((r.translate_y - (-100.0)).abs() < 0.01);
        assert!((r.scroll_left - 200.0).abs() < 0.01);
        assert!((r.scroll_top - 300.0).abs() < 0.01);
    }

    // ─── compute_preview_translate ─────────────────────────────────────

    #[test]
    fn preview_translate_no_change() {
        let (tx, ty) = compute_preview_translate(100.0, 50.0, 200.0, 100.0, 100.0, 50.0, 200.0, 100.0);
        assert!(tx.abs() < 0.01);
        assert!(ty.abs() < 0.01);
    }

    #[test]
    fn preview_translate_compensates_layout_shift() {
        // current: content_left=50, scroll=200 → visible = 50-200 = -150
        // next: content_left=80, scroll=200 → visible = 80-200 = -120
        // translate = -120 - (-150) = 30
        let (tx, _) = compute_preview_translate(50.0, 0.0, 200.0, 0.0, 80.0, 0.0, 200.0, 0.0);
        assert!((tx - 30.0).abs() < 0.01);
    }
}
