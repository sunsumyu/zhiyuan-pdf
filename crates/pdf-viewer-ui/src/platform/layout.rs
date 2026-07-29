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
