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
    let fit_zoom = raw.max(MIN_ZOOM).min(MAX_ZOOM);
    FitToWidthResult {
        fit_zoom,
        should_fit: true,
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
