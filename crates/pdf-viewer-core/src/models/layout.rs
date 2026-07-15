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

fn default_scale() -> f32 {
    1.0
}

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
    #[serde(default)]
    pub font_weight_numeric: u16,
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
    /// Clone and clear geometry/metadata fields for use as a style template.
    /// - preserve_char_geometry: keep char_origins and char_widths (for preserved runs).
    /// - preserve_underline: keep the underline flag (otherwise clear it).
    /// - sanitize_style: normalize scale_x to 1.0 if invalid.
    pub fn cleared_style(
        &self,
        preserve_char_geometry: bool,
        preserve_underline: bool,
        sanitize_style: bool,
    ) -> Self {
        let mut c = self.clone();
        if !preserve_char_geometry {
            c.char_origins.clear();
            c.char_widths.clear();
        }
        if !preserve_underline {
            c.style.is_underline = false;
        }
        c.object_ids.clear();
        c.object_indices.clear();
        c.origin_x = 0.0;
        c.origin_y = 0.0;
        c.bbox = BoundingBox::default();
        if sanitize_style {
            if !c.style.scale_x.is_finite() || c.style.scale_x < 0.5 || c.style.scale_x > 2.0 {
                c.style.scale_x = 1.0;
            }
        }
        c
    }

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
                    } else if i < self.char_widths.len() {
                        self.char_widths[i]
                    } else {
                        self.style.font_size * 0.5
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
            style: self.style.clone(),
            object_ids: self.object_ids.clone(),
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

    /// 转换为使用绝对坐标的 TextParagraph
    pub fn to_text_paragraph(&self) -> TextParagraph {
        TextParagraph {
            id: self.id.clone(),
            runs: self.runs.iter().map(|r| r.to_text_run()).collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphEditContext {
    pub anchor_bbox: BoundingBox,
    pub paragraph: LayoutParagraph,
}

impl ParagraphEditContext {
    /// 转换为使用绝对坐标的 EditorSession
    pub fn to_editor_session(&self) -> EditorSession {
        EditorSession {
            anchor_bbox: self.anchor_bbox,
            paragraph: self.paragraph.to_text_paragraph(),
        }
    }
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

// ============================================================================
// 新坐标系统数据结构 - Phase 1
// ============================================================================

/// 单个 glyph 的位置信息
/// 所有坐标都是绝对页面坐标（Y-Down 体系）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphPosition {
    /// glyph 原点的绝对页面 X 坐标
    pub x: f32,
    /// glyph 的物理宽度
    pub width: f32,
}

impl GlyphPosition {
    /// glyph 的右边界（绝对坐标）
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// 计算 glyph 的位置（可用于字段的默认值）
    pub fn new(x: f32, width: f32) -> Self {
        Self { x, width }
    }
}

#[cfg(test)]
mod coordinate_system_tests {
    use super::*;

    // ---- GlyphPosition tests ----

    #[test]
    fn test_glyph_position_right() {
        let pos = GlyphPosition::new(10.0, 5.0);
        assert_eq!(pos.right(), 15.0);
    }

    #[test]
    fn test_glyph_position_zero_width() {
        let pos = GlyphPosition {
            x: 10.0,
            width: 0.0,
        };
        assert_eq!(pos.right(), 10.0);
    }

    // ---- TextRun tests ----

    fn sample_run() -> TextRun {
        TextRun {
            id: "test-run".to_string(),
            text: "Hello".to_string(),
            origin_x: 72.0,
            baseline_y: 112.0,
            glyphs: vec![
                GlyphPosition::new(72.0, 8.0), // H
                GlyphPosition::new(80.0, 5.0), // e
                GlyphPosition::new(85.0, 3.0), // l
                GlyphPosition::new(88.0, 3.0), // l
                GlyphPosition::new(91.0, 6.0), // o
            ],
            style: RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                char_spacing: 0.0,
                scale_x: 1.0,
            },
            object_ids: vec!["obj1".to_string()],
        }
    }

    #[test]
    fn test_text_run_compute_bbox() {
        let run = sample_run();
        let bbox = run.compute_bbox();
        assert_eq!(bbox.left, 72.0);
        assert_eq!(bbox.top, 100.0); // 112.0 - 12.0
        assert_eq!(bbox.right, 97.0); // 91.0 + 6.0
        assert_eq!(bbox.bottom, 112.0);
    }

    #[test]
    fn test_text_run_physical_width() {
        let run = sample_run();
        assert_eq!(run.physical_width(), 25.0); // 97.0 - 72.0
    }

    #[test]
    fn test_text_run_glyph_x() {
        let run = sample_run();
        assert_eq!(run.glyph_x(0), Some(72.0));
        assert_eq!(run.glyph_x(4), Some(91.0));
        assert_eq!(run.glyph_x(5), None); // 越界
    }

    #[test]
    fn test_text_run_empty_glyphs() {
        let run = TextRun {
            id: "empty".to_string(),
            text: String::new(),
            origin_x: 72.0,
            baseline_y: 112.0,
            glyphs: vec![],
            style: RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#000000".to_string(),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                char_spacing: 0.0,
                scale_x: 1.0,
            },
            object_ids: vec![],
        };
        let bbox = run.compute_bbox();
        assert_eq!(bbox, BoundingBox::default());
        assert_eq!(run.physical_width(), 0.0);
    }

    // ---- split_at tests ----

    #[test]
    fn test_split_at_middle() {
        let run = sample_run();
        let (left, right) = run.split_at(2);

        let left = left.expect("left should exist");
        assert_eq!(left.text, "He");
        assert_eq!(left.origin_x, 72.0);
        assert_eq!(left.glyphs.len(), 2);
        assert_eq!(left.glyphs[0].x, 72.0); // 绝对坐标不变
        assert_eq!(left.glyphs[1].x, 80.0);

        let right = right.expect("right should exist");
        assert_eq!(right.text, "llo");
        assert_eq!(right.origin_x, 85.0); // 第一个 glyph 的绝对坐标
        assert_eq!(right.glyphs.len(), 3);
        assert_eq!(right.glyphs[0].x, 85.0); // 绝对坐标不变
        assert_eq!(right.glyphs[1].x, 88.0);
        assert_eq!(right.glyphs[2].x, 91.0);
    }

    #[test]
    fn test_split_at_zero() {
        let run = sample_run();
        let (left, right) = run.split_at(0);
        assert!(left.is_none());
        assert_eq!(right.expect("right should exist").text, "Hello");
    }

    #[test]
    fn test_split_at_end() {
        let run = sample_run();
        let (left, right) = run.split_at(5);
        assert_eq!(left.expect("left should exist").text, "Hello");
        assert!(right.is_none());
    }

    #[test]
    fn test_split_at_beyond_end() {
        let run = sample_run();
        let (left, right) = run.split_at(10);
        assert_eq!(left.expect("left should exist").text, "Hello");
        assert!(right.is_none());
    }

    // ---- TextParagraph tests ----

    #[test]
    fn test_text_paragraph_compute_bbox() {
        let para = TextParagraph {
            id: "para1".to_string(),
            runs: vec![sample_run()],
        };
        let bbox = para.compute_bbox();
        assert_eq!(bbox.left, 72.0);
        assert_eq!(bbox.top, 100.0);
        assert_eq!(bbox.right, 97.0);
        assert_eq!(bbox.bottom, 112.0);
    }

    #[test]
    fn test_text_paragraph_multiple_runs() {
        let run1 = sample_run();
        let mut run2 = sample_run();
        run2.id = "run2".to_string();
        run2.origin_x = 120.0;
        run2.baseline_y = 130.0;
        run2.glyphs = vec![GlyphPosition::new(120.0, 10.0)];

        let para = TextParagraph {
            id: "para2".to_string(),
            runs: vec![run1, run2],
        };
        let bbox = para.compute_bbox();
        assert_eq!(bbox.left, 72.0);
        assert_eq!(bbox.top, 100.0); // min(100, 118)
        assert_eq!(bbox.right, 130.0); // max(97, 130)
        assert_eq!(bbox.bottom, 130.0); // max(112, 130)
    }

    // ---- EditorSession tests ----

    #[test]
    fn test_editor_session_glyph_local_x() {
        let session = EditorSession {
            anchor_bbox: BoundingBox {
                left: 70.0,
                top: 100.0,
                right: 130.0,
                bottom: 130.0,
            },
            paragraph: TextParagraph {
                id: "para1".to_string(),
                runs: vec![sample_run()],
            },
        };

        // glyph 0: x=72.0, anchor.left=70.0, local_x=2.0
        assert_eq!(session.glyph_local_x(0, 0), Some(2.0));
        // glyph 4: x=91.0, anchor.left=70.0, local_x=21.0
        assert_eq!(session.glyph_local_x(0, 4), Some(21.0));
        // out of bounds
        assert_eq!(session.glyph_local_x(1, 0), None);
    }

    // ---- StyledRun → TextRun conversion tests ----

    fn sample_styled_run() -> StyledRun {
        // 创建一个 StyledRun，其中 char_origins 相对于 tx
        let run = StyledRun {
            text: "Hello".to_string(),
            color: "#000000".to_string(),
            stroke_color: None,
            stroke_width: 0.0,
            tx: 72.0,    // run 起点 X
            ty: 112.0,   // run 基线 Y（Y-Up 体系）
            width: 25.0, // glyph 总宽度
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_underline: false,
            font_name: "Arial".to_string(),
            a: 100.0, // scale_x
            b: 0.0,   // shear_y
            c: 0.0,   // shear_x
            d: 100.0, // scale_y
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            font_hints: None,
            char_origins: vec![0.0, 8.0, 11.0, 14.0, 19.0], // 相对于 tx 的偏移
            char_widths: vec![8.0, 3.0, 3.0, 5.0, 6.0],     // glyph 宽度 (H, e, l, l, o)
            pdf_char_codes: vec![],
            render_mode: 0,
            object_id: None,
            font_post_script_name: None,
            font_family_hint: None,
            font_subtype: None,
            embedded_font_key: None,
            has_embedded_font_program: false,
            has_to_unicode_cmap: false,
            z_index: 0,
        };
        run
    }

    #[test]
    fn test_from_styled_preserves_absolute_coordinates() {
        let run = sample_styled_run();

        let text_run = TextRun::from_styled(&run);

        // 验证绝对坐标转换
        assert_eq!(text_run.origin_x, run.tx); // 绝对 X
        assert_eq!(text_run.baseline_y, run.ty); // 绝对 Y

        // 验证 glyphs 的绝对位置：tx + char_origins[i]
        assert_eq!(text_run.glyphs[0].x, 72.0 + 0.0); // H
        assert_eq!(text_run.glyphs[1].x, 72.0 + 8.0); // e
        assert_eq!(text_run.glyphs[2].x, 72.0 + 11.0); // l
        assert_eq!(text_run.glyphs[3].x, 72.0 + 14.0); // l
        assert_eq!(text_run.glyphs[4].x, 72.0 + 19.0); // o

        // 验证 glyphs 的宽度
        assert_eq!(text_run.glyphs[0].width, 8.0); // H
        assert_eq!(text_run.glyphs[1].width, 3.0); // e
        assert_eq!(text_run.glyphs[2].width, 3.0); // l
        assert_eq!(text_run.glyphs[3].width, 5.0); // l
        assert_eq!(text_run.glyphs[4].width, 6.0); // o (由 width 推导: 19.0 + 6.0 - 19.0)

        // 验证 run 的物理宽度从 glyphs 推导
        assert_eq!(text_run.physical_width(), 25.0); // 72.0 + 25.0 - 72.0

        // 验证 bbox 从 glyphs 推导（单一算法）
        let bbox = text_run.compute_bbox();
        assert_eq!(bbox.left, 72.0);
        assert_eq!(bbox.top, 100.0); // 112.0 - 12.0
        assert_eq!(bbox.right, 97.0); // 72.0 + 25.0
    }

    #[test]
    fn test_from_styled_empty_glyphs() {
        let mut run = sample_styled_run();
        run.char_origins.clear();
        run.char_widths.clear();

        let text_run = TextRun::from_styled(&run);

        // 当没有 char_origins/char_widths 时，合成一个 glyph 保证 bbox 非零
        assert_eq!(text_run.glyphs.len(), 1);
        assert_eq!(text_run.origin_x, run.tx);
        assert_eq!(text_run.baseline_y, run.ty);
        // 合成 glyph 的宽度应等于 StyledRun.width
        assert_eq!(text_run.glyphs[0].x, run.tx);
        assert_eq!(text_run.glyphs[0].width, run.width);
    }

    #[test]
    fn test_from_styled_with_object_id() {
        let mut run = sample_styled_run();
        run.object_id = Some("obj-123".to_string());

        let text_run = TextRun::from_styled(&run);

        assert_eq!(text_run.object_ids, vec!["obj-123".to_string()]);
        assert_eq!(text_run.id, "obj-123"); // id 直接使用 object_id
    }

    #[test]
    fn test_glyph_x_returns_actual_positions() {
        let run = sample_styled_run();
        let text_run = TextRun::from_styled(&run);

        // 验证 glyph_x 返回正确的绝对坐标
        assert_eq!(text_run.glyph_x(0), Some(72.0));
        assert_eq!(text_run.glyph_x(2), Some(83.0));
        assert_eq!(text_run.glyph_x(4), Some(91.0));
        assert_eq!(text_run.glyph_x(5), None); // 越界
    }

    #[test]
    fn test_split_at_middle_preserves_absolute_positions() {
        let run = sample_styled_run();
        let text_run = TextRun::from_styled(&run);

        let (left, right) = text_run.split_at(2);

        let left = left.expect("left should exist");
        let right = right.expect("right should exist");

        // 验证左侧 run 的绝对位置
        assert_eq!(left.text, "He");
        assert_eq!(left.origin_x, 72.0); // 保持原起点
        assert_eq!(left.glyphs[0].x, 72.0); // 绝对坐标不变
        assert_eq!(left.glyphs[1].x, 80.0);

        // 验证右侧 run 的绝对位置（新起点 = 第一个 glyph 的绝对坐标）
        assert_eq!(right.text, "llo");
        assert_eq!(right.origin_x, 83.0); // 72.0 + 11.0
        assert_eq!(right.glyphs[0].x, 83.0); // 绝对坐标不变
        assert_eq!(right.glyphs[1].x, 86.0);
    }

    #[test]
    fn test_split_at_end_handles_last_glyph_correctly() {
        let run = sample_styled_run();
        let text_run = TextRun::from_styled(&run);

        let (left, _right) = text_run.split_at(5);
        assert_eq!(left.expect("left should exist").text, "Hello");
    }

    #[test]
    fn test_layout_run_to_text_run_conversion() {
        let run = sample_styled_run();
        let text_run = TextRun::from_styled(&run);
        let layout_run = text_run.to_layout_run();

        // 验证字段一致
        assert_eq!(layout_run.id, text_run.id);
        assert_eq!(layout_run.text, text_run.text);
        assert_eq!(layout_run.origin_x, text_run.origin_x);
        assert_eq!(layout_run.origin_y, text_run.baseline_y);
        assert_eq!(layout_run.style, text_run.style);
        assert_eq!(layout_run.object_ids, text_run.object_ids);

        // 验证 char_origins 相对偏移正确
        assert_eq!(layout_run.char_origins.len(), text_run.glyphs.len());
        for (i, glyph) in text_run.glyphs.iter().enumerate() {
            assert_eq!(layout_run.char_origins[i], glyph.x - text_run.origin_x);
        }

        // 验证 char_widths 正确
        for (i, glyph) in text_run.glyphs.iter().enumerate() {
            assert_eq!(layout_run.char_widths[i], glyph.width);
        }

        // 验证 bbox 从绝对坐标推导正确
        let expected_bbox = text_run.compute_bbox();
        assert_eq!(layout_run.bbox.left, expected_bbox.left);
        assert_eq!(layout_run.bbox.top, expected_bbox.top);
        assert_eq!(layout_run.bbox.right, expected_bbox.right);
        assert_eq!(layout_run.bbox.bottom, expected_bbox.bottom);
    }

    #[test]
    fn test_text_run_from_layout_run_roundtrip() {
        let run = sample_styled_run();
        let text_run_orig = TextRun::from_styled(&run);
        let layout_run = text_run_orig.to_layout_run();
        let text_run_restored = layout_run.to_text_run();

        // 验证往返转换保持一致
        assert_eq!(text_run_restored.id, text_run_orig.id);
        assert_eq!(text_run_restored.text, text_run_orig.text);
        assert_eq!(text_run_restored.origin_x, text_run_orig.origin_x);
        assert_eq!(text_run_restored.baseline_y, text_run_orig.baseline_y);
        assert_eq!(text_run_restored.style, text_run_orig.style);
        assert_eq!(text_run_restored.object_ids, text_run_orig.object_ids);

        // 验证 glyphs 绝对坐标一致
        assert_eq!(text_run_restored.glyphs.len(), text_run_orig.glyphs.len());
        for (i, glyph_orig) in text_run_orig.glyphs.iter().enumerate() {
            let glyph_restored = &text_run_restored.glyphs[i];
            assert_eq!(glyph_restored.x, glyph_orig.x);
            // 宽度可能有微小差异（默认值 vs 推导值），但应该接近
            assert!((glyph_restored.width - glyph_orig.width).abs() < 1.0);
        }
    }

    #[test]
    fn test_layout_paragraph_to_text_paragraph() {
        let layout_para = LayoutParagraph {
            id: "para1".to_string(),
            bbox: BoundingBox {
                left: 10.0,
                top: 40.0,
                right: 100.0,
                bottom: 52.0,
            },
            style: ParagraphStyle::default(),
            runs: vec![TextRun::from_styled(&sample_styled_run()).to_layout_run()],
            object_ids: vec!["obj1".to_string()],
            origin_x: 10.0,
            origin_y: 52.0,
            wrap_width: 90.0,
        };

        let text_para = layout_para.to_text_paragraph();

        assert_eq!(text_para.id, layout_para.id);
        assert_eq!(text_para.runs.len(), layout_para.runs.len());

        // 验证每个 run 的转换正确
        for (i, text_run) in text_para.runs.iter().enumerate() {
            let layout_run = &layout_para.runs[i];
            assert_eq!(text_run.origin_x, layout_run.origin_x);
            assert_eq!(text_run.baseline_y, layout_run.origin_y);
        }
    }

    #[test]
    fn test_paragraph_edit_context_to_editor_session() {
        let layout_para = LayoutParagraph {
            id: "para1".to_string(),
            bbox: BoundingBox {
                left: 10.0,
                top: 40.0,
                right: 100.0,
                bottom: 52.0,
            },
            style: ParagraphStyle::default(),
            runs: vec![TextRun::from_styled(&sample_styled_run()).to_layout_run()],
            object_ids: vec!["obj1".to_string()],
            origin_x: 10.0,
            origin_y: 52.0,
            wrap_width: 90.0,
        };

        let edit_context = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 10.0,
                top: 40.0,
                right: 100.0,
                bottom: 52.0,
            },
            paragraph: layout_para,
        };

        let session = edit_context.to_editor_session();

        assert_eq!(session.anchor_bbox, edit_context.anchor_bbox);
        assert_eq!(session.paragraph.id, edit_context.paragraph.id);
        assert_eq!(
            session.paragraph.runs.len(),
            edit_context.paragraph.runs.len()
        );
    }

    // ---- GlyphPaintRun → TextRun conversion tests ----

    fn sample_glyph_paint_run() -> crate::models::GlyphPaintRun {
        use crate::models::{GlyphPaintRun, PaintMode, ResolvedFontFace};

        GlyphPaintRun {
            id: "paint-run-1".to_string(),
            page_index: 0,
            region_id: "region-1".to_string(),
            paragraph_id: "para-1".to_string(),
            text: "Hello".to_string(),
            bbox: BoundingBox {
                left: 72.0,
                top: 100.0,
                right: 97.0,
                bottom: 112.0,
            },
            origin_x: 72.0,
            origin_y: 112.0,
            char_origins: vec![0.0, 8.0, 11.0, 14.0, 19.0], // 相对于 origin_x 的偏移
            color: "#000000".to_string(),
            resolved_font: ResolvedFontFace {
                render_family: "Arial".to_string(),
                ..Default::default()
            },
            font_size: 12.0,
            scale_x: 1.0,
            is_bold: false,
            is_italic: false,
            is_underline: false,
            paint_mode: PaintMode::Fill,
            object_ids: vec!["obj1".to_string()],
            object_indices: vec![0],
        }
    }

    #[test]
    fn test_glyph_paint_run_to_text_run() {
        let paint_run = sample_glyph_paint_run();
        let text_run = paint_run.to_text_run();

        // 验证绝对坐标转换
        assert_eq!(text_run.origin_x, 72.0);
        assert_eq!(text_run.baseline_y, 112.0);

        // 验证 glyphs 的绝对位置：origin_x + char_origins[i]
        assert_eq!(text_run.glyphs[0].x, 72.0 + 0.0); // H
        assert_eq!(text_run.glyphs[1].x, 72.0 + 8.0); // e
        assert_eq!(text_run.glyphs[2].x, 72.0 + 11.0); // l
        assert_eq!(text_run.glyphs[3].x, 72.0 + 14.0); // l
        assert_eq!(text_run.glyphs[4].x, 72.0 + 19.0); // o

        // 验证 glyph 宽度推导
        assert_eq!(text_run.glyphs[0].width, 8.0); // next - current = 8 - 0
        assert_eq!(text_run.glyphs[1].width, 3.0); // next - current = 11 - 8
        assert_eq!(text_run.glyphs[2].width, 3.0); // next - current = 14 - 11
        assert_eq!(text_run.glyphs[3].width, 5.0); // next - current = 19 - 14
                                                   // 最后一个 glyph 的宽度使用默认值
        assert_eq!(text_run.glyphs[4].width, 6.0); // font_size * 0.5

        // 验证 compute_bbox 从绝对坐标推导
        let bbox = text_run.compute_bbox();
        assert_eq!(bbox.left, 72.0);
        assert_eq!(bbox.top, 100.0); // 112 - 12
        assert_eq!(bbox.right, 97.0); // 72 + 19 + 6 = 97
        assert_eq!(bbox.bottom, 112.0);
    }

    #[test]
    fn test_glyph_paint_run_to_text_run_split() {
        let paint_run = sample_glyph_paint_run();
        let text_run = paint_run.to_text_run();

        let (left, right) = text_run.split_at(2);

        let left = left.expect("left should exist");
        let right = right.expect("right should exist");

        // marker 部分：绝对坐标不变
        assert_eq!(left.text, "He");
        assert_eq!(left.origin_x, 72.0);
        assert_eq!(left.glyphs[0].x, 72.0);
        assert_eq!(left.glyphs[1].x, 80.0);

        // body 部分：origin_x = 第一个 glyph 的绝对坐标
        assert_eq!(right.text, "llo");
        assert_eq!(right.origin_x, 83.0); // 72 + 11
        assert_eq!(right.glyphs[0].x, 83.0);
        assert_eq!(right.glyphs[1].x, 86.0);
    }
}

