use serde::{Deserialize, Serialize};

pub mod document_runtime;
pub mod interaction;

pub use document_runtime::*;
pub use interaction::*;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FontHints {
    pub flags: i32,
    pub weight: i32,
    pub italic_angle: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cap_height: f32,
    pub x_height: f32,
    pub is_fixed_pitch: bool,
    pub is_serif: bool,
    pub is_italic: bool,
    pub is_bold: bool,
}

fn default_horizontal_scaling() -> f32 { 100.0 }

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FontSourceKind {
    Embedded,
    SystemMatched,
    Substituted,
    #[default]
    Fallback,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SymbolClass {
    #[default]
    None,
    Symbol,
    Dingbat,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedFontIdentity {
    pub raw_name: String,
    pub canonical_family: String,
    pub style_name: String,
    pub weight: i32,
    pub is_italic: bool,
    pub symbol_class: SymbolClass,
    pub subset_stripped: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedFontFace {
    pub identity: ResolvedFontIdentity,
    pub render_family: String,
    pub metrics_family: String,
    pub source: FontSourceKind,
    pub confidence: f32,
}

/// 从 PDF 底层绘制流（Content Stream）中直接抽取出的极小粒度文本游程串。
/// 
/// # Overview (架构定位)
/// `StyledRun` 是整个文本提取管线的原子级数据载体 (Atomic Data Carrier)。
/// 由于 PDF 格式并没有“段落”或甚至“单词”的强制约束，连续的视觉单词往往会被切割成多个 `StyledRun`。
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

/// 表示 2D 空间内的绝对物理包围盒。
///
/// # Invariants & Coordinate Systems (重要坐标系声明)
/// `BoundingBox` 被设计为兼容两种极性空间，但**一旦被实例化进入流转层，必须约定其处于 Y-Down 规范**。
/// 在 `Y-Down` 的前提下，`top` 始终表示视觉上方的边际，其数值 **严格小于** `bottom`。
/// 
/// 违反 `top < bottom` 的包围盒被视为非法坍缩体，将直接导致碰撞检测抛出和选区渲染错误。
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoundingBox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl BoundingBox {
    /// 执行原地 Y 轴极性反转。通常在数据从解析器（Parser）送入呈现层（Presentation Pipeline）时触发。
    ///
    /// # Arguments
    /// * `h` - 作为反射轴基准的页面总高度。
    /// 
    /// # Thread Safety
    /// 原地就地修改，开销仅为几个 f32 指令。
    pub fn flip_y(&mut self, h: f32) {
        let old_top = self.top;
        let old_bottom = self.bottom;
        self.top = h - old_bottom;
        self.bottom = h - old_top;
    }
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
}

fn default_scale() -> f32 { 1.0 }

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
pub struct EditorSession {
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

fn default_scale_x() -> f32 {
    1.0
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
    pub editor_session: EditorSession,
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorPathSegment {
    pub command: String,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorPathObject {
    pub id: String,
    #[serde(default)]
    pub segments: Vec<VectorPathSegment>,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    #[serde(default)]
    pub fill: bool,
    #[serde(default)]
    pub stroke: bool,
    #[serde(default)]
    pub stroke_width: f32,
    #[serde(default)]
    pub z_index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorImageObject {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub z_index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorTextObject {
    pub id: String,
    #[serde(default)]
    pub runs: Vec<StyledRun>,
    #[serde(default)]
    pub z_index: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VectorRenderObject {
    Text(VectorTextObject),
    Path(VectorPathObject),
    Image(VectorImageObject),
}

/// 大一统的渲染原语容器版本。包含了当前页面所有的纯几何/排版对象。
///
/// # Overview (架构定位)
/// 它是整个 Core 离线计算与前端的骨架。承载着从 Wasm 到 TS 层的桥接数据负担。
/// 这里的 `objects` 不包含深层语义（不知道什么是段落），只知道这里有一堆字，有一群几何图形。
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorPageModel {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub objects: Vec<VectorRenderObject>,
}

impl VectorPageModel {
    /// 文档级空间标准化关卡 (The Great Normalization Gate)
    /// 
    /// 这是 `Y-Up` 历史遗留问题被阻绝在外的最后防线。
    /// 当底层的拉流解析器 (Stream Parser) 组装完原始树后，必须调用这个方法。
    /// 一旦该方法执行完毕，模型连带其挂载的子流（Text, Paths, Images）将不可逆地转换为安全的 Y-Down 坐标域。
    pub fn flip_y(&mut self) {
        let h = self.height;
        for obj in &mut self.objects {
            match obj {
                VectorRenderObject::Text(t) => {
                    for run in &mut t.runs {
                        run.flip_y(h);
                    }
                }
                VectorRenderObject::Path(p) => {
                    for seg in &mut p.segments {
                        for pt in &mut seg.points {
                            pt[1] = h - pt[1];
                        }
                    }
                }
                VectorRenderObject::Image(img) => {
                    img.y = h - (img.y + img.height);
                }
            }
        }
    }
}
