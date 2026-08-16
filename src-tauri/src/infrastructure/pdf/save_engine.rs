use crate::infrastructure::pdf::commands::PdfEditCommand;
use lopdf::Document;

/// PDF 修改执行中心 (Command Invoker)
/// 该服务负责管理文件的声明周期，并按顺序执行所有编辑指令。
pub fn apply_pdf_commands(
    mut doc: Document,
    page_index: u16,
    commands: Vec<Box<dyn PdfEditCommand>>,
) -> Result<Document, String> {
    crate::log_step!(
        "[PDF-SAVE] apply_pdf_commands START (in-memory) page_index={}",
        page_index
    );
    let page_num = (page_index + 1) as u32;

    crate::log_step!("[PDF-SAVE] Executing {} commands", commands.len());
    for cmd in commands {
        cmd.execute(&mut doc, page_num)?;
    }

    Ok(doc)
}