/// 文本 run，所有坐标都是绝对页面坐标（Y-Down 体系）
/// 这是 LayoutRun 的重构版本，核心变化：
/// 1. char_origins/char_widths 合并为 glyphs: Vec<GlyphPosition>
/// 2. bbox 改为 compute_bbox() 方法（消除冗余存储）
/// 3. width 改为 physical_width() 方法（从 glyphs 推导）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextRun {
    pub id: String,
    pub text: String,
    /// run 起始 glyph 的 X 坐标（绝对页面坐标）
    pub origin_x: f32,
    /// run 基线的 Y 坐标（绝对页面坐标）
    pub baseline_y: f32,
    /// 每个 glyph 的绝对位置和宽度
    pub glyphs: Vec<GlyphPosition>,
    /// 样式（复用现有的 RunStyle）
    pub style: RunStyle,
    /// 来源 PDF object IDs
    pub object_ids: Vec<String>,
}

impl TextRun {
    /// 计算 run 的包围盒（不存储，按需计算）
    /// 单一算法：从 glyphs 推导，消除三处不一致的计算
    pub fn compute_bbox(&self) -> BoundingBox {
        if self.glyphs.is_empty() {
            return BoundingBox::default();
        }
        let left = self.origin_x;
        let right = self.glyphs.iter().map(|g| g.right()).fold(left, f32::max);
        let top = self.baseline_y - self.style.font_size;
        let bottom = self.baseline_y;
        BoundingBox {
            left,
            top,
            right,
            bottom,
        }
    }

