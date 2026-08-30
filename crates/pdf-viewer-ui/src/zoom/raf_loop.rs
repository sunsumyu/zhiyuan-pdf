//! Zoom RAF loop — Rust-driven requestAnimationFrame for smooth zoom.
//!
//! The RAF loop runs entirely in Rust: each frame advances the animation
//! state machine and applies CSS transforms / scroll / layout via web-sys.
//! TS only needs to:
//!   1. Call `start_zoom_raf_loop()` once after init
//!   2. Bind wheel events to `on_wheel_event()`
//!   3. Push committed frames via `commit_rendered_frame()`
//!
//! Sub-modules:
//! - `raf_dom_cache`: DOM element caching
//! - `raf_transform`: CSS transform computation and application
//! - `raf_settle`: Settle cleanup RAF
//! - `raf_committed`: Committed frame queue and application
//! - `raf_dispatch`: Settle envelope dispatch (ADR-0001)

use std::cell::RefCell;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use pdf_viewer_core::render::zoom::animation::{
    advance_zoom_animation_state, resolve_wheel_zoom_request, WheelZoomRequest,
};
use pdf_viewer_core::render::zoom::decision::{
    should_reknock_preview_render, PreviewReknockRequest,
};

use crate::zoom::zoom_store::ZOOM_STATE;

use super::raf_dom_cache::{init_dom_cache, with_dom_cache, clear_dom_cache};
use super::raf_transform::{apply_css_transform, LAST_APPLIED_SCALE};
use super::raf_settle::{cancel_settle_cleanup, schedule_settle_cleanup};
use super::raf_committed::{pop_committed_frame, apply_committed_frame};
use super::raf_dispatch::dispatch_settle_envelope;

// ─── RAF closure storage ──────────────────────────────────────────

thread_local! {
    /// The currently scheduled RAF handle (non-zero means loop is active).
    static RAF_HANDLE: RefCell<Option<i32>> = RefCell::new(None);

    /// The stored RAF closure. We use `JsValue` (from `Closure::once_into_js`)
    /// because it can be stored without knowing the concrete closure type.
    static RAF_CLOSURE: RefCell<Option<JsValue>> = RefCell::new(None);

    /// Timestamp of the last mid-animation render knock (throttle window).
    static LAST_PREVIEW_KNOCK: RefCell<f64> = RefCell::new(0.0);
}

// ─── Animation constants ──────────────────────────────────────────

/// Drawing delay after animation settles before requesting the final render.
const SETTLE_DRAWING_DELAY_MS: f64 = 50.0;

// ─── Public API ───────────────────────────────────────────────────

/// Start the zoom RAF loop. Safe to call multiple times (no-op if already running).
pub fn start_zoom_raf_loop() {
    // Cancel any pending settle cleanup — a new gesture supersedes it
    cancel_settle_cleanup();

    RAF_HANDLE.with(|handle| {
        if handle.borrow().is_some() {
            return; // already running
        }
    });

    // Initialize DOM cache on first use
    init_dom_cache();

    // ADR-0002 I3: gesture start must leave exactly one active surface.
    // The raster sibling (width:100% of the wrapper) can never track the
    // transform-driven container, so it goes; the container keeps its last
    // settled bitmap (display:none never clears a canvas).
    let raster_visible = with_dom_cache(|dom| {
        dom.and_then(|d| d.raster.as_ref()).map(|raster| {
            let display = raster.style().get_property_value("display").unwrap_or_default();
            let visible = display != "none";
            if visible {
                let _ = raster.style().set_property("display", "none");
            }
            visible
        }).unwrap_or(false)
    });
    if raster_visible {
        with_dom_cache(|dom| {
            if let Some(dom) = dom {
                let _ = dom.container.style().set_property("display", "block");
            }
        });
    }

    // Reset last applied scale so first frame always applies
    LAST_APPLIED_SCALE.with(|s| *s.borrow_mut() = (f32::NAN, (0.0, 0.0)));

    schedule_next_frame();
}

