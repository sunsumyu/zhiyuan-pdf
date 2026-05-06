// ─────────────────────────────────────────────────────────────────────────────
// Comment facade — frozen v1 API surface for PDF comments / annotation list.
//
// See docs/api-contract.md.
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::document::comment::{
    add_region_comment as host_add_region_comment,
    delete_page_annotation as host_delete_page_annotation,
    list_page_annotation_targets as host_list_page_annotation_targets,
    list_page_comments as host_list_page_comments,
    load_comment_overlay as host_load_comment_overlay,
    load_comment_review as host_load_comment_review,
    load_comment_target_overlay as host_load_comment_target_overlay,
    review_document_comments as host_review_document_comments,
    select_comment_review_and_load as host_select_review_and_load,
    set_comment_review_panel_open_and_load as host_set_panel_and_load,
    set_comment_review_query_and_load as host_set_query_and_load,
    set_comment_review_scope_and_load as host_set_scope_and_load,
    toggle_comment_review_panel_and_load as host_toggle_panel_and_load,
    update_page_comment as host_update_page_comment,
    PdfCommentReviewRequest, PdfDeleteAnnotationRequest, PdfRegionCommentRequest,
    PdfUpdateCommentRequest,
};
use crate::viewer::comment_review::{
    clear_comment_review_session as host_clear_review_session,
    get_comment_review_session as host_get_review_session,
    HostCommentReviewScope,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct StubResult {
    implemented: bool,
    error: String,
}

fn stub(api: &str) -> JsValue {
    let result = StubResult {
        implemented: false,
        error: format!("{} is reserved but not yet implemented", api),
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

fn parse_scope(scope: &str) -> HostCommentReviewScope {
    match scope {
        "document" => HostCommentReviewScope::Document,
        _ => HostCommentReviewScope::Page,
    }
}

// ─── Stable — session state ──────────────────────────────────────────────────

#[wasm_bindgen(js_name = "commentFacadeClearReviewSession")]
pub fn facade_clear_review_session() {
    host_clear_review_session();
}

#[wasm_bindgen(js_name = "commentFacadeReadReviewSession")]
pub fn facade_read_review_session() -> JsValue {
    to_value(&host_get_review_session()).unwrap_or(JsValue::NULL)
}

// ─── Stable — listings ───────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "commentFacadeListPageComments")]
pub async fn facade_list_page_comments(path: String, page_index: u16) -> Result<JsValue, JsValue> {
    let result = host_list_page_comments(path, page_index).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeListPageAnnotationTargets")]
pub async fn facade_list_page_annotation_targets(
    path: String,
    page_index: u16,
) -> Result<JsValue, JsValue> {
    let result = host_list_page_annotation_targets(path, page_index).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

// ─── Stable — review pipeline ────────────────────────────────────────────────

#[wasm_bindgen(js_name = "commentFacadeReviewDocument")]
pub async fn facade_review_document(path: String, request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: PdfCommentReviewRequest = from_value(request_js).unwrap_or_default();
    let result = host_review_document_comments(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeLoadReview")]
pub async fn facade_load_review(path: String, current_page: u16) -> Result<JsValue, JsValue> {
    let result = host_load_comment_review(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeLoadOverlay")]
pub async fn facade_load_overlay(path: String, current_page: u16) -> Result<JsValue, JsValue> {
    let result = host_load_comment_overlay(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeLoadTargetOverlay")]
pub async fn facade_load_target_overlay(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = host_load_comment_target_overlay(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeSetPanelOpenAndLoad")]
pub async fn facade_set_panel_open_and_load(
    path: String,
    current_page: u16,
    panel_open: bool,
) -> Result<JsValue, JsValue> {
    let result = host_set_panel_and_load(path, current_page, panel_open).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeTogglePanelAndLoad")]
pub async fn facade_toggle_panel_and_load(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = host_toggle_panel_and_load(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeSetScopeAndLoad")]
pub async fn facade_set_scope_and_load(
    path: String,
    current_page: u16,
    scope: String,
) -> Result<JsValue, JsValue> {
    let result = host_set_scope_and_load(path, current_page, parse_scope(&scope)).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeSetQueryAndLoad")]
pub async fn facade_set_query_and_load(
    path: String,
    current_page: u16,
    query: String,
) -> Result<JsValue, JsValue> {
    let result = host_set_query_and_load(path, current_page, query).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeSelectAndLoad")]
pub async fn facade_select_and_load(
    path: String,
    current_page: u16,
    selected_comment_id: Option<String>,
) -> Result<JsValue, JsValue> {
    let result = host_select_review_and_load(path, current_page, selected_comment_id).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

// ─── Stable — mutation ───────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "commentFacadeAddRegionComment")]
pub async fn facade_add_region_comment(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfRegionCommentRequest = from_value(request_js).unwrap_or_default();
    let result = host_add_region_comment(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeDeleteAnnotation")]
pub async fn facade_delete_annotation(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfDeleteAnnotationRequest = from_value(request_js).unwrap_or_default();
    let result = host_delete_page_annotation(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "commentFacadeUpdateComment")]
pub async fn facade_update_comment(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfUpdateCommentRequest = from_value(request_js).unwrap_or_default();
    let result = host_update_page_comment(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

// ─── Stubs ───────────────────────────────────────────────────────────────────

/// Reserved: reply to an existing comment (threaded discussion).
#[wasm_bindgen(js_name = "commentFacadeReplyComment")]
pub fn facade_reply_comment(
    _path: String,
    _parent_annotation_id: String,
    _contents: String,
) -> JsValue {
    stub("comment.replyComment")
}

/// Reserved: resolve / unresolve a comment thread.
#[wasm_bindgen(js_name = "commentFacadeSetResolved")]
pub fn facade_set_resolved(
    _path: String,
    _annotation_id: String,
    _resolved: bool,
) -> JsValue {
    stub("comment.setResolved")
}

/// Reserved: export comments to JSON / CSV / PDF summary.
#[wasm_bindgen(js_name = "commentFacadeExport")]
pub fn facade_export(_path: String, _format: String) -> JsValue {
    stub("comment.export")
}

/// Reserved: import comments from another PDF (merge).
#[wasm_bindgen(js_name = "commentFacadeImport")]
pub fn facade_import(_target_path: String, _source_path: String) -> JsValue {
    stub("comment.import")
}
