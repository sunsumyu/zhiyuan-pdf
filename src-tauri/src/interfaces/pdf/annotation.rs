//! Annotation commands: targets / highlights / generic delete.

use crate::application::pdf::page_annotation::{
    PdfDeleteAnnotationRequest, PdfDeleteAnnotationResult, PdfPageAnnotationTargetResult,
    PdfPageHighlightList, PdfRegionHighlightRequest, PdfRegionHighlightResult,
};
use tauri::command;

#[command]
pub async fn read_annotation_targets(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<PdfPageAnnotationTargetResult, String> {
    crate::application::pdf::page_annotation::list_page_annotation_targets(&state, &path, page_index).await
}

#[command]
pub async fn read_highlights(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<PdfPageHighlightList, String> {
    crate::application::pdf::page_annotation::list_page_highlights(&state, &path, page_index).await
}

#[command]
pub async fn apply_highlight(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfRegionHighlightRequest,
) -> Result<PdfRegionHighlightResult, String> {
    crate::application::pdf::page_annotation::add_region_highlight(&state, &path, &request).await
}

#[command]
pub async fn delete_annotation(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfDeleteAnnotationRequest,
) -> Result<PdfDeleteAnnotationResult, String> {
    crate::application::pdf::page_annotation::delete_page_annotation(&state, &path, &request).await
}
