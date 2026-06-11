use serde::{Deserialize, Serialize};

use crate::present::plan_builder::compute_viewport_layout_result;
use crate::common::sanitize::{sanitize_non_negative, sanitize_positive};
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
    pub display_width: f32,
    pub display_height: f32,
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
}

pub fn sync_host_layout(request: SyncHostLayoutRequest) -> SyncHostLayoutResult {
    let display_zoom = sanitize_positive(request.display_zoom, 1.0);
    let display_width = sanitize_positive(request.page_width, 1.0) * display_zoom;
    let display_height = sanitize_positive(request.page_height, 1.0) * display_zoom;

    let layout = request
        .layout_override
        .map(|layout| HostLayoutOverride {
            host_width: sanitize_positive(
                layout.host_width,
                display_width.max(request.viewport_width),
            ),
            host_height: sanitize_positive(
                layout.host_height,
                display_height.max(request.viewport_height),
            ),
            content_left: sanitize_non_negative(layout.content_left, 0.0),
            content_top: sanitize_non_negative(layout.content_top, 0.0),
        })
        .unwrap_or_else(|| {
            let computed = compute_viewport_layout_result(
                display_width,
                display_height,
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
        display_width,
        display_height,
        host_width: layout.host_width,
        host_height: layout.host_height,
        content_left: layout.content_left,
        content_top: layout.content_top,
    }
}
