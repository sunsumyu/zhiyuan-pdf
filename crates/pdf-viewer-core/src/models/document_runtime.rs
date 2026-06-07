use super::{GlyphPaintPlan, LayoutInferenceResult, VectorPageModel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PageState {
    pub zoom: f32,
    pub dpr: f32,
    pub viewport_left: f32,
    pub viewport_top: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub paint_plan: Option<GlyphPaintPlan>,
    pub vector_model: Option<VectorPageModel>,
    pub inference: Option<LayoutInferenceResult>,
    pub path: Option<String>,
    pub current_page: u16,
    pub total_pages: u16,
    pub anchor_pdf_x: f32,
    pub anchor_pdf_y: f32,
    pub anchor_viewport_x: f32,
    pub anchor_viewport_y: f32,
    pub page_dimensions: Vec<(f32, f32)>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct BaseEditIntent {
    pub selection_start: usize,
    pub selection_end: usize,
    pub new_text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EditIntent {
    Paragraph(BaseEditIntent),
    Field {
        #[serde(flatten)]
        base: BaseEditIntent,
        active_part: String,
        new_key_text: String,
        new_value_text: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LightPageKind {
    #[default]
    Pending,
    Scanned,
    Mixed,
    Text,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LightPageModel {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    pub kind: LightPageKind,
    pub preview_image_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PdfDocumentKind {
    #[default]
    Unknown,
    Scanned,
    Mixed,
    Vector,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ClassificationReason {
    FullPageImageNoText,
    FullPageImageWithOcrLayer,
    TextOperatorsDominant,
    FontResourcesDominant,
    LowConfidenceFallback,
    #[default]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReadDocumentMeta {
    pub doc_id: String,
    pub path: String,
    pub page_count: usize,
    pub kind: PdfDocumentKind,
    pub confidence: f32,
    pub allow_scan_preview_first_paint: bool,
    pub classification_reason: ClassificationReason,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaginationAction {
    Prefetch,
    Release,
    Upgrade,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaginationCommand {
    pub action: PaginationAction,
    pub page_index: usize,
    pub path: String,
    pub zoom: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeletePageCommand {
    pub page_num: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RotatePageCommand {
    pub page_num: u32,
    pub delta: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct InsertPageCommand {
    pub at_index: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddHighlightCommand {
    pub page_num: u32,
    pub rect: [f32; 4],
    pub color: [f32; 3],
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadataCommand {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
}
