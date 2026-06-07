use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::bridge::target_invoke;
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
    let page_count = target_invoke(
        "open_pdf".into(),
        serde_wasm_bindgen::to_value(&serde_json::json!({ "path": path })).unwrap_or(JsValue::NULL),
    )
    .await?;

    let page_count = serde_wasm_bindgen::from_value::<u16>(page_count).unwrap_or(0);
    Ok(OpenPdfFileResult {
        page_count,
        opened: page_count > 0,
    })
}

pub async fn pick_pdf_file() -> Result<Option<String>, JsValue> {
    let picked = target_invoke(
        "pick_file".into(),
        serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap_or(JsValue::NULL),
    )
    .await?;

    Ok(serde_wasm_bindgen::from_value::<Option<String>>(picked).unwrap_or(None))
}

pub async fn rotate_current_page(delta: i32) -> Result<RotateCurrentPageResult, JsValue> {
    let session = read_session();
    let Some(path) = session.path else {
        return Ok(RotateCurrentPageResult { rotated: false });
    };

    target_invoke(
        "save_pdf".into(),
        serde_wasm_bindgen::to_value(&serde_json::json!({
            "path": path,
            "modifications": {
                "rotations": {
                    session.current_page.to_string(): delta,
                },
                "regionPatches": [],
                "textReflows": [],
            },
        }))
        .unwrap_or(JsValue::NULL),
    )
    .await?;

    Ok(RotateCurrentPageResult { rotated: true })
}
