//! Zoom RAF loop — Rust-driven requestAnimationFrame for zoom.
//!
//! CSS transform zoom has been removed. Container dimensions are set directly
//! by apply_committed_frame via SetBox. The RAF loop handles:
//!   - Animation state machine (advance_zoom_animation_state)
//!   - Committed frame queue polling
//!   - Drawing delay after settle
//!
//! TS only needs to:
//!   1. Call `start_zoom_raf_loop()` once after init
//!   2. Bind wheel events to `on_wheel_event()`
//!   3. Push committed frames via `commit_rendered_frame()`

use std::cell::RefCell;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use pdf_viewer_core::render::plan_builder::compute_viewport_layout_result;
use pdf_viewer_core::render::zoom::animation::{
    advance_zoom_animation_state, resolve_wheel_zoom_request, WheelZoomRequest,
};

use crate::zoom::zoom_store::ZOOM_STATE;

use super::raf_committed::{apply_committed_frame, pop_committed_frame};
use super::raf_dispatch::dispatch_settle_envelope;
use super::raf_dom_cache::{clear_dom_cache, init_dom_cache, with_dom_cache};
use super::raf_settle::cancel_settle_cleanup;

// ─── RAF closure storage ──────────────────────────────────────────

thread_local! {
    /// The currently scheduled RAF handle (non-zero means loop is active).
    static RAF_HANDLE: RefCell<Option<i32>> = const { RefCell::new(None) };

    /// The stored RAF closure.
    static RAF_CLOSURE: RefCell<Option<JsValue>> = const { RefCell::new(None) };

    /// Timestamp of the last mid-animation render knock (throttle window).
    static LAST_PREVIEW_KNOCK: RefCell<f64> = const { RefCell::new(0.0) };

    /// Whether a wheel gesture owns the container geometry. The only writer
    /// is this module: set on every wheel event, cleared when the RAF loop
    /// stops. The commit path reads it through the narrow accessor below to
    /// decide between queueing and applying a frame.
    static WHEEL_GESTURE_ACTIVE: RefCell<bool> = const { RefCell::new(false) };
}

/// Read-only gesture-ownership probe for the commit path (raf_committed).
pub(super) fn is_wheel_gesture_active() -> bool {
    WHEEL_GESTURE_ACTIVE.with(|flag| *flag.borrow())
}

fn set_wheel_gesture_active(active: bool) {
    WHEEL_GESTURE_ACTIVE.with(|flag| *flag.borrow_mut() = active);
}

// ─── Animation constants ──────────────────────────────────────────

/// Drawing delay after animation settles before requesting the final render.
/// Reduced for snappier zoom in direct-redraw mode.
const SETTLE_DRAWING_DELAY_MS: f64 = 30.0;

// ─── Public API ───────────────────────────────────────────────────

/// Start the zoom RAF loop. Safe to call multiple times (no-op if already running).
pub fn start_zoom_raf_loop() {
    cancel_settle_cleanup();

    // No-op if already running: the previous closure-based guard never exited
    // the function (its `return` only left the closure), so the documented
    // contract was broken. Use the shared handle directly.
    if is_raf_loop_running() {
        return;
    }

    init_dom_cache();

    // Hide raster sibling, show vector container (ADR-0002 I3)
    let raster_visible = with_dom_cache(|dom| {
        dom.and_then(|d| d.raster.as_ref())
            .map(|raster| {
                let display = raster
                    .style()
                    .get_property_value("display")
                    .unwrap_or_default();
                let visible = display != "none";
                if visible {
                    let _ = raster.style().set_property("display", "none");
                }
                visible
            })
            .unwrap_or(false)
    });
    if raster_visible {
        with_dom_cache(|dom| {
            if let Some(dom) = dom {
                let _ = dom.container.style().set_property("display", "block");
            }
        });
    }

    schedule_next_frame();
}

/// Stop the zoom RAF loop immediately.
pub fn stop_zoom_raf_loop() {
    cancel_settle_cleanup();

    set_wheel_gesture_active(false);

    RAF_HANDLE.with(|handle| {
        if let Some(_h) = handle.borrow_mut().take() {
            // Next tick will be a no-op because the closure checks RAF_HANDLE.
        }
    });
    RAF_CLOSURE.with(|c| *c.borrow_mut() = None);
    clear_dom_cache();
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
}

