pub use pdf_viewer_core::models::{
    document_runtime::*,
    font::*,
    geometry::*,
    glyph::*,
    interaction::*,
    layout::*,
    styled_run::{NativePageModel, NativePageObject, StyledRun},
    vector::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use pdf_viewer_core::models::NativeTextModel;

fn is_false(v: &bool) -> bool {
    !*v
}
fn default_alpha() -> f32 {
    1.0
}
fn is_default_alpha(v: &f32) -> bool {
    (v - 1.0).abs() < 0.001
}

// LayoutRole, LayoutAlignment, LayoutMode moved to pdf-viewer-core

// BoundingBox moved to pdf-viewer-core

// LayoutRun, LayoutParagraph, SemanticRegion, etc. moved to pdf-viewer-core

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageModel {
    pub page_index: u16,
    pub width: f32,         // Fixed at 1000.0 in V17 for reference
    pub height: f32,        // Calculated based on aspect ratio in V17
    pub native_width: f32,  // V18: True physical pixel width of the rendered bitmap
    pub native_height: f32, // V18: True physical pixel height of the rendered bitmap
    pub paragraphs: Vec<NativeTextModel>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageTextInfo {
    pub index: usize,
    pub text: String,
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub font_name: String,
    pub color: String,
    pub clear_indices: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextObjectInfo {
    pub index: usize,
    pub text: String,
    pub rect: [f32; 4],
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextReflowPatch {
    pub page_index: u16,
    pub target_indices: Vec<usize>,
    pub new_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_runs: Option<Vec<LayoutRun>>, // V3: Multi-styled runs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<LayoutAlignment>, // V3: Paragraph alignment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>, // V3: Paragraph line height
    #[serde(skip_serializing_if = "Option::is_none")]
    pub displacement_y: Option<f32>, // V265: Reflow shift
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap_width: Option<f32>, // V267: Dynamic wrapping width
    #[serde(default)]
    pub char_spacing: f32,
    #[serde(default = "default_scale_x")]
    pub horizontal_scaling: f32,
}
fn default_scale_x() -> f32 {
    100.0
}
pub use pdf_viewer_core::persistence::models::PersistableRegionPatch;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PdfMaterializationDecisionReport {
    pub region_id: String,
    pub source: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PdfMaterializationSourceStats {
    pub source: String,
    pub materialized: usize,
    pub skipped: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PdfMaterializationReport {
    pub path: String,
    pub region_patch_count: usize,
    pub explicit_text_reflow_count: usize,
    pub effective_text_reflow_count: usize,
    pub materialized_count: usize,
    pub skipped_count: usize,
    pub by_source: Vec<PdfMaterializationSourceStats>,
    pub decisions: Vec<PdfMaterializationDecisionReport>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfModifications {
    pub rotations: HashMap<u16, i32>,
    #[serde(default)]
    pub region_patches: Vec<PersistableRegionPatch>,
    #[serde(default)]
    pub text_reflows: Vec<TextReflowPatch>,
    #[serde(default)]
    pub text_patches: Vec<TextPatch>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PathSegment {
    pub command: String, // "move", "line", "bezier", "close"
    pub points: Vec<[f32; 2]>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NativePathModel {
    pub id: String,
    pub segments: Vec<PathSegment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub fill: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stroke: bool,
    pub stroke_width: f32, // Width is usually unique, don't skip
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub line_cap: u8,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub line_join: u8,
    pub miter_limit: f32,
    #[serde(default = "default_alpha", skip_serializing_if = "is_default_alpha")]
    pub alpha: f32,
    pub z_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color_index: Option<u8>, // V197: Palette Index
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_color_index: Option<u8>, // V197: Palette Index
}

impl NativePathModel {
    pub fn flip_y(&mut self, h: f32) {
        for seg in &mut self.segments {
            for p in &mut seg.points {
                p[1] = h - p[1];
            }
        }
    }
}

impl Default for NativePathModel {
    fn default() -> Self {
        Self {
            id: String::new(),
            segments: Vec::new(),
            fill_color: None,
            stroke_color: None,
            fill: false,
            stroke: false,
            stroke_width: 0.0,
            line_cap: 0,
            line_join: 0,
            miter_limit: 0.0,
            alpha: default_alpha(),
            z_index: 0,
            fill_color_index: None,
            stroke_color_index: None,
        }
    }
}
fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NativeImageModel {
    pub id: String,
    pub data_url: String, // Base64 for pure canvas
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    // V206.32 Matrix Components
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
    pub z_index: usize,
    pub extraction_method: String, // V206.32: "Standard" or "Fallback"
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorPalette {
    pub colors: Vec<String>,
    pub fonts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextPatch {
    pub id: String,
    pub old_text: String,
    pub new_text: String,
    pub offset_x: Option<f32>, // V226: Horizontal shift for reflow
    pub target_index: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RenderObject {
    Text(NativeTextModel),
    Path(NativePathModel),
    Image(NativeImageModel),
}

#[derive(Clone)]
pub struct PageDisplayList {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    pub objects: Vec<RenderObject>,
    pub text_runs: Vec<StyledRun>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NativeVectorPageModel {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    pub objects: Vec<RenderObject>,
    pub palette: VectorPalette,
    pub background_image: Option<String>,
}

impl NativeVectorPageModel {
    pub fn flip_y(&mut self) {
        let h = self.height;
        for obj in &mut self.objects {
            match obj {
                RenderObject::Text(t) => t.flip_y(h),
                RenderObject::Path(p) => p.flip_y(h),
                RenderObject::Image(img) => {
                    img.y = h - img.y - img.height;
                    img.f = h - img.f;
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LightPageKind {
    Pending,
    Scanned,
    Mixed,
    Text,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LightPageModel {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    pub kind: LightPageKind,
    pub preview_image_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
    pub page_count: usize,
}
