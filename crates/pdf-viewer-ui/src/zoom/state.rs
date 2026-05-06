use std::cell::RefCell;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomAnchorState {
    pub anchor_page_x: f32,
    pub anchor_page_y: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub viewport_x: f32,
    pub viewport_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VisualLayoutState {
    pub display_zoom: f32,
    pub content_left: f32,
    pub content_top: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTransformState {
    pub translate_x: f32,
    pub translate_y: f32,
    pub css_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PendingCommittedFrame {
    pub display_zoom: f32,
    pub render_zoom: f32,
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewHostState {
    pub preview_active: bool,
    pub wheel_render_pending: bool,
    pub pending_committed_frame: Option<PendingCommittedFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostZoomState {
    pub current_zoom: f32,
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    pub last_animation_timestamp_ms: f64,
    pub pending_anchor: Option<ZoomAnchorState>,
    pub visual_layout: Option<VisualLayoutState>,
    pub preview_transform: Option<PreviewTransformState>,
    pub preview_host: PreviewHostState,
}

impl Default for HostZoomState {
    fn default() -> Self {
        Self {
            current_zoom: 1.0,
            target_zoom: 1.0,
            visual_zoom: 1.0,
            last_rendered_zoom: 1.0,
            last_animation_timestamp_ms: 0.0,
            pending_anchor: None,
            visual_layout: None,
            preview_transform: None,
            preview_host: PreviewHostState::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomAnimationStep {
    pub visual_zoom: f32,
    pub css_scale: f32,
    pub settled: bool,
}

thread_local! {
    pub static HOST_ZOOM_STATE: RefCell<HostZoomState> =
        RefCell::new(HostZoomState::default());
}

pub fn reset_zoom_state(initial_zoom: f32) {
    let zoom = sanitize_zoom(initial_zoom);
    HOST_ZOOM_STATE.with(|state| {
        *state.borrow_mut() = HostZoomState {
            current_zoom: zoom,
            target_zoom: zoom,
            visual_zoom: zoom,
            last_rendered_zoom: zoom,
            last_animation_timestamp_ms: 0.0,
            pending_anchor: None,
            visual_layout: None,
            preview_transform: None,
            preview_host: PreviewHostState::default(),
        };
    });
}

fn sanitize_zoom(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}
