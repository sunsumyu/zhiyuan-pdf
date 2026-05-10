//! System-level utilities: demo PDF, log level, asset URL, file picker.

use crate::infrastructure::pdf::engine::PdfDocumentService;
use tauri::command;

#[command]
pub fn create_demo_pdf(path: String) -> Result<String, String> {
    PdfDocumentService::generate_demo_pdf(&path)
}

#[command]
pub fn set_log_level(level: u8) {
    crate::infrastructure::pdf::log_utils::set_pdf_log_level(level);
}

#[command]
pub fn get_asset_url(path: String) -> String {
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
