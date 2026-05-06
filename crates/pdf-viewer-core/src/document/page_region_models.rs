use crate::models::{EditableSegment, FontHints, NativeTextModel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphRegionSnapshotLine {
    pub line_index: usize,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub font_name: String,
    pub font_size: f32,
    pub color: String,
    pub is_bold: bool,
    pub is_italic: bool,
    #[serde(default)]
    pub is_underline: bool,
    pub font_hints: Option<FontHints>,
    pub render_mode: Option<i64>,
    pub object_ids: Vec<String>,
    pub object_indices: Vec<usize>,
    pub width: f32,
    pub char_origins: Vec<f32>,
    pub char_widths: Vec<f32>,
    pub rendered_text: String,
    pub marker_text: Option<String>,
    pub marker_char_len: Option<usize>,
    pub body_char_start: Option<usize>,
    pub body_text: Option<String>,
    pub body_left: Option<f32>,
    pub marker_runs: Option<Vec<StyleRunSnapshot>>,
    pub style_runs: Vec<StyleRunSnapshot>,
    pub char_spacing: f32,
    pub scale_x: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphRegionSnapshot {
    pub region_id: String,
    pub kind: String,
    pub text: String,
    pub lines: Vec<ParagraphRegionSnapshotLine>,
    pub style_runs: Vec<StyleRunSnapshot>,
    pub object_ids: Vec<String>,
    pub object_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldGroupSnapshot {
    pub group_id: String,
    pub pair_id: String,
    pub key_text: String,
    pub value_text: String,
    pub key_runs: Vec<StyleRunSnapshot>,
    pub value_runs: Vec<StyleRunSnapshot>,
    pub key_box: BoundingBoxOutput,
    pub value_box: BoundingBoxOutput,
    pub object_ids: Vec<String>,
    pub object_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBoxOutput {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphProjectionOutput {
    pub region_id: String,
    pub kind: String,
    pub region_box: BoundingBoxOutput,
    pub line_boxes: Vec<ParagraphLineProjectionOutput>,
    pub tight_line_boxes: Vec<ParagraphLineProjectionOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphLineProjectionOutput {
    pub line_index: usize,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub baseline_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldGroupProjectionOutput {
    pub text_box: BoundingBoxOutput,
    pub shell_box: BoundingBoxOutput,
    pub label_box: BoundingBoxOutput,
    pub value_box: BoundingBoxOutput,
    pub editor_box: BoundingBoxOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StyleSource {
    pub font_name: String,
    pub font_size: f32,
    pub color: String,
    pub is_bold: bool,
    pub is_italic: bool,
    #[serde(default)]
    pub is_underline: bool,
    pub font_hints: Option<FontHints>,
    pub render_mode: i64,
    #[serde(default)]
    pub char_spacing: f32,
    #[serde(default = "default_scale_x_persistence")]
    pub scale_x: f32,
}

fn default_scale_x_persistence() -> f32 { 100.0 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StyleRunSnapshot {
    pub id: String,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub style: StyleSource,
    pub width: f32,
    pub char_origins: Vec<f32>,
    pub char_widths: Vec<f32>,
    #[serde(default)]
    pub object_ids: Vec<String>,
    #[serde(default)]
    pub object_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphLineOutput {
    pub line_index: usize,
    pub text: String,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub font_name: String,
    pub font_size: f32,
    pub color: String,
    pub is_bold: bool,
    pub is_italic: bool,
    #[serde(default)]
    pub is_underline: bool,
    pub font_hints: Option<FontHints>,
    pub render_mode: i64,
    pub object_ids: Vec<String>,
    pub object_indices: Vec<usize>,
    pub width: f32,
    pub char_origins: Vec<f32>,
    pub char_widths: Vec<f32>,
    pub style_runs: Vec<StyleRunSnapshot>,
    pub char_spacing: f32,
    pub scale_x: f32,
    pub projection: ParagraphProjectionOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphRegionOutput {
    pub kind: String,
    pub id: String,
    pub page_index: u16,
    pub line_index_start: usize,
    pub line_index_end: usize,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub text: String,
    pub lines: Vec<ParagraphLineOutput>,
    pub object_ids: Vec<String>,
    pub object_indices: Vec<usize>,
    pub width: f32,
    pub char_origins: Vec<f32>,
    pub char_widths: Vec<f32>,
    pub wrap_width: f32,
    pub char_spacing: f32,
    pub scale_x: f32,
    pub projection: ParagraphProjectionOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListItemRegionOutput {
    pub kind: String,
    pub id: String,
    pub wrap_width: f32,
    pub page_index: u16,
    pub line_index: usize,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub text: String,
    pub marker_text: Option<String>,
    pub marker_char_len: Option<usize>,
    pub body_char_start: Option<usize>,
    pub body_text: Option<String>,
    pub body_left: Option<f32>,
    pub label_text: String,
    pub value_text: String,
    pub font_name: String,
    pub font_size: f32,
    pub color: String,
    pub is_bold: bool,
    pub is_italic: bool,
    pub char_spacing: f32,
    pub scale_x: f32,
    pub font_hints: Option<FontHints>,
    pub render_mode: i64,
    pub object_ids: Vec<String>,
    pub object_indices: Vec<usize>,
    pub width: f32,
    pub char_origins: Vec<f32>,
    pub char_widths: Vec<f32>,
    pub marker_runs: Option<Vec<StyleRunSnapshot>>,
    pub style_runs: Vec<StyleRunSnapshot>,
    pub projection: ParagraphProjectionOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyBox {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyValuePairOutput {
    pub id: String,
    pub field_name: String,
    pub field_kind: String,
    pub key_text: String,
    pub value_text: String,
    pub key_style: StyleSource,
    pub value_style: StyleSource,
    pub key_object_ids: Vec<String>,
    pub value_object_ids: Vec<String>,
    pub key_run_keys: Vec<String>,
    pub value_run_keys: Vec<String>,
    pub key_object_indices: Vec<usize>,
    pub value_object_indices: Vec<usize>,
    pub key_box: KeyBox,
    pub value_box: KeyBox,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldRowRegionGroupOutput {
    pub id: String,
    pub segment_key: String,
    pub object_ids: Vec<String>,
    pub object_indices: Vec<usize>,
    pub run_keys: Vec<String>,
    pub first_object_id: String,
    pub field_name: String,
    pub field_kind: String,
    pub column_index: usize,
    pub left: f32,
    pub right: f32,
    pub slot_left: f32,
    pub slot_right: f32,
    pub label_left: f32,
    pub label_right: f32,
    pub value_left: f32,
    pub value_right: f32,
    pub top: f32,
    pub bottom: f32,
    pub pair: KeyValuePairOutput,
    pub segment: EditableSegment,
    pub projection: FieldGroupProjectionOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldRowRegionOutput {
    pub id: String,
    pub page_index: u16,
    pub line_index: usize,
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub confidence: f32,
    pub semantic_reason: String,
    pub column_bands: Vec<serde_json::Value>,
    pub groups: Vec<FieldRowRegionGroupOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LineProjectionOutput {
    pub line_index: usize,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LineRegionModelOutput {
    pub id: String,
    pub kind: String,
    pub page_index: u16,
    pub line_index: usize,
    pub objects: Vec<NativeTextModel>,
    pub projection: LineProjectionOutput,
    pub field_row: Option<FieldRowRegionOutput>,
    pub paragraph_region: Option<ParagraphRegionOutput>,
    pub list_item_region: Option<ListItemRegionOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PageRegionContextOutput {
    pub scene_hint: String,
    pub text_objects: Vec<NativeTextModel>,
    pub visual_lines: Vec<Vec<NativeTextModel>>,
    pub line_regions: Vec<LineRegionModelOutput>,
    pub field_rows: Vec<FieldRowRegionOutput>,
    pub paragraph_regions: Vec<ParagraphRegionOutput>,
    pub list_item_regions: Vec<ListItemRegionOutput>,
}
