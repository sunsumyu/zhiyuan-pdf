//! TDD: 缩放单一权威（ADR-0001 候选1）
//!
//! 不变量：
//! I1. VIEWER_SESSION 不再存储 zoom —— read_viewer_session() 快照中的
//!     current_zoom 恒等于 ZOOM_STATE.target_zoom（派生投影）。
//! I2. 所有写入收敛到单入口 set_target_zoom_authoritative()：
//!     wheel / apply_zoom_selection / follow-up / setState 写回都走它。
//! I3. wheel 路径不再需要镜像 —— 权威写后投影立即跟随。

use crate::viewer::viewer_store::read_viewer_session;
use crate::zoom::zoom_controller::{read_zoom_state, set_target_zoom_authoritative};

#[wasm_bindgen_test::wasm_bindgen_test]
fn i1_session_snapshot_current_zoom_is_projection_of_target() {
    // 无文档默认态：快照 current_zoom == ZOOM_STATE.target_zoom
    let session = read_viewer_session();
    let target = read_zoom_state().target_zoom;
    assert!(
        (session.current_zoom - target).abs() < f32::EPSILON,
        "snapshot current_zoom {} != authority target {}",
        session.current_zoom,
        target
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn i2_single_entry_write_updates_projection_immediately() {
    // 经唯一入口写 2.5 → 快照立即反映，无需任何镜像调用
    set_target_zoom_authoritative(2.5);
    let session = read_viewer_session();
    assert!(
        (session.current_zoom - 2.5).abs() < 0.001,
        "projection did not follow authority write: {}",
        session.current_zoom
    );

    // 非法值被 sanitize 为 1.0
    set_target_zoom_authoritative(-3.0);
    assert!((read_viewer_session().current_zoom - 1.0).abs() < 0.001);

    set_target_zoom_authoritative(f32::NAN);
    assert!((read_viewer_session().current_zoom - 1.0).abs() < 0.001);

    // 还原
    set_target_zoom_authoritative(1.0);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn i3_wheel_path_no_longer_mirrors_into_session_store() {
    // 权威合并后 on_wheel_event 只写权威；投影自动跟随。
    // viewer_controller::set_zoom 已改为单入口薄委托 —— 若有人恢复
    // 向 session 存储的镜像写，I1 会失败。
    set_target_zoom_authoritative(1.75);
    assert!((read_viewer_session().current_zoom - 1.75).abs() < 0.001);
    set_target_zoom_authoritative(1.0);
}
