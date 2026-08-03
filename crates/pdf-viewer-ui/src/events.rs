//! Unified EventBus — DOM-style multi-listener event system (Nutrient borrowing #1).
//!
//! Replaces the single-slot `on_state_change` / `on_change` callback pattern
//! with a Nutrient-style `addEventListener` / `removeEventListener` API.
//!
//! # Event name convention
//!
//! Events follow a `domain.verb` namespace (matches Nutrient's `EventName` enum):
//!
//! | Event name | Emitted when | Payload |
//! |---|---|---|
//! | `editor.stateChange` | EditorSession state transition | state string |
//! | `editor.change` | Any editor mutation (state / block) | — |
//! | `viewer.pageChange` | Current page index changed | page index |
//! | `viewer.zoomChange` | Zoom factor changed | zoom value |
//! | `document.open` | Document opened | path |
//! | `document.close` | Document closed | — |
//! | `document.saveStateChange` | hasUnsavedChanges toggled | bool |
//! | `find.stateChange` | FindSessionState changed | state string |
//! | `review.stateChange` | ReviewSessionState changed | state string |
//!
//! # Thread safety
//!
//! The wasm event bus is stored in `app_context::AppStores`, which is backed by
//! a WASM thread-local root. On native (test/Tauri), events are silently dropped.

// ── Event name constants ────────────────────────────────────────

pub mod event_names {
    pub const EDITOR_STATE_CHANGE: &str = "editor.stateChange";
    pub const EDITOR_CHANGE: &str = "editor.change";
    pub const VIEWER_PAGE_CHANGE: &str = "viewer.pageChange";
    pub const VIEWER_ZOOM_CHANGE: &str = "viewer.zoomChange";
    pub const PAGE_TURN_INTENT: &str = "pageTurn.intent";
    pub const PAGE_TURN_REJECT: &str = "pageTurn.reject";
    pub const PAGE_TURN_VISIBLE: &str = "pageTurn.visible";
    pub const DOCUMENT_OPEN: &str = "document.open";
    pub const DOCUMENT_CLOSE: &str = "document.close";
    pub const DOCUMENT_SAVE_STATE_CHANGE: &str = "document.saveStateChange";
    pub const FIND_STATE_CHANGE: &str = "find.stateChange";
    pub const REVIEW_STATE_CHANGE: &str = "review.stateChange";
}

// ── EventBus ────────────────────────────────────────────────────

/// Register a listener for the given event name.
///
/// Multiple listeners per event are supported (DOM-style).
/// Duplicate function references are allowed (each call adds one more).
#[cfg(target_arch = "wasm32")]
pub fn add_listener(event: &str, cb: js_sys::Function) {
    crate::app_context::with_event_bus_mut(|bus| {
        bus.listeners.entry(event.to_string()).or_default().push(cb);
    });
}

/// Remove the first listener matching the given function reference.
///
/// Uses JS strict equality (`===`) via `js_sys::Function` identity.
/// Returns `true` if a listener was removed.
#[cfg(target_arch = "wasm32")]
pub fn remove_listener(event: &str, cb: &js_sys::Function) -> bool {
    crate::app_context::with_event_bus_mut(|bus| {
        if let Some(listeners) = bus.listeners.get_mut(event) {
            if let Some(pos) = listeners.iter().position(|f| f == cb) {
                listeners.remove(pos);
                return true;
            }
        }
        false
    })
}

/// Emit an event with an optional payload to all registered listeners.
///
/// Listeners are called synchronously in registration order.
/// Exceptions thrown by listeners are silently caught (one bad listener
/// must not prevent others from firing).
#[cfg(target_arch = "wasm32")]
pub fn emit(event: &str, payload: &wasm_bindgen::JsValue) {
    // Clone the listener list to avoid borrow issues if a listener
    // calls add/removeEventListener during dispatch.
    let listeners: Vec<js_sys::Function> = crate::app_context::with_event_bus(|bus| {
        bus.listeners.get(event).cloned().unwrap_or_default()
    });

    for listener in &listeners {
        let _ = listener.call1(&wasm_bindgen::JsValue::NULL, payload);
    }
}

/// Remove all listeners (used during Application.resetAll).
#[cfg(target_arch = "wasm32")]
pub fn clear_all() {
    crate::app_context::with_event_bus_mut(|bus| {
        bus.listeners.clear();
    });
}

/// Return the number of registered listeners for diagnostics.
#[cfg(target_arch = "wasm32")]
pub fn listener_count(event: &str) -> usize {
    crate::app_context::with_event_bus(|bus| bus.listeners.get(event).map(|v| v.len()).unwrap_or(0))
}

// ── Native stubs (no-op) ────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub fn add_listener(_event: &str, _cb: js_sys::Function) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn remove_listener(_event: &str, _cb: &js_sys::Function) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn emit(_event: &str, _payload: &wasm_bindgen::JsValue) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_all() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn listener_count(_event: &str) -> usize {
    0
}
