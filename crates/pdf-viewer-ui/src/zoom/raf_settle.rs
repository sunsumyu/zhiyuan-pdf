//! Settle cleanup RAF — one-shot animation frame that clears the compensating
//! translate after a committed frame is applied.
//!
//! After `apply_committed_frame` applies a compensating translate to bridge
//! the visual gap between the CSS-transformed state and the new layout,
//! a one-shot RAF clears it on the next frame. The anchor layout already
//! positions content correctly without the translate.

use std::cell::RefCell;
use wasm_bindgen::prelude::*;

use super::raf_dom_cache::{with_dom_cache};
use super::raf_transform::LAST_APPLIED_SCALE;

thread_local! {
    /// Handle + closure for the settle cleanup RAF (clears compensating translate).
    static SETTLE_CLEANUP: RefCell<Option<(i32, JsValue)>> = RefCell::new(None);
}

pub(super) fn cancel_settle_cleanup() {
    SETTLE_CLEANUP.with(|c| {
        if let Some((handle, _)) = c.borrow_mut().take() {
            if let Some(w) = web_sys::window() {
                let _ = w.cancel_animation_frame(handle);
            }
        }
    });
}

pub(super) fn schedule_settle_cleanup() {
    cancel_settle_cleanup();

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };

    let closure = Closure::once_into_js(move || {
        with_dom_cache(|dom| {
            if let Some(dom) = dom {
                let _ = dom.container.style().set_property("transform", "");
            }
        });
        LAST_APPLIED_SCALE.with(|s| *s.borrow_mut() = (f32::NAN, (0.0, 0.0)));
        // Remove self from thread-local so it can be GC'd
        SETTLE_CLEANUP.with(|c| *c.borrow_mut() = None);
    });

    let handle = window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .unwrap_or(0);

    SETTLE_CLEANUP.with(|c| *c.borrow_mut() = Some((handle, closure)));
}
