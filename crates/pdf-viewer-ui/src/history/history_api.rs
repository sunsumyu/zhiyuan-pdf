//! HistoryController — struct-based WASM API for unified document undo/redo.
//!
//! Mirrors the P0–P3 pattern: zero-sized struct + camelCase methods + thin
//! delegation. Backed by `state_manager`'s patch-based undo/redo stacks.
//!
//! **Coexistence with `DocumentSession`**:
//! - `DocumentSession.undo()` / `DocumentSession.redo()` remain for back-compat.
//! - Both `HistoryController` and `DocumentSession` delegate to the same
//!   `state_manager::undo` / `state_manager::redo` — there is exactly one
//!   shared history stack. Mixing the two APIs is safe.
//!
//! **Difference from `EditorSession.commit/discard`**:
//! - `EditorSession` manages an in-progress text-block edit (transient).
//! - `HistoryController` manages persisted patches (post-commit).

use wasm_bindgen::prelude::*;

use crate::history::history_types::{ok_response, HistoryState, HistoryStepResult};
use crate::state_manager::{
    can_redo, can_undo, clear_history_stacks, current_patch_revision, redo, redo_depth, undo,
    undo_depth,
};

fn current_state() -> HistoryState {
    HistoryState {
        can_undo: can_undo(),
        can_redo: can_redo(),
        undo_depth: undo_depth() as u32,
        redo_depth: redo_depth() as u32,
        revision: current_patch_revision(),
    }
}

// ── HistoryController ───────────────────────────────────────────

#[wasm_bindgen]
pub struct HistoryController;

#[wasm_bindgen]
impl HistoryController {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        HistoryController
    }

    /// Read the current history state snapshot (depths + revision).
    #[wasm_bindgen(js_name = "getState")]
    pub fn get_state(&self) -> JsValue {
        ok_response(current_state())
    }

    /// Whether `undo()` would have an effect.
    #[wasm_bindgen(js_name = "canUndo")]
    pub fn can_undo(&self) -> bool {
        can_undo()
    }

    /// Whether `redo()` would have an effect.
    #[wasm_bindgen(js_name = "canRedo")]
    pub fn can_redo(&self) -> bool {
        can_redo()
    }

    /// Step backward one patch on the undo stack.
    ///
    /// Returns a structured `HistoryResponse<HistoryStepResult>` with
    /// `changed` indicating whether anything actually moved (false = no-op,
    /// not an error — matches Nutrient's `instance.history.undo()` shape).
    #[wasm_bindgen(js_name = "undo")]
    pub fn undo(&self) -> JsValue {
        let changed = undo();
        ok_response(HistoryStepResult {
            changed,
            state: current_state(),
        })
    }

    /// Step forward one patch on the redo stack.
    #[wasm_bindgen(js_name = "redo")]
    pub fn redo(&self) -> JsValue {
        let changed = redo();
        ok_response(HistoryStepResult {
            changed,
            state: current_state(),
        })
    }

    /// Clear both undo and redo stacks without touching applied patches.
    ///
    /// This is *history-only* — does not revert applied edits. Use
    /// `DocumentSession.close()` if you need to reset patches as well.
    #[wasm_bindgen(js_name = "clear")]
    pub fn clear(&self) -> JsValue {
        clear_history_stacks();
        ok_response(current_state())
    }

    /// Number of entries on the undo stack.
    #[wasm_bindgen(js_name = "undoDepth")]
    pub fn undo_depth(&self) -> u32 {
        undo_depth() as u32
    }

    /// Number of entries on the redo stack.
    #[wasm_bindgen(js_name = "redoDepth")]
    pub fn redo_depth(&self) -> u32 {
        redo_depth() as u32
    }
}

impl Default for HistoryController {
    fn default() -> Self {
        Self::new()
    }
}
