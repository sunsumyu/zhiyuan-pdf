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
    /// When true, the next commit should discard stale renders.
    pub cancel_pending_render: bool,
}

/// Drawing delay timer — delays final render after zoom settles.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DrawingDelayState {
    pub active: bool,
    pub started_at_ms: f64,
    pub delay_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostZoomState {
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    /// Precomputed css_scale = visual_zoom / last_rendered_zoom.
    /// Maintained by `recompute_css_scale()` — never set directly.
    pub css_scale: f32,
    pub last_animation_timestamp_ms: f64,
    pub pending_anchor: Option<ZoomAnchorState>,
    pub visual_layout: Option<VisualLayoutState>,
    pub preview_transform: Option<PreviewTransformState>,
    pub preview_host: PreviewHostState,
    pub drawing_delay: DrawingDelayState,
}

impl HostZoomState {
    /// Recompute the cached `css_scale` from `visual_zoom` and
    /// `last_rendered_zoom`.  Call after any mutation that changes either.
    ///
    /// ADR-0004 (revised): this ratio is the interpolation bridge between the
    /// committed bitmap (last_rendered_zoom) and the visual intent. Because the
    /// pipeline renders AT visual_zoom, every commit drives the ratio back to
    /// 1.0 — no stretch at rest. Do NOT pin it to 1.0: that desyncs the
    /// transform state from the actual DOM scale and destabilizes zoom.
    pub fn recompute_css_scale(&mut self) {
        let base = if self.last_rendered_zoom > 0.0 {
            self.last_rendered_zoom
        } else {
            1.0
        };
        self.css_scale = self.visual_zoom / base;
    }
}

impl Default for HostZoomState {
    fn default() -> Self {
        Self {
            target_zoom: 1.0,
            visual_zoom: 1.0,
            last_rendered_zoom: 1.0,
            css_scale: 1.0,
            last_animation_timestamp_ms: 0.0,
            pending_anchor: None,
            visual_layout: None,
            preview_transform: None,
            preview_host: PreviewHostState::default(),
            drawing_delay: DrawingDelayState::default(),
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