    /// run 的物理宽度（从 glyphs 推导）
    pub fn physical_width(&self) -> f32 {
        if self.glyphs.is_empty() {
            return 0.0;
        }
        let first_x = self.origin_x;
        let last_right = self
            .glyphs
            .iter()
            .map(|g| g.right())
            .fold(first_x, f32::max);
        last_right - first_x
    }

    /// 获取第 i 个 glyph 的绝对 X 坐标
    pub fn glyph_x(&self, index: usize) -> Option<f32> {
        self.glyphs.get(index).map(|g| g.x)
    }

    /// 在字符边界分割 TextRun
    /// 切割零变换：glyphs 数组直接切割，不需要重新计算偏移
    /// 当 glyphs 数量与字符数不匹配时（如合成单 glyph），按比例分割 glyph 宽度
    pub fn split_at(&self, char_index: usize) -> (Option<TextRun>, Option<TextRun>) {
        let char_count = self.text.chars().count();

        if char_index == 0 {
            return (None, Some(self.clone()));
        }
        if char_index >= char_count {
            return (Some(self.clone()), None);
        }

        // 当 glyphs 数量与字符数匹配时，直接切割
        if self.glyphs.len() == char_count {
            let left_glyphs = self.glyphs[..char_index].to_vec();
            let right_glyphs = self.glyphs[char_index..].to_vec();

            let left_text: String = self.text.chars().take(char_index).collect();
            let left_run = if left_glyphs.is_empty() {
                None
            } else {
                Some(TextRun {
                    id: format!("{}::split::{}", self.id, char_index),
                    text: left_text,
                    origin_x: self.origin_x,
                    baseline_y: self.baseline_y,
                    glyphs: left_glyphs,
                    style: self.style.clone(),
                    object_ids: self.object_ids.clone(),
                })
            };

            let right_text: String = self.text.chars().skip(char_index).collect();
            let right_run = if right_glyphs.is_empty() {
                None
            } else {
                let right_origin_x = right_glyphs[0].x;
                Some(TextRun {
                    id: format!("{}::split::{}", self.id, char_index),
                    text: right_text,
                    origin_x: right_origin_x,
                    baseline_y: self.baseline_y,
                    glyphs: right_glyphs,
                    style: self.style.clone(),
                    object_ids: self.object_ids.clone(),
                })
            };

            return (left_run, right_run);
        }

        // glyphs 数量不匹配时（如合成单 glyph），按比例分割宽度
        // 这种情况发生在 char_origins 为空时的 fallback
        let total_width = self.physical_width();
        let left_width = total_width * (char_index as f32 / char_count as f32);
        let right_width = total_width - left_width;

        let left_text: String = self.text.chars().take(char_index).collect();
        let right_text: String = self.text.chars().skip(char_index).collect();

        let left_run = if left_width > 0.0 {
            Some(TextRun {
                id: format!("{}::split::{}", self.id, char_index),
                text: left_text,
                origin_x: self.origin_x,
                baseline_y: self.baseline_y,
                glyphs: vec![GlyphPosition::new(self.origin_x, left_width)],
                style: self.style.clone(),
                object_ids: self.object_ids.clone(),
            })
        } else {
            None
        };

        let right_run = if right_width > 0.0 {
            Some(TextRun {
                id: format!("{}::split::{}", self.id, char_index),
                text: right_text,
                origin_x: self.origin_x + left_width,
                baseline_y: self.baseline_y,
                glyphs: vec![GlyphPosition::new(self.origin_x + left_width, right_width)],
                style: self.style.clone(),
                object_ids: self.object_ids.clone(),
            })
        } else {
            None
        };

        (left_run, right_run)
    }

