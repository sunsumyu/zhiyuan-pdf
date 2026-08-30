//! Settle envelope dispatch (ADR-0001).
//!
//! When the zoom animation settles, this module knocks the TS render loop
//! via a fixed global knock function. The TS side reads the current zoom
//! state from WASM and schedules the final render itself.
//! No registrable JS callback is involved.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Fixed global knock function — Rust calls it once after parking a settle
/// envelope. Not a registrable callback: TS assigns the property, Rust only
/// ever invokes whatever single function sits there.
const DRAIN_KNOCK_GLOBAL: &str = "__pdfDrainPendingRenderFrame";

/// Dispatch the settle envelope by knocking the TS render loop.
pub fn dispatch_settle_envelope() {
    knock_render_loop();
}

fn knock_render_loop() {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let knock = js_sys::Reflect::get(
        &window.into(),
        &JsValue::from_str(DRAIN_KNOCK_GLOBAL),
    )
    .ok();
    if let Some(knock) = knock {
        if knock.is_function() {
            let _ = knock.unchecked_into::<js_sys::Function>().call0(&JsValue::NULL);
        }
    }
}
