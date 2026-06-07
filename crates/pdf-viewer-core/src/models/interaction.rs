use super::{BoundingBox, EditorControlStyle, GlyphPaintRun, ParagraphEditContext};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RectBox {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldProjection {
    pub text_box: RectBox,
    pub shell_box: RectBox,
    pub label_box: RectBox,
    pub value_box: RectBox,
    pub editor_box: RectBox,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldProjectionRequest {
    pub page_height: f32,
    pub group_left: f32,
    pub group_right: f32,
    pub slot_left: f32,
    pub slot_right: f32,
    pub label_left: f32,
    pub label_right: f32,
    pub value_left: f32,
    pub value_right: f32,
    pub top: f32,
    pub bottom: f32,
    #[serde(default)]
    pub has_field_meta: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum FieldPartKind {
    Key,
    #[default]
    Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldHitRequest {
    pub projection: FieldProjection,
    pub editable_key_text: String,
    pub editable_value_text: String,
    pub click_page_x: f32,
    #[serde(default)]
    pub key_session: Option<ParagraphEditContext>,
    #[serde(default)]
    pub value_session: Option<ParagraphEditContext>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldHitResolution {
    pub active_part: FieldPartKind,
    pub initial_caret_index: usize,
    pub measured_key_width: f32,
    pub measured_value_width: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldHitTarget {
    pub projection: FieldProjection,
    pub editable_key_text: String,
    pub editable_value_text: String,
    #[serde(default)]
    pub key_session: Option<ParagraphEditContext>,
    #[serde(default)]
    pub value_session: Option<ParagraphEditContext>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldHitBatchRequest {
    pub targets: Vec<FieldHitTarget>,
    pub click_page_x: f32,
    pub click_page_y: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldHitMatch {
    pub target_index: usize,
    pub resolution: FieldHitResolution,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldEditorParamsRequest {
    pub runs: Vec<GlyphPaintRun>,
    pub anchor_bbox: BoundingBox,
    pub paragraph_id: String,
    #[serde(default)]
    pub line_height: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldEditorParams {
    #[serde(default)]
    pub session: Option<ParagraphEditContext>,
    #[serde(default)]
    pub control_style: Option<EditorControlStyle>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct InteractionProjection {
    pub region_box: BoundingBox,
    #[serde(default)]
    pub line_boxes: Vec<RectBox>,
    pub label_box: Option<RectBox>,
    pub value_box: Option<RectBox>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InteractionTarget {
    pub kind: String,
    pub region_id: String,
    pub object_id: String,
    pub text: String,
    pub projection: InteractionProjection,
    pub object_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldEditorProjection {
    pub pixel_rect: RectBox,
    pub scale_x: f32,
    pub font_size: f32,
    pub render_family: String,
    pub color: String,
}
