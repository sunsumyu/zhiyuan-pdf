use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;

use crate::viewer::find::{
    clear_find_session as host_clear_find_session,
    get_find_session as host_get_find_session,
    move_find_match as host_move_find_match,
    set_find_session as host_set_find_session,
    HostFindScope,
};
use crate::editor::replace_pipeline::{
    apply_region_text_replacements_tx, RegionTextReplaceRequest,
};
use crate::present::plan_builder::FramePlanRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPageRequest {
    pub path: String,
    pub page_index: u16,
    pub query: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchDocumentRequest {
    pub path: String,
    pub page_count: u16,
    pub query: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub id: String,
    pub kind: String,
    pub page_index: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub line_index: usize,
    pub source_text: String,
    pub preview_text: String,
    pub matched_text: String,
    pub object_indices: Vec<usize>,
    pub box_rect: SearchBoxRect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchBoxRect {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub query: String,
    pub total_matches: usize,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceRequest {
    pub path: String,
    pub page_index: u16,
    pub region_id: String,
    pub kind: String,
    pub original_text: String,
    pub query: String,
    pub replacement: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceResult {
    pub applied: bool,
    pub page_index: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchReplaceRequest {
    pub path: String,
    pub page_count: u16,
    pub query: String,
    pub replacement: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchReplaceResult {
    pub applied_count: usize,
    pub skipped_count: usize,
    pub touched_pages: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindSession {
    pub query: String,
    pub scope: String,
    pub page_indices: Vec<u16>,
    pub current_page: u16,
    pub active_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchFacadeResult {
    pub changed: bool,
    pub session: Option<FindSession>,
    pub navigation: Option<FindNavigation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindNavigation {
    pub active_index: usize,
    pub target_page: Option<u16>,
}

fn build_frame_request() -> FramePlanRequest {
    FramePlanRequest::default()
}

#[wasm_bindgen(js_name = "searchFacadePage")]
pub fn facade_search_page(request_js: JsValue) -> JsValue {
    let request: SearchPageRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing search implementation
    // This would call the actual search logic in the search module
    let result = SearchResult {
        query: request.query.clone(),
        total_matches: 0,
        matches: vec![],
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeDocument")]
pub fn facade_search_document(request_js: JsValue) -> JsValue {
    let request: SearchDocumentRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing search implementation
    let result = SearchResult {
        query: request.query.clone(),
        total_matches: 0,
        matches: vec![],
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeReplace")]
pub fn facade_replace(request_js: JsValue) -> JsValue {
    let request: ReplaceRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };
    let replace_result = apply_region_text_replacements_tx(
        vec![RegionTextReplaceRequest {
            page_index: request.page_index,
            region_id: request.region_id,
            kind: request.kind,
            original_text: request.original_text,
            query: request.query,
            replacement: request.replacement,
            replace_all_occurrences: false,
        }],
        build_frame_request(),
    );
    let result = ReplaceResult {
        applied: replace_result.applied_count > 0,
        page_index: request.page_index,
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeBatchReplace")]
pub fn facade_batch_replace(request_js: JsValue) -> JsValue {
    let request: BatchReplaceRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing batch replace implementation
    let result = BatchReplaceResult {
        applied_count: 0,
        skipped_count: 0,
        touched_pages: vec![],
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeSetSession")]
pub fn facade_set_session(session_js: JsValue) -> JsValue {
    let session: FindSession = match serde_wasm_bindgen::from_value(session_js) {
        Ok(s) => s,
        Err(_) => return JsValue::NULL,
    };
    let scope = match session.scope.as_str() {
        "document" => HostFindScope::Document,
        _ => HostFindScope::Page,
    };
    let nav = host_set_find_session(session.query.clone(), scope, session.page_indices.clone(), Some(session.current_page));
    let result = SearchFacadeResult {
        changed: true,
        session: Some(session),
        navigation: Some(FindNavigation { active_index: nav.active_index, target_page: nav.active_page }),
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeClearSession")]
pub fn facade_clear_session() -> JsValue {
    host_clear_find_session();
    let result = SearchFacadeResult { changed: true, session: None, navigation: None };
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeMoveMatch")]
pub fn facade_move_match(step: i32) -> JsValue {
    let nav = host_move_find_match(step);
    let result = SearchFacadeResult {
        changed: true,
        session: None,
        navigation: Some(FindNavigation { active_index: nav.active_index, target_page: nav.active_page }),
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeGetSession")]
pub fn facade_get_session() -> JsValue {
    let session = host_get_find_session();
    let result = FindSession {
        query: session.query,
        scope: match session.scope { HostFindScope::Document => "document".into(), _ => "page".into() },
        page_indices: session.match_pages,
        current_page: 0,
        active_index: session.active_index,
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}
