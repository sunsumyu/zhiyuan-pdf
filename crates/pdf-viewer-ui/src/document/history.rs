use crate::state_manager::{redo, undo};

pub fn undo_document_edit() -> bool {
    undo()
}

pub fn redo_document_edit() -> bool {
    redo()
}
