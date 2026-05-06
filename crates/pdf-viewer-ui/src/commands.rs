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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeletePageCommand {
    pub page_num: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RotatePageCommand {
    pub page_num: u32,
    pub delta: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InsertPageCommand {
    pub at_index: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddHighlightCommand {
    pub page_num: u32,
    pub rect: [f32; 4],
    pub color: [f32; 3],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateMetadataCommand {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
}

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