/// Stop the zoom RAF loop immediately.
///
/// NOTE: Does NOT reset `LAST_APPLIED_SCALE` — the settle path needs the
/// last CSS scale and cursor position to compute the compensating translate
/// in `apply_committed_frame`. The cleanup RAF (`schedule_settle_cleanup`)
/// handles resetting it after the translate is applied.
pub fn stop_zoom_raf_loop() {
    // Cancel any pending settle cleanup
    cancel_settle_cleanup();

    RAF_HANDLE.with(|handle| {
        if let Some(_h) = handle.borrow_mut().take() {
            // Note: we can't easily cancel a Closure::once_into_js RAF
            // because we don't have the original handle. The next tick
            // will be a no-op because the closure checks RAF_HANDLE.
        }
    });
    RAF_CLOSURE.with(|c| *c.borrow_mut() = None);
    clear_dom_cache();
    // Intentionally NOT resetting LAST_APPLIED_SCALE here.
    // see doc comment above.
}

/// Check if the RAF loop is currently running.
pub fn is_raf_loop_running() -> bool {
    RAF_HANDLE.with(|h| h.borrow().is_some())
}

/// Wheel event input from TS — all raw DOM values.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WheelEventInput {
    pub delta_y: f32,
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub timestamp_ms: f64,
}

/// Wheel event output — minimal data TS needs for sync.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WheelEventOutput {
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub css_scale: f32,
}

/// Push a committed frame into the queue (re-export from raf_committed).
pub use super::raf_committed::commit_rendered_frame;
/// Committed frame type (re-export from raf_committed).
pub use super::raf_committed::CommittedFrame;

/// Handle a complete wheel event. TS only passes raw DOM values.
pub fn on_wheel_event(input: WheelEventInput) -> WheelEventOutput {
    let max_zoom = 30.0_f32; // TODO: derive from page size + DPR
    let min_zoom = 0.1_f32;
    // Content box at the currently displayed zoom — matches the old TS wheel
    // path (`pageWidth * visualZoom`). The anchor resolution in core compares
    // scroll offsets against this box, so it must reflect what is on screen,
    // not the base page size.
    let (content_width, content_height) = ZOOM_STATE.with(|state| {
        let s = state.borrow();
        let current = if s.visual_zoom > 0.0 { s.visual_zoom } else { 1.0 };
        (input.page_width * current, input.page_height * current)
    });

    let output = ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();

        let request = WheelZoomRequest {
            delta_y: input.delta_y,
            viewport_x: input.viewport_x,
            viewport_y: input.viewport_y,
            viewport_width: input.viewport_width,
            viewport_height: input.viewport_height,
            page_width: input.page_width,
            page_height: input.page_height,
            anchor_page_x: None,
            anchor_page_y: None,
            page_ratio_x: None,
            page_ratio_y: None,
            scroll_left: input.scroll_left,
            scroll_top: input.scroll_top,
            content_width,
            content_height,
            target_zoom: if s.target_zoom > 0.0 { s.target_zoom } else { 1.0 },
            min_zoom,
            max_zoom,
        };

        // Use the core resolve_wheel_zoom_request for proper anchor computation
        let (result, pending_anchor) = resolve_wheel_zoom_request(
            &request,
            s.visual_layout.as_ref(),
            s.preview_transform.as_ref(),
        );

        // Update state — reset animation timestamp so first tick computes dt correctly
        s.target_zoom = result.target_zoom;
        s.last_animation_timestamp_ms = 0.0;
        s.pending_anchor = Some(pending_anchor);

        // ADR-0004 (revised): css_scale = visual / last_rendered. It returns to
        // 1.0 at every commit (render tracks visual_zoom) — reporting 1.0 here
        // desyncs the RAF's change-detection from the DOM and destabilizes zoom.
        let base = if s.last_rendered_zoom > 0.0 { s.last_rendered_zoom } else { 1.0 };
        let css_scale = s.visual_zoom / base;

        WheelEventOutput {
            target_zoom: result.target_zoom,
            visual_zoom: s.visual_zoom,
            css_scale,
        }
    });

    output
}

/// Called after wheel input is applied: guarantee the RAF loop is ticking.
///
/// The loop stops itself shortly after settle (drawing delay expires), so it
/// must be (re)started on every wheel event — starting it only at bind time
/// leaves zoom dead once the initial loop session has ended. Starting is
/// idempotent when the loop is already running.
pub fn ensure_raf_loop_after_wheel() {
    start_zoom_raf_loop();
}

// ─── RAF tick implementation ──────────────────────────────────────