    /// 从 StyledRun 构造（PDF 解析层的转换）
    pub fn from_styled(run: &StyledRun) -> Self {
        let glyphs: Vec<GlyphPosition> =
            if run.char_origins.is_empty() || run.char_widths.is_empty() {
                // 当没有 char_origins/char_widths 时，用 width 推导一个合成 glyph
                // 这保证了 compute_bbox 能正确计算（用 origin_x + width）
                vec![GlyphPosition::new(run.tx, run.width)]
            } else {
                run.char_origins
                    .iter()
                    .zip(run.char_widths.iter())
                    .map(|(origin, width)| GlyphPosition::new(run.tx + *origin, *width))
                    .collect()
            };

        Self {
            id: run
                .object_id
                .clone()
                .unwrap_or_else(|| format!("run-{}", run.tx)),
            text: run.text.clone(),
            origin_x: run.tx,
            baseline_y: run.ty,
            glyphs,
            style: RunStyle {
                font_name: run.font_name.clone(),
                font_size: run.font_size,
                color: run.color.clone(),
                is_bold: run.is_bold,
                is_italic: run.is_italic,
                is_underline: run.is_underline,
                char_spacing: run.char_spacing,
                scale_x: run.horizontal_scaling / 100.0,
                font_weight_numeric: run
                    .font_hints
                    .as_ref()
                    .map(|h| h.weight as u16)
                    .unwrap_or(if run.is_bold { 700 } else { 400 }),
            },
            object_ids: run.object_id.clone().map(|id| vec![id]).unwrap_or_default(),
        }
    }

