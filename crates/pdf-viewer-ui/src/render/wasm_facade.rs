// ─────────────────────────────────────────────────────────────────────────────
// Render facade — frozen v1 wasm API surface for the progressive render pipeline.
//
// Note: existing `wasm_api/viewer.rs` exposes raw render entrypoints under
// `start_progressive_render` / `step_progressive_render` / etc. This file
// re-exports them under canonical `renderFacade*` js_names so the frontend has
// a single, stable namespace for render operations.
//
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::present::present_store::{
    is_render_frame_current, reset_frame_cache, settle_render_frame, store_frame_cache_entry,
    touch_frame_cache_entry,
};
use crate::render::commit::commit_render_result;
use crate::render::progressive_workflow::{
    cancel_progressive_render, render_page, start_progressive_render, step_progressive_render,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StubResult {
    implemented: bool,
    error: String,
}

fn stub(api: &str) -> JsValue {
    let result = StubResult {
        implemented: false,
        error: format!("{} is reserved but not yet implemented", api),
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

// ─── Stable — progressive render lifecycle ───────────────────────────────────

#[wasm_bindgen(js_name = "renderFacadeStartProgressive")]
pub fn facade_start_progressive() -> JsValue {
    to_value(&start_progressive_render()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeStepProgressive")]
pub fn facade_step_progressive(
    canvas_id: String,
    image_cache: JsValue,
    budget_ms: f64,
    max_items: u32,
) -> JsValue {
    to_value(&step_progressive_render(
        canvas_id,
        image_cache,
        budget_ms,
        max_items,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeCancelProgressive")]
pub fn facade_cancel_progressive() {
    cancel_progressive_render();
}

#[wasm_bindgen(js_name = "renderFacadeRenderPage")]
pub fn facade_render_page(canvas_id: String, image_cache: JsValue) {
    render_page(canvas_id, image_cache);
}

// ─── Stable — frame commit / settle ──────────────────────────────────────────

#[wasm_bindgen(js_name = "renderFacadeCommitResult")]
pub fn facade_commit_result(
    frame_token: u32,
    rendered_zoom: f32,
    page_width: f32,
    page_height: f32,
) -> JsValue {
    to_value(&commit_render_result(
        frame_token,
        rendered_zoom,
        page_width,
        page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeAbortFrame")]
pub fn facade_abort_frame(frame_token: u32) -> JsValue {
    let transition = settle_render_frame(frame_token, None);
    to_value(&transition).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeIsFrameCurrent")]
pub fn facade_is_frame_current(frame_token: u32) -> bool {
    is_render_frame_current(frame_token)
}

// ─── Stable — frame cache ────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "renderFacadeTouchCache")]
pub fn facade_touch_cache(is_detail: bool, key: String) -> bool {
    touch_frame_cache_entry(is_detail, &key)
}

#[wasm_bindgen(js_name = "renderFacadeStoreCache")]
pub fn facade_store_cache(is_detail: bool, key: String) -> JsValue {
    to_value(&store_frame_cache_entry(is_detail, key)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "renderFacadeResetCache")]
pub fn facade_reset_cache() {
    reset_frame_cache();
}

// ─── Tile-based rendering (C1) ──────────────────────────────────────────────
//
// All entrypoints share ONE TileManager via render::tile_host. Do NOT declare
// function-local thread_locals here — they would be distinct statics.

use crate::render::tile_host::with_tile_manager;

/// Update viewport and schedule tile rendering
#[wasm_bindgen(js_name = "renderFacadeUpdateViewport")]
pub fn facade_update_viewport(
    page: u16,
    zoom: f32,
    dpr: f32,
    viewport_x: f32,
    viewport_y: f32,
    viewport_width: f32,
    viewport_height: f32,
    frame_token: u32,
) -> JsValue {
    with_tile_manager(|mgr| {
        mgr.update_viewport(page, zoom, dpr, viewport_x, viewport_y, viewport_width, viewport_height, frame_token);
        to_value(&mgr.stats()).unwrap_or(JsValue::NULL)
    })
}

/// Start zoom animation (marks tiles eligible for eviction)
#[wasm_bindgen(js_name = "renderFacadeStartTileAnimation")]
pub fn facade_start_tile_animation(target_zoom: f32) {
    with_tile_manager(|mgr| mgr.start_animation(target_zoom));
}

/// Update animation state (called each frame during zoom)
#[wasm_bindgen(js_name = "renderFacadeUpdateTileAnimation")]
pub fn facade_update_tile_animation(visual_zoom: f32, frame_token: u32) {
    with_tile_manager(|mgr| mgr.update_animation(visual_zoom, frame_token));
}

/// End zoom animation (schedules final high-res tiles)
#[wasm_bindgen(js_name = "renderFacadeEndTileAnimation")]
pub fn facade_end_tile_animation(frame_token: u32) {
    with_tile_manager(|mgr| mgr.end_animation(frame_token));
}

/// Get next tile render request from queue
#[wasm_bindgen(js_name = "renderFacadeNextTileRequest")]
pub fn facade_next_tile_request() -> JsValue {
    with_tile_manager(|mgr| match mgr.next_render_request() {
        Some(request) => to_value(&request).unwrap_or(JsValue::NULL),
        None => JsValue::NULL,
    })
}

/// Mark a tile as rendering
#[wasm_bindgen(js_name = "renderFacadeMarkTileRendering")]
pub fn facade_mark_tile_rendering(page: u16, zoom: f32, dpr: f32, x: i32, y: i32) -> bool {
    let key = pdf_viewer_core::render::tile_v2::TileKey::new(page, zoom, dpr, x, y);
    with_tile_manager(|mgr| mgr.mark_rendering(&key))
}

/// Mark a tile as ready
#[wasm_bindgen(js_name = "renderFacadeMarkTileReady")]
pub fn facade_mark_tile_ready(page: u16, zoom: f32, dpr: f32, x: i32, y: i32) -> bool {
    let key = pdf_viewer_core::render::tile_v2::TileKey::new(page, zoom, dpr, x, y);
    with_tile_manager(|mgr| mgr.mark_ready(&key))
}

/// Flip a Rendering tile back to Pending (dropped render / render error)
#[wasm_bindgen(js_name = "renderFacadeResetTile")]
pub fn facade_reset_tile(page: u16, zoom: f32, dpr: f32, x: i32, y: i32) -> bool {
    let key = pdf_viewer_core::render::tile_v2::TileKey::new(page, zoom, dpr, x, y);
    with_tile_manager(|mgr| mgr.reset_stale_rendering(&key))
}

/// Check if a tile is ready for display
#[wasm_bindgen(js_name = "renderFacadeIsTileReady")]
pub fn facade_is_tile_ready(page: u16, zoom: f32, dpr: f32, x: i32, y: i32) -> bool {
    let key = pdf_viewer_core::render::tile_v2::TileKey::new(page, zoom, dpr, x, y);
    with_tile_manager(|mgr| mgr.is_tile_ready(&key))
}

/// Clear tile cache for a specific page
#[wasm_bindgen(js_name = "renderFacadeClearTileCache")]
pub fn facade_clear_tile_cache(page: u16) {
    with_tile_manager(|mgr| mgr.clear_page(page));
}

/// Get tile cache statistics
#[wasm_bindgen(js_name = "renderFacadeTileStats")]
pub fn facade_tile_stats() -> JsValue {
    with_tile_manager(|mgr| to_value(&mgr.stats()).unwrap_or(JsValue::NULL))
}

// ─── Progressive Quality Rendering (ADR-0004) ────────────────────────────────

/// Start animation quality state machine (reset to Low quality)
#[wasm_bindgen(js_name = "renderFacadeStartQualityAnimation")]
pub fn facade_start_quality_animation() {
    use pdf_viewer_core::render::quality::QualityStateMachine;
    use std::cell::RefCell;

    thread_local! {
        static QUALITY_SM: RefCell<QualityStateMachine> = RefCell::new(QualityStateMachine::new());
    }

    QUALITY_SM.with(|sm| {
        sm.borrow_mut().start_animation();
    });
}

/// Update quality based on animation state
#[wasm_bindgen(js_name = "renderFacadeUpdateQuality")]
pub fn facade_update_quality(is_animating: bool, settled: bool) -> u32 {
    use pdf_viewer_core::render::quality::QualityStateMachine;
    use std::cell::RefCell;

    thread_local! {
        static QUALITY_SM: RefCell<QualityStateMachine> = RefCell::new(QualityStateMachine::new());
    }

    QUALITY_SM.with(|sm| {
        let quality = sm.borrow_mut().update(is_animating, settled);
        quality as u32
    })
}

/// Get current quality level
#[wasm_bindgen(js_name = "renderFacadeGetQuality")]
pub fn facade_get_quality() -> u32 {
    use pdf_viewer_core::render::quality::QualityStateMachine;
    use std::cell::RefCell;

    thread_local! {
        static QUALITY_SM: RefCell<QualityStateMachine> = RefCell::new(QualityStateMachine::new());
    }

    QUALITY_SM.with(|sm| {
        sm.borrow().current() as u32
    })
}

/// Get quality DPI multiplier
#[wasm_bindgen(js_name = "renderFacadeGetQualityDpi")]
pub fn facade_get_quality_dpi(quality: u32) -> f32 {
    use pdf_viewer_core::render::quality::RenderQuality;

    let q = match quality {
        0 => RenderQuality::Low,
        1 => RenderQuality::Medium,
        2 => RenderQuality::High,
        _ => RenderQuality::Medium,
    };
    q.dpi_multiplier()
}

/// Get quality budget in milliseconds
#[wasm_bindgen(js_name = "renderFacadeGetQualityBudget")]
pub fn facade_get_quality_budget(quality: u32) -> f64 {
    use pdf_viewer_core::render::quality::RenderQuality;

    let q = match quality {
        0 => RenderQuality::Low,
        1 => RenderQuality::Medium,
        2 => RenderQuality::High,
        _ => RenderQuality::Medium,
    };
    q.budget_ms()
}

// ─── Stubs ───────────────────────────────────────────────────────────────────

/// Reserved: render the current page off-screen and return a PNG buffer.
#[wasm_bindgen(js_name = "renderFacadeSnapshotPng")]
pub fn facade_snapshot_png(_dpi: f32) -> JsValue {
    stub("render.snapshotPng")
}

/// Reserved: pre-warm the frame cache for an upcoming page.
#[wasm_bindgen(js_name = "renderFacadePrewarmCache")]
pub fn facade_prewarm_cache(_page_index: u16) -> JsValue {
    stub("render.prewarmCache")
}

/// Reserved: configure rendering quality presets (draft / normal / high).
#[wasm_bindgen(js_name = "renderFacadeSetQuality")]
pub fn facade_set_quality(_preset: String) -> JsValue {
    stub("render.setQuality")
}

/// Reserved: enable/disable debug overlay (tile boundaries, frame tokens).
#[wasm_bindgen(js_name = "renderFacadeSetDebugOverlay")]
pub fn facade_set_debug_overlay(_enabled: bool) -> JsValue {
    stub("render.setDebugOverlay")
}
