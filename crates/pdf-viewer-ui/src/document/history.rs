use crate::ui_state_store::{redo, undo};

pub fn undo_document_edit() -> bool {
    undo()
}

pub fn redo_document_edit() -> bool {
    redo()
}