    /// 转换回 LayoutRun（旧代码兼容，逐步删除）
    pub fn to_layout_run(&self) -> LayoutRun {
        let char_origins: Vec<f32> = self.glyphs.iter().map(|g| g.x - self.origin_x).collect();
        let char_widths: Vec<f32> = self.glyphs.iter().map(|g| g.width).collect();

        LayoutRun {
            id: self.id.clone(),
            text: self.text.clone(),
            origin_x: self.origin_x,
            origin_y: self.baseline_y,
            char_origins,
            char_widths,
            style: self.style.clone(),
            bbox: self.compute_bbox(),
            object_ids: self.object_ids.clone(),
            object_indices: vec![],
        }
    }
}

/// 文本段落，由多个 TextRun 组成
/// 这是 LayoutParagraph 的重构版本，核心变化：
/// 1. 删除 origin_x/origin_y 字段（与 anchor_bbox.left/top 重复）
/// 2. 删除 wrap_width 字段（可从 runs 推导）
/// 3. bbox 改为 compute_bbox() 方法（从 runs 推导）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextParagraph {
    pub id: String,
    pub runs: Vec<TextRun>,
}

impl TextParagraph {
    /// 计算段落的包围盒（所有 run 的并集）
    pub fn compute_bbox(&self) -> BoundingBox {
        self.runs
            .iter()
            .filter(|run| !run.text.is_empty())
            .map(|run| run.compute_bbox())
            .reduce(|acc, bbox| BoundingBox {
                left: acc.left.min(bbox.left),
                top: acc.top.min(bbox.top),
                right: acc.right.max(bbox.right),
                bottom: acc.bottom.max(bbox.bottom),
            })
            .unwrap_or_default()
    }

    /// Y-Down 翻转：将基线坐标从 Y-Up 转换为 Y-Down
    pub fn flip_y(&mut self, h: f32) {
        for run in &mut self.runs {
            run.baseline_y = h - run.baseline_y;
        }
    }
}

/// 编辑器会话上下文
/// 使用 anchor_bbox 作为参考，不存储冗余的 origin_x/origin_y
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorSession {
    /// 编辑区域的绝对页面坐标
    pub anchor_bbox: BoundingBox,
    pub paragraph: TextParagraph,
}

impl EditorSession {
    /// 计算相对于 anchor 的 glyph 位置（仅用于 caret 计算）
    pub fn glyph_local_x(&self, run_index: usize, glyph_index: usize) -> Option<f32> {
        let run = self.paragraph.runs.get(run_index)?;
        let glyph = run.glyphs.get(glyph_index)?;
        Some(glyph.x - self.anchor_bbox.left)
    }
}
