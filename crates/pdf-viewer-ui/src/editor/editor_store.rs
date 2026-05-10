use std::cell::Cell;

use crate::editor::editor_types::SessionState;

// ── Thread-local state ──────────────────────────────────────────

thread_local! {
    static SESSION_STATE: Cell<SessionState> = Cell::new(SessionState::Viewing);
    static ACTIVE_BLOCK_ID: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

// ── Public accessors ────────────────────────────────────────────

pub fn get_state() -> SessionState {
    SESSION_STATE.with(|s| s.get())
}

pub fn set_state(state: SessionState) {
    SESSION_STATE.with(|s| s.set(state));
}

pub fn get_active_block_id() -> Option<String> {
    ACTIVE_BLOCK_ID.with(|id| id.borrow().clone())
}

pub fn set_active_block_id(block_id: Option<String>) {
    ACTIVE_BLOCK_ID.with(|id| {
        *id.borrow_mut() = block_id;
    });
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
