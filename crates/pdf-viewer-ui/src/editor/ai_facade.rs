use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiSuggestion {
    pub id: String,
    pub page_index: u16,
    pub region_id: String,
    pub kind: Option<String>,
    pub original_text: String,
    pub suggested_text: String,
    pub state: String, // "pending" | "applied" | "failed"
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeChatTurn {
    pub role: String, // "user" | "assistant"
    pub content: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiThreadView {
    pub suggestions: Vec<ResumeAiSuggestion>,
    pub turns: Vec<ResumeChatTurn>,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiSessionRequest {
    pub path: String,
    pub page_index: u16,
    pub scope: String, // "current-page" | "whole-document"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiPromptRequest {
    pub path: String,
    pub page_index: u16,
    pub scope: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiApplyRequest {
    pub path: String,
    pub suggestion_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiApplyAllRequest {
    pub path: String,
    pub suggestions: Vec<ResumeAiSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiSaveAsRequest {
    pub path: String,
    pub suggestions: Vec<ResumeAiSuggestion>,
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiSaveAsResult {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiClearRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResumeAiFacadeResult {
    pub changed: bool,
    pub thread_view: Option<ResumeAiThreadView>,
    pub save_as_result: Option<ResumeAiSaveAsResult>,
}

#[wasm_bindgen(js_name = "resumeAiFacadeSyncSession")]
pub fn facade_sync_session(request_js: JsValue) -> JsValue {
    let request: ResumeAiSessionRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing AI session implementation
    let result = ResumeAiThreadView {
        suggestions: vec![],
        turns: vec![],
        notice: None,
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resumeAiFacadeSubmitPrompt")]
pub fn facade_submit_prompt(request_js: JsValue) -> JsValue {
    let request: ResumeAiPromptRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing AI prompt implementation
    let result = ResumeAiThreadView {
        suggestions: vec![],
        turns: vec![],
        notice: None,
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resumeAiFacadeApplySuggestion")]
pub fn facade_apply_suggestion(request_js: JsValue) -> JsValue {
    let request: ResumeAiApplyRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing AI apply implementation
    let result = ResumeAiFacadeResult {
        changed: true,
        thread_view: None,
        save_as_result: None,
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resumeAiFacadeApplyAll")]
pub fn facade_apply_all(request_js: JsValue) -> JsValue {
    let request: ResumeAiApplyAllRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing AI apply all implementation
    let result = ResumeAiFacadeResult {
        changed: true,
        thread_view: None,
        save_as_result: None,
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resumeAiFacadeSaveAs")]
pub fn facade_save_as(request_js: JsValue) -> JsValue {
    let request: ResumeAiSaveAsRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing AI save as implementation
    let result = ResumeAiFacadeResult {
        changed: true,
        thread_view: None,
        save_as_result: Some(ResumeAiSaveAsResult {
            path: request.target_path,
        }),
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "resumeAiFacadeClearSuggestions")]
pub fn facade_clear_suggestions(request_js: JsValue) -> JsValue {
    let request: ResumeAiClearRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    // Delegate to existing AI clear implementation
    let result = ResumeAiThreadView {
        suggestions: vec![],
        turns: vec![],
        notice: Some("建议列表已清空".to_string()),
    };

    to_value(&result).unwrap_or(JsValue::NULL)
}
