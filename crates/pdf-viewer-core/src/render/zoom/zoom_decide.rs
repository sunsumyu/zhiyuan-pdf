//! Zoom-level decisions: wheel render timing, commit/flush staleness guards.
//!
//! Pure logic — no DOM, no WASM, no thread_local. All functions accept
//! value-type request structs and return decision structs.

use serde::{Deserialize, Serialize};

use crate::render::present_plan::preview_is_settled;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelRenderDecisionRequest {
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    pub preview_active: bool,
    pub allow_render_during_preview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WheelRenderDecision {
    pub request_render_now: bool,
    pub defer_until_settled: bool,
    pub skip_render: bool,
    pub delay_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTickDecisionRequest {
    pub settled: bool,
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    pub wheel_render_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTickDecision {
    pub continue_preview: bool,
    pub flush_committed_frame: bool,
    pub request_render_now: bool,
    pub keep_wheel_render_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderFollowUpDecision {
    pub schedule_latest_target: bool,
    pub target_zoom: f32,
}

fn needs_render(target_zoom: f32, rendered_zoom: f32) -> bool {
    (target_zoom - rendered_zoom).abs() >= 0.001
}

/// Choose the render target during preview.
///
/// Returns `visual_zoom` directly so the bitmap is rendered at exactly the
/// zoom level the user sees. CSS scale ≈ 1.0 → no stretch blur.
pub fn resolve_preview_render_zoom(
    visual_zoom: f32,
    _last_rendered_zoom: f32,
    _target_zoom: f32,
) -> f32 {
    visual_zoom
}

fn wheel_render_idle_ms(preview_active: bool, allow_render_during_preview: bool) -> u32 {
    if preview_active && allow_render_during_preview {
        16 // RAF frame interval — render every frame during active preview
    } else if preview_active {
        48 // Preview active but rendering deferred
    } else {
        64
    }
}

fn preview_is_active(preview_active: bool, target_zoom: f32, visual_zoom: f32) -> bool {
    preview_active || (target_zoom - visual_zoom).abs() >= 0.001
}

pub fn resolve_wheel_render_decision(request: WheelRenderDecisionRequest) -> WheelRenderDecision {
    let is_preview = preview_is_active(
        request.preview_active,
        request.target_zoom,
        request.visual_zoom,
    );

    // During preview, use the adaptive render target (close to lastRenderedZoom)
    // instead of targetZoom for the needs_render check. This avoids unnecessary
    // renders when the adaptive target hasn't moved far from lastRenderedZoom.
    let effective_target = if is_preview {
        resolve_preview_render_zoom(
            request.visual_zoom,
            request.last_rendered_zoom,
            request.target_zoom,
        )
    } else {
        request.target_zoom
    };

    if !needs_render(effective_target, request.last_rendered_zoom) {
        return WheelRenderDecision {
            skip_render: true,
            delay_ms: wheel_render_idle_ms(
                request.preview_active,
                request.allow_render_during_preview,
            ),
            ..WheelRenderDecision::default()
        };
    }

    if is_preview {
        if request.allow_render_during_preview {
            return WheelRenderDecision {
                request_render_now: true,
                delay_ms: wheel_render_idle_ms(
                    request.preview_active,
                    request.allow_render_during_preview,
                ),
                ..WheelRenderDecision::default()
            };
        }
        return WheelRenderDecision {
            defer_until_settled: true,
            delay_ms: wheel_render_idle_ms(
                request.preview_active,
                request.allow_render_during_preview,
            ),
            ..WheelRenderDecision::default()
        };
    }

    WheelRenderDecision {
        request_render_now: true,
        delay_ms: wheel_render_idle_ms(request.preview_active, request.allow_render_during_preview),
        ..WheelRenderDecision::default()
    }
}

pub fn resolve_preview_tick_decision(request: PreviewTickDecisionRequest) -> PreviewTickDecision {
    if request.settled {
        return PreviewTickDecision {
            continue_preview: false,
            flush_committed_frame: true,
            request_render_now: request.wheel_render_pending
                && needs_render(request.target_zoom, request.last_rendered_zoom),
            keep_wheel_render_pending: false,
        };
    }

    PreviewTickDecision {
        continue_preview: true,
        flush_committed_frame: false,
        request_render_now: false,
        keep_wheel_render_pending: request.wheel_render_pending,
    }
}

pub fn resolve_render_follow_up_decision(
    rendered_display_zoom: f32,
    current_target_zoom: f32,
    current_visual_zoom: f32,
) -> RenderFollowUpDecision {
    // Preview 期间用 visualZoom（bitmap 应追踪视觉状态），
    // settled 后用 targetZoom（精确到达目标）。
    let preview_settled =
        (current_target_zoom - current_visual_zoom).abs() < 0.001;
    let effective_target = if preview_settled {
        current_target_zoom
    } else {
        current_visual_zoom
    };
    RenderFollowUpDecision {
        schedule_latest_target: needs_render(effective_target, rendered_display_zoom),
        target_zoom: effective_target,
    }
}

// ─── Committed-frame decision ────────────────────────────────────────────────
//
// Single-owner: ALL stale-frame threshold logic lives here.  TS reads the
// returned decision and applies it — no thresholds, no comparisons.

/// Thresholds that were previously hardcoded in TS `zoom_controller.ts`.
/// Centralised here so they can be tuned in one place and tested in pure Rust.
const COMMIT_STALE_RATIO: f32 = 0.10;
const FLUSH_STALE_RATIO: f32 = 0.15;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomCommitDecisionRequest {
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    pub frame_render_zoom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomCommitDecision {
    /// true  → caller should `stopSmoothZoomPreview` + `applyCommittedFrame`
    /// false → caller should `queueCommittedFrame` and continue preview
    pub apply_now: bool,
    /// true  → frame is stale; skip it and force a fresh render
    /// false → frame is usable
    pub skip_stale: bool,
    /// true  → preview is settled (target ≈ visual)
    pub preview_settled: bool,
}

/// Decide how `commitRenderedFrame` should handle a newly rendered frame.
///
/// Replaces the three decision points that were in TS:
///   1. `preview_is_settled` (was reimplemented with same 0.001 epsilon)
///   2. stale-frame guard in `commitRenderedFrame` (was 0.10 ratio)
///   3. the "apply now vs queue" branching
pub fn resolve_zoom_commit_decision(request: ZoomCommitDecisionRequest) -> ZoomCommitDecision {
    let settled = preview_is_settled(request.target_zoom, request.visual_zoom);

    if !settled {
        // Preview is active — queue the frame, continue animation.
        return ZoomCommitDecision {
            apply_now: false,
            skip_stale: false,
            preview_settled: false,
        };
    }

    // Preview is settled — check staleness.
    let frame_zoom = request.frame_render_zoom;
    let settled_zoom = request.target_zoom;
    let stale = (frame_zoom - settled_zoom).abs() / settled_zoom.max(0.01) > COMMIT_STALE_RATIO;

    ZoomCommitDecision {
        apply_now: !stale,
        skip_stale: stale,
        preview_settled: true,
    }
}

// ─── Flush decision ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomFlushDecisionRequest {
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub last_rendered_zoom: f32,
    pub frame_render_zoom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZoomFlushDecision {
    /// true  → call `applyCommittedFrame`
    /// false → skip, force fresh render
    pub apply: bool,
    /// true  → frame is stale; force `documentMutation` render
    pub skip_stale: bool,
}

/// Decide how `flushCommittedFrameIfSettled` should handle a queued frame.
///
/// Replaces the TS-side 0.15 ratio threshold that was independent of Rust.
pub fn resolve_flush_decision(request: ZoomFlushDecisionRequest) -> ZoomFlushDecision {
    let frame_zoom = request.frame_render_zoom;
    let settled_zoom = request.target_zoom;
    let stale =
        (frame_zoom - settled_zoom).abs() / settled_zoom.max(0.01) > FLUSH_STALE_RATIO;

    ZoomFlushDecision {
        apply: !stale,
        skip_stale: stale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit_request(target: f32, visual: f32, last_rendered: f32, frame_render: f32) -> ZoomCommitDecisionRequest {
        ZoomCommitDecisionRequest {
            target_zoom: target,
            visual_zoom: visual,
            last_rendered_zoom: last_rendered,
            frame_render_zoom: frame_render,
        }
    }

    fn flush_request(target: f32, visual: f32, last_rendered: f32, frame_render: f32) -> ZoomFlushDecisionRequest {
        ZoomFlushDecisionRequest {
            target_zoom: target,
            visual_zoom: visual,
            last_rendered_zoom: last_rendered,
            frame_render_zoom: frame_render,
        }
    }

    // ─── resolve_zoom_commit_decision ────────────────────────────────────

    #[test]
    fn commit_preview_not_settled_queues_frame() {
        let d = resolve_zoom_commit_decision(commit_request(1.6, 1.2, 1.0, 1.2));
        assert!(!d.apply_now);
        assert!(!d.skip_stale);
        assert!(!d.preview_settled);
    }

    #[test]
    fn commit_settled_frame_close_applies() {
        // Frame at 1.05, target 1.0 → 5% deviation, below 10% threshold
        let d = resolve_zoom_commit_decision(commit_request(1.0, 1.0, 0.63, 1.05));
        assert!(d.apply_now);
        assert!(!d.skip_stale);
        assert!(d.preview_settled);
    }

    #[test]
    fn commit_settled_frame_far_skips() {
        // Frame at 0.63, target 1.0 → 37% deviation, above 10% threshold
        let d = resolve_zoom_commit_decision(commit_request(1.0, 1.0, 0.63, 0.63));
        assert!(!d.apply_now);
        assert!(d.skip_stale);
        assert!(d.preview_settled);
    }

    #[test]
    fn commit_boundary_10pct_is_not_stale() {
        // ~10% (slightly under) → NOT stale (threshold is strictly greater than)
        // Note: 1.0 - 0.9 = 0.100000024 in f32, so use 0.901 to stay safely under
        let d = resolve_zoom_commit_decision(commit_request(1.0, 1.0, 1.0, 0.901));
        assert!(d.apply_now);
        assert!(!d.skip_stale);
    }

    #[test]
    fn commit_above_10pct_is_stale() {
        // 11% deviation → stale
        let d = resolve_zoom_commit_decision(commit_request(1.0, 1.0, 1.0, 0.89));
        assert!(!d.apply_now);
        assert!(d.skip_stale);
    }

    // ─── resolve_flush_decision ──────────────────────────────────────────

    #[test]
    fn flush_frame_close_applies() {
        let d = resolve_flush_decision(flush_request(1.0, 1.0, 0.63, 0.9));
        assert!(d.apply);
        assert!(!d.skip_stale);
    }

    #[test]
    fn flush_frame_far_skips() {
        // 0.63 vs 1.0 → 37%, above 15% threshold
        let d = resolve_flush_decision(flush_request(1.0, 1.0, 0.63, 0.63));
        assert!(!d.apply);
        assert!(d.skip_stale);
    }

    #[test]
    fn flush_boundary_15pct_is_not_stale() {
        // Exactly 15% deviation → NOT stale (threshold is strictly greater than)
        let d = resolve_flush_decision(flush_request(1.0, 1.0, 1.0, 0.85));
        assert!(d.apply);
        assert!(!d.skip_stale);
    }

    #[test]
    fn flush_above_15pct_is_stale() {
        // 16% deviation → stale
        let d = resolve_flush_decision(flush_request(1.0, 1.0, 1.0, 0.84));
        assert!(!d.apply);
        assert!(d.skip_stale);
    }

    #[test]
    fn flush_below_15pct_applies() {
        let d = resolve_flush_decision(flush_request(1.0, 1.0, 1.0, 0.86));
        assert!(d.apply);
        assert!(!d.skip_stale);
    }

    // ─── resolve_preview_render_zoom ────────────────────────────────────
    // C1: Always returns visualZoom — bitmap tracks the animation exactly,
    // cssScale ≈ 1.0, no CSS stretch blur.

    #[test]
    fn preview_render_zoom_returns_visual_zoom() {
        let r = resolve_preview_render_zoom(1.2, 1.0, 1.6);
        assert!((r - 1.2).abs() < 0.01);
    }

    #[test]
    fn preview_render_zoom_ignores_last_rendered() {
        // Even when lastRendered is far away, returns visualZoom
        let r = resolve_preview_render_zoom(0.5, 1.5, 1.0);
        assert!((r - 0.5).abs() < 0.01);
    }

    #[test]
    fn preview_render_zoom_equal_visual_and_target() {
        let r = resolve_preview_render_zoom(1.5, 1.0, 1.5);
        assert!((r - 1.5).abs() < 0.01);
    }

    #[test]
    fn preview_render_zoom_visual_far_from_last_rendered() {
        // visual=2.0, lastRendered=1.0 → returns 2.0 (not clamped)
        let r = resolve_preview_render_zoom(2.0, 1.0, 3.0);
        assert!((r - 2.0).abs() < 0.01);
    }

    #[test]
    fn preview_render_zoom_visual_behind_target() {
        // visual=1.0, target=1.5 → returns 1.0 (exact visual state)
        let r = resolve_preview_render_zoom(1.0, 1.0, 1.5);
        assert!((r - 1.0).abs() < 0.01);
    }
}
