use pdf_viewer_core::models::LayoutAlignment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphRegionSnapshot {
    pub region_id: String,
    pub kind: String,
    pub text: String,
    // Add other fields from core::models if needed
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistableRegionPatch {
    pub patch_key: String,
    pub page_index: u16,
    pub region_id: String,
    pub original_text: String,
    pub new_text: String,
    pub new_runs: Option<Vec<pdf_viewer_core::models::LayoutRun>>,
    pub source: String,
    pub marker_text: Option<String>,
    pub new_marker_text: Option<String>,
    pub snapshot: Option<serde_json::Value>,
    pub kind: Option<String>,
    pub pair_id: Option<String>,
    pub group_id: Option<String>,
    pub field_kind: Option<String>,
    pub field_name: Option<String>,
    pub original_value_text: Option<String>,
    pub new_value_text: Option<String>,
    #[serde(default)]
    pub target_indices: Vec<usize>,
    #[serde(default)]
    pub full_target_indices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub displacement_y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<LayoutAlignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LayoutInferenceResult {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct LayoutEditorSession {
    pub paragraph: pdf_viewer_core::models::LayoutParagraph,
    // Add other session state
}

impl Default for LayoutEditorSession {
    fn default() -> Self {
        Self {
            paragraph: pdf_viewer_core::models::LayoutParagraph::default(),
        }
    }
}
