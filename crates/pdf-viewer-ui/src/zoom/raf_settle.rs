//! Settle cleanup RAF — one-shot animation frame after a committed frame
//! is applied at settle. Removes itself from thread-local for GC.

use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    /// Handle + closure for the settle cleanup RAF.
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
        // No CSS transform to clear — dimensions are set directly via SetBox.
        // Just clean up the thread-local.
        SETTLE_CLEANUP.with(|c| *c.borrow_mut() = None);
    });

    let handle = window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .unwrap_or(0);

    SETTLE_CLEANUP.with(|c| *c.borrow_mut() = Some((handle, closure)));
}
