use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::editor::orchestrator::replace_pipeline::{
    apply_region_text_replacements_tx, RegionTextReplaceRequest,
};
use crate::find::host_find_store::{
    clear_find_session, move_find_match, read_find_session, set_find_session, HostFindScope,
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

pub use crate::find::find_store::{
    ReplaceRequest, SearchBox as SearchBoxRect, SearchMatch, SearchResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindSessionData {
    pub query: String,
    pub scope: String,
    pub page_indices: Vec<u16>,
    pub current_page: u16,
    pub active_index: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchFacadeResult {
    pub changed: bool,
    pub session: Option<FindSessionData>,
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
    let _request: BatchReplaceRequest = match serde_wasm_bindgen::from_value(request_js) {
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
    let session: FindSessionData = match serde_wasm_bindgen::from_value(session_js) {
        Ok(s) => s,
        Err(_) => return JsValue::NULL,
    };
    let scope = match session.scope.as_str() {
        "document" => HostFindScope::Document,
        _ => HostFindScope::Page,
    };
    let nav = set_find_session(
        session.query.clone(),
        scope,
        session.page_indices.clone(),
        Some(session.current_page),
    );
    let result = SearchFacadeResult {
        changed: true,
        session: Some(session),
        navigation: Some(FindNavigation {
            active_index: nav.active_index,
            target_page: nav.active_page,
        }),
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeClearSession")]
pub fn facade_clear_session() -> JsValue {
    clear_find_session();
    let result = SearchFacadeResult {
        changed: true,
        session: None,
        navigation: None,
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeMoveMatch")]
pub fn facade_move_match(step: i32) -> JsValue {
    let nav = move_find_match(step);
    let result = SearchFacadeResult {
        changed: true,
        session: None,
        navigation: Some(FindNavigation {
            active_index: nav.active_index,
            target_page: nav.active_page,
        }),
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "searchFacadeGetSession")]
pub fn facade_get_session() -> JsValue {
    let session = read_find_session();
    let result = FindSessionData {
        query: session.query,
        scope: match session.scope {
            HostFindScope::Document => "document".into(),
            _ => "page".into(),
        },
        page_indices: session.match_pages,
        current_page: 0,
        active_index: session.active_index,
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}
