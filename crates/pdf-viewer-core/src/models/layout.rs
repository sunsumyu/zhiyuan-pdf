use serde::{Deserialize, Serialize};

use super::font::FontHints;
use super::geometry::BoundingBox;
use super::styled_run::StyledRun;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum FieldKind {
    #[default]
    Unknown,
    LabelValue,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticRole {
    #[default]
    None,
    Title,
    Header,
    Date,
    Amount,
    Email,
    PhoneNumber,
    Contact,
    Address,
    GenericField,
    BodyText,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditableFieldGroup {
    pub label_text: String,
    pub value_text: String,
    pub value_start_index: usize,
    pub field_name: String,
    pub field_kind: FieldKind,
    pub label_start_run_index: usize,
    pub label_end_run_index: usize,
    pub value_start_run_index: usize,
    pub value_end_run_index: usize,
    #[serde(default)]
    pub semantic_role: SemanticRole,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditableSegment {
    pub key: String,
    pub object_id: String,
    pub start_run_index: usize,
    pub end_run_index: usize,
    #[serde(default)]
    pub run_indices: Vec<usize>,
    pub text: String,
    pub width: f32,
    pub tx: f32,
    pub ty: f32,
    pub font_size: f32,
    pub font_name: String,
    pub is_bold: bool,
    pub is_italic: bool,
    #[serde(default)]
    pub is_underline: bool,
    pub char_spacing: f32,
    pub scale_x: f32,
    pub color: String,
    pub font_hints: Option<FontHints>,
    #[serde(default)]
    pub object_indices: Vec<usize>,
    #[serde(default)]
    pub char_origins: Vec<f32>,
    #[serde(default)]
    pub char_widths: Vec<f32>,
    pub field_group: Option<EditableFieldGroup>,
    #[serde(default)]
    pub semantic_role: SemanticRole,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LayoutRole {
    Title,
    SectionHeader,
    KvField,
    ListItem,
    #[default]
    Paragraph,
    PageMeta,
    FixedBlock,
    AnchoredObject,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LayoutAlignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LayoutMode {
    #[default]
    Flow,
    Fixed,
    Anchored,
}

fn default_scale() -> f32 { 1.0 }

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunStyle {
    pub font_name: String,
    pub font_size: f32,
    pub color: String,
    pub is_bold: bool,
    pub is_italic: bool,
    #[serde(default)]
    pub is_underline: bool,
    #[serde(default)]
    pub char_spacing: f32,
    #[serde(default = "default_scale")]
    pub scale_x: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutRun {
    pub id: String,
    pub text: String,
    pub style: RunStyle,
    pub bbox: BoundingBox,
    pub origin_x: f32,
    pub origin_y: f32,
    #[serde(default)]
    pub char_origins: Vec<f32>,
    #[serde(default)]
    pub char_widths: Vec<f32>,
    #[serde(default)]
    pub object_ids: Vec<String>,
    #[serde(default)]
    pub object_indices: Vec<usize>,
}

impl LayoutRun {
    pub fn from_styled(run: &StyledRun) -> Self {
        Self {
            id: run.object_id.clone().unwrap_or_else(|| format!("run-{}", run.tx)),
            text: run.text.clone(),
            style: RunStyle {
                font_name: run.font_name.clone(),
                font_size: run.font_size,
                color: run.color.clone(),
                is_bold: run.is_bold,
                is_italic: run.is_italic,
                is_underline: run.is_underline,
                char_spacing: run.char_spacing,
                scale_x: run.horizontal_scaling,
            },
            bbox: BoundingBox {
                left: run.tx,
                top: run.ty - run.font_size.max(0.0),
                right: run.tx + run.width,
                bottom: run.ty,
            },
            origin_x: run.tx,
            origin_y: run.ty,
            char_origins: run.char_origins.clone(),
            char_widths: run.char_widths.clone(),
            object_ids: run.object_id.clone().map(|id| vec![id]).unwrap_or_default(),
            object_indices: vec![run.z_index],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphStyle {
    pub align: LayoutAlignment,
    pub line_height: f32,
    pub first_line_indent: f32,
    pub left_indent: f32,
    #[serde(default)]
    pub tab_stops: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutParagraph {
    pub id: String,
    pub bbox: BoundingBox,
    pub style: ParagraphStyle,
    pub runs: Vec<LayoutRun>,
    #[serde(default)]
    pub object_ids: Vec<String>,
    #[serde(default)]
    pub origin_x: f32,
    #[serde(default)]
    pub origin_y: f32,
    #[serde(default)]
    pub wrap_width: f32,
}

impl LayoutParagraph {
    pub fn flip_y(&mut self, h: f32) {
        self.bbox.flip_y(h);
        self.origin_y = h - self.origin_y;
        for run in &mut self.runs {
            run.bbox.flip_y(h);
            run.origin_y = h - run.origin_y;
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphEditContext {
    pub anchor_bbox: BoundingBox,
    pub paragraph: LayoutParagraph,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SemanticRegion {
    pub id: String,
    pub kind: LayoutRole,
    pub layout_mode: LayoutMode,
    pub bbox: BoundingBox,
    pub paragraphs: Vec<LayoutParagraph>,
    #[serde(default)]
    pub semantic_role: SemanticRole,
    #[serde(default)]
    pub object_ids: Vec<String>,
}

impl SemanticRegion {
    pub fn flip_y(&mut self, h: f32) {
        self.bbox.flip_y(h);
        for para in &mut self.paragraphs {
            para.flip_y(h);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutInferenceResult {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    pub regions: Vec<SemanticRegion>,
    pub column_bands: Vec<f32>,
}

impl LayoutInferenceResult {
    pub fn flip_y(&mut self) {
        let h = self.height;
        for region in &mut self.regions {
            region.flip_y(h);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaintMode {
    #[default]
    Fill,
    Stroke,
    FillStroke,
}
