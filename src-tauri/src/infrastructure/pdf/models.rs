pub use pdf_viewer_core::models::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct EmbeddedGlyphMap {
pub identity: bool,
pub cid_to_gid: HashMap<u32, u16>,
}

// FontHints and StyledRun moved to pdf-viewer-core

// StyledRun moved to pdf-viewer-core

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NativeTextModel {
pub id: String,
pub text: String,
pub left: f32,
pub top: f32,
pub baseline_y: f32,
pub width: f32,
pub height: f32,
pub font_size: f32,
    #[serde(skip_serializing_if = "String::is_empty")]
pub font_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
pub stroke_color: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
pub stroke_width: f32,
    // V31.60 Matrix Components (Tm)
    #[serde(default = "default_scale", skip_serializing_if = "is_default_scale")]
pub scale_x: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
pub shear_x: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
pub shear_y: f32,
    #[serde(default = "default_scale", skip_serializing_if = "is_default_scale")]
pub scale_y: f32,
pub tx: f32,
pub ty: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
pub letter_spacing: f32,
    #[serde(default)]
pub char_spacing: f32,
    #[serde(default = "default_scale_x")]
pub horizontal_scaling: f32,
    #[serde(default, skip_serializing_if = "is_false")]
pub is_faux_bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
pub is_serif: bool,
    #[serde(default, skip_serializing_if = "is_false")]
pub is_italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
pub is_bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
pub is_underline: bool,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
pub rendering_mode: i32,
    #[serde(default = "default_alpha", skip_serializing_if = "is_default_alpha")]
pub alpha: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
pub glyph_bounds: Option<Vec<[f32; 4]>>, // V8: For frontend interaction layer (x, y, width, height)
pub runs: Vec<StyledRun>,
pub object_indices: Vec<usize>,
pub z_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
pub font_hints: Option<FontHints>,
    #[serde(skip_serializing_if = "Option::is_none")]
pub font_post_script_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
pub font_family_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
pub font_subtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
pub embedded_font_key: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
pub has_embedded_font_program: bool,
    #[serde(default, skip_serializing_if = "is_false")]
pub has_to_unicode_cmap: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
pub paragraph_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
pub wrap_width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
pub min_tx: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
pub color_index: Option<u8>, // V197: Palette Index
    #[serde(skip_serializing_if = "Option::is_none")]
pub font_index: Option<u8>, // V197: Palette Index
    #[serde(skip_serializing_if = "Option::is_none")]
pub role: Option<LayoutRole>, // V263: Logical Role
    #[serde(skip_serializing_if = "Option::is_none")]
pub alignment: Option<LayoutAlignment>, // V263: Visual Alignment
    #[serde(skip_serializing_if = "Option::is_none")]
pub indent: Option<f32>, // V263: Paragraph Indentation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
pub char_origins: Vec<[f32; 2]>, // V311: Character-level positions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
pub char_widths: Vec<f32>, // V311: Character-level widths
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
pub pdf_char_codes: Vec<u32>, // 鍘熷 PDF charcode/CID 搴忓垪
}

impl NativeTextModel {
    pub fn flip_y(&mut self, h: f32) {
        self.ty = h - self.ty;
        self.baseline_y = h - self.baseline_y;
        self.top = h - self.top - self.height;
        for origin in &mut self.char_origins {
            origin[1] = h - origin[1];
        }
        for run in &mut self.runs {
            run.ty = h - run.ty;
        }
    }
}

impl Default for NativeTextModel {
fn default() -> Self {
        Self {
            id: String::new(),
            text: String::new(),
            left: 0.0,
            top: 0.0,
            baseline_y: 0.0,
            width: 0.0,
            height: 0.0,
            font_size: 0.0,
            font_name: String::new(),
            color: String::new(),
            stroke_color: None,
            stroke_width: 0.0,
            scale_x: default_scale(),
            shear_x: 0.0,
            shear_y: 0.0,
            scale_y: default_scale(),
            tx: 0.0,
            ty: 0.0,
            letter_spacing: 0.0,
            char_spacing: 0.0,
            horizontal_scaling: default_scale_x(),
            is_faux_bold: false,
            is_serif: false,
            is_italic: false,
            is_bold: false,
            is_underline: false,
            rendering_mode: 0,
            alpha: default_alpha(),
            glyph_bounds: None,
            runs: Vec::new(),
            object_indices: Vec::new(),
            z_index: 0,
            font_hints: None,
            font_post_script_name: None,
            font_family_hint: None,
            font_subtype: None,
            embedded_font_key: None,
            has_embedded_font_program: false,
            has_to_unicode_cmap: false,
            paragraph_id: None,
            wrap_width: None,
            min_tx: None,
            color_index: None,
            font_index: None,
            role: None,
            alignment: None,
            indent: None,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            pdf_char_codes: Vec::new(),
        }
    }
}
fn is_zero(v: &f32) -> bool {
    v.abs() < 0.001
}
fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}
fn is_false(v: &bool) -> bool {
    !*v
}
fn default_scale() -> f32 {
    1.0
}
fn is_default_scale(v: &f32) -> bool {
    (v - 1.0).abs() < 0.001
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
pub use pdf_viewer_core::persistence_models::PersistableRegionPatch;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VectorPageModel {
pub page_index: u16,
pub width: f32,
pub height: f32,
pub objects: Vec<RenderObject>,
pub palette: VectorPalette,
pub background_image: Option<String>,
}

impl VectorPageModel {
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
