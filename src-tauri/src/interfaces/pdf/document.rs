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
    let span = crate::infrastructure::pdf::log_service::PdfEventSpan::begin(
        1,
        "document.open",
        vec![("pathHash", format!("{:x}", md5::compute(&path)))],
    );
    crate::infrastructure::pdf::cache::invalidate_pdf_page_cache(&state, &path);
    let page_count = PdfDocumentService::open_pdf(app_handle, state, &path).await?;
    span.finish("accepted", vec![("pageCount", page_count.to_string())]);
    Ok(page_count)
}

#[command]
pub fn clear_cache(state: tauri::State<'_, crate::AppState>) -> Result<(), String> {
    let span = crate::infrastructure::pdf::log_service::PdfEventSpan::begin(
        1,
        "document.clearCache",
        Vec::new(),
    );
    PdfDocumentService::release_all_resources(&state);
    {
        let mut cache = state.docs.read_document_meta_cache.lock().unwrap();
        cache.clear();
    }
    {
        let mut cache = state.cache.page_preview_cache.lock().unwrap();
        cache.clear();
    }
    span.finish("accepted", Vec::new());
    Ok(())
}

#[command]
pub async fn save_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    modifications: PdfModifications,
) -> Result<(), String> {
    let region_patches = modifications.region_patches.len();
    let text_reflows = modifications.text_reflows.len();
    let span = crate::infrastructure::pdf::log_service::PdfEventSpan::begin(
        1,
        "document.save",
        vec![
            ("pathHash", format!("{:x}", md5::compute(&path))),
            ("regionPatches", region_patches.to_string()),
            ("textReflows", text_reflows.to_string()),
        ],
    );
    PdfDocumentService::save_pdf(state, &path, modifications).await?;
    span.finish("accepted", Vec::new());
    Ok(())
}

#[command]
pub async fn undo(state: tauri::State<'_, crate::AppState>, path: String) -> Result<(), String> {
    let span = crate::infrastructure::pdf::log_service::PdfEventSpan::begin(
        1,
        "document.undo",
        vec![("pathHash", format!("{:x}", md5::compute(&path)))],
    );
    PdfDocumentService::rollback_pdf(state, &path).await?;
    span.finish("accepted", Vec::new());
    Ok(())
}

#[command]
pub async fn redo(state: tauri::State<'_, crate::AppState>, path: String) -> Result<(), String> {
    let span = crate::infrastructure::pdf::log_service::PdfEventSpan::begin(
        1,
        "document.redo",
        vec![("pathHash", format!("{:x}", md5::compute(&path)))],
    );
    PdfDocumentService::redo_pdf(state, &path).await?;
    span.finish("accepted", Vec::new());
    Ok(())
}
