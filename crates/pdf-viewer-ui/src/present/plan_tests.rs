// ADR-0012 contract tests — a zoom-gesture refresh must render only the
// visible viewport tile, not the full page.
//
// Measured root cause (zoom_p2_probe, 2026-10-01): every wheel step knocked a
// full-page reknock render whose stage bitmap reached 3920×5546 (≈21.7M px),
// blocking the main thread 100–126ms per step and starving the tile stream
// (queue 15 / ready 0). The viewport-tile path exists and is proven at
// display_zoom > safe_render_zoom (9.73× here); the decision just never
// considered that a mid-gesture refresh only needs the viewport.

use crate::present::plan_builder::FramePlanRequest;
use crate::present::present_store::build_frame_plan_result;
use crate::zoom::zoom_store;

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

fn reset_zoom(target: f32, visual: f32) {
    zoom_store::with_zoom_state_mut(|state| {
        state.target_zoom = target;
        state.visual_zoom = visual;
    });
}

/// A mid-gesture refresh (the reknock captured display_zoom 2.30 while the
/// animation has already moved target to 2.50) must take the viewport-tile
/// path: full-page stage bitmaps are O(page × zoom²) main-thread cost.
#[wasm_bindgen_test::wasm_bindgen_test]
fn gesture_refresh_renders_viewport_tile_not_full_page() {
    reset_zoom(2.5, 2.42);
    let plan = build_frame_plan_result(&request(2.30));

    assert!(
        plan.use_viewport_tile,
        "a gesture refresh below the canvas-dim limit must still use the viewport tile (ADR-0012), got use_viewport_tile=false"
    );
    assert!(
        !plan.render_base_layer,
        "the full-page base must be reused while the gesture refreshes only the viewport"
    );
    assert!(
        plan.render_detail_layer,
        "the viewport detail tile must be the layer that renders"
    );
    assert!(
        (plan.render_zoom - 2.30).abs() < 0.001,
        "the viewport tile renders at native display zoom, got {}",
        plan.render_zoom
    );
}

/// Far below the canvas-dim limit the criterion must be the ONLY reason the
/// path flips — 1.3× is the range where the video still showed ~120ms stalls.
#[wasm_bindgen_test::wasm_bindgen_test]
fn low_zoom_gesture_refresh_also_uses_viewport_tile() {
    reset_zoom(1.5, 1.45);
    let plan = build_frame_plan_result(&request(1.30));

    assert!(
        plan.use_viewport_tile,
        "a low-zoom gesture refresh must not re-render the full page either"
    );
}

/// ADR-0016 moves the settle boundary from the 10240px memory guard (9.73×)
/// down to the viewport pixel budget. Below the budget the settle render is
/// unchanged: full-page base, sharp.
#[wasm_bindgen_test::wasm_bindgen_test]
fn settle_render_within_pixel_budget_keeps_full_page() {
    // viewport 1200×800@1.25 → 1.5M px; budget = 2× = 3M px; the page is
    // 595×842@1.25 ≈ 0.78M px per zoom unit → budget_zoom ≈ 1.96.
    reset_zoom(1.5, 1.5);
    let plan = build_frame_plan_result(&request(1.5));

    assert!(
        !plan.use_viewport_tile,
        "1.5× (≈0.78M px·1.5² ≈ 1.76M < 3M budget) must keep the full-page path"
    );
    assert!(
        (plan.base_render_zoom - 1.5).abs() < 0.001,
        "within budget the base renders at the full display zoom, got {}",
        plan.base_render_zoom
    );
}

/// ADR-0016: the SETTLE render must also flip to the viewport-tile path once
/// the full-page bitmap exceeds the viewport-relative pixel budget — this is
/// the 190-264ms freeze the user feels when the wheel is released.
#[wasm_bindgen_test::wasm_bindgen_test]
fn settle_render_above_pixel_budget_uses_viewport_tile() {
    reset_zoom(3.0, 3.0);
    let plan = build_frame_plan_result(&request(3.0));

    assert!(
        plan.use_viewport_tile,
        "a settle render whose full-page bitmap exceeds the pixel budget must use the viewport tile (ADR-0016)"
    );
    assert!(
        (plan.render_zoom - 3.0).abs() < 0.001,
        "the detail tile renders at native display zoom, got {}",
        plan.render_zoom
    );
    assert!(
        plan.base_render_zoom < 3.0,
        "the base bitmap must be clamped to the budget, got {}",
        plan.base_render_zoom
    );
    assert!(
        (plan.css_scale - 1.0).abs() < 0.001,
        "the viewport-tile path must not CSS-scale the base (ADR-0004), got {}",
        plan.css_scale
    );
}

