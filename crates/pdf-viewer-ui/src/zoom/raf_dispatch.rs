//! Settle envelope dispatch (ADR-0001) + animation-frame knock (ADR-0018).
//!
//! When the zoom animation settles, this module knocks the TS render loop
//! via a fixed global knock function. The TS side reads the current zoom
//! state from WASM and schedules the final render itself.
//! No registrable JS callback is involved.
//!
//! ADR-0018 adds a second, per-animation-frame knock: the canvas transform
//! must be re-synced in the same JS turn that advances `visual_zoom`, or it
//! lags the animation by one rAF scheduling offset (a full wheel band at
//! gesture reversal → 4.6–5.3% canvas width drift).

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Fixed global knock function — Rust calls it once after parking a settle
/// envelope. Not a registrable callback: TS assigns the property, Rust only
/// ever invokes whatever single function sits there.
const DRAIN_KNOCK_GLOBAL: &str = "__pdfDrainPendingRenderFrame";

/// Fixed global knock function — Rust calls it once per animation frame,
/// immediately after advancing the zoom animation state (ADR-0018). TS
/// assigns the property; Rust only ever invokes whatever sits there.
const ANIMATION_FRAME_KNOCK_GLOBAL: &str = "__pdfZoomAnimationFrame";

/// Dispatch the settle envelope by knocking the TS render loop.
pub fn dispatch_settle_envelope() {
    knock_global(DRAIN_KNOCK_GLOBAL);
}

/// Knock the TS per-animation-frame transform sync (ADR-0018).
pub fn dispatch_animation_frame() {
    knock_global(ANIMATION_FRAME_KNOCK_GLOBAL);
}

/// Invoke a fixed global knock by name. `js_sys::global()` (not
/// `web_sys::window()`) so the knock is exercisable from Node wasm tests —
/// in the browser main frame the two are the same object.
fn knock_global(name: &str) {
    let global = js_sys::global();
    let knock = js_sys::Reflect::get(&global, &JsValue::from_str(name)).ok();
    if let Some(knock) = knock {
        if knock.is_function() {
            let _ = knock
                .unchecked_into::<js_sys::Function>()
                .call0(&JsValue::NULL);
        }
    }
}
