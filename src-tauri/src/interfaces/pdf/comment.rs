//! Comment-related commands: list, review, apply, update.

use crate::application::pdf::comment_review::{PdfCommentReviewRequest, PdfCommentReviewResult};
use crate::application::pdf::page_annotation::{
    PdfPageCommentList, PdfRegionCommentRequest, PdfRegionCommentResult, PdfUpdateCommentRequest,
    PdfUpdateCommentResult,
};
use tauri::command;

#[command]
pub async fn read_comments(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<PdfPageCommentList, String> {
    crate::application::pdf::page_annotation::list_page_comments(&state, &path, page_index).await
}

#[command]
pub async fn read_comment_review(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfCommentReviewRequest,
) -> Result<PdfCommentReviewResult, String> {
    crate::application::pdf::comment_review::review_document_comments(&state, &path, &request).await
}

#[command]
pub async fn apply_comment(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfRegionCommentRequest,
) -> Result<PdfRegionCommentResult, String> {
    crate::application::pdf::page_annotation::add_region_comment(&state, &path, &request).await
}

#[command]
pub async fn apply_comment_update(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    request: PdfUpdateCommentRequest,
) -> Result<PdfUpdateCommentResult, String> {
    crate::application::pdf::page_annotation::update_page_comment(&state, &path, &request).await
}
