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
// Do NOT add new functions here — all new APIs go through the Session structs
// (DocumentSession / FindSession / CommentManager / ReviewSession / ...).
// ─────────────────────────────────────────────────────────────────────────────

#![allow(deprecated)]

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::document::comment::{
    PdfCommentReviewRequest, PdfDeleteAnnotationRequest, PdfRegionCommentRequest,
    PdfUpdateCommentRequest,
};
use crate::document::host_pipeline::{OpenDocumentPipelineRequest, PickDocumentPipelineRequest};
use crate::document::{comment, host_pipeline, mutation_pipeline, review};
use crate::host::command::OpenDocumentSessionRequest;
use crate::host::command;
use crate::present::plan_builder::FramePlanRequest;
use crate::viewer::find_store::HostFindScope;
use crate::viewer::review_store::HostCommentReviewScope;
use crate::viewer::{find_store, review_store, viewer_controller};

#[wasm_bindgen]
pub fn undo_document_pipeline() -> JsValue {
    to_value(&host_pipeline::undo_document_pipeline()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn redo_document_pipeline() -> JsValue {
    to_value(&host_pipeline::redo_document_pipeline()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub async fn open_document_pipeline(request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: OpenDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
    let result = host_pipeline::open_document_pipeline(request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn pick_document_pipeline(request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: PickDocumentPipelineRequest = from_value(request_js).unwrap_or_default();
    host_pipeline::pick_document_pipeline(request).await
}

#[wasm_bindgen]
pub async fn rotate_document_pipeline(delta: i32) -> Result<JsValue, JsValue> {
    let result = host_pipeline::rotate_document_pipeline(delta).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub fn reset_viewer_session() {
    viewer_controller::reset_session();
}

#[wasm_bindgen]
pub fn get_viewer_session() -> JsValue {
    to_value(&viewer_controller::get_session()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn note_document_mutation(reason: String) -> u64 {
    viewer_controller::note_document_mutation(&reason)
}

#[wasm_bindgen]
pub fn request_document_refresh(reason: String, frame_request_js: JsValue) -> JsValue {
    let frame_request: FramePlanRequest = from_value(frame_request_js).unwrap_or_default();
    let result = mutation_pipeline::request_document_refresh(&reason, frame_request);
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn set_viewer_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    viewer_controller::set_document(path, page_count, initial_zoom);
}

#[wasm_bindgen]
pub fn set_page_dimensions(page_width: f32, page_height: f32) {
    log::info!(
        "[PAGE-SIZE] wasm_set_page_dimensions called. Width={}, Height={}",
        page_width,
        page_height
    );
    viewer_controller::set_page_size(page_width, page_height);
}

#[wasm_bindgen]
pub fn open_document_session(request_js: JsValue) -> JsValue {
    let request: OpenDocumentSessionRequest = from_value(request_js).unwrap_or_default();
    to_value(&command::open_document_session(request)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reset_host_document_session(
    default_page_width: f32,
    default_page_height: f32,
) -> JsValue {
    to_value(&command::reset_host_document_session(
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
    to_value(&host_pipeline::close_document_pipeline(
        default_page_width,
        default_page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn clear_find_session() {
    find_store::clear_find_session();
}

#[wasm_bindgen]
pub fn get_find_session() -> JsValue {
    to_value(&find_store::get_find_session()).unwrap_or(JsValue::NULL)
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
    to_value(&find_store::set_find_session(
        query,
        scope,
        match_pages,
        preferred_active_page,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn move_find_match(step: i32) -> JsValue {
    to_value(&find_store::move_find_match(step)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn clear_comment_review_session() {
    review_store::clear_comment_review_session();
}

#[wasm_bindgen]
pub fn get_comment_review_session() -> JsValue {
    to_value(&review_store::get_comment_review_session()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn get_review_feed() -> JsValue {
    to_value(&review::get_review_feed()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn accept_review_change(patch_key: String) -> JsValue {
    to_value(&review::accept_review_change(&patch_key)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reject_review_change(patch_key: String) -> JsValue {
    to_value(&review::reject_review_change(&patch_key)).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn accept_all_review_changes() -> JsValue {
    to_value(&review::accept_all_review_changes()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn reject_all_review_changes() -> JsValue {
    to_value(&review::reject_all_review_changes()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub async fn list_page_comments(
    path: String,
    page_index: u16,
) -> Result<JsValue, JsValue> {
    let result = comment::list_page_comments(path, page_index).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn list_page_annotation_targets(
    path: String,
    page_index: u16,
) -> Result<JsValue, JsValue> {
    let result = comment::list_page_annotation_targets(path, page_index).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn review_document_comments(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfCommentReviewRequest = from_value(request_js).unwrap_or_default();
    let result = comment::review_document_comments(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn load_comment_review(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = comment::load_comment_review(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn load_comment_overlay(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = comment::load_comment_overlay(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn load_comment_target_overlay(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = comment::load_comment_target_overlay(path, current_page).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn set_comment_review_panel_open_and_load(
    path: String,
    current_page: u16,
    panel_open: bool,
) -> Result<JsValue, JsValue> {
    let result =
        comment::set_comment_review_panel_open_and_load(path, current_page, panel_open).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn toggle_comment_review_panel_and_load(
    path: String,
    current_page: u16,
) -> Result<JsValue, JsValue> {
    let result = comment::toggle_comment_review_panel_and_load(path, current_page).await?;
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
    let result = comment::set_comment_review_scope_and_load(path, current_page, scope).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn set_comment_review_query_and_load(
    path: String,
    current_page: u16,
    query: String,
) -> Result<JsValue, JsValue> {
    let result = comment::set_comment_review_query_and_load(path, current_page, query).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn select_comment_review_and_load(
    path: String,
    current_page: u16,
    selected_comment_id: Option<String>,
) -> Result<JsValue, JsValue> {
    let result =
        comment::select_comment_review_and_load(path, current_page, selected_comment_id).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn add_region_comment(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfRegionCommentRequest = from_value(request_js).unwrap_or_default();
    let result = comment::add_region_comment(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn delete_page_annotation(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfDeleteAnnotationRequest = from_value(request_js).unwrap_or_default();
    let result = comment::delete_page_annotation(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen]
pub async fn update_page_comment(
    path: String,
    request_js: JsValue,
) -> Result<JsValue, JsValue> {
    let request: PdfUpdateCommentRequest = from_value(request_js).unwrap_or_default();
    let result = comment::update_page_comment(path, request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}



