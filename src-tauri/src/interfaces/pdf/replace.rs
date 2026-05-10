//! Text/region replace commands and patch application.

use super::helpers::{execute_pdf_commands_with_app_state, execute_region_patches};
use crate::application::pdf::page_replace::{
    PdfDocumentReplaceRequest, PdfDocumentReplaceResult, PdfRegionReplaceRequest,
    PdfRegionReplaceResult,
};
use crate::infrastructure::pdf::commands::{PdfEditCommand, ReplaceTextCommand};
use crate::log_step;
use pdf_viewer_core::persistence::models::PersistableRegionPatch;
use tauri::command;

#[command]
pub async fn apply_text_patches(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    patches: Vec<crate::infrastructure::pdf::models::TextPatch>,
) -> Result<(), String> {
    println!(
        ">>>>> [ENTRY] commit_document_edits | path={} | count={}",
        path,
        patches.len()
    );
    log_step!(
        "[PDF-SAVE-CMD] Received commit_document_edits: path={} page={} patches={}",
        path,
        page_index,
        patches.len()
    );

    let mut commands: Vec<Box<dyn PdfEditCommand>> = Vec::new();
    for patch in patches {
        commands.push(Box::new(ReplaceTextCommand { patch }));
    }

    execute_pdf_commands_with_app_state(&state, path, page_index, commands).await
}

#[command]
pub async fn apply_region_patches(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    patches: Vec<PersistableRegionPatch>,
) -> Result<(), String> {
    println!(
        ">>>>> [ENTRY] apply_region_patches | path={} | count={}",
        path,
        patches.len()
    );
    log_step!(
        "[V3-SAVE-CMD] Applying region patches: path={} page={} count={}",
        path,
        page_index,
        patches.len()
    );
    execute_region_patches(&state, path, page_index, patches).await
}

#[command]
pub async fn apply_batch_replace(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_count: usize,
    query: String,
    replacement: String,
    case_sensitive: Option<bool>,
) -> Result<PdfDocumentReplaceResult, String> {
    crate::application::pdf::page_replace::replace_document_regions(
        &state,
        &path,
        page_count,
        &PdfDocumentReplaceRequest {
            query,
            replacement,
            case_sensitive: case_sensitive.unwrap_or(false),
        },
    )
    .await
}

#[command]
pub async fn apply_replace(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    region_id: String,
    kind: String,
    original_text: String,
    query: String,
    replacement: String,
    case_sensitive: Option<bool>,
) -> Result<PdfRegionReplaceResult, String> {
    crate::application::pdf::page_replace::replace_region_match(
        &state,
        &path,
        &PdfRegionReplaceRequest {
            page_index,
            region_id,
            kind,
            original_text,
            query,
            replacement,
            case_sensitive: case_sensitive.unwrap_or(false),
        },
    )
    .await
}
