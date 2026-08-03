//! Document-domain free wasm exports retained for the TS bridge.
//!
//! 这些函数原位于 `wasm_api/document.rs`（已删除）。它们都是 3 行薄 JS 适配器
//! （serde 进 → 域函数 → serde 出），属于"未升级到 Session 模式但仍在 TS 调用"
//! 的兼容层。新代码请走 `DocumentSession` / `CommentManager` / `ReviewSession`。

use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

use crate::document::document_types::parse_request;
use crate::document::host_pipeline::{
    self, OpenDocumentPipelineRequest, PickDocumentPipelineRequest,
};
use crate::viewer::viewer_controller;

#[wasm_bindgen(js_name = "undoDocumentPipeline")]
pub fn undo_document_pipeline() -> JsValue {
    to_value(&host_pipeline::undo_document_pipeline()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "redoDocumentPipeline")]
pub fn redo_document_pipeline() -> JsValue {
    to_value(&host_pipeline::redo_document_pipeline()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "openDocumentPipeline")]
pub async fn open_document_pipeline(request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: OpenDocumentPipelineRequest = parse_request(request_js, "openDocumentPipeline")?;
    let result = host_pipeline::open_document_pipeline(request).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "pickDocumentPipeline")]
pub async fn pick_document_pipeline(request_js: JsValue) -> Result<JsValue, JsValue> {
    let request: PickDocumentPipelineRequest = parse_request(request_js, "pickDocumentPipeline")?;
    host_pipeline::pick_document_pipeline(request).await
}

#[wasm_bindgen(js_name = "rotateDocumentPipeline")]
pub async fn rotate_document_pipeline(delta: i32) -> Result<JsValue, JsValue> {
    let result = host_pipeline::rotate_document_pipeline(delta).await?;
    Ok(to_value(&result).unwrap_or(JsValue::NULL))
}

#[wasm_bindgen(js_name = "closeDocumentPipeline")]
pub fn close_document_pipeline(default_page_width: f32, default_page_height: f32) -> JsValue {
    to_value(&host_pipeline::close_document_pipeline(
        default_page_width,
        default_page_height,
    ))
    .unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "readViewerSession")]
pub fn read_viewer_session() -> JsValue {
    to_value(&viewer_controller::read_session()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "setViewerDocument")]
pub fn set_viewer_document(path: Option<String>, page_count: u16, initial_zoom: f32) {
    viewer_controller::set_document(path, page_count, initial_zoom);
}
