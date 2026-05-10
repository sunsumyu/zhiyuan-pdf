//! ZoomController — P3 struct-based WASM API for zoom, wheel, preview & animation.
//!
//! Mirrors the P0/P1/P2 pattern: zero-sized struct + camelCase methods + thin
//! delegation. The flat `wasm_api::zoom_api` functions remain for backward
//! compatibility while TS migrates.
//!
//! All operations are infallible, so no response wrapper is needed.

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::host::command::apply_zoom_selection;
use crate::present::facade::resolve_render_zoom;
use crate::present::plan_builder::{FramePlanRequest, RenderZoomRequest};
use crate::viewer::viewer_controller::{
    reset_zoom_view, set_zoom,
};
use crate::zoom::event::{
    handle_wheel_zoom_host,
    step_preview_host, PreviewHostStepRequest,
    WheelZoomHostRequest,
};
use crate::zoom::host::{
    resolve_preview_tick_decision,
    resolve_wheel_render_decision,
    PreviewTickDecisionRequest, WheelRenderDecisionRequest,
};
use crate::zoom::interaction::{
    resolve_zoom_limits_result, AnchorScrollRequest, WheelZoomRequest, ZoomLimitsRequest,
};
use crate::zoom::preview_host::{
    clear_zoom_preview_host_state,
    get_wheel_render_pending,
    queue_committed_frame,
    reset_zoom_preview_host,
    set_wheel_render_pending,
    take_ready_committed_frame,
};
use crate::zoom::request::{
    resolve_anchor_scroll,
    resolve_wheel_zoom,
};
use crate::zoom::zoom_controller::{
    clear_pending_anchor,
    clear_preview_present,
    get_zoom_state,
    mark_rendered_zoom,
    set_target_zoom,
    step_zoom_animation,
    step_zoom_frame_plan,
};
use crate::zoom::zoom_store::PendingCommittedFrame;

// ── ZoomController ──────────────────────────────────────────────

#[wasm_bindgen]
pub struct ZoomController;

