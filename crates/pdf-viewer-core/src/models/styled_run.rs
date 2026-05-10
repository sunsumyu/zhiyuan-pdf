use serde::{Deserialize, Serialize};

use super::font::FontHints;
use super::layout::{LayoutAlignment, LayoutRole};

fn default_horizontal_scaling() -> f32 { 100.0 }

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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct NativeTextModel {
    #[serde(default)]
    pub r#type: String,
    pub id: String,
    pub text: String,
    pub tx: f32,
    pub ty: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub font_name: String,
    pub color: String,
    pub is_bold: bool,
    pub is_italic: bool,
    #[serde(default)]
    pub is_underline: bool,
    #[serde(default)]
    pub runs: Vec<StyledRun>,
    pub z_index: usize,
    pub font_hints: Option<FontHints>,
    #[serde(default)]
    pub object_indices: Vec<usize>,
    #[serde(default)]
    pub paragraph_id: Option<String>,
    #[serde(default)]
    pub wrap_width: Option<f32>,
    #[serde(default)]
    pub min_tx: Option<f32>,
    #[serde(default)]
    pub render_mode: i64,
    #[serde(default)]
    pub role: Option<LayoutRole>,
    #[serde(default)]
    pub alignment: Option<LayoutAlignment>,
    #[serde(default)]
    pub char_spacing: f32,
    #[serde(default = "default_horizontal_scaling")]
    pub horizontal_scaling: f32,
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
