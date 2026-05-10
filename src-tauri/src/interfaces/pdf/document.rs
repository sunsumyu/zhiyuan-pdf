//! Document lifecycle commands: open / read / probe / save / undo / redo / clear_cache.

use crate::infrastructure::pdf::engine::{PdfDocumentService, PdfPageModelService};
use crate::infrastructure::pdf::models::{PdfMaterializationReport, PdfMetadata, PdfModifications};
use crate::infrastructure::pdf_read::facade::PdfReadFacade;
use crate::infrastructure::pdf_read::types::ReadDocumentMeta;
use crate::{log_step, pdf_log};
use tauri::command;

#[command]
pub async fn read_metadata(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<PdfMetadata, String> {
    PdfPageModelService::get_pdf_metadata(state, &path).await
}

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
pub async fn read_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<ReadDocumentMeta, String> {
    let total_start = std::time::Instant::now();
    if let Some(meta) = state
        .docs.read_document_meta_cache
        .lock()
        .unwrap()
        .get(&path)
        .cloned()
    {
        log_step!(
            "[PDF-READ][cmd][open] cache_hit=true total={:?} pages={} path={}",
            total_start.elapsed(),
            meta.page_count,
            path
        );
        return Ok(meta);
    }

    let path_for_task = path.clone();
    pdf_log!(2, "[PDF-READ][cmd][open][detail] spawn path={}", path);
    let meta = tokio::task::spawn_blocking(move || {
        let facade = PdfReadFacade::new();
        facade.open(&path_for_task)
    })
    .await
    .map_err(|e: tokio::task::JoinError| e.to_string())??;

    state
        .docs.read_document_meta_cache
        .lock()
        .unwrap()
        .insert(path.clone(), meta.clone());
    log_step!(
        "[PDF-READ][cmd][open] cache_hit=false total={:?} pages={} kind={:?} path={}",
        total_start.elapsed(),
        meta.page_count,
        meta.kind,
        path
    );
    Ok(meta)
}

#[command]
pub async fn probe_pdf(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<ReadDocumentMeta, String> {
    let total_start = std::time::Instant::now();
    if let Some(meta) = state
        .docs.read_document_meta_cache
        .lock()
        .unwrap()
        .get(&path)
        .cloned()
    {
        log_step!(
            "[PDF-READ][cmd][probe] cache_hit=true total={:?} pages={} kind={:?} path={}",
            total_start.elapsed(),
            meta.page_count,
            meta.kind,
            path
        );
        return Ok(meta);
    }

    let path_for_task = path.clone();
    let meta = tokio::task::spawn_blocking(move || {
        let facade = PdfReadFacade::new();
        facade.probe_kind_fast(&path_for_task)
    })
    .await
    .map_err(|e: tokio::task::JoinError| e.to_string())??;

    state
        .docs.read_document_meta_cache
        .lock()
        .unwrap()
        .insert(path.clone(), meta.clone());

    log_step!(
        "[PDF-READ][cmd][probe] cache_hit=false total={:?} pages={} kind={:?} path={}",
        total_start.elapsed(),
        meta.page_count,
        meta.kind,
        path
    );
    Ok(meta)
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
pub fn read_materialization_report(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<Option<PdfMaterializationReport>, String> {
    PdfDocumentService::read_last_pdf_materialization_report(state, &path)
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
