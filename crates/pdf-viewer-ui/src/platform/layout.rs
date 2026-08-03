use serde::{Deserialize, Serialize};

use crate::common::sanitize::{sanitize_non_negative, sanitize_positive};
use crate::present::plan_builder::compute_viewport_layout_result;
use crate::zoom::zoom_controller::set_visual_layout;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostLayoutOverride {
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncHostLayoutRequest {
    pub display_zoom: f32,
    pub render_zoom: Option<f32>,
    pub page_width: f32,
    pub page_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub layout_override: Option<HostLayoutOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncHostLayoutResult {
    pub display_zoom: f32,
    pub render_zoom: f32,
    pub dom_width: f32,
    pub dom_height: f32,
    pub display_width: f32,
    pub display_height: f32,
    pub css_scale: f32,
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
}

pub fn sync_host_layout(request: SyncHostLayoutRequest) -> SyncHostLayoutResult {
    let display_zoom = sanitize_positive(request.display_zoom, 1.0);
    let render_zoom = sanitize_positive(request.render_zoom.unwrap_or(display_zoom), display_zoom);
    let page_w = sanitize_positive(request.page_width, 1.0);
    let page_h = sanitize_positive(request.page_height, 1.0);

    let dom_width = page_w * render_zoom;
    let dom_height = page_h * render_zoom;
    let display_width = page_w * display_zoom;
    let display_height = page_h * display_zoom;
    let css_scale = if render_zoom > 0.0001 {
        display_zoom / render_zoom
    } else {
        1.0
    };

    let layout = request
        .layout_override
        .map(|layout| HostLayoutOverride {
            host_width: sanitize_positive(
                layout.host_width,
                dom_width.max(request.viewport_width),
            ),
            host_height: sanitize_positive(
                layout.host_height,
                dom_height.max(request.viewport_height),
            ),
            content_left: sanitize_non_negative(layout.content_left, 0.0),
            content_top: sanitize_non_negative(layout.content_top, 0.0),
        })
        .unwrap_or_else(|| {
            let computed = compute_viewport_layout_result(
                dom_width,
                dom_height,
                sanitize_non_negative(request.viewport_width, 0.0),
                sanitize_non_negative(request.viewport_height, 0.0),
            );
            HostLayoutOverride {
                host_width: computed.host_width,
                host_height: computed.host_height,
                content_left: computed.content_left,
                content_top: computed.content_top,
            }
        });

    set_visual_layout(display_zoom, layout.content_left, layout.content_top);

    SyncHostLayoutResult {
        display_zoom,
        render_zoom,
        dom_width,
        dom_height,
        display_width,
        display_height,
        css_scale,
        host_width: layout.host_width,
        host_height: layout.host_height,
        content_left: layout.content_left,
        content_top: layout.content_top,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE_W: f32 = 612.0;
    const PAGE_H: f32 = 792.0;
    const VIEWPORT_W: f32 = 800.0;
    const VIEWPORT_H: f32 = 600.0;

    fn request(display_zoom: f32, render_zoom: Option<f32>) -> SyncHostLayoutRequest {
        SyncHostLayoutRequest {
            display_zoom,
            render_zoom,
            page_width: PAGE_W,
            page_height: PAGE_H,
            viewport_width: VIEWPORT_W,
            viewport_height: VIEWPORT_H,
            layout_override: None,
        }
    }

    fn assert_close(actual: f32, expected: f32, msg: &str) {
        let tolerance = 1e-3;
        assert!(
            (actual - expected).abs() <= tolerance,
            "{msg}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn committed_state_has_identity_css_scale() {
        // Committed: rendered zoom equals display zoom => css_scale == 1, dom == display.
        let result = sync_host_layout(request(1.25, Some(1.25)));
        assert_close(result.css_scale, 1.0, "css_scale");
        assert_close(result.dom_width, result.display_width, "dom_width == display_width");
        assert_close(result.dom_height, result.display_height, "dom_height == display_height");
        assert_close(result.dom_width, PAGE_W * 1.25, "dom_width == page_w * zoom");
        assert!(result.host_width >= result.display_width, "host covers display");
    }

    #[test]
    fn preview_state_cancels_css_scale_against_render_zoom() {
        // Preview: rendered zoom lags behind display zoom.
        let result = sync_host_layout(request(1.25, Some(1.0)));
        assert_close(result.dom_width, PAGE_W * 1.0, "dom_width == page_w * render_zoom");
        assert_close(result.display_width, PAGE_W * 1.25, "display_width == page_w * display_zoom");
        assert_close(result.css_scale, 1.25, "css_scale == display / render");
        // The core cancellation guarantee from the design spec:
        // visual width = dom_width * css_scale == display_width.
        assert_close(
            result.dom_width * result.css_scale,
            result.display_width,
            "dom_width * css_scale == display_width",
        );
    }

    #[test]
    fn zoom_in_preview_does_not_flash_larger() {
        // Z_display=1.25, Z_rendered=1.0: visual width must equal the target display width,
        // not the pre-fix quadratic overshoot W * Z_display^2 / Z_rendered.
        let result = sync_host_layout(request(1.25, Some(1.0)));
        let visual_width = result.dom_width * result.css_scale;
        let quadratic_overshoot = PAGE_W * (1.25f32 * 1.25) / 1.0;
        assert!(
            (visual_width - quadratic_overshoot).abs() > 10.0,
            "must not exhibit quadratic double-scaling flash"
        );
        assert_close(visual_width, PAGE_W * 1.25, "visual width == display target");
    }

    #[test]
    fn render_zoom_defaults_to_display_zoom() {
        // render_zoom omitted => defaults to display_zoom (committed-like).
        let result = sync_host_layout(request(1.5, None));
        assert_close(result.render_zoom, 1.5, "render_zoom fallback");
        assert_close(result.css_scale, 1.0, "css_scale identity");
        assert_close(result.dom_width * result.css_scale, result.display_width, "cancellation");
    }

    #[test]
    fn sanitizes_invalid_zoom_inputs() {
        // Invalid display zoom falls back to 1.0; invalid render zoom falls back to display.
        let result = sync_host_layout(request(f32::NAN, Some(f32::NAN)));
        assert_close(result.display_zoom, 1.0, "display_zoom fallback");
        assert_close(result.render_zoom, 1.0, "render_zoom fallback");
        assert_close(result.css_scale, 1.0, "css_scale identity");
        assert_close(result.dom_width * result.css_scale, result.display_width, "cancellation");
    }
}
