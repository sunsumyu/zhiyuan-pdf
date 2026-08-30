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

#[wasm_bindgen_test::wasm_bindgen_test]
fn i4_instant_zoom_snaps_visual_and_commit_lands_at_scale_one() {
    // 回归：程序化缩放（下拉框/自适应宽度）只写 target、没有 RAF 推进
    // visual，提交 0.48 渲染后 css_scale = visual/last_rendered = 1/0.48
    // = 2.0833，页面被放大 2 倍（用户报告的"内容双份/巨大"）。
    // set_target_zoom_instant 必须把 visual 快照到 target。
    use crate::zoom::zoom_controller::{mark_rendered_zoom, set_target_zoom_instant};

    // 前置：布局已提交在 1.0
    mark_rendered_zoom(1.0);
    set_target_zoom_authoritative(1.0);
    mark_rendered_zoom(1.0);

    // 程序化跳到 48%：target 与 visual 必须同时到位
    set_target_zoom_instant(0.48);
    let state = read_zoom_state();
    assert!(
        (state.target_zoom - 0.48).abs() < 0.001,
        "target {} != 0.48",
        state.target_zoom
    );
    assert!(
        (state.visual_zoom - 0.48).abs() < 0.001,
        "visual {} did not snap to target",
        state.visual_zoom
    );
    // 提交前：s = visual/layout = 0.48/1.0（正确缩小，I1 成立）
    assert!(
        (state.css_scale - 0.48).abs() < 0.005,
        "pre-commit css_scale {} != 0.48",
        state.css_scale
    );

    // 提交 0.48 渲染后：s 必须精确回到 1.0（禁止 1/0.48 反转）
    mark_rendered_zoom(0.48);
    let state = read_zoom_state();
    assert!(
        (state.css_scale - 1.0).abs() < 0.001,
        "post-commit css_scale {} != 1.0 (inverted ratio regression)",
        state.css_scale
    );

    // 还原
    set_target_zoom_instant(1.0);
    mark_rendered_zoom(1.0);
}
