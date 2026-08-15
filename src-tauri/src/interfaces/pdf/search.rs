//! Full-text search commands.

use crate::application::pdf::page_search::{
    PdfDocumentSearchResult, PdfPageSearchRequest, PdfPageSearchResult,
};
use crate::infrastructure::pdf::page_intermediate_service::PdfPageIntermediateService;
use tauri::command;

#[command]
pub async fn find_in_page(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    query: String,
    case_sensitive: Option<bool>,
) -> Result<PdfPageSearchResult, String> {
    let page_model = PdfPageIntermediateService::resolve_vector_page_model(
        state.clone(),
        path,
        page_index,
        1.0,
        None,
    )
    .await?;
    Ok(crate::application::pdf::page_search::search_page_regions(
        &page_model,
        &PdfPageSearchRequest {
            query,
            case_sensitive: case_sensitive.unwrap_or(false),
        },
    ))
}

#[command]
pub async fn find_in_document(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_count: usize,
    query: String,
    case_sensitive: Option<bool>,
) -> Result<PdfDocumentSearchResult, String> {
    let request = PdfPageSearchRequest {
        query,
        case_sensitive: case_sensitive.unwrap_or(false),
    };
    let mut page_models = Vec::with_capacity(page_count);
    for page_index in 0..page_count {
        let page_model = PdfPageIntermediateService::resolve_vector_page_model(
            state.clone(),
            path.clone(),
            page_index as u16,
            1.0,
            None,
        )
        .await?;
        page_models.push(page_model);
    }
    Ok(crate::application::pdf::page_search::search_document_regions(&page_models, &request))
}
