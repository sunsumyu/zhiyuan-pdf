// ADR-0017 contract tests — the zoom follow-up frame MUST converge to the
// decision's target zoom.
//
// Measured root cause (CDP `followup.decide`, 2026-10-02, zero-shim repro):
// after a burst of ctrl+wheel the renderer entered a non-converging follow-up
// loop, ~200 frames/s, forever identical:
//
//   rendered=1.1249064 target=1.142407 visual=1.142407 schedule=true
//   decideTarget=1.142407 reqZoom=1.1249064
//
// `resolve_render_follow_up_decision` correctly returned target=1.142407 and
// `set_zoom(1.142407)` ran, but `schedule_render_frame_request` was handed the
// caller's request, still carrying display_zoom=1.1249064 (the *rendered* zoom).
// The scheduled frame therefore re-rendered 1.1249064, `commit_rendered_zoom`
// re-recorded it, and `needs_render(target, last_rendered)` stayed true forever.
//
// These tests exercise the real wasm export so the exact production defect is
// the red light.

use serde_wasm_bindgen::{from_value, to_value};

use super::free_api::schedule_render_follow_up;
use crate::present::plan_builder::FramePlanRequest;
use crate::render::host_runtime::reset_render_loop_runtime;
use crate::render::render_store::reset_render_state;
use crate::render::workflow::RenderFrameEnvelope;
use crate::present::present_store::reset_present_runtime;
use crate::zoom::zoom_store::{reset_zoom_state, with_zoom_state_mut};

fn request(display_zoom: f32) -> FramePlanRequest {
    FramePlanRequest {
        display_zoom,
        render_reason: "zoom".to_string(),
        page_width: 595.0,
        page_height: 842.0,
        viewport_width: 1200.0,
        viewport_height: 800.0,
        scroll_left: 0.0,
        scroll_top: 0.0,
        device_pixel_ratio: 1.25,
        max_zoom: 30.0,
        max_canvas_dim: 10240.0,
        timestamp_ms: 0.0,
        force_static_render_scale: None,
    }
}

fn setup(target: f32, visual: f32, rendered: f32) {
    reset_render_state();
    reset_present_runtime(true, true);
    reset_render_loop_runtime();
    reset_zoom_state(rendered);
    with_zoom_state_mut(|state| {
        state.target_zoom = target;
        state.visual_zoom = visual;
    });
}

fn frame_plan(frame: wasm_bindgen::JsValue) -> Option<RenderFrameEnvelope> {
    from_value::<RenderFrameEnvelope>(frame).ok()
}

/// The exact measured freeze: the request still carries the rendered zoom
/// (1.1249064) while the decision target is 1.142407. The follow-up frame must
/// render the target, not re-render the stale zoom — otherwise the loop never
/// converges.
#[wasm_bindgen_test::wasm_bindgen_test]
fn follow_up_rebuilds_request_at_decided_target() {
    setup(1.142407, 1.142407, 1.1249064);

    let frame = schedule_render_follow_up(1.1249064, to_value(&request(1.1249064)).unwrap());
    let envelope = frame_plan(frame).expect("a converging follow-up frame must be scheduled");

    let rendered = envelope.frame_plan.display_zoom;
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-FOLLOWUP-001\",\"method\":\"schedule_render_follow_up\",\"actual\":{{\"display_zoom\":{}}},\"outcome\":\"{}\"}}",
        rendered,
        if (rendered - 1.142407).abs() < 1e-4 { "PASS" } else { "FAIL" },
    );
    assert!(
        (rendered - 1.142407).abs() < 1e-4,
        "the follow-up frame must render the decision target 1.142407, got {rendered} (stale request zoom not overridden)"
    );
    assert!(
        (rendered - 1.1249064).abs() > 1e-4,
        "the follow-up frame must NOT re-render the already-rendered zoom 1.1249064"
    );
}

