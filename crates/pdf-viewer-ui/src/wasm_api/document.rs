// ─────────────────────────────────────────────────────────────────────────────
// LEGACY document/find/comment/review wasm API.
//
// Document-domain functions here are SUPERSEDED by `crate::document::facade::*`
// (js_name `documentFacade*`). Keep these bindings only until the TS bridge
// layer is migrated.
//
// Find / comment / review groupings will be split into dedicated facades
// (find::facade, comment::facade, review::facade) in subsequent phases.
//
// Do NOT add new functions here. (Tracked in progress.txt phase 5.)
// ─────────────────────────────────────────────────────────────────────────────

#![allow(deprecated)]

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
    select_comment_review_and_load as host_select_comment_review_and_load,
    set_comment_review_panel_open_and_load as host_set_comment_review_panel_open_and_load,
    set_comment_review_query_and_load as host_set_comment_review_query_and_load,
    set_comment_review_scope_and_load as host_set_comment_review_scope_and_load,
    toggle_comment_review_panel_and_load as host_toggle_comment_review_panel_and_load,
    update_page_comment as host_update_page_comment, PdfCommentReviewRequest,
    PdfDeleteAnnotationRequest, PdfRegionCommentRequest, PdfUpdateCommentRequest,
};
use crate::document::host_pipeline::{
    close_document_pipeline as host_close_document_pipeline,
    open_document_pipeline as host_open_document_pipeline,
    pick_document_pipeline as host_pick_document_pipeline,
    redo_document_pipeline as host_redo_document_pipeline,
    rotate_document_pipeline as host_rotate_document_pipeline,
    undo_document_pipeline as host_undo_document_pipeline, OpenDocumentPipelineRequest,
    PickDocumentPipelineRequest,
};
use crate::document::mutation_pipeline::request_document_refresh as host_request_document_refresh;
use crate::document::review::{
    accept_all_review_changes as host_accept_all_review_changes,
    accept_review_change as host_accept_review_change,
    get_review_feed as host_get_review_feed,
    reject_all_review_changes as host_reject_all_review_changes,
    reject_review_change as host_reject_review_change,
};
use crate::host::command::{
    open_document_session as host_open_document_session,
    reset_host_document_session as host_reset_host_document_session,
    OpenDocumentSessionRequest,
};
use crate::present::plan_builder::FramePlanRequest;
use crate::viewer::comment_review::{
    clear_comment_review_session as host_clear_comment_review_session,
    get_comment_review_session as host_get_comment_review_session, HostCommentReviewScope,
};
use crate::viewer::find::{
    clear_find_session as host_clear_find_session,
    get_find_session as host_get_find_session, move_find_match as host_move_find_match,
    set_find_session as host_set_find_session, HostFindScope,
};
use crate::viewer::runtime::{
    get_session as host_get_viewer_session,
    note_document_mutation as host_note_document_mutation,
    reset_session as host_reset_viewer_session,
    set_document as host_set_viewer_document, set_page_size as host_set_page_size,
};

