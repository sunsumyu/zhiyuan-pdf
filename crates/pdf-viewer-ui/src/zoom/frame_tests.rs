use super::zoom_authority::{set_target_zoom_authoritative, set_target_zoom_instant};
use super::zoom_frame::{queue_committed_frame, take_ready_committed_frame};
use crate::zoom::zoom_store::{PendingCommittedFrame, ZOOM_STATE};

fn frame(zoom: f32) -> PendingCommittedFrame {
    PendingCommittedFrame {
        display_zoom: zoom,
        render_zoom: zoom,
        host_width: 600.0 * zoom,
        host_height: 800.0 * zoom,
        content_left: 0.0,
        content_top: 0.0,
        scroll_left: 0.0,
        scroll_top: 0.0,
    }
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn committed_frame_waits_until_zoom_is_settled() {
    set_target_zoom_instant(1.0);
    queue_committed_frame(&frame(1.0));
    set_target_zoom_authoritative(2.0);

    let blocked = take_ready_committed_frame();
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-QUEUE-001\",\"method\":\"take_ready_committed_frame\",\"actual\":{{\"ready\":{}}},\"outcome\":\"{}\"}}",
        blocked.is_some(),
        if blocked.is_none() { "PASS" } else { "FAIL" },
    );
    assert!(blocked.is_none());

    set_target_zoom_instant(1.0);
    let ready = take_ready_committed_frame().expect("frame should be ready after settle");
    assert!((ready.display_zoom - 1.0).abs() < 0.001);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn queue_replaces_pending_frame_with_latest_plan() {
    set_target_zoom_instant(1.0);
    queue_committed_frame(&frame(1.25));
    queue_committed_frame(&frame(1.5));

    let ready = take_ready_committed_frame().expect("latest frame should be available");
    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-QUEUE-002\",\"method\":\"queue_committed_frame\",\"actual\":{{\"display_zoom\":{}}},\"outcome\":\"{}\"}}",
        ready.display_zoom,
        if (ready.display_zoom - 1.5).abs() < 0.001 { "PASS" } else { "FAIL" },
    );
    assert!((ready.display_zoom - 1.5).abs() < 0.001);
    assert!(take_ready_committed_frame().is_none());

    ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.preview_host.pending_committed_frame = None;
    });
}