/// Push a committed frame into the queue (re-export from raf_committed).
pub use super::raf_committed::commit_rendered_frame;
/// Committed frame type (re-export from raf_committed).
pub use super::raf_committed::CommittedFrame;

/// Handle a complete wheel event. TS only passes raw DOM values.
///
/// Implements SumatraPDF-style "virtual zoom": immediately updates container
/// dimensions and scroll position for instant visual feedback, then dispatches
/// an async render at the target zoom. The existing canvas content gets
/// stretched/compressed by the browser (like CSS transform but via layout).
pub fn on_wheel_event(input: WheelEventInput) -> WheelEventOutput {
    let max_zoom = 30.0_f32;
    let min_zoom = 0.1_f32;
    // Content box sized by the last committed render zoom, matching actual DOM layout.
    let (content_width, content_height) = ZOOM_STATE.with(|state| {
        let s = state.borrow();
        let rendered = if s.last_rendered_zoom > 0.0 {
            s.last_rendered_zoom
        } else {
            1.0
        };
        (input.page_width * rendered, input.page_height * rendered)
    });

    let output = ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();

        // A real wheel event means a gesture owns the geometry from now on.
        set_wheel_gesture_active(true);

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
            target_zoom: if s.target_zoom > 0.0 {
                s.target_zoom
            } else {
                1.0
            },
            min_zoom,
            max_zoom,
        };

        let result = resolve_wheel_zoom_request(&request, s.visual_layout.as_ref());

        s.target_zoom = result.target_zoom;
        s.last_animation_timestamp_ms = 0.0;

        // ── Virtual zoom: immediately apply target layout ──
        // Compute container dimensions at target zoom — always centers content.
        // This gives instant visual feedback — the browser stretches/compresses
        // the existing canvas.
        let display_width = input.page_width * result.target_zoom;
        let display_height = input.page_height * result.target_zoom;
        let layout = compute_viewport_layout_result(
            display_width,
            display_height,
            input.viewport_width,
            input.viewport_height,
        );

        // Update visual_layout to match the virtual zoom state
        s.visual_layout = Some(crate::zoom::zoom_store::VisualLayoutState {
            display_zoom: result.target_zoom,
            content_left: layout.content_left,
            content_top: layout.content_top,
        });

        // Apply to DOM immediately for instant visual feedback.
        with_dom_cache(|dom| {
            if let Some(dom) = dom {
                let style = dom.container.style();
                // Match compute_viewport_layout_result: host = max(display, viewport)
                let host_width = layout.host_width.max(input.viewport_width);
                let host_height = layout.host_height.max(input.viewport_height);
                let _ = style.set_property("width", &format!("{}px", host_width));
                let _ = style.set_property("height", &format!("{}px", host_height));
                let _ = style.set_property("left", &format!("{}px", layout.content_left));
                let _ = style.set_property("top", &format!("{}px", layout.content_top));
            }
        });

        WheelEventOutput {
            target_zoom: result.target_zoom,
            visual_zoom: s.visual_zoom,
        }
    });

    output
}

/// Called after wheel input is applied: guarantee the RAF loop is ticking.
pub fn ensure_raf_loop_after_wheel() {
    start_zoom_raf_loop();
}

// ─── RAF tick implementation ──────────────────────────────────────

/// Drive the canvas CSS scale for continuous visual zoom.
///
/// scale = visual_zoom / last_rendered_zoom: the bitmap on screen grows
/// continuously with the animation while the bitmap underneath stays at the
/// last committed render zoom. A scale of ~1 or a missing canvas is a no-op.
/// When the animation settles the transform is cleared — the committed frames
/// at settle carry target-zoom geometry and the presenter re-boxes the canvas
/// atomically, so no stale transform may remain.
fn apply_canvas_visual_scale(visual_zoom: f32) {
    let (scale, rendered) = ZOOM_STATE.with(|state| {
        let s = state.borrow();
        let rendered = if s.last_rendered_zoom > 0.0 {
            s.last_rendered_zoom
        } else {
            1.0
        };
        (visual_zoom / rendered, s.last_rendered_zoom)
    });
    if rendered <= 0.0 || !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let settled = ZOOM_STATE.with(|state| {
        (state.borrow().visual_zoom - state.borrow().target_zoom).abs() < GESTURE_THRESHOLD
    });
    let transform = if settled {
        // Settled: clear — presenter-owned canvas box already matches target.
        "none".to_string()
    } else {
        format!("scale({})", scale)
    };
    with_dom_cache(|dom| {
        if let Some(canvas) = dom.and_then(|d| d.main_canvas.as_ref()) {
            let _ = canvas.style().set_property("transform", &transform);
        }
    });
}

