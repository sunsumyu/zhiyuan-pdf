//! Layout / hit-test / projection geometry commands.

use crate::infrastructure::pdf::engine::PdfEditorGeometryService;
use crate::infrastructure::pdf::models::LayoutInferenceResult;
use tauri::command;

#[command]
pub async fn resolve_layout(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<LayoutInferenceResult, String> {
    PdfEditorGeometryService::get_layout_inference(state, path, page_index).await
}

#[command]
pub fn resolve_caret(
    session: pdf_viewer_core::models::ParagraphEditContext,
    click_x_from_anchor_left: f32,
) -> Result<usize, String> {
    PdfEditorGeometryService::resolve_editor_caret_index(session, click_x_from_anchor_left)
}

#[command]
pub fn resolve_hit(
    request: pdf_viewer_core::models::FieldHitRequest,
) -> Result<pdf_viewer_core::models::FieldHitResolution, String> {
    PdfEditorGeometryService::resolve_field_hit(request)
}

#[command]
pub fn resolve_hit_target(
    request: pdf_viewer_core::models::FieldHitBatchRequest,
) -> Result<Option<pdf_viewer_core::models::FieldHitMatch>, String> {
    PdfEditorGeometryService::resolve_field_hit_target(request)
}

#[command]
pub fn resolve_projection(
    request: pdf_viewer_core::models::FieldProjectionRequest,
) -> Result<pdf_viewer_core::models::FieldProjection, String> {
    PdfEditorGeometryService::resolve_field_projection(request)
}

#[command]
pub fn resolve_params(
    request: pdf_viewer_core::models::FieldEditorParamsRequest,
) -> Result<pdf_viewer_core::models::FieldEditorParams, String> {
    PdfEditorGeometryService::resolve_field_editor_params(request)
}
