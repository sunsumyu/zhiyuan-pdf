use crate::infrastructure::pdf::commands::PdfEditCommand;
use lopdf::Document;

/// PDF 淇敼鎵ц涓績 (Command Invoker)
/// 璇ユ湇鍔¤礋璐ｇ鐞嗘枃浠剁殑澹版槑鍛ㄦ湡锛屽苟鎸夐『搴忔墽琛屾墍鏈夌紪杈戞寚浠ゃ€?
pub fn apply_pdf_commands(
    mut doc: Document,
    page_index: u16,
    commands: Vec<Box<dyn PdfEditCommand>>,
) -> Result<Document, String> {
use crate::log_step;
    log_step!(
        "[PDF-SAVE] apply_pdf_commands START (in-memory) page_index={}",
        page_index
    );
    let page_num = (page_index + 1) as u32;

    log_step!("[PDF-SAVE] Executing {} commands", commands.len());
    for cmd in commands {
        cmd.execute(&mut doc, page_num)?;
    }

    Ok(doc)
}
