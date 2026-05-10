use std::sync::RwLock;
use crate::persistence::state_manager::{GLOBAL_PATCH_STATE};
use crate::persistence::models::PersistableRegionPatch;

#[derive(Debug, Clone)]
pub struct PatchCommand {
    pub patch_key: String,
    pub old_patch: Option<PersistableRegionPatch>,
    pub new_patch: PersistableRegionPatch,
}

impl PatchCommand {
    pub fn execute(&self) {
        crate::persistence::state_manager::apply_patch(self.new_patch.clone());
    }

    pub fn undo(&self) {
        if let Some(old) = &self.old_patch {
            crate::persistence::state_manager::apply_patch(old.clone());
        } else if let Ok(mut state) = GLOBAL_PATCH_STATE.write() {
            // If there was no old patch, we revert to original text.
            // For simplicity, we remove the patch key entry.
            if self.new_patch.source == "field-row" {
                state.field_group_texts.remove(&self.patch_key);
                state.field_group_snapshots.remove(&self.patch_key);
            } else {
                state.paragraph_texts.remove(&self.patch_key);
                state.paragraph_snapshots.remove(&self.patch_key);
            }
        }
    }
}

pub struct HistoryManager {
    undo_stack: Vec<PatchCommand>,
    redo_stack: Vec<PatchCommand>,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, cmd: PatchCommand, execute_immediately: bool) {
        if execute_immediately {
            cmd.execute();
        }
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> bool {
        if let Some(cmd) = self.undo_stack.pop() {
            cmd.undo();
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(cmd) = self.redo_stack.pop() {
            cmd.execute();
            self.undo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_HISTORY: RwLock<HistoryManager> = RwLock::new(HistoryManager::new());
}

pub fn push_command(cmd: PatchCommand) {
    if let Ok(mut history) = GLOBAL_HISTORY.write() {
        history.push(cmd, true);
    }
}

pub fn undo() -> bool {
    if let Ok(mut history) = GLOBAL_HISTORY.write() {
        history.undo()
    } else {
        false
    }
}

pub fn redo() -> bool {
    if let Ok(mut history) = GLOBAL_HISTORY.write() {
        history.redo()
    } else {
        false
    }
}

pub fn clear_history() {
    if let Ok(mut history) = GLOBAL_HISTORY.write() {
        history.clear();
    }
}
