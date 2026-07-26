use serde::{Deserialize, Serialize};

use super::font::FontHints;
use super::layout::{LayoutAlignment, LayoutRole};

fn default_horizontal_scaling() -> f32 {
    100.0
}

/// 从 PDF 底层绘制流（Content Stream）中直接抽取出的极小粒度文本游程串。
///
/// # Overview (架构定位)
/// `StyledRun` 是整个文本提取管线的原子级数据载体 (Atomic Data Carrier)。
/// 由于 PDF 格式并没有"段落"或甚至"单词"的强制约束，连续的视觉单词往往会被切割成多个 `StyledRun`。
/// 它完整捕获了字体、字重、颜色以及仿射矩阵等绘制环境（Graphics State）。
///
/// # Thread Safety & Serialization
/// 支持跨 WASM 边界与 JavaScript 进行 `Serde` 无损序列化。
/// 内建深拷贝支持 `Clone`。
///
/// # Invariants (不变式约定)
/// - **坐标体系未定态 (Polarity Agnostic)**:
///   刚从解析器吐出的 `StyledRun` 仍然保留 PDF 原始的 `Y-Up` 体系。
///   它依赖于所属上级容器（如 `VectorPageModel`）调用 `flip_y` 强制规范化为 `Y-Down` 结构。
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct StyledRun {
    /// 提取出的原始 UTF-8 纯文本。
    pub text: String,
    /// 填充颜色，采用 `#RRGGBB` 格式。
    pub color: String,
    /// 描边外轮廓颜色。
    #[serde(default)]
    pub stroke_color: Option<String>,
    /// 描边粗细。
    #[serde(default)]
    pub stroke_width: f32,

    /// **核心位标 X**: 首字符基准原点 (Origin) 的绝对页面横坐标。
    pub tx: f32,
    /// **核心位标 Y**: 首字符基准线的纵坐标。注意它的极性依赖于当前管线阶段。
    pub ty: f32,

    /// 根据全部字符字宽聚合出的预期逻辑物理宽度。
    pub width: f32,
    /// 依据仿射矩阵测算出来的绝对字号。
    pub font_size: f32,

    // --- 字体加粗与斜体为根据变体名或字典标志硬解码得出的速查布尔值 ---
    pub is_bold: bool,
    pub is_italic: bool,
    #[serde(default)]
    pub is_underline: bool,
    pub font_name: String,

    // --- 内建变换矩阵 (Affine Text Matrix Tm) ---
    pub a: f32, // scale_x
    pub b: f32, // shear_y
    pub c: f32, // shear_x
    pub d: f32, // scale_y

    #[serde(default)]
    pub char_spacing: f32,
    #[serde(default)]
    pub word_spacing: f32,
    #[serde(default = "default_horizontal_scaling")]
    pub horizontal_scaling: f32,

    /// 层级：源 PDF 图层叠放深度。
    pub z_index: usize,
    /// 辅助字形推断。
    pub font_hints: Option<FontHints>,

    /// 每个字符在该局部游程里的横向偏移量（相对于 tx）。
    pub char_origins: Vec<f32>,
    /// 每个字符由于字距调整 (TJ) 和内置字模映射带来的不同宽度。
    pub char_widths: Vec<f32>,
    #[serde(default)]
    pub pdf_char_codes: Vec<u32>,

    /// PDF 渲染模式指令 (0 表示只填充不描边, 3 表示不可见占位符)。
    pub render_mode: i64,
    /// 用于将此游程绑定倒查回 PDF 源对象节点的追踪 ID。
    pub object_id: Option<String>,
    #[serde(default)]
    pub font_post_script_name: Option<String>,
    #[serde(default)]
    pub font_family_hint: Option<String>,
    #[serde(default)]
    pub font_subtype: Option<String>,
    #[serde(default)]
    pub embedded_font_key: Option<String>,
    #[serde(default)]
    pub has_embedded_font_program: bool,
    #[serde(default)]
    pub has_to_unicode_cmap: bool,
}

impl StyledRun {
    /// 强制对 `ty` 执行数学反转，将坐标空间推入 `Y-Down` 常规态。
    ///
    /// # Arguments
    /// * `h` - 所在页面的绝对逻辑物理高度区域。
    ///
    /// # Implementation Note
    /// 该操作不会顺绑修改 `BoundingBox`，因为 `StyledRun` 层级过低，不内置缓存包围盒，
    /// 其边际拓展计算需要延后至 `LayoutRun` 或 `GlyphPaintRun` 进行。
    pub fn flip_y(&mut self, h: f32) {
        self.ty = h - self.ty;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NativeTextModel {
    #[serde(default)]
    pub r#type: String,
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
    #[serde(default = "default_horizontal_scaling")]
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
    #[serde(default)]
    pub render_mode: i64,
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
    pub pdf_char_codes: Vec<u32>, // 原始 PDF charcode/CID 序列
}

impl NativeTextModel {
    pub fn flip_y(&mut self, h: f32) {
        self.ty = h - self.ty;
        self.baseline_y = h - self.baseline_y;
        self.top = h - self.top - self.height;
        for origin in &mut self.char_origins {
            origin[1] = -origin[1];
        }
        for run in &mut self.runs {
            run.ty = h - run.ty;
        }
    }
}

impl Default for NativeTextModel {
    fn default() -> Self {
        Self {
            r#type: "text".to_string(),
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
            horizontal_scaling: default_horizontal_scaling(),
            is_faux_bold: false,
            is_serif: false,
            is_italic: false,
            is_bold: false,
            is_underline: false,
            rendering_mode: 0,
            render_mode: 0,
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct NativePathObject {}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct NativeImageObject {}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum NativePageObject {
    Text(NativeTextModel),
    Path(NativePathObject),
    Image(NativeImageObject),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct NativePageModel {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub objects: Vec<NativePageObject>,
}