fn schedule_next_frame() {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    let closure = Closure::once_into_js(move |timestamp_ms: f64| {
        tick(timestamp_ms);
    });

    let handle = window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .unwrap_or(0);

    RAF_HANDLE.with(|h| *h.borrow_mut() = Some(handle));
    RAF_CLOSURE.with(|c| *c.borrow_mut() = Some(closure));
}

/// Threshold for considering the zoom animation as "in gesture" (visual_zoom
/// is still catching up to target_zoom). When |visual_zoom - target_zoom| is
/// above this threshold, the RAF loop skips re-renders and frame application
/// to avoid geometry fights with on_wheel_event. Single source —
/// raf_committed.rs imports this value.
pub(super) const GESTURE_THRESHOLD: f32 = 0.001;

fn tick(timestamp_ms: f64) {
    let still_active = RAF_HANDLE.with(|h| h.borrow().is_some());
    if !still_active {
        return;
    }

    let dom_cache_ready = with_dom_cache(|d| d.is_some());
    if !dom_cache_ready {
        init_dom_cache();
    }

    // ── 1. Advance animation (for render timing only — visual feedback
    //       comes from the virtual zoom layout applied in on_wheel_event) ──
    let (settled, visual, in_gesture) = ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();
        let step = advance_zoom_animation_state(&mut s, Some(timestamp_ms));
        let gap = (s.visual_zoom - s.target_zoom).abs();
        (step.settled, step.visual_zoom, gap > GESTURE_THRESHOLD)
    });

    // ── 1b. Continuous visual zoom: compositor-only CSS scale on the canvas ──
    // The canvas bitmap only refreshes when a reknock frame presents (~every
    // 60ms + render time), so without this the page scales in visible steps.
    // Scaling the canvas element by visual/last_rendered each RAF frame gives
    // 60fps continuous zoom; when a reknock presents, the presenter re-boxes
    // the canvas to the new bitmap zoom and resets the transform to exactly
    // visual/newRendered in the same frame — visually continuous (identical
    // on-screen size before and after the swap).
    apply_canvas_visual_scale(visual);

    // ── 2. Mid-animation re-render when blur exceeds threshold ──
    // Re-knocks fire during the gesture too (not just after it): without them
    // the visible canvas keeps its pre-gesture bitmap until settle, which is
    // exactly the "page suddenly jumps to N× on wheel release" defect. The
    // gesture-safe guards are:
    //   - apply_committed_frame skips container geometry writes while
    //     in_gesture (step 3), so the frame cannot fight on_wheel_event;
    //   - the TS presenter re-boxes only the canvas element (base-layer
    //     bitmap), which is what produces live zoom feedback.
    // The frame renders at visualZoom, so its renderZoom tracks the
    // interpolated state and the presenter can commit it seamlessly.
    if !settled {
        // Re-render when the blur between visual_zoom and last_rendered_zoom
        // exceeds the threshold — knocks the TS render pipeline to pick up
        // the mid-animation visual state.
        let blur = ZOOM_STATE.with(|state| {
            let s = state.borrow();
            let base = if s.last_rendered_zoom > 0.0 {
                s.last_rendered_zoom
            } else {
                1.0
            };
            (s.visual_zoom / base - 1.0).abs()
        });
        let render_in_flight = crate::render::render_store::RENDER_STATE
            .with(|state| state.borrow().in_flight_frame_token != 0);
        let elapsed_ms = LAST_PREVIEW_KNOCK.with(|t| timestamp_ms - *t.borrow());
        use pdf_viewer_core::render::zoom::decision::{
            should_reknock_preview_render, PreviewReknockRequest,
        };
        if should_reknock_preview_render(PreviewReknockRequest {
            blur,
            elapsed_ms,
            render_in_flight,
        }) {
            LAST_PREVIEW_KNOCK.with(|t| *t.borrow_mut() = timestamp_ms);
            dispatch_settle_envelope();
        }
    }

    // ── 3. Poll committed frame queue ──
    // During an active wheel gesture, skip frame application — the frame
    // contains geometry computed from visualZoom which differs from the
    // target_zoom geometry set by on_wheel_event. Applying it would jump.
    if !in_gesture {
        if let Some(frame) = pop_committed_frame() {
            apply_committed_frame(frame);
        }
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
