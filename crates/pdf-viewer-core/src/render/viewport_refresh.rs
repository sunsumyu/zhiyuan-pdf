use serde::{Deserialize, Serialize};

const VIEWPORT_REFRESH_COMMIT_SUPPRESS_MS: f64 = 120.0;
const VIEWPORT_REFRESH_DELAY_MS: u32 = 56;

#[derive(Debug, Clone, Default)]
pub struct HostViewportRefreshState {
    pub suppress_until_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewportRefreshDecision {
    pub should_refresh: bool,
    pub delay_ms: u32,
}

pub fn note_viewport_render_commit(state: &mut HostViewportRefreshState, timestamp_ms: f64) {
    let timestamp_ms = if timestamp_ms.is_finite() && timestamp_ms >= 0.0 {
        timestamp_ms
    } else {
        0.0
    };
    state.suppress_until_ms = timestamp_ms + VIEWPORT_REFRESH_COMMIT_SUPPRESS_MS;
}

pub fn resolve_viewport_refresh_decision(
    state: &HostViewportRefreshState,
    use_viewport_tile: bool,
    render_detail_layer: bool,
    timestamp_ms: f64,
) -> ViewportRefreshDecision {
    if !use_viewport_tile || !render_detail_layer {
        return ViewportRefreshDecision::default();
    }

    let timestamp_ms = if timestamp_ms.is_finite() && timestamp_ms >= 0.0 {
        timestamp_ms
    } else {
        0.0
    };

    if timestamp_ms < state.suppress_until_ms {
        return ViewportRefreshDecision::default();
    }

    ViewportRefreshDecision {
        should_refresh: true,
        delay_ms: VIEWPORT_REFRESH_DELAY_MS,
    }
}
