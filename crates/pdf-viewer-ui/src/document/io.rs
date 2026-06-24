use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::viewer::viewer_controller::read_session;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenPdfFileResult {
    pub page_count: u16,
    pub opened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RotateCurrentPageResult {
    pub rotated: bool,
}

pub async fn open_pdf_file(path: String) -> Result<OpenPdfFileResult, JsValue> {
    let page_count: u16 = crate::app_controller::smart_invoke(
        "open_pdf",
        &serde_json::json!({ "path": path }),
    )
    .await.unwrap_or(0);

    Ok(OpenPdfFileResult {
        page_count,
        opened: page_count > 0,
    })
}

pub async fn pick_pdf_file() -> Result<Option<String>, JsValue> {
    crate::app_controller::smart_invoke("pick_file", &serde_json::json!({})).await
}

pub async fn rotate_current_page(delta: i32) -> Result<RotateCurrentPageResult, JsValue> {
    let session = read_session();
    let Some(path) = session.path else {
        return Ok(RotateCurrentPageResult { rotated: false });
    };

    let _: JsValue = crate::app_controller::raw_invoke(
        "save_pdf",
        &serde_json::json!({
            "path": path,
            "modifications": {
                "rotations": {
                    session.current_page.to_string(): delta,
                },
                "regionPatches": [],
                "textReflows": [],
            },
        }),
    )
    .await?;

    Ok(RotateCurrentPageResult { rotated: true })
}
