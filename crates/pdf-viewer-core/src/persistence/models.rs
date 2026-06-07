use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistableRegionPatch {
    pub patch_key: String,
    pub page_index: u16,
    pub region_id: String,
    pub original_text: String,
    pub new_text: String,
    pub new_runs: Option<Vec<crate::models::LayoutRun>>,
    pub source: String, // 'paragraph-region', 'list-item-region', 'field-row'
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_marker_text: Option<String>,

    pub snapshot: Option<serde_json::Value>,
    pub kind: Option<String>,
    pub pair_id: Option<String>,
    pub group_id: Option<String>,
    pub field_kind: Option<String>,
    pub field_name: Option<String>,
    pub original_value_text: Option<String>,
    pub new_value_text: Option<String>,
    pub target_indices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub full_target_indices: Vec<usize>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub displacement_y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<crate::models::LayoutAlignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
    #[serde(default)]
    pub char_spacing: f32,
    #[serde(default = "default_scale_x_model")]
    pub horizontal_scaling: f32,
}

fn default_scale_x_model() -> f32 {
    100.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionTextReflow {
    pub page_index: u16,
    pub target_indices: Vec<usize>,
    pub new_text: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistableSavePlan {
    pub region_patches: Vec<PersistableRegionPatch>,
    pub text_reflows: Vec<RegionTextReflow>,
    pub suppressed_text_reflows: Vec<RegionTextReflow>,
    pub covered_field_row_object_ids: HashSet<String>,
    pub covered_paragraph_object_ids: HashSet<String>,
}
