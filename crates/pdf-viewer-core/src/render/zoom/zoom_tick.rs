//! Zoom state machine: per-frame tick that advances animation and decides
//! render timing.
//!
//! Pure logic — takes `&mut HostZoomState` as explicit parameter.
//! No thread_local access; the UI crate passes the state in.

use serde::{Deserialize, Serialize};

/// DOM operation that TS must execute synchronously.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DomOp {
    /// Update container layout dimensions.
    UpdateLayout {
        display_zoom: f32,
        render_zoom: f32,
        host_width: f32,
        host_height: f32,
        content_left: f32,
        content_top: f32,
        dom_width: f32,
        dom_height: f32,
    },
    /// Set scroll position on the scroll container.
    SetScroll { scroll_left: f32, scroll_top: f32 },
    /// Set wrapper (parent) dimensions.
    SetWrapperSize { width: f32, height: f32 },
}

/// Async operation that TS must schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AsyncOp {
    /// Request a render at the given reason.
    RequestRender { reason: String },
    /// Schedule the next RAF frame (continue animation loop).
    ScheduleNextFrame,
    /// Stop the RAF loop (animation settled).
    StopRafLoop,
    /// Start a drawing delay timer. TS must call back after delay_ms.
    StartDrawingDelay { delay_ms: u32 },
    /// Cancel any active drawing delay timer.
    CancelDrawingDelay,
}

/// Input from TS to the zoom state machine (per frame).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomTickInput {
    pub timestamp_ms: f64,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

/// Output from the zoom state machine to TS (per frame).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomTickOutput {
    pub visual_zoom: f32,
    pub target_zoom: f32,
    pub settled: bool,
    pub dom_ops: Vec<DomOp>,
    pub async_ops: Vec<AsyncOp>,
}

