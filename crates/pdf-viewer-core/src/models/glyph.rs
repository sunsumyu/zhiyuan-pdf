use serde::{Deserialize, Serialize};

use super::font::ResolvedFontFace;
use super::geometry::BoundingBox;
use super::layout::{
    LayoutMode, LayoutRole, PaintMode, ParagraphEditContext, ParagraphStyle, SemanticRole,
};

fn default_scale_x() -> f32 {
    1.0
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlyphPaintRun {
    pub id: String,
    pub page_index: u16,
    pub region_id: String,
    pub paragraph_id: String,
    pub text: String,
    pub bbox: BoundingBox,
    pub origin_x: f32,
    pub origin_y: f32,
    #[serde(default)]
    pub char_origins: Vec<f32>,
    pub color: String,
    pub resolved_font: ResolvedFontFace,
    pub font_size: f32,
    #[serde(default = "default_scale_x")]
    pub scale_x: f32,
    pub is_bold: bool,
    pub is_italic: bool,
    #[serde(default)]
    pub is_underline: bool,
    pub paint_mode: PaintMode,
    #[serde(default)]
    pub object_ids: Vec<String>,
    #[serde(default)]
    pub object_indices: Vec<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorControlStyle {
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: String,
    pub font_style: String,
    pub color: String,
    #[serde(default)]
    pub text_decoration: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlyphPaintParagraph {
    pub id: String,
    pub region_id: String,
    pub bbox: BoundingBox,
    pub style: ParagraphStyle,
    pub editor_session: ParagraphEditContext,
    pub control_style: EditorControlStyle,
    #[serde(default)]
    pub semantic_role: SemanticRole,
    #[serde(default)]
    pub runs: Vec<GlyphPaintRun>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExternalObject {
    Image {
        id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        z_index: i32,
    },
    Path {
        id: String,
        stroke_width: f32,
        stroke_color: Option<String>,
        fill_color: Option<String>,
        z_index: i32,
        commands: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlyphPaintRegion {
    pub id: String,
    pub kind: LayoutRole,
    pub layout_mode: LayoutMode,
    pub bbox: BoundingBox,
    #[serde(default)]
    pub paragraphs: Vec<GlyphPaintParagraph>,
    #[serde(default)]
    pub object_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlyphPaintPlan {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub regions: Vec<GlyphPaintRegion>,
    #[serde(default)]
    pub external_objects: Vec<ExternalObject>,
}

impl GlyphPaintPlan {
    pub fn flip_y(&mut self) {
        let h = self.height;
        for region in &mut self.regions {
            region.bbox.flip_y(h);
            for paragraph in &mut region.paragraphs {
                paragraph.bbox.flip_y(h);
                // Paragraph session
                paragraph.editor_session.anchor_bbox.flip_y(h);
                paragraph.editor_session.paragraph.bbox.flip_y(h);
                for run in &mut paragraph.editor_session.paragraph.runs {
                    run.origin_y = h - run.origin_y;
                    run.bbox.flip_y(h);
                }
                // Paragraph render runs
                for run in &mut paragraph.runs {
                    run.origin_y = h - run.origin_y;
                    run.bbox.flip_y(h);
                }
            }
        }
        for ext in &mut self.external_objects {
            match ext {
                ExternalObject::Image { y, height, .. } => {
                    *y = h - (*y + *height);
                }
                _ => {} // Path commands are complex, usually use separate transform
            }
        }
    }
}
