//! Render decision engine: blur-based timing and mid-animation re-render gating.
//!
//! Pure logic — no DOM, no WASM, no thread_local.

use serde::{Deserialize, Serialize};

/// Decision on whether to render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShouldRender {
    /// Render now at the given zoom.
    Yes { render_zoom: f32 },
    /// Render on the next frame.
    Soon { render_zoom: f32 },
    /// Skip this frame.
    Skip,
}

/// Blur threshold constants.
const BLUR_HIGH_THRESHOLD: f32 = 0.10;  // > 10% blur → render immediately
const BLUR_LOW_THRESHOLD: f32 = 0.03;   // > 3% blur → render next frame
const SETTLE_DELAY_MS: f64 = 80.0;      // delay after settle before final render

// ─── Mid-animation re-render knock (ADR-0002) ───────────────────────────────
//
// The RAF loop is the only animation driver, and the render pipeline renders
// nothing between wheel gestures — so without a knock the user stares at a
// stretched stale bitmap (the "zoom blur"). The RAF loop therefore knocks the
// TS render loop DURING the animation whenever the css_scale blur exceeds a
// threshold, throttled and gated on render availability. The resulting frame
// renders at visualZoom (C1: render tracks visual), so its renderZoom ==
// displayZoom and the presenter can commit it seamlessly (I1/I2 hold).

/// Blur at or above this triggers a mid-animation re-render. Slightly under
/// BLUR_HIGH_THRESHOLD so a knock fires before the old threshold would have.
pub const PREVIEW_REKNOCK_BLUR_THRESHOLD: f32 = 0.06;
/// Minimum spacing between mid-animation knocks — a Vello render takes
/// O(100ms); knocking faster only queues stale frames.
pub const PREVIEW_REKNOCK_INTERVAL_MS: f64 = 140.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PreviewReknockRequest {
    /// |css_scale − 1| — how far the displayed transform is from the bitmap.
    pub blur: f32,
    pub elapsed_ms: f64,
    /// A render is already in flight (RENDER_STATE.in_flight_frame_token != 0).
    pub render_in_flight: bool,
}

/// Decide whether the RAF loop should knock the TS render loop mid-animation.
pub fn should_reknock_preview_render(request: PreviewReknockRequest) -> bool {
    !request.render_in_flight
        && request.blur >= PREVIEW_REKNOCK_BLUR_THRESHOLD
        && request.elapsed_ms >= PREVIEW_REKNOCK_INTERVAL_MS
}

/// Predict the visual zoom at render completion time.
///
/// Uses the animation velocity and estimated render duration to forecast
/// where visualZoom will be when the bitmap is ready. Rendering at this
/// predicted value minimizes the cssScale gap at commit time.
pub fn predict_render_target(
    visual_zoom: f32,
    target_zoom: f32,
    animation_velocity: f32,
    estimated_render_ms: f32,
) -> f32 {
    let predicted = visual_zoom + animation_velocity * (estimated_render_ms / 1000.0);
    // Clamp to [min(visual, target), max(visual, target)] — don't overshoot target
    let lo = visual_zoom.min(target_zoom);
    let hi = visual_zoom.max(target_zoom);
    predicted.clamp(lo, hi)
}