/// Pure function: advance zoom animation, decide render timing.
/// Takes `&mut HostZoomState` directly — no thread_local access.
pub fn tick_zoom_state_core(
    state: &mut crate::render::zoom_state::HostZoomState,
    input: &ZoomTickInput,
) -> ZoomTickOutput {
    use super::zoom_render::should_render;
    use super::zoom_render::ShouldRender;
    use crate::render::zoom_interaction::advance_zoom_animation_state;

    let dom_ops: Vec<DomOp> = Vec::new();
    let mut async_ops: Vec<AsyncOp> = Vec::new();

    // 1. Advance animation
    let step = advance_zoom_animation_state(state, Some(input.timestamp_ms));

    // 2. Decide whether to render
    let animation_velocity = if state.last_animation_timestamp_ms > 0.0 {
        let dt = (input.timestamp_ms - state.last_animation_timestamp_ms) / 1000.0;
        if dt > 0.001 {
            (state.visual_zoom - step.visual_zoom).abs() / dt as f32
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Drawing delay: when animation settles, delay final render
    const DRAWING_DELAY_MS: u32 = 30;
    if step.settled && !state.drawing_delay.active {
        state.drawing_delay.active = true;
        state.drawing_delay.started_at_ms = input.timestamp_ms;
        state.drawing_delay.delay_ms = DRAWING_DELAY_MS;
        async_ops.push(AsyncOp::StartDrawingDelay {
            delay_ms: DRAWING_DELAY_MS,
        });
    }

    let render_decision = if state.drawing_delay.active {
        let elapsed = input.timestamp_ms - state.drawing_delay.started_at_ms;
        if elapsed >= state.drawing_delay.delay_ms as f64 {
            state.drawing_delay.active = false;
            ShouldRender::Yes {
                render_zoom: state.target_zoom,
            }
        } else {
            ShouldRender::Skip
        }
    } else {
        should_render(
            step.visual_zoom,
            state.last_rendered_zoom,
            state.target_zoom,
            step.settled,
            animation_velocity,
            16.0,
        )
    };

    match render_decision {
        ShouldRender::Yes { render_zoom: _ } => {
            async_ops.push(AsyncOp::RequestRender {
                reason: if step.settled { "settle" } else { "zoom" }.into(),
            });
        }
        ShouldRender::Soon { .. } => {}
        ShouldRender::Skip => {}
    }

    // 3. Decide RAF continuation
    if step.settled && !state.drawing_delay.active {
        async_ops.push(AsyncOp::StopRafLoop);
    } else {
        async_ops.push(AsyncOp::ScheduleNextFrame);
    }

    ZoomTickOutput {
        visual_zoom: step.visual_zoom,
        target_zoom: state.target_zoom,
        settled: step.settled,
        dom_ops,
        async_ops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::zoom_state::HostZoomState;

    fn make_state(target: f32) -> HostZoomState {
        HostZoomState {
            target_zoom: target,
            visual_zoom: 1.0,
            last_rendered_zoom: 1.0,
            ..HostZoomState::default()
        }
    }

    fn tick(state: &mut HostZoomState, ts: f64) -> ZoomTickOutput {
        tick_zoom_state_core(
            state,
            &ZoomTickInput {
                timestamp_ms: ts,
                scroll_left: 0.0,
                scroll_top: 0.0,
                viewport_width: 800.0,
                viewport_height: 600.0,
            },
        )
    }

    #[test]
    fn tick_first_frame_advances_animation() {
        let mut state = make_state(1.5);
        let out = tick(&mut state, 100.0);
        assert!(
            out.visual_zoom > 1.0,
            "visual_zoom should advance: {}",
            out.visual_zoom
        );
        assert!(
            out.visual_zoom <= 1.5,
            "visual_zoom should not overshoot: {}",
            out.visual_zoom
        );
        assert!(out
            .async_ops
            .iter()
            .any(|op| matches!(op, AsyncOp::ScheduleNextFrame)));
    }

    #[test]
    fn tick_settled_triggers_drawing_delay() {
        let mut state = make_state(1.0);
        state.visual_zoom = 1.0;
        let out = tick(&mut state, 100.0);
        assert!(out
            .async_ops
            .iter()
            .any(|op| matches!(op, AsyncOp::StartDrawingDelay { .. })));
        assert!(!out
            .async_ops
            .iter()
            .any(|op| matches!(op, AsyncOp::StopRafLoop)));
    }

    #[test]
    fn tick_drawing_delay_expired_requests_render() {
        let mut state = make_state(1.0);
        state.visual_zoom = 1.0;
        let _out1 = tick(&mut state, 100.0);
        assert!(state.drawing_delay.active);
        let out2 = tick(&mut state, 200.0);
        assert!(
            !state.drawing_delay.active,
            "drawing_delay should be cleared"
        );
        assert!(out2
            .async_ops
            .iter()
            .any(|op| matches!(op, AsyncOp::RequestRender { .. })));
        assert!(out2
            .async_ops
            .iter()
            .any(|op| matches!(op, AsyncOp::StopRafLoop)));
    }

    #[test]
    fn tick_drawing_delay_not_expired_skips_render() {
        let mut state = make_state(1.0);
        state.visual_zoom = 1.0;
        let _out1 = tick(&mut state, 100.0);
        // Tick 20ms later — drawing delay (30ms) not yet expired
        let out2 = tick(&mut state, 120.0);
        assert!(state.drawing_delay.active);
        assert!(!out2
            .async_ops
            .iter()
            .any(|op| matches!(op, AsyncOp::RequestRender { .. })));
        assert!(out2
            .async_ops
            .iter()
            .any(|op| matches!(op, AsyncOp::ScheduleNextFrame)));
    }

    #[test]
    fn tick_rapid_wheel_events_advance_gradually() {
        let mut state = make_state(2.0);
        for i in 0..5 {
            let ts = 100.0 + (i as f64) * 16.0;
            let out = tick(&mut state, ts);
            assert!(
                state.visual_zoom > 1.0 + (i as f32) * 0.01,
                "tick {}: visual_zoom should advance: {}",
                i,
                state.visual_zoom
            );
            assert!(out
                .async_ops
                .iter()
                .any(|op| matches!(op, AsyncOp::ScheduleNextFrame)));
        }
    }
}