// ─── ADR-0023: base cache entries bind to real bitmaps ──────────────────────
// Root cause of the user-visible "zoomed text stays blurry / looks non-vector"
// report: mid-gesture frames remembered base-layer cache entries for zooms
// whose base bitmap was NEVER rendered (phantom entries). At settle those
// entries matched, the settle render was suppressed, and the initial-zoom
// bitmap stayed CSS-upscaled (measured R=0.812, edges 2x softer than native).

use crate::editor::session::render_scene_key;
use crate::render::tile_cache::{BaseLayerCacheEntry, HostPresentState};
use crate::present::present_store::{
    reset_frame_cache, store_frame_cache_entry, PRESENT_STATE,
};

fn poison_active_base_layer(key: &str, cache_zoom: f32) {
    PRESENT_STATE.with(|state| {
        state.borrow_mut().active_base_layer = Some(BaseLayerCacheEntry {
            key: key.to_string(),
            cache_zoom,
            scene_key: render_scene_key(),
        });
    });
}

fn reset_present_and_frame_cache() {
    PRESENT_STATE.with(|state| {
        *state.borrow_mut() = HostPresentState::default();
    });
    reset_frame_cache();
}

/// A phantom entry (zoom whose base bitmap was never rendered) must NOT
/// suppress the settle render: the plan must demand a fresh base render.
#[wasm_bindgen_test::wasm_bindgen_test]
fn phantom_base_cache_entry_does_not_suppress_settle_render() {
    reset_present_and_frame_cache();
    reset_zoom(1.231, 1.231);
    poison_active_base_layer("doc|0|scene|base|1.2300|1.2500", 1.23);

    let plan = build_frame_plan_result(&request(1.231));

    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-PHANTOM-002\",\"method\":\"build_frame_plan_result\",\"actual\":{{\"renderBaseLayer\":{}}},\"outcome\":\"{}\"}}",
        plan.render_base_layer,
        if plan.render_base_layer { "PASS" } else { "FAIL" },
    );
    assert!(
        plan.render_base_layer,
        "a settle whose only cached base has no bitmap in the frame cache must re-render the base"
    );
    assert!(
        !plan.reuse_active_base_layer,
        "a phantom entry must not be reported as reusable"
    );
}

/// Control: when the bitmap for the cached entry REALLY exists in the frame
/// cache, the 2% settle reuse stays in place (no re-render storm).
#[wasm_bindgen_test::wasm_bindgen_test]
fn legitimate_base_reuse_still_works_when_bitmap_exists() {
    reset_present_and_frame_cache();
    reset_zoom(1.231, 1.231);
    poison_active_base_layer("doc|0|scene|base|1.2300|1.2500", 1.23);
    // The bitmap was really rendered and stored by the TS frame-cache path.
    store_frame_cache_entry(false, "doc|0|scene|base|1.2300|1.2500".to_string());

    let plan = build_frame_plan_result(&request(1.231));

    println!(
        "{{\"seq\":1,\"event\":\"zoom.test.assertion\",\"case_id\":\"Z-PHANTOM-003\",\"method\":\"build_frame_plan_result\",\"actual\":{{\"renderBaseLayer\":{}}},\"outcome\":\"{}\"}}",
        plan.render_base_layer,
        if !plan.render_base_layer { "PASS" } else { "FAIL" },
    );
    assert!(
        !plan.render_base_layer,
        "a cached base whose bitmap exists in the frame cache must be reused at settle"
    );
    assert!(
        plan.reuse_active_base_layer,
        "the reuse flag must reflect the legitimate reuse"
    );
}
