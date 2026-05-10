use std::cell::{Cell, RefCell};

use crate::editor::editor_types::SessionState;

// ── Thread-local state ──────────────────────────────────────────

thread_local! {
    static SESSION_STATE: Cell<SessionState> = Cell::new(SessionState::Viewing);
    static ACTIVE_BLOCK_ID: RefCell<Option<String>> = RefCell::new(None);

    // ── §14.7 event callbacks ───────────────────────────────────
    // `STATE_CHANGE_CB` fires only on `SessionState` transitions.
    // `CHANGE_CB` fires on any session-relevant mutation (state OR active block).
    // Both are optional and replace any previously registered callback.
    #[cfg(target_arch = "wasm32")]
    static STATE_CHANGE_CB: RefCell<Option<js_sys::Function>> = RefCell::new(None);
    #[cfg(target_arch = "wasm32")]
    static CHANGE_CB: RefCell<Option<js_sys::Function>> = RefCell::new(None);
}

// ── Public accessors ────────────────────────────────────────────

pub fn get_state() -> SessionState {
    SESSION_STATE.with(|s| s.get())
}

pub fn set_state(state: SessionState) {
    let prev = SESSION_STATE.with(|s| {
        let p = s.get();
        s.set(state);
        p
    });
    if prev != state {
        notify_state_change(state);
    }
    notify_change();
}

pub fn get_active_block_id() -> Option<String> {
    ACTIVE_BLOCK_ID.with(|id| id.borrow().clone())
}

pub fn set_active_block_id(block_id: Option<String>) {
    let changed = ACTIVE_BLOCK_ID.with(|id| {
        let mut slot = id.borrow_mut();
        let differs = *slot != block_id;
        *slot = block_id;
        differs
    });
    if changed {
        notify_change();
    }
}

// ── §14.7 callback registration / dispatch ──────────────────────

/// Install a callback fired on every `SessionState` transition.
/// Argument received by JS: the new state as a camelCase string
/// (`"viewing"` / `"editing"` / `"editingBlock"` / `"saving"`).
#[cfg(target_arch = "wasm32")]
pub fn set_state_change_callback(cb: Option<js_sys::Function>) {
    STATE_CHANGE_CB.with(|slot| *slot.borrow_mut() = cb);
}

/// Install a callback fired on any session mutation (state OR active block).
/// Arity-0; JS reads fresh state via `EditorSession.getSnapshot()` if needed.
#[cfg(target_arch = "wasm32")]
pub fn set_change_callback(cb: Option<js_sys::Function>) {
    CHANGE_CB.with(|slot| *slot.borrow_mut() = cb);
}

#[cfg(target_arch = "wasm32")]
fn notify_state_change(state: SessionState) {
    let cb = STATE_CHANGE_CB.with(|slot| slot.borrow().clone());
    if let Some(cb) = cb {
        let arg = wasm_bindgen::JsValue::from_str(state_camel_case(state));
        let _ = cb.call1(&wasm_bindgen::JsValue::NULL, &arg);
    }
}

#[cfg(target_arch = "wasm32")]
fn notify_change() {
    let cb = CHANGE_CB.with(|slot| slot.borrow().clone());
    if let Some(cb) = cb {
        let _ = cb.call0(&wasm_bindgen::JsValue::NULL);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn notify_state_change(_state: SessionState) {}

#[cfg(not(target_arch = "wasm32"))]
fn notify_change() {}

#[cfg(target_arch = "wasm32")]
fn state_camel_case(state: SessionState) -> &'static str {
    // Matches `serde(rename_all = "camelCase")` on `SessionState` so JS
    // observers see the same string that `getSnapshot().state` produces.
    match state {
        SessionState::Viewing => "viewing",
        SessionState::Editing => "editing",
        SessionState::EditingBlock => "editingBlock",
        SessionState::Saving => "saving",
    }
}

// ── Transition helpers ──────────────────────────────────────────

/// Viewing → Editing
pub fn transition_to_editing() {
    set_state(SessionState::Editing);
}

/// Editing → EditingBlock
pub fn transition_to_editing_block(block_id: String) {
    set_active_block_id(Some(block_id));
    set_state(SessionState::EditingBlock);
}

/// EditingBlock → Viewing (close / commit / discard)
pub fn transition_to_viewing() {
    set_active_block_id(None);
    set_state(SessionState::Viewing);
}

/// EditingBlock(A) → EditingBlock(B) (block switch)
pub fn transition_switch_block(new_block_id: String) {
    set_active_block_id(Some(new_block_id));
    // state stays EditingBlock
}

/// EditingBlock → Saving
pub fn transition_to_saving() {
    set_state(SessionState::Saving);
}

/// Saving → Viewing (save complete)
pub fn transition_save_complete() {
    set_active_block_id(None);
    set_state(SessionState::Viewing);
}

// ── guard_state! macro ──────────────────────────────────────────

/// State guard macro. Returns an error JsValue if the current state
/// does not match the expected pattern.
///
/// Usage:
/// ```ignore
/// guard_state!(SessionState::EditingBlock, "commit");
/// ```
#[macro_export]
macro_rules! guard_state {
    ($expected:pat, $fn_name:expr) => {
        let current = $crate::editor::editor_store::get_state();
        if !matches!(current, $expected) {
            log::warn!(
                "[EditorSession::{}] invalid state: expected {}, got {}",
                $fn_name,
                stringify!($expected),
                current.as_str(),
            );
            return $crate::editor::editor_types::err_response(
                $crate::editor::editor_types::EditorError::InvalidState {
                    expected: stringify!($expected).to_string(),
                    actual: current.as_str().to_string(),
                },
            );
        }
    };
}