/// Once the decision target equals the rendered zoom the follow-up must return
/// no frame — this is what terminates the loop.
#[wasm_bindgen_test::wasm_bindgen_test]
fn follow_up_converges_when_target_equals_rendered() {
    setup(1.142407, 1.142407, 1.142407);

    let frame = schedule_render_follow_up(1.142407, to_value(&request(1.142407)).unwrap());
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-FOLLOWUP-002\",\"method\":\"schedule_render_follow_up\",\"actual\":{{\"is_null\":{}}},\"outcome\":\"{}\"}}",
        frame.is_null(),
        if frame.is_null() { "PASS" } else { "FAIL" },
    );
    assert!(
        frame.is_null(),
        "a settled follow-up (target == rendered) must schedule nothing"
    );
}

/// C1 contract: mid-gesture the bitmap must track the visual zoom (so
/// css_scale stays ~1.0). The convergence fix must not regress this.
#[wasm_bindgen_test::wasm_bindgen_test]
fn follow_up_tracks_visual_zoom_during_gesture() {
    setup(1.5, 1.30, 1.10);

    let frame = schedule_render_follow_up(1.10, to_value(&request(1.10)).unwrap());
    let envelope = frame_plan(frame).expect("a mid-gesture follow-up frame must be scheduled");

    let rendered = envelope.frame_plan.display_zoom;
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-FOLLOWUP-003\",\"method\":\"schedule_render_follow_up\",\"actual\":{{\"display_zoom\":{}}},\"outcome\":\"{}\"}}",
        rendered,
        if (rendered - 1.30).abs() < 1e-4 { "PASS" } else { "FAIL" },
    );
    assert!(
        (rendered - 1.30).abs() < 1e-4,
        "mid-gesture the follow-up must render visual_zoom (1.30), got {rendered}"
    );
}

/// ADR-0019: the follow-up is a render-side actor — it must never write the
/// zoom authority. Mid-gesture the decision's effective target is the CURRENT
/// VISUAL; `set_zoom`ing it over the user's target truncates the zoom step to
/// wherever the animation happened to be (observed: a single wheel step
/// landing at 0.9601/0.9258 instead of 0.9013 in ~1/3 of E2E runs).
#[wasm_bindgen_test::wasm_bindgen_test]
fn follow_up_never_writes_the_zoom_authority_mid_gesture() {
    setup(1.5, 1.30, 1.10);

    let frame = schedule_render_follow_up(1.10, to_value(&request(1.10)).unwrap());
    let envelope = frame_plan(frame).expect("a mid-gesture follow-up frame must be scheduled");
    assert!((envelope.frame_plan.display_zoom - 1.30).abs() < 1e-4);

    let target = crate::zoom::zoom_store::read_zoom_state().target_zoom;
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-FOLLOWUP-004\",\"method\":\"schedule_render_follow_up\",\"actual\":{{\"target_zoom\":{}}},\"outcome\":\"{}\"}}",
        target,
        if (target - 1.5).abs() < 1e-4 { "PASS" } else { "FAIL" },
    );
    assert!(
        (target - 1.5).abs() < 1e-4,
        "the follow-up must not write target_zoom: user target 1.5 survives, got {target}"
    );
}

/// Settled path: writing was a no-op before; after the fix the invariant is
/// structural — the follow-up simply has no write path to the authority.
#[wasm_bindgen_test::wasm_bindgen_test]
fn follow_up_never_writes_the_zoom_authority_settled() {
    setup(1.142407, 1.142407, 1.1249064);

    let frame = schedule_render_follow_up(1.1249064, to_value(&request(1.1249064)).unwrap());
    let envelope = frame_plan(frame).expect("a settled follow-up frame must be scheduled");
    assert!((envelope.frame_plan.display_zoom - 1.142407).abs() < 1e-4);

    let target = crate::zoom::zoom_store::read_zoom_state().target_zoom;
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-FOLLOWUP-005\",\"method\":\"schedule_render_follow_up\",\"actual\":{{\"target_zoom\":{}}},\"outcome\":\"{}\"}}",
        target,
        if (target - 1.142407).abs() < 1e-4 { "PASS" } else { "FAIL" },
    );
    assert!(
        (target - 1.142407).abs() < 1e-4,
        "settled target_zoom must survive the follow-up unchanged, got {target}"
    );
}
