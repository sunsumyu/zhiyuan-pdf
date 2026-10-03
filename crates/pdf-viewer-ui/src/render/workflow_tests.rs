// ADR-0023 contract — base-layer cache entries must only be created for
// bitmaps that were actually rendered.
//
// Root cause of the user-visible "zoomed text stays blurry / looks non-vector"
// report: mid-gesture frames carry `reuse_active_base_layer=true` (preview is
// always "active") with `base_cache_zoom = quantize(visual)` — and
// `settle_render_frame_inner` remembered those as base-layer cache entries even
// though the base bitmap at that zoom was NEVER rendered (the plan reused the
// old bitmap + viewport tiles). The phantom entries then matched at settle, so
// the settle render was skipped entirely and the initial-zoom bitmap stayed on
// screen, CSS-upscaled (measured R=0.812, edges 2x softer than native).

use super::workflow::settle_render_frame_inner;
use crate::render::render_store::{reset_render_state, schedule_render_frame};
use crate::viewport_refresh::HostViewportRefreshState;
use crate::zoom::zoom_store::HostZoomState;

fn mid_gesture_plan_json() -> serde_json::Value {
    // A reknock frame mid-gesture: viewport-tile path, base reused (never
    // rendered at this zoom), fresh base cache key/zoom for the visual.
    serde_json::json!({
        "renderSceneKey": "scene-test",
        "renderReason": "zoom",
        "prepareVisibleLayout": true,
        "displayZoom": 0.94,
        "renderZoom": 0.94,
        "baseRenderZoom": 0.94,
        "baseCacheZoom": 0.94,
        "baseCacheKey": "doc|0|scene-test|base|0.9400|1.2500",
        "detailCacheKey": "doc|0|scene-test|detail|0.9400|1.2500",
        "detailCacheZoom": 0.94,
        "cssScale": 1.0,
        "useViewportTile": true,
        "previewSettled": false,
        "allowRenderDuringPreview": true,
        "showDetailOverlay": true,
        "reuseActiveBaseLayer": true,
        "renderBaseLayer": false,
        "preferProgressiveBase": false,
        "reuseActiveDetailTile": false,
        "renderDetailLayer": true,
        "preferProgressiveDetail": false,
        "hostWidth": 743.0,
        "hostHeight": 1051.0,
        "contentLeft": 0.0,
        "contentTop": 0.0,
        "scrollLeft": 0.0,
        "scrollTop": 0.0,
        "tileLeft": 0.0,
        "tileTop": 0.0,
        "tileWidth": 743.0,
        "tileHeight": 1051.0
    })
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn mid_gesture_commit_must_not_create_phantom_base_cache_entry() {
    reset_render_state();
    let frame = schedule_render_frame(
        &mid_gesture_plan_json(),
        |_: &serde_json::Value| true,
        |_a: &serde_json::Value, _b: &serde_json::Value| false,
        |v: &serde_json::Value| v.clone(),
        |v: &serde_json::Value| serde_json::from_value(v.clone()).ok(),
    )
    .expect("mid-gesture frame must schedule (detail layer renders)");

    let mut zoom_state = HostZoomState::default();
    let mut present_state = pdf_viewer_ui_present_state_default();
    let mut refresh_state = HostViewportRefreshState::default();

    let transition = settle_render_frame_inner(
        frame.frame_token,
        Some(0.94),
        &mut zoom_state,
        &mut present_state,
        &mut refresh_state,
    );
    assert!(transition.accepted);

    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-PHANTOM-001\",\"method\":\"settle_render_frame_inner\",\"actual\":{{\"active_base_layer\":{}}},\"outcome\":\"{}\"}}",
        present_state.active_base_layer.is_some(),
        if present_state.active_base_layer.is_none() { "PASS" } else { "FAIL" },
    );
    assert!(
        present_state.active_base_layer.is_none(),
        "a frame that did not render the base must not create a base cache entry (phantom)"
    );
}

// helper: fresh present state without reaching into another crate's internals
fn pdf_viewer_ui_present_state_default() -> crate::render::tile_cache::HostPresentState {
    crate::render::tile_cache::HostPresentState::default()
}