#[wasm_bindgen]
impl ZoomController {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        ZoomController
    }

    // ── Zoom state ──────────────────────────────────────────────

    /// Reset the zoom view to a known initial zoom factor.
    #[wasm_bindgen(js_name = "resetState")]
    pub fn reset_state(&self, initial_zoom: f32) {
        reset_zoom_view(initial_zoom);
    }

    /// Read the current zoom state snapshot.
    #[wasm_bindgen(js_name = "getState")]
    pub fn get_state(&self) -> JsValue {
        to_value(&get_zoom_state()).unwrap_or(JsValue::NULL)
    }

    /// Set the target zoom (animation drives the visible zoom toward it).
    #[wasm_bindgen(js_name = "setTargetZoom")]
    pub fn set_target_zoom(&self, target_zoom: f32) {
        set_target_zoom(target_zoom);
    }

    /// Mark the current rendered zoom level after a render commit.
    #[wasm_bindgen(js_name = "markRenderedZoom")]
    pub fn mark_rendered_zoom(&self, rendered_zoom: f32) {
        mark_rendered_zoom(rendered_zoom);
    }

    /// Set the page-level current zoom (viewer-level snapshot).
    #[wasm_bindgen(js_name = "setCurrentZoom")]
    pub fn set_current_zoom(&self, zoom: f32) {
        set_zoom(zoom);
    }

    /// Apply a discrete zoom selection (e.g. dropdown choice).
    #[wasm_bindgen(js_name = "applySelection")]
    pub fn apply_selection(&self, zoom: f32) -> JsValue {
        to_value(&apply_zoom_selection(zoom)).unwrap_or(JsValue::NULL)
    }

    // ── Wheel & anchor scroll ───────────────────────────────────

    /// Resolve a wheel-zoom interaction (returns target zoom + anchor info).
    #[wasm_bindgen(js_name = "resolveWheel")]
    pub fn resolve_wheel(&self, request_js: JsValue) -> JsValue {
        let request: WheelZoomRequest = from_value(request_js).unwrap_or_default();
        to_value(&resolve_wheel_zoom(&request)).unwrap_or(JsValue::NULL)
    }

    /// Handle the host-side wheel-zoom event (preview + scheduling).
    #[wasm_bindgen(js_name = "handleWheelHost")]
    pub fn handle_wheel_host(&self, request_js: JsValue) -> JsValue {
        let request: WheelZoomHostRequest = from_value(request_js).unwrap_or_default();
        to_value(&handle_wheel_zoom_host(request)).unwrap_or(JsValue::NULL)
    }

    /// Resolve anchor-scroll behaviour after a zoom (keeps point under cursor stable).
    #[wasm_bindgen(js_name = "resolveAnchorScroll")]
    pub fn resolve_anchor_scroll(&self, request_js: JsValue) -> JsValue {
        let request: AnchorScrollRequest = from_value(request_js).unwrap_or_default();
        to_value(&resolve_anchor_scroll(&request)).unwrap_or(JsValue::NULL)
    }

    /// Resolve whether wheel motion should trigger a full render.
    #[wasm_bindgen(js_name = "resolveWheelRenderDecision")]
    pub fn resolve_wheel_render_decision(&self, request_js: JsValue) -> JsValue {
        let request: WheelRenderDecisionRequest = from_value(request_js).unwrap_or_default();
        to_value(&resolve_wheel_render_decision(request)).unwrap_or(JsValue::NULL)
    }

    /// Resolve zoom min/max bounds for the current viewport.
    #[wasm_bindgen(js_name = "resolveLimits")]
    pub fn resolve_limits(&self, request_js: JsValue) -> JsValue {
        let request: ZoomLimitsRequest = from_value(request_js).unwrap_or_default();
        to_value(&resolve_zoom_limits_result(&request)).unwrap_or(JsValue::NULL)
    }

    /// Resolve the render-time zoom (separate from preview zoom).
    #[wasm_bindgen(js_name = "resolveRenderZoom")]
    pub fn resolve_render_zoom(&self, request_js: JsValue) -> JsValue {
        let request: RenderZoomRequest = from_value(request_js).unwrap_or_default();
        to_value(&resolve_render_zoom(&request)).unwrap_or(JsValue::NULL)
    }

    // ── Preview host ────────────────────────────────────────────

    /// Resolve the next preview-tick scheduling decision.
    #[wasm_bindgen(js_name = "resolvePreviewTickDecision")]
    pub fn resolve_preview_tick_decision(&self, request_js: JsValue) -> JsValue {
        let request: PreviewTickDecisionRequest = from_value(request_js).unwrap_or_default();
        to_value(&resolve_preview_tick_decision(request)).unwrap_or(JsValue::NULL)
    }

    /// Step the preview host one tick.
    #[wasm_bindgen(js_name = "stepPreviewHost")]
    pub fn step_preview_host(&self, request_js: JsValue) -> JsValue {
        let request: PreviewHostStepRequest = from_value(request_js).unwrap_or_default();
        to_value(&step_preview_host(request)).unwrap_or(JsValue::NULL)
    }

    /// Reset the preview-host state to a known target zoom.
    #[wasm_bindgen(js_name = "resetPreviewHost")]
    pub fn reset_preview_host(&self, target_zoom: f32) {
        reset_zoom_preview_host(target_zoom);
    }

    /// Clear all pending preview-host bookkeeping.
    #[wasm_bindgen(js_name = "clearPreviewHostState")]
    pub fn clear_preview_host_state(&self, clear_pending_anchor: bool) {
        clear_zoom_preview_host_state(clear_pending_anchor);
    }

    /// Clear pending preview-present buffer.
    #[wasm_bindgen(js_name = "clearPreviewPresent")]
    pub fn clear_preview_present(&self) {
        clear_preview_present();
    }

    // ── Wheel render pending flag ───────────────────────────────

    /// Check whether a wheel-induced render is currently pending.
    #[wasm_bindgen(js_name = "getWheelRenderPending")]
    pub fn get_wheel_render_pending(&self) -> bool {
        get_wheel_render_pending()
    }

    /// Set/clear the wheel-induced render pending flag.
    #[wasm_bindgen(js_name = "setWheelRenderPending")]
    pub fn set_wheel_render_pending(&self, pending: bool) {
        set_wheel_render_pending(pending);
    }

    // ── Committed frame queue ───────────────────────────────────

    /// Queue a committed frame for later present.
    #[wasm_bindgen(js_name = "queueCommittedFrame")]
    pub fn queue_committed_frame(&self, frame_js: JsValue) {
        let frame: PendingCommittedFrame = from_value(frame_js).unwrap_or_default();
        queue_committed_frame(&frame);
    }

    /// Take the next ready committed frame (consumes it).
    #[wasm_bindgen(js_name = "takeReadyCommittedFrame")]
    pub fn take_ready_committed_frame(&self) -> JsValue {
        to_value(&take_ready_committed_frame()).unwrap_or(JsValue::NULL)
    }

    // ── Zoom animation ──────────────────────────────────────────

    /// Step the zoom animation by one frame.
    #[wasm_bindgen(js_name = "stepAnimation")]
    pub fn step_animation(&self) -> JsValue {
        to_value(&step_zoom_animation()).unwrap_or(JsValue::NULL)
    }

    /// Step the zoom-driven frame plan one tick.
    #[wasm_bindgen(js_name = "stepFramePlan")]
    pub fn step_frame_plan(&self, request_js: JsValue) -> JsValue {
        let request: FramePlanRequest = from_value(request_js).unwrap_or_default();
        to_value(&step_zoom_frame_plan(&request)).unwrap_or(JsValue::NULL)
    }

    /// Clear any pending anchor scroll bookkeeping.
    #[wasm_bindgen(js_name = "clearPendingAnchor")]
    pub fn clear_pending_anchor(&self) {
        clear_pending_anchor();
    }
}

impl Default for ZoomController {
    fn default() -> Self {
        Self::new()
    }
}
