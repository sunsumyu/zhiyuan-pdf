use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum PdfEditCommand {
    DeletePage(DeletePageCommand),
    RotatePage(RotatePageCommand),
    InsertPage(InsertPageCommand),
    AddHighlight(AddHighlightCommand),
    UpdateMetadata(UpdateMetadataCommand),
    Undo,
    Redo,
}

pub use pdf_viewer_core::models::{
    DeletePageCommand, RotatePageCommand, InsertPageCommand, AddHighlightCommand, UpdateMetadataCommand,
};

impl From<DeletePageCommand> for PdfEditCommand {
    fn from(c: DeletePageCommand) -> Self {
        PdfEditCommand::DeletePage(c)
    }
}
impl From<RotatePageCommand> for PdfEditCommand {
    fn from(c: RotatePageCommand) -> Self {
        PdfEditCommand::RotatePage(c)
    }
}
impl From<InsertPageCommand> for PdfEditCommand {
    fn from(c: InsertPageCommand) -> Self {
        PdfEditCommand::InsertPage(c)
    }
}
impl From<AddHighlightCommand> for PdfEditCommand {
    fn from(c: AddHighlightCommand) -> Self {
        PdfEditCommand::AddHighlight(c)
    }
}
impl From<UpdateMetadataCommand> for PdfEditCommand {
    fn from(c: UpdateMetadataCommand) -> Self {
        PdfEditCommand::UpdateMetadata(c)
    }
}