#[wasm_bindgen]
pub fn undo_document_pipeline() -> JsValue {
    to_value(&host_undo_document_pipeline()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn redo_document_pipeline() -> JsValue {
    to_value(&host_redo_document_pipeline()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub async fn open_document_pipeline(request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: OpenDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
    let result = host_open_document_pipeline(request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn pick_document_pipeline(request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: PickDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
    host_pick_document_pipeline(request).await
}

#[wasm_bindgen]
pub async fn rotate_document_pipeline(delta: i32) -> Result<JsValue, JsValue> {
    let result = host_rotate_document_pipeline(delta).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub fn reset_viewer_session() {
    host_reset_viewer_session();
}

#[wasm_bindgen]
pub fn get_viewer_session() -> JsValue {
    to_value(&host_get_viewer_session()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn note_document_mutation(reason: String) -> u64 {
    host_note_document_mutation(&reason)
}

#[wasm_bindgen]
pub fn request_document_refresh(reason: String, frame_request_js: JsValue) -> JsValue {
    let frame_request: FramePlanRequest = from_value(frame_request_js).unwrap_or_default();
    let result = host_request_document_refresh(&reason, frame_request);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn set_viewer_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    host_set_viewer_document(path, page_count, initial_zoom);
}

#[wasm_bindgen]
pub fn set_page_dimensions(page_width: f32, page_height: f32) {
    log::info!(
        "[PAGE-SIZE] wasm_set_page_dimensions called. Width={}, Height={}",
        page_width,
        page_height
    );
    host_set_page_size(page_width, page_height);
}

#[wasm_bindgen]
pub fn open_document_session(request_js: JsValue) -> JsValue {
    let request: OpenDocumentSessionRequest = from_value(request_js).unwrap_or_default();
    to_value(&host_open_document_session(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reset_host_document_session(
    default_page_width: f32,
    default_page_height: f32,
) -> JsValue {
    to_value(&host_reset_host_document_session(
        default_page_width,
        default_page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn close_document_pipeline(
    default_page_width: f32,
    default_page_height: f32,
) -> JsValue {
    to_value(&host_close_document_pipeline(
        default_page_width,
        default_page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn clear_find_session() {
    host_clear_find_session();
}

#[wasm_bindgen]
pub fn get_find_session() -> JsValue {
    to_value(&host_get_find_session()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn set_find_session(
    query: String,
    scope: String,
    match_pages_js: JsValue,
    preferred_active_page: Option<u16>,
) -> JsValue {
    let match_pages: Vec<u16> = from_value(match_pages_js).unwrap_or_default();
    let scope = match scope.as_str() {
        "document" => HostFindScope::Document,
        _ => HostFindScope::Page,
    };
    to_value(&host_set_find_session(
        query,
        scope,
        match_pages,
        preferred_active_page,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn move_find_match(step: i32) -> JsValue {
    to_value(&host_move_find_match(step)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn clear_comment_review_session() {
    host_clear_comment_review_session();
}

#[wasm_bindgen]
pub fn get_comment_review_session() -> JsValue {
    to_value(&host_get_comment_review_session()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn get_review_feed() -> JsValue {
    to_value(&host_get_review_feed()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn accept_review_change(patch_key: String) -> JsValue {
    to_value(&host_accept_review_change(&patch_key)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reject_review_change(patch_key: String) -> JsValue {
    to_value(&host_reject_review_change(&patch_key)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn accept_all_review_changes() -> JsValue {
    to_value(&host_accept_all_review_changes()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reject_all_review_changes() -> JsValue {
    to_value(&host_reject_all_review_changes()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub async fn list_page_comments(
    path: String,
    page_index: u16,
) -> Result<JsValue, JsValue> {
    let result = host_list_page_comments(path, page_index).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn list_page_annotation_targets(
    path: String,
    page_index: u16,
) -> Result<JsValue, JsValue> {
    let result = host_list_page_annotation_targets(path, page_index).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn review_document_comments(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfCommentReviewRequest = from_value(request_js).unwrap_or_default();
    let result = host_review_document_comments(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn load_comment_review(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = host_load_comment_review(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn load_comment_overlay(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = host_load_comment_overlay(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn load_comment_target_overlay(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = host_load_comment_target_overlay(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn set_comment_review_panel_open_and_load(
    path: String,
    current_page: u16,
    panel_open: bool,
) -> Result<JsValue, JsValue> {
    let result =
        host_set_comment_review_panel_open_and_load(path, current_page, panel_open).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn toggle_comment_review_panel_and_load(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = host_toggle_comment_review_panel_and_load(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn set_comment_review_scope_and_load(
    path: String,
    current_page: u16,
    scope: String,
) -> Result<JsValue, JsValue> {
    let scope = match scope.as_str() {
        "document" => HostCommentReviewScope::Document,
        _ => HostCommentReviewScope::Page,
    };
    let result = host_set_comment_review_scope_and_load(path, current_page, scope).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn set_comment_review_query_and_load(
    path: String,
    current_page: u16,
    query: String,
) -> Result<JsValue, JsValue> {
    let result = host_set_comment_review_query_and_load(path, current_page, query).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn select_comment_review_and_load(
    path: String,
    current_page: u16,
    selected_comment_id: Option<String>,
) -> Result<JsValue, JsValue> {
    let result =
        host_select_comment_review_and_load(path, current_page, selected_comment_id).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn add_region_comment(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfRegionCommentRequest = from_value(request_js).unwrap_or_default();
    let result = host_add_region_comment(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn delete_page_annotation(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfDeleteAnnotationRequest = from_value(request_js).unwrap_or_default();
    let result = host_delete_page_annotation(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn update_page_comment(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfUpdateCommentRequest = from_value(request_js).unwrap_or_default();
    let result = host_update_page_comment(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}