fn schedule_next_frame() {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    // Create the RAF closure
    let closure = Closure::once_into_js(move |timestamp_ms: f64| {
        tick(timestamp_ms);
    });

    let handle = window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .unwrap_or(0);

    RAF_HANDLE.with(|h| *h.borrow_mut() = Some(handle));
    RAF_CLOSURE.with(|c| *c.borrow_mut() = Some(closure));
}

fn tick(timestamp_ms: f64) {
    // Check if we're still the active loop
    let still_active = RAF_HANDLE.with(|h| h.borrow().is_some());
    if !still_active {
        return;
    }

    // The container may be created after this loop session started (e.g. the
    // first render builds pdf-page-container lazily). Retry until it exists —
    // a permanently-empty cache would make every transform a silent no-op.
    let dom_cache_ready = with_dom_cache(|d| d.is_some());
    if !dom_cache_ready {
        init_dom_cache();
    }

    // ── 1. Advance animation ──
    let (settled, visual_zoom, css_scale) = ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();
        let step = advance_zoom_animation_state(&mut s, Some(timestamp_ms));
        (step.settled, step.visual_zoom, step.css_scale)
    });

    // ── 2. Apply CSS transform (skip if unchanged) ──
    let (last_scale, last_cursor) = LAST_APPLIED_SCALE.with(|s| {
        let b = s.borrow();
        (b.0, b.1)
    });
    let (cursor_x, cursor_y) = ZOOM_STATE.with(|state| {
        let s = state.borrow();
        if let Some(ref anchor) = s.pending_anchor {
            (anchor.viewport_x, anchor.viewport_y)
        } else {
            (0.0, 0.0)
        }
    });
    let scale_changed = last_scale.is_nan() || (css_scale - last_scale).abs() >= 0.0005;
    let cursor_changed = (cursor_x - last_cursor.0).abs() >= 0.5
        || (cursor_y - last_cursor.1).abs() >= 0.5;
    if scale_changed || cursor_changed {
        apply_css_transform();
        LAST_APPLIED_SCALE.with(|s| *s.borrow_mut() = (css_scale, (cursor_x, cursor_y)));
    }

    // ── 2.5 Mid-animation re-render when blur exceeds threshold (ADR-0002) ──
    if !settled {
        let (blur, anchor_active) = ZOOM_STATE.with(|state| {
            let s = state.borrow();
            let base = if s.last_rendered_zoom > 0.0 { s.last_rendered_zoom } else { 1.0 };
            ((s.visual_zoom / base - 1.0).abs(), s.pending_anchor.is_some())
        });
        if anchor_active {
            let render_in_flight =
                crate::render::render_store::RENDER_STATE.with(|state| {
                    state.borrow().in_flight_frame_token != 0
                });
            let elapsed_ms = LAST_PREVIEW_KNOCK.with(|t| timestamp_ms - *t.borrow());
            if should_reknock_preview_render(PreviewReknockRequest {
                blur,
                elapsed_ms,
                render_in_flight,
            }) {
                LAST_PREVIEW_KNOCK.with(|t| *t.borrow_mut() = timestamp_ms);
                dispatch_settle_envelope();
            }
        }
    }

    // ── 3. Poll committed frame queue ──
    if let Some(frame) = pop_committed_frame() {
        apply_committed_frame(frame, visual_zoom);
    }

    // ── 4. Drawing delay after settle ──
    if settled {
        let should_render = ZOOM_STATE.with(|state| {
            let mut s = state.borrow_mut();
            if !s.drawing_delay.active {
                s.drawing_delay.active = true;
                s.drawing_delay.started_at_ms = timestamp_ms;
                s.drawing_delay.delay_ms = SETTLE_DRAWING_DELAY_MS as u32;
                false
            } else {
                let elapsed = timestamp_ms - s.drawing_delay.started_at_ms;
                if elapsed >= s.drawing_delay.delay_ms as f64 {
                    s.drawing_delay.active = false;
                    true
                } else {
                    false
                }
            }
        });

        if should_render {
            stop_zoom_raf_loop();
            dispatch_settle_envelope();
            return;
        }
    }

    // ── 5. Schedule next frame ──
    if settled && !ZOOM_STATE.with(|s| s.borrow().drawing_delay.active) {
        stop_zoom_raf_loop();
        dispatch_settle_envelope();
    } else {
        schedule_next_frame();
    }
}
