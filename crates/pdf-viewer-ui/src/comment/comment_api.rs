//! CommentManager — P2 struct-based WASM API for PDF comments / annotation list.
//!
//! Mirrors the P0/P1 pattern. All operations are infallible delegations to
//! `crate::document::comment` and `crate::review::review_store`, so no
//! response wrapper is needed (unlike sessions with stubs).

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::document::comment::{
    add_region_comment, delete_page_annotation, load_comment_overlay, load_comment_review,
    load_comment_target_overlay, select_comment_review_and_load,
    set_comment_review_panel_open_and_load, set_comment_review_query_and_load,
    set_comment_review_scope_and_load, toggle_comment_review_panel_and_load, update_page_comment,
    PdfDeleteAnnotationRequest, PdfRegionCommentRequest, PdfUpdateCommentRequest,
};
use crate::review::review_store::{
    clear_review_session as clear_comment_review_session,
    read_review_session as read_comment_review_session, HostCommentReviewScope,
};

fn parse_scope(scope: &str) -> HostCommentReviewScope {
    match scope {
        "document" => HostCommentReviewScope::Document,
        _ => HostCommentReviewScope::Page,
    }
}

fn parse_request<T: serde::de::DeserializeOwned>(
    request_js: JsValue,
    method: &str,
) -> Result<T, JsValue> {
    from_value(request_js)
        .map_err(|e| JsValue::from_str(&format!("CommentManager.{method}: invalid request: {e}")))
}

// ── CommentManager ──────────────────────────────────────────────

#[wasm_bindgen]
pub struct CommentManager;

#[wasm_bindgen]
impl CommentManager {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        CommentManager
    }

    // ── Review session state ────────────────────────────────────

    #[wasm_bindgen(js_name = "clearReviewSession")]
    pub fn clear_review_session(&self) {
        clear_comment_review_session();
    }

    #[wasm_bindgen(js_name = "readReviewSession")]
    pub fn read_review_session(&self) -> JsValue {
        to_value(&read_comment_review_session()).unwrap_or(JsValue::NULL)
    }

    // ── Review pipeline ─────────────────────────────────────────

    #[wasm_bindgen(js_name = "loadReview")]
    pub async fn load_review(&self, path: String, current_page: u16) -> Result<JsValue, JsValue> {
        let result = load_comment_review(path, current_page).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "loadOverlay")]
    pub async fn load_overlay(&self, path: String, current_page: u16) -> Result<JsValue, JsValue> {
        let result = load_comment_overlay(path, current_page).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "loadTargetOverlay")]
    pub async fn load_target_overlay(
        &self,
        path: String,
        current_page: u16,
    ) -> Result<JsValue, JsValue> {
        let result = load_comment_target_overlay(path, current_page).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "setPanelOpenAndLoad")]
    pub async fn set_panel_open_and_load(
        &self,
        path: String,
        current_page: u16,
        panel_open: bool,
    ) -> Result<JsValue, JsValue> {
        let result = set_comment_review_panel_open_and_load(path, current_page, panel_open).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "togglePanelAndLoad")]
    pub async fn toggle_panel_and_load(
        &self,
        path: String,
        current_page: u16,
    ) -> Result<JsValue, JsValue> {
        let result = toggle_comment_review_panel_and_load(path, current_page).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "setScopeAndLoad")]
    pub async fn set_scope_and_load(
        &self,
        path: String,
        current_page: u16,
        scope: String,
    ) -> Result<JsValue, JsValue> {
        let result =
            set_comment_review_scope_and_load(path, current_page, parse_scope(&scope)).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "setQueryAndLoad")]
    pub async fn set_query_and_load(
        &self,
        path: String,
        current_page: u16,
        query: String,
    ) -> Result<JsValue, JsValue> {
        let result = set_comment_review_query_and_load(path, current_page, query).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "selectAndLoad")]
    pub async fn select_and_load(
        &self,
        path: String,
        current_page: u16,
        selected_comment_id: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let result =
            select_comment_review_and_load(path, current_page, selected_comment_id).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    // ── Mutation ────────────────────────────────────────────────

    #[wasm_bindgen(js_name = "addRegionComment")]
    pub async fn add_region_comment(
        &self,
        path: String,
        request_js: JsValue,
    ) -> Result<JsValue, JsValue> {
        let request: PdfRegionCommentRequest = parse_request(request_js, "addRegionComment")?;
        let result = add_region_comment(path, request).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "deleteAnnotation")]
    pub async fn delete_annotation(
        &self,
        path: String,
        request_js: JsValue,
    ) -> Result<JsValue, JsValue> {
        let request: PdfDeleteAnnotationRequest = parse_request(request_js, "deleteAnnotation")?;
        let result = delete_page_annotation(path, request).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = "updateComment")]
    pub async fn update_comment(
        &self,
        path: String,
        request_js: JsValue,
    ) -> Result<JsValue, JsValue> {
        let request: PdfUpdateCommentRequest = parse_request(request_js, "updateComment")?;
        let result = update_page_comment(path, request).await?;
        Ok(to_value(&result).unwrap_or(JsValue::NULL))
    }
}

impl Default for CommentManager {
    fn default() -> Self {
        Self::new()
    }
}
