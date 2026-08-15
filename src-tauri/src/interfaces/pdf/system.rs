//! System-level utilities: demo PDF, log level, asset URL, file picker.

use crate::infrastructure::pdf::document_service::PdfDocumentService;
use tauri::command;

#[command]
pub fn create_demo_pdf(path: String) -> Result<String, String> {
    PdfDocumentService::generate_demo_pdf(&path)
}

#[command]
pub fn set_log_level(level: u8) {
    crate::infrastructure::pdf::log_service::set_pdf_log_level(level);
}

#[command]
pub fn clear_pdf_event_log() {
    crate::infrastructure::pdf::log_service::clear_pdf_event_log();
}

#[command]
pub fn read_pdf_event_log() -> Vec<String> {
    crate::infrastructure::pdf::log_service::read_pdf_event_log()
}

#[command]
pub fn set_page_asset_test_delay_ms(delay_ms: u64) {
    crate::application::pdf::page_asset::PageAssetAdmissionService::set_test_delay_ms(delay_ms);
}

#[command]
pub fn terminal_log(message: String) {
    crate::infrastructure::pdf::log_service::log_terminal_message(&message);
}

#[command]
pub fn resolve_asset_url(path: String) -> String {
    let binding = path.replace("\\", "/");
    let encoded = urlencoding::encode(&binding);
    format!("http://asset.localhost/{}", encoded.replace("%2F", "/"))
}

#[command]
pub async fn pick_file(app_handle: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let file_path = app_handle
        .dialog()
        .file()
        .add_filter("PDF Documents", &["pdf"])
        .blocking_pick_file();
    Ok(file_path.map(|p| p.to_string()))
}
