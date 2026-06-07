use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::document::history::{redo_document_edit, undo_document_edit};
use crate::document::io::{
    open_pdf_file, pick_pdf_file, rotate_current_page, OpenPdfFileResult, RotateCurrentPageResult,
};
use crate::host::command::{
    open_document_session, reset_host_document_session, HostActionResult,
    OpenDocumentSessionRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenDocumentPipelineRequest {
    pub path: String,
    pub initial_zoom: f32,
    pub default_page_width: f32,
    pub default_page_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenDocumentPipelineResult {
    pub opened: bool,
    pub path: Option<String>,
    pub page_count: u16,
    pub current_page: u16,
    pub current_zoom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CloseDocumentPipelineResult {
    pub closed: bool,
    pub current_page: u16,
    pub current_zoom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PickDocumentPipelineRequest {
    pub initial_zoom: f32,
    pub default_page_width: f32,
    pub default_page_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RotateDocumentPipelineResult {
    pub rotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMutationPipelineResult {
    pub changed: bool,
}

fn open_session_from_file_result(
    path: String,
    open_result: OpenPdfFileResult,
    initial_zoom: f32,
    default_page_width: f32,
    default_page_height: f32,
) -> OpenDocumentPipelineResult {
    if !open_result.opened || open_result.page_count == 0 {
        return OpenDocumentPipelineResult {
            opened: false,
            path: None,
            page_count: open_result.page_count,
            current_page: 0,
            current_zoom: initial_zoom,
        };
    }

    let session = open_document_session(OpenDocumentSessionRequest {
        path: path.clone(),
        page_count: open_result.page_count,
        initial_zoom,
        default_page_width,
        default_page_height,
    });

    OpenDocumentPipelineResult {
        opened: true,
        path: Some(path),
        page_count: open_result.page_count,
        current_page: session.current_page,
        current_zoom: session.current_zoom,
    }
}

pub async fn open_document_pipeline(
    request: OpenDocumentPipelineRequest,
) -> Result<OpenDocumentPipelineResult, JsValue> {
    let open_result = open_pdf_file(request.path.clone()).await?;
    Ok(open_session_from_file_result(
        request.path,
        open_result,
        request.initial_zoom,
        request.default_page_width,
        request.default_page_height,
    ))
}

pub async fn pick_document_pipeline(
    request: PickDocumentPipelineRequest,
) -> Result<JsValue, JsValue> {
    let Some(path) = pick_pdf_file().await? else {
        return Ok(JsValue::NULL);
    };

    let result = open_document_pipeline(OpenDocumentPipelineRequest {
        path,
        initial_zoom: request.initial_zoom,
        default_page_width: request.default_page_width,
        default_page_height: request.default_page_height,
    })
    .await?;

    serde_wasm_bindgen::to_value(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}

pub fn close_document_pipeline(
    default_page_width: f32,
    default_page_height: f32,
) -> CloseDocumentPipelineResult {
    let result: HostActionResult =
        reset_host_document_session(default_page_width, default_page_height);
    CloseDocumentPipelineResult {
        closed: result.changed,
        current_page: result.current_page,
        current_zoom: result.current_zoom,
    }
}

pub async fn rotate_document_pipeline(delta: i32) -> Result<RotateDocumentPipelineResult, JsValue> {
    let result: RotateCurrentPageResult = rotate_current_page(delta).await?;
    Ok(RotateDocumentPipelineResult {
        rotated: result.rotated,
    })
}

pub fn undo_document_pipeline() -> DocumentMutationPipelineResult {
    DocumentMutationPipelineResult {
        changed: undo_document_edit(),
    }
}

pub fn redo_document_pipeline() -> DocumentMutationPipelineResult {
    DocumentMutationPipelineResult {
        changed: redo_document_edit(),
    }
}
