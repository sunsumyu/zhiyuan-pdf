use serde::{Deserialize, Serialize};
use serde_json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "targetInvoke")]
    async fn target_invoke(cmd: String, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_name = "onDebug")]
    fn on_debug(label: String, payload: String);
}

pub const TRACE_KEY_ORDER: &[&str] = &[
    "pageIndex",
    "traceId",
    "regionId",
    "regionKind",
    "lineIndex",
    "textPreview",
    "currentTextPreview",
    "lineCount",
    "fontFamily",
    "fontSizePx",
    "fontWeight",
    "fontStyle",
    "color",
    "lineHeightPx",
    "baselineOffsetPx",
    "domToPdfScaleX",
    "domToPdfScaleY",
    "pdfRegionBounds",
    "pdfRegionRect",
    "projectionPdfRect",
    "projectionDeltaFromRegion",
    "anchorPdfRect",
    "anchorDeltaFromRegion",
    "blockStylePdfRect",
    "blockStyleDeltaFromAnchor",
    "blockDomPdfRect",
    "blockDomDeltaFromAnchor",
    "sourcePdfBounds",
    "sourceProjectedLineRect",
    "interactionRect",
    "interactionLayerClientRect",
    "expectedInteractionRect",
    "shellClientRect",
    "shellLocalRect",
    "shellDeltaFromExpected",
    "textareaClientRect",
    "textareaLocalRect",
    "textareaDeltaFromExpected",
    "hitLocalX",
    "zoom",
];

pub struct PdfLogger;

#[allow(dead_code)]
impl PdfLogger {
    pub fn info(msg: String) {
        on_debug("INFO".to_string(), msg);
    }

    pub fn debug(msg: String) {
        on_debug("DEBUG".to_string(), msg);
    }

    pub fn error(msg: String) {
        on_debug("ERROR".to_string(), msg);
    }

    pub fn trace(label: &str, payload: &serde_json::Value) {
        let msg = format_structured_trace(label, payload);
        on_debug("TRACE".to_string(), msg);
    }
}

fn format_structured_trace(label: &str, payload: &serde_json::Value) -> String {
    if !payload.is_object() {
        return format!("{}: {}", label, payload);
    }

    let obj = payload.as_object().unwrap();
    let mut keys: Vec<String> = obj.keys().cloned().collect();

    // 物理排序逻辑
    keys.sort_by(|a: &String, b: &String| {
        let pos_a = TRACE_KEY_ORDER
            .iter()
            .position(|&k| k == a)
            .unwrap_or(usize::MAX);
        let pos_b = TRACE_KEY_ORDER
            .iter()
            .position(|&k| k == b)
            .unwrap_or(usize::MAX);
        pos_a.cmp(&pos_b).then_with(|| a.cmp(b))
    });

    let max_key_len = keys.iter().map(|k: &String| k.len()).max().unwrap_or(8);
    let mut lines = Vec::new();
    lines.push(label.to_string());

    for key in keys {
        let value = format_value(&obj[&key]);
        lines.push(format!("  {:width$} : {}", key, value, width = max_key_len));
    }

    lines.join("\n")
}

fn format_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => v.to_string(),
    }
}

#[allow(dead_code)]
pub async fn smart_invoke<T>(cmd: &str, args: impl Serialize) -> Result<T, JsValue>
where
    T: for<'de> Deserialize<'de>,
{
    let args_js = serde_wasm_bindgen::to_value(&args)?;
    let result_js = target_invoke(cmd.to_string(), args_js).await;

    match serde_wasm_bindgen::from_value::<T>(result_js) {
        Ok(val) => Ok(val),
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}
