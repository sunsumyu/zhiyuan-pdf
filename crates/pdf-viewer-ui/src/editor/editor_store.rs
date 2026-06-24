use std::cell::RefCell;

use crate::app_context;
use crate::editor::editor_types::SessionState;

// ── Thread-local state ──────────────────────────────────────────

thread_local! {
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

pub fn read_state() -> SessionState {
    app_context::with_editor_session(|session| session.session_state)
}

pub fn set_state(state: SessionState) {
    let changed = app_context::with_editor_session_mut(|session| {
        let changed = session.session_state != state;
        session.session_state = state;
        changed
    });
    if changed {
        notify_state_change(state);
    }
    notify_change();
}

pub fn read_block_id() -> Option<String> {
    app_context::with_editor_session(|session| session.active_block_id.clone())
}

pub fn set_block_id(block_id: Option<String>) {
    let changed = app_context::with_editor_session_mut(|session| {
        let differs = session.active_block_id != block_id;
        session.active_block_id = block_id;
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
/// Arity-0; JS reads fresh state via `EditorSession.readSnapshot()` if needed.
#[cfg(target_arch = "wasm32")]
pub fn set_change_callback(cb: Option<js_sys::Function>) {
    CHANGE_CB.with(|slot| *slot.borrow_mut() = cb);
}

#[cfg(target_arch = "wasm32")]
fn notify_state_change(state: SessionState) {
    let arg = wasm_bindgen::JsValue::from_str(state_camel_case(state));
    // Legacy single-slot callback (backward compat)
    let cb = STATE_CHANGE_CB.with(|slot| slot.borrow().clone());
    if let Some(cb) = cb {
        let _ = cb.call1(&wasm_bindgen::JsValue::NULL, &arg);
    }
    // Unified EventBus (Nutrient borrowing #1)
    crate::events::emit(crate::events::event_names::EDITOR_STATE_CHANGE, &arg);
}

#[cfg(target_arch = "wasm32")]
fn notify_change() {
    // Legacy single-slot callback (backward compat)
    let cb = CHANGE_CB.with(|slot| slot.borrow().clone());
    if let Some(cb) = cb {
        let _ = cb.call0(&wasm_bindgen::JsValue::NULL);
    }
    // Unified EventBus (Nutrient borrowing #1)
    crate::events::emit(
        crate::events::event_names::EDITOR_CHANGE,
        &wasm_bindgen::JsValue::UNDEFINED,
    );
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
pub fn transition_editing(block_id: String) {
    set_block_id(Some(block_id));
    set_state(SessionState::EditingBlock);
}

/// EditingBlock → Viewing (close / commit / discard)
pub fn transition_to_viewing() {
    reset_session();
}

/// Reset the public editor session state to its idle shape.
pub fn reset_session() {
    let (state_changed, block_changed) = app_context::with_editor_session_mut(|session| {
        let state_changed = session.session_state != SessionState::Viewing;
        let block_changed = session.active_block_id.is_some();
        session.session_state = SessionState::Viewing;
        session.active_block_id = None;
        (state_changed, block_changed)
    });
    if state_changed {
        notify_state_change(SessionState::Viewing);
    }
    if state_changed || block_changed {
        notify_change();
    }
}

/// EditingBlock(A) → EditingBlock(B) (block switch)
pub fn transition_switch_block(new_block_id: String) {
    set_block_id(Some(new_block_id));
    // state stays EditingBlock
}

/// EditingBlock → Saving
pub fn transition_to_saving() {
    set_state(SessionState::Saving);
}

/// Saving → Viewing (save complete)
pub fn transition_save_complete() {
    set_block_id(None);
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
        let current = $crate::editor::editor_store::read_state();
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
