use super::render_store::{
    is_render_frame_current, reset_render_state, schedule_render_frame, settle_render_frame,
};
use pdf_viewer_core::render::plan_builder::FramePlanResult;

fn plan(render_zoom: f32) -> FramePlanResult {
    FramePlanResult {
        render_zoom,
        ..Default::default()
    }
}

fn share_by_render_zoom(a: &FramePlanResult, b: &FramePlanResult) -> bool {
    a.render_zoom == b.render_zoom
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn frame_tokens_reject_stale_settle_and_promote_queued_frame() {
    reset_render_state();
    let first = schedule_render_frame(&plan(1.0), |_| true, share_by_render_zoom)
        .expect("first frame should be scheduled");
    let queued = schedule_render_frame(&plan(2.0), |_| true, share_by_render_zoom);
    assert!(queued.is_none());
    assert!(is_render_frame_current(first.frame_token));

    let stale = settle_render_frame(first.frame_token + 100);
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-FRAME-001\",\"method\":\"settle_render_frame\",\"actual\":{{\"accepted\":{},\"has_next\":{}}},\"outcome\":\"{}\"}}",
        stale.accepted,
        stale.next_frame.is_some(),
        if !stale.accepted && stale.next_frame.is_none() { "PASS" } else { "FAIL" },
    );
    assert!(!stale.accepted);
    assert!(stale.next_frame.is_none());
    assert!(is_render_frame_current(first.frame_token));

    let settled = settle_render_frame(first.frame_token);
    assert!(settled.accepted);
    let next = settled.next_frame.expect("queued frame should be promoted");
    assert_eq!(next.frame_plan.render_zoom, 2.0);
    assert!(is_render_frame_current(next.frame_token));
    assert_ne!(first.frame_token, next.frame_token);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn reset_invalidates_current_token_and_zero_token_is_never_current() {
    reset_render_state();
    let frame = schedule_render_frame(&plan(3.0), |_| true, share_by_render_zoom)
        .expect("frame should be scheduled");
    assert!(is_render_frame_current(frame.frame_token));
    reset_render_state();
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-FRAME-RESET\",\"method\":\"reset_render_state\",\"actual\":{{\"old_current\":{},\"zero_current\":{}}},\"outcome\":\"{}\"}}",
        is_render_frame_current(frame.frame_token),
        is_render_frame_current(0),
        if !is_render_frame_current(frame.frame_token) && !is_render_frame_current(0) { "PASS" } else { "FAIL" },
    );
    assert!(!is_render_frame_current(frame.frame_token));
    assert!(!is_render_frame_current(0));
}
