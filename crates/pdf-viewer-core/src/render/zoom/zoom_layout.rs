//! Zoom layout geometry — layout fallback, fit-to-width, render-reason
//! classification.
//!
//! Pure logic — no DOM, no WASM, no thread_local.
//!
//! CSS transform zoom has been removed (ADR-0006): container dimensions track
//! display_zoom directly via SetBox and no transform compensation exists.
//! Only layout fallback, fit-to-width, and mutation classification remain as
//! active code.

use serde::{Deserialize, Serialize};

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
/// Post-ADR-0006 there is no lagging render-zoom DOM box: the container box
/// tracks display_zoom directly and no transform compensation exists, so
/// every dimension derives from display_zoom and `css_scale` is 1.0.
pub fn resolve_layout_fallback(request: LayoutFallbackRequest) -> LayoutFallback {
    let page_width = if request.page_width > 0.0 {
        request.page_width
    } else {
        1.0
    };
    let page_height = if request.page_height > 0.0 {
        request.page_height
    } else {
        1.0
    };
    let display_zoom = if request.display_zoom > 0.0 {
        request.display_zoom
    } else {
        1.0
    };
    let dom_width = page_width * display_zoom;
    let dom_height = page_height * display_zoom;
    let display_width = dom_width;
    let display_height = dom_height;
    LayoutFallback {
        dom_width,
        dom_height,
        display_width,
        display_height,
        host_width: display_width,
        host_height: display_height,
        content_left: 0.0,
        content_top: 0.0,
        css_scale: 1.0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LayoutFallbackRequest {
    pub page_width: f32,
    pub page_height: f32,
    pub display_zoom: f32,
}

// ─── Zoom limits constants ──────────────────────────────────────────────────

pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 30.0;

// ─── Fit-to-width computation ───────────────────────────────────────────────

/// Result of `resolve_fit_to_width`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FitToWidthResult {
    pub fit_zoom: f32,
    pub should_fit: bool,
}

/// Compute the fit-to-width zoom level for a document.
pub fn resolve_fit_to_width(viewport_width: f32, page_width: f32) -> FitToWidthResult {
    let vp = if viewport_width > 0.0 {
        viewport_width
    } else {
        1.0
    };
    let pw = if page_width > 0.0 { page_width } else { 1.0 };
    if pw <= vp {
        return FitToWidthResult {
            fit_zoom: 1.0,
            should_fit: false,
        };
    }
    let raw = vp / pw;
    let fit_zoom = raw.clamp(MIN_ZOOM, MAX_ZOOM);
    FitToWidthResult {
        fit_zoom,
        should_fit: true,
    }
}

// ─── Canvas CSS box ─────────────────────────────────────────────────────────
//
// Single source for the vector-canvas element box geometry. The TS side
// (vector_canvas_host) consumes this via WASM instead of inlining the
// formula (ADR-0002 leftover: "第二处计算" is how the cssScale-split bug
// class was born).

/// Inputs for [`resolve_canvas_css_box`]. f64 throughout so the arithmetic
/// is bit-identical to the TS caller's float math.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CanvasCssBoxRequest {
    pub display_width: f64,
    pub display_height: f64,
    pub display_zoom: f64,
    pub base_render_zoom: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasCssBox {
    pub dom_width: f64,
    pub dom_height: f64,
}

/// Element CSS box for the vector canvases inside the container: the canvas
/// carries the base layer at `base_render_zoom` resolution, so its CSS box is
/// the display box rescaled from display_zoom to base_render_zoom. Degenerate
/// zooms fall back to the display box (matches the historical TS formula,
/// including its 0.0001 guard).
pub fn resolve_canvas_css_box(request: CanvasCssBoxRequest) -> CanvasCssBox {
    let zooms_valid = request.display_zoom > 0.0001 && request.base_render_zoom > 0.0001;
    if zooms_valid {
        let scale = request.base_render_zoom / request.display_zoom;
        CanvasCssBox {
            dom_width: request.display_width * scale,
            dom_height: request.display_height * scale,
        }
    } else {
        CanvasCssBox {
            dom_width: request.display_width,
            dom_height: request.display_height,
        }
    }
}

// ─── Immediate mutation check ─────────────────────────────────────────────

/// Check if a render reason indicates an immediate (non-preview) mutation.
pub fn is_immediate_mutation_frame(render_reason: &str) -> bool {
    render_reason == "editorVisibility" || render_reason == "documentMutation"
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── resolve_layout_fallback ──────────────────────────────────────────

    #[test]
    fn layout_fallback_dimensions_track_display_zoom() {
        let f = resolve_layout_fallback(LayoutFallbackRequest {
            page_width: 595.0,
            page_height: 842.0,
            display_zoom: 2.0,
        });
        assert!((f.dom_width - 595.0 * 2.0).abs() < 0.01);
        assert!((f.dom_height - 842.0 * 2.0).abs() < 0.01);
        assert!((f.display_width - 595.0 * 2.0).abs() < 0.01);
        assert!((f.display_height - 842.0 * 2.0).abs() < 0.01);
        assert!((f.host_width - 595.0 * 2.0).abs() < 0.01);
        assert!((f.host_height - 842.0 * 2.0).abs() < 0.01);
    }

    #[test]
    fn layout_fallback_css_scale_is_always_one() {
        let f = resolve_layout_fallback(LayoutFallbackRequest {
            page_width: 595.0,
            page_height: 842.0,
            display_zoom: 2.0,
        });
        assert!((f.css_scale - 1.0).abs() < 0.001);
    }

    #[test]
    fn layout_fallback_zero_display_zoom_clamps_to_one() {
        let f = resolve_layout_fallback(LayoutFallbackRequest {
            page_width: 595.0,
            page_height: 842.0,
            display_zoom: 0.0,
        });
        assert!((f.dom_width - 595.0).abs() < 0.01);
        assert!((f.css_scale - 1.0).abs() < 0.001);
    }

    // ─── resolve_canvas_css_box ───────────────────────────────────────────

    fn canvas_box_request(
        display_zoom: f64,
        base_render_zoom: f64,
    ) -> CanvasCssBoxRequest {
        CanvasCssBoxRequest {
            display_width: 595.0 * display_zoom,
            display_height: 842.0 * display_zoom,
            display_zoom,
            base_render_zoom,
        }
    }

    #[test]
    fn canvas_css_box_identity_when_base_matches_display() {
        let r = resolve_canvas_css_box(canvas_box_request(1.25, 1.25));
        assert!((r.dom_width - 595.0 * 1.25).abs() < 1e-9);
        assert!((r.dom_height - 842.0 * 1.25).abs() < 1e-9);
    }

    #[test]
    fn canvas_css_box_rescales_to_base_render_zoom() {
        let r = resolve_canvas_css_box(canvas_box_request(2.0, 1.5));
        assert!((r.dom_width - 595.0 * 1.5).abs() < 1e-9);
        assert!((r.dom_height - 842.0 * 1.5).abs() < 1e-9);
    }

    #[test]
    fn canvas_css_box_falls_back_to_display_box_on_degenerate_zoom() {
        let r = resolve_canvas_css_box(canvas_box_request(0.0, 1.5));
        assert!((r.dom_width - 0.0).abs() < 1e-9);

        let r = resolve_canvas_css_box(CanvasCssBoxRequest {
            display_width: 100.0,
            display_height: 200.0,
            display_zoom: 1.0,
            base_render_zoom: f64::NAN,
        });
        assert!((r.dom_width - 100.0).abs() < 1e-9);
        assert!((r.dom_height - 200.0).abs() < 1e-9);
    }

    // ─── resolve_fit_to_width ────────────────────────────────────────────

    #[test]
    fn fit_to_width_page_wider_than_viewport() {
        let r = resolve_fit_to_width(800.0, 595.0 * 2.0);
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
        assert!(r.should_fit);
        assert!((r.fit_zoom - MIN_ZOOM).abs() < 0.001);
    }

    #[test]
    fn fit_to_width_zero_page_width() {
        let r = resolve_fit_to_width(800.0, 0.0);
        assert!(!r.should_fit);
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
}
