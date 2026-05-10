//! Document lifecycle commands: open / save / undo / redo / clear_cache.

use crate::infrastructure::pdf::engine::PdfDocumentService;
use crate::infrastructure::pdf::models::PdfModifications;
use tauri::command;

#[command]
pub async fn open_pdf(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<usize, String> {
    PdfDocumentService::open_pdf(app_handle, state, &path).await
}

#[command]
pub fn clear_cache(state: tauri::State<'_, crate::AppState>) -> Result<(), String> {
    PdfDocumentService::release_all_pdf_resources(&state);
    {
        let mut cache = state.docs.read_document_meta_cache.lock().unwrap();
        cache.clear();
    }
    {
        let mut cache = state.cache.page_preview_cache.lock().unwrap();
        cache.clear();
    }
    Ok(())
}

#[command]
pub async fn save_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    modifications: PdfModifications,
) -> Result<(), String> {
    PdfDocumentService::save_pdf(state, &path, modifications).await
}

#[command]
pub async fn undo(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<(), String> {
    PdfDocumentService::rollback_pdf(state, &path).await
}

#[command]
pub async fn redo(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<(), String> {
    PdfDocumentService::redo_pdf(state, &path).await
}
