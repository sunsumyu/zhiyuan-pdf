use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewportLayoutRequest {
    pub display_width: f32,
    pub display_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewportTileRequest {
    pub display_width: f32,
    pub display_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub content_left: f32,
    pub content_top: f32,
    pub overscan: f32,
}
