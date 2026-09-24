use super::render_store::{
    is_render_frame_current, reset_render_state, schedule_render_frame, settle_render_frame,
};

fn plan(value: i32) -> i32 {
    value
}

fn encode(value: &i32) -> serde_json::Value {
    serde_json::json!({ "value": value })
}

fn decode(value: &serde_json::Value) -> Option<i32> {
    value.get("value")?.as_i64().map(|value| value as i32)
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn frame_tokens_reject_stale_settle_and_promote_queued_frame() {
    reset_render_state();
    let first = schedule_render_frame(&plan(1), |_| true, |a, b| a == b, encode, decode)
        .expect("first frame should be scheduled");
    let queued = schedule_render_frame(&plan(2), |_| true, |a, b| a == b, encode, decode);
    assert!(queued.is_none());
    assert!(is_render_frame_current(first.frame_token));

    let stale = settle_render_frame(first.frame_token + 100, decode);
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-FRAME-001\",\"method\":\"settle_render_frame\",\"actual\":{{\"accepted\":{},\"has_next\":{}}},\"outcome\":\"{}\"}}",
        stale.accepted,
        stale.next_frame.is_some(),
        if !stale.accepted && stale.next_frame.is_none() { "PASS" } else { "FAIL" },
    );
    assert!(!stale.accepted);
    assert!(stale.next_frame.is_none());
    assert!(is_render_frame_current(first.frame_token));

    let settled = settle_render_frame(first.frame_token, decode);
    assert!(settled.accepted);
    let next = settled.next_frame.expect("queued frame should be promoted");
    assert_eq!(next.frame_plan, 2);
    assert!(is_render_frame_current(next.frame_token));
    assert_ne!(first.frame_token, next.frame_token);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn reset_invalidates_current_token_and_zero_token_is_never_current() {
    reset_render_state();
    let frame = schedule_render_frame(&plan(3), |_| true, |a, b| a == b, encode, decode)
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
