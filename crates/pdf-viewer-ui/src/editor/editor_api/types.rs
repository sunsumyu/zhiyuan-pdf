use serde::Deserialize;
use wasm_bindgen::prelude::*;

// ── Incoming request DTOs (JS → Rust) ───────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HitTestRequest {
    pub client_x: f32,
    pub client_y: f32,
    pub reference_left: f32,
    pub reference_top: f32,
    pub reference_width: f32,
    pub reference_height: f32,
    pub page_width: f32,
    pub page_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenBlockRequest {
    pub block_id: String,
    pub client_x: f32,
    pub client_y: f32,
    pub reference_left: f32,
    pub reference_top: f32,
    pub reference_width: f32,
    pub reference_height: f32,
    pub page_width: f32,
    pub page_height: f32,
    #[serde(default)]
    pub fallback_page_x: f32,
    #[serde(default)]
    pub fallback_page_y: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoveCaretRequest {
    pub client_x: f32,
    pub client_y: f32,
    pub reference_left: f32,
    pub reference_top: f32,
    pub reference_width: f32,
    pub reference_height: f32,
    pub page_width: f32,
    pub page_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitRequest {
    pub draft_text: String,
    pub caret_index: u32,
}

// ── EditorSession ───────────────────────────────────────────────

#[wasm_bindgen]
pub struct EditorSession;

impl Default for EditorSession {
    fn default() -> Self {
        Self::new()
    }
}