/// Decide whether to render based on current blur level.
pub fn should_render(
    visual_zoom: f32,
    last_rendered_zoom: f32,
    target_zoom: f32,
    animation_settled: bool,
    animation_velocity: f32,
    estimated_render_ms: f32,
) -> ShouldRender {
    let base = last_rendered_zoom.max(0.001);
    let css_scale = visual_zoom / base;
    let blur = (css_scale - 1.0).abs();

    if animation_settled {
        return ShouldRender::Yes { render_zoom: target_zoom };
    }

    let predicted = predict_render_target(
        visual_zoom, target_zoom, animation_velocity, estimated_render_ms,
    );

    if blur > BLUR_HIGH_THRESHOLD {
        ShouldRender::Yes { render_zoom: predicted }
    } else if blur > BLUR_LOW_THRESHOLD {
        ShouldRender::Soon { render_zoom: predicted }
    } else {
        ShouldRender::Skip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── should_reknock_preview_render ────────────────────────────────

    #[test]
    fn reknock_below_blur_threshold_is_skipped() {
        let r = should_reknock_preview_render(PreviewReknockRequest {
            blur: 0.05,
            elapsed_ms: 1000.0,
            render_in_flight: false,
        });
        assert!(!r);
    }

    #[test]
    fn reknock_at_blur_threshold_and_elapsed_fires() {
        let r = should_reknock_preview_render(PreviewReknockRequest {
            blur: 0.06,
            elapsed_ms: 140.0,
            render_in_flight: false,
        });
        assert!(r);
    }

    #[test]
    fn reknock_while_render_in_flight_is_suppressed() {
        let r = should_reknock_preview_render(PreviewReknockRequest {
            blur: 0.5,
            elapsed_ms: 1000.0,
            render_in_flight: true,
        });
        assert!(!r);
    }

    #[test]
    fn reknock_inside_throttle_window_is_suppressed() {
        let r = should_reknock_preview_render(PreviewReknockRequest {
            blur: 0.5,
            elapsed_ms: 139.0,
            render_in_flight: false,
        });
        assert!(!r);
    }

    #[test]
    fn reknock_fresh_gesture_elapsed_zero_is_suppressed() {
        let r = should_reknock_preview_render(PreviewReknockRequest {
            blur: 0.4,
            elapsed_ms: 0.0,
            render_in_flight: false,
        });
        assert!(!r);
    }

    // ─── predict_render_target ─────────────────────────────────────────

    #[test]
    fn predict_render_target_no_velocity() {
        // No velocity → predicted = visual_zoom
        let r = predict_render_target(1.0, 2.0, 0.0, 16.0);
        assert!((r - 1.0).abs() < 0.01);
    }

    #[test]
    fn predict_render_target_zoom_in() {
        // Zooming in: velocity positive, predicted ahead of visual
        let r = predict_render_target(1.0, 2.0, 2.0, 16.0);
        // predicted = 1.0 + 2.0 * 0.016 = 1.032
        assert!((r - 1.032).abs() < 0.01);
    }

    #[test]
    fn predict_render_target_clamped_to_target() {
        // Predicted exceeds target → clamped to target
        let r = predict_render_target(1.5, 1.6, 10.0, 50.0);
        // predicted = 1.5 + 10.0 * 0.05 = 2.0, clamped to 1.6
        assert!((r - 1.6).abs() < 0.01);
    }

    #[test]
    fn predict_render_target_zoom_out() {
        // Zooming out: velocity negative
        let r = predict_render_target(2.0, 1.0, -2.0, 16.0);
        // predicted = 2.0 + (-2.0) * 0.016 = 1.968
        assert!((r - 1.968).abs() < 0.01);
    }

    // ─── should_render ─────────────────────────────────────────────────

    #[test]
    fn should_render_settled() {
        let r = should_render(1.5, 1.0, 1.5, true, 0.0, 16.0);
        match r {
            ShouldRender::Yes { render_zoom } => assert!((render_zoom - 1.5).abs() < 0.01),
            _ => panic!("expected Yes"),
        }
    }

    #[test]
    fn should_render_high_blur() {
        // cssScale = 1.2 / 1.0 = 1.2, blur = 0.20 > 0.10
        let r = should_render(1.2, 1.0, 2.0, false, 2.0, 16.0);
        match r {
            ShouldRender::Yes { .. } => {} // good
            _ => panic!("expected Yes for high blur"),
        }
    }

    #[test]
    fn should_render_low_blur() {
        // cssScale = 1.05 / 1.0 = 1.05, blur = 0.05 in (0.03, 0.10]
        let r = should_render(1.05, 1.0, 2.0, false, 2.0, 16.0);
        match r {
            ShouldRender::Soon { .. } => {} // good
            _ => panic!("expected Soon for low blur"),
        }
    }

    #[test]
    fn should_render_no_blur() {
        // cssScale = 1.01 / 1.0 = 1.01, blur = 0.01 < 0.03
        let r = should_render(1.01, 1.0, 2.0, false, 2.0, 16.0);
        match r {
            ShouldRender::Skip => {} // good
            _ => panic!("expected Skip for no blur"),
        }
    }
}
