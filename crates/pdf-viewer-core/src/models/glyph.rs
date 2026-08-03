use serde::{Deserialize, Serialize};

use super::font::ResolvedFontFace;
use super::geometry::BoundingBox;
use super::layout::{
    EditorSession, GlyphPosition, LayoutMode, LayoutRole, PaintMode, ParagraphEditContext,
    ParagraphStyle, RunStyle, SemanticRole, TextRun,
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

impl GlyphPaintRun {
    /// 转换为使用绝对坐标的 TextRun
    /// char_origins 是相对于 origin_x 的偏移，转换为 GlyphPosition 的绝对坐标
    pub fn to_text_run(&self) -> TextRun {
        let glyphs: Vec<GlyphPosition> = if self.char_origins.is_empty() {
            // 没有 char_origins 时，用 bbox 宽度推导一个合成 glyph
            let run_width = self.bbox.right - self.bbox.left;
            vec![GlyphPosition::new(self.origin_x, run_width.max(1.0))]
        } else {
            self.char_origins
                .iter()
                .enumerate()
                .map(|(i, origin)| {
                    let absolute_x = self.origin_x + *origin;
                    let width = if i + 1 < self.char_origins.len() {
                        self.char_origins[i + 1] - *origin
                    } else {
                        self.font_size * 0.5
                    };
                    GlyphPosition::new(absolute_x, width)
                })
                .collect()
        };

        TextRun {
            id: self.id.clone(),
            text: self.text.clone(),
            origin_x: self.origin_x,
            baseline_y: self.origin_y,
            glyphs,
            style: RunStyle {
                font_name: self.resolved_font.render_family.clone(),
                font_size: self.font_size,
                color: self.color.clone(),
                is_bold: self.is_bold,
                is_italic: self.is_italic,
                is_underline: self.is_underline,
                char_spacing: 0.0,
                scale_x: self.scale_x,
                font_weight_numeric: self.resolved_font.identity.weight as u16,
            },
            object_ids: self.object_ids.clone(),
        }
    }
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
    /// 旧版编辑器会话（使用 LayoutRun），逐步迁移到 editor_session_v2
    pub editor_session: ParagraphEditContext,
    pub control_style: EditorControlStyle,
    #[serde(default)]
    pub semantic_role: SemanticRole,
    #[serde(default)]
    pub runs: Vec<GlyphPaintRun>,
    /// 新版编辑器会话（使用 TextRun 绝对坐标）
    /// 当此字段为 Some 时，优先使用；否则回退到 editor_session
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_session_v2: Option<EditorSession>,
}

impl GlyphPaintParagraph {
    /// 获取编辑器会话（优先 v2，回退 v1）
    pub fn editor_session(&self) -> &ParagraphEditContext {
        // v2 存在时仍返回 v1，因为下游代码仍依赖 ParagraphEditContext
        // TODO: Phase 7 完成后改为返回 &EditorSession
        &self.editor_session
    }
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
                // Paragraph session (v1)
                paragraph.editor_session.anchor_bbox.flip_y(h);
                paragraph.editor_session.paragraph.bbox.flip_y(h);
                for run in &mut paragraph.editor_session.paragraph.runs {
                    run.origin_y = h - run.origin_y;
                    run.bbox.flip_y(h);
                }
                // Paragraph session (v2 - absolute coordinates)
                if let Some(session) = &mut paragraph.editor_session_v2 {
                    session.anchor_bbox.flip_y(h);
                    session.paragraph.flip_y(h);
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
