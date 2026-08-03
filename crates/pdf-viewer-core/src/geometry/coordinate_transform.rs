//! PDF 坐标映射与渲染投影矩阵层 (Coordinate Spaces & Rendering Projection Matrices)
//!
//! # Historical Context (历史演进背景)
//! 原生的 PDF 矢量数据天然采用笛卡尔坐标系的 "Y-Up" 极性（原点 `(0, 0)` 位于页面左下角）。
//! 然而，大多数现代渲染宿主（如 HTML DOM, Canvas, 原生窗口系统）均采用 "Y-Down" 极性（原点位于左上角）。
//!
//! 早期版本中，UI 层在进行交互时动态计算 Y 轴反转，导致了所谓的 "交互偏移漂移 (Interaction Drift)" BUG。
//! 为此，系统确立了以下绝对不变式：
//! **核心流水线入口处，所有 `Y-Up` 坐标必须被归一化为 `Y-Down`，严禁在后续的图处理、排版计算以及此处叠加额外的 Y 轴极性反转。**
//!
//! # Module Purpose
//! 本模块提供的仿射矩阵仅负责进行由全局空间到局部空间的 `O(1)` 平移投影，而不包含任何极性反转逻辑。

use crate::models::BoundingBox;

/// 映射到整个渲染页面（Page View）视窗内的绝对像素坐标点。
///
/// # Invariants (不变式)
/// 该坐标基于 Y-Down 规范。`y = 0` 严格代表渲染页面的最顶端边界。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageViewPoint {
    pub x: f32,
    pub y: f32,
}

/// 局限于某个富文本编辑器宿主（Editor Overlay）内部的相对坐标点。
///
/// # Usage (用途)
/// 主要用于处理鼠标及多点触控设备的坐标命中测试（Hit Testing），或者在局部 DOM 内计算光标与选区的生成。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorLocalPoint {
    pub x: f32,
    pub y: f32,
}

/// 宿主层提供的 DOM/窗口参考框。它只描述输入事实，不承载 PDF 领域规则。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostReferenceRect {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

/// 宿主层捕获到的 client 坐标点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClientPoint {
    pub x: f32,
    pub y: f32,
}

/// 逻辑页面尺寸，已经是 Y-Down 语义。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

/// 宿主参考框到页面逻辑空间的比例。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageScale {
    pub x: f32,
    pub y: f32,
}

/// 统一处理宿主 client 坐标与页面坐标之间的投影。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostPageTransform {
    pub reference: HostReferenceRect,
    pub page: PageSize,
}

impl HostPageTransform {
    pub fn new(reference: HostReferenceRect, page: PageSize) -> Self {
        Self { reference, page }
    }

    pub fn scale(&self) -> PageScale {
        PageScale {
            x: positive_ratio(self.reference.width, self.page.width),
            y: positive_ratio(self.reference.height, self.page.height),
        }
    }

    /// 将 client 坐标转换为 page 坐标。
    /// `clamp_box` 可选，提供时将结果限制在指定边界框内。
    pub fn to_page(&self, point: ClientPoint, clamp_box: Option<BoundingBox>) -> PageViewPoint {
        let scale = self.scale();
        let mut result = PageViewPoint {
            x: (point.x - self.reference.left) / scale.x,
            y: (point.y - self.reference.top) / scale.y,
        };
        if let Some(bbox) = clamp_box {
            let box_width = (bbox.right - bbox.left).max(1.0);
            let box_height = (bbox.bottom - bbox.top).max(1.0);
            let scale_x = positive_ratio(self.reference.width, box_width);
            let scale_y = positive_ratio(self.reference.height, box_height);
            result.x =
                bbox.left + ((point.x - self.reference.left) / scale_x).clamp(0.0, box_width);
            result.y = bbox.top + ((point.y - self.reference.top) / scale_y).clamp(0.0, box_height);
        }
        result
    }

    /// 将 client 坐标转换为编辑器局部坐标（相对于边界框左上角）。
    pub fn to_local(&self, point: ClientPoint, page_box: BoundingBox) -> EditorLocalPoint {
        let page_point = self.to_page(point, Some(page_box));
        EditorLocalPoint {
            x: page_point.x - page_box.left,
            y: page_point.y - page_box.top,
        }
    }
}

#[inline]
fn positive_ratio(numerator: f32, denominator: f32) -> f32 {
    let value =
        if numerator.is_finite() && denominator.is_finite() && numerator > 0.0 && denominator > 0.0
        {
            numerator / denominator
        } else {
            1.0
        };
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}

/// 负责将 PDF 内部逻辑规范的全局点挂载到前端画布视图系（View）的基础投影能力。
///
/// # Design Rationale (设计决策)
/// 你可能会疑惑这里为什么只保存了 `page_height` 且 `point()` 方法没有做任何反转。
/// 实际上，在之前架构中，它肩负着 `y = page_height - y` 的反转职责。
/// 现在的架构下，其翻转的副作用被前置（详见反序列化过程）。它暂时保持为空的投影屏障，
/// 是为了之后承接 DPI 屏幕倍率扩展以及视差缩放（Zoom Matrix）铺平道路。
#[derive(Debug, Clone, Copy)]
pub struct PdfToPageViewTransform {
    pub page_height: f32,
}

impl PdfToPageViewTransform {
    /// 构造一个新的全局投影实例。
    ///
    /// # Arguments
    /// * `_page_height` - 目标界面的逻辑总高度。当前版本此参数作为架构占位保留符。
    pub fn new(_page_height: f32) -> Self {
        Self {
            page_height: _page_height,
        }
    }

    /// 将逻辑页面点投射进入目标观察者的视图原点系中。
    ///
    /// # Implementation Note
    /// 由于内部模型流经 `VectorPageModel` 解析时已统一为 `Y-Down (Normal)` 极性系，
    /// 当前步骤的时间复杂度严格限定在 O(1) 的浅表拷贝并杜绝加法浮点运算开支。
    #[inline(always)]
    pub fn point(&self, x: f32, y: f32) -> PageViewPoint {
        PageViewPoint { x, y }
    }
}

/// 负责处理 PDF 原始物理空间（Raw Cartesian）与逻辑空间之间的主权对齐。
///
/// # Responsibility
/// 在基础设施层从 `lopdf` 或其他原始引擎获取数据时，必须立即通过此类进行归一化。
pub struct PdfCoordinateSpace;

impl PdfCoordinateSpace {
    /// 将 PDF 原始 Y-Up 坐标转换为 Y-Down 坐标。
    #[inline(always)]
    pub fn to_y_down(y_up: f32, page_height: f32) -> f32 {
        page_height - y_up
    }

    /// 将 Y-Down 坐标还原为 PDF Y-Up 坐标。
    #[inline(always)]
    pub fn to_y_up(y_down: f32, page_height: f32) -> f32 {
        page_height - y_down
    }
}

/// 负责将绝对逻辑页面上的坐标系（Y-Down）收敛向特定编辑视区（Editor Viewport）拉伸变换矩阵。
///
/// # Overview (架构总览)
/// `EditorViewportTransform` 弥合了基于物理位移的绝对排版系统，与基于 DOM / HTML 表层渲染环境间的语义误差。
///
/// # Invariants (不变式约束)
/// * **输入法预校验**: 提供给 `project_point` 的参数 `x`, `y` **绝对不可是原始 Y-Up** 数据。
/// * **锚点生死周期 (Anchor Binding)**: `anchor` 代表其寄生成素的绝对基准框。无论何时这个文本流遭遇 `Relayout`（重排），
///    该变换实例应当立即在堆栈中释放重构。
///
/// # Thread Safety
/// 本结构未发生堆分配且均为栈原生的浮点基元。
/// 100% 遵照 Rust 标准库的 `Send + Sync + Copy` 底层契约，可跨越 WASM / WebWorker 边界自由穿梭。
#[derive(Debug, Clone, Copy)]
pub struct EditorViewportTransform {
    /// 绑定目标所在的 PDF 地理原点。
    pub anchor: BoundingBox,
    /// 用于缓冲基于宿主系统字体绘制在 Ascent（爬升界限）之上的溢出安全界限（防止 h, b 等字母被削顶）。
    pub top_buffer: f32,
}

impl EditorViewportTransform {
    /// 利用特定图层框与上行偏移距派生一个变换接口。
    pub fn new(anchor: BoundingBox, top_buffer: f32) -> Self {
        Self { anchor, top_buffer }
    }

    /// 将在画板监听域（Pointer Events Realm）所捕捉到的点击射线位置，射入当前的 DOM Component 子树。
    ///
    /// # Arguments
    /// * `pdf_x` - 画板 X。
    /// * `pdf_y` - 画板 Y，要求预先进行过 Y-Down 反卷。
    ///
    /// # Returns
    /// 提供出可供 `<textarea>` 或 `<div>` 消费定位的光标点。
    pub fn project_point(&self, pdf_x: f32, pdf_y: f32) -> EditorLocalPoint {
        EditorLocalPoint {
            x: pdf_x - self.anchor.left,
            // 在内部 Y-Down 坐标系中，视觉顶部即为较小的 Y 值 (anchor.top)
            // 距离顶部的纯量位移则为 = 绝对点击目标 - 基准锚点。
            y: self.top_buffer + (pdf_y - self.anchor.top),
        }
    }

    /// 投影 X 轴坐标到编辑器局部坐标。
    #[inline]
    pub fn project_x(&self, x: f32) -> f32 {
        x - self.anchor.left
    }

    /// 投影基线 Y 坐标到编辑器局部坐标。
    #[inline]
    pub fn project_baseline_y(&self, baseline_y: f32) -> f32 {
        self.top_buffer + (baseline_y - self.anchor.top)
    }

    /// 投影相对锚点的 Y 偏移量到编辑器局部坐标。
    #[inline]
    pub fn project_relative_y(&self, relative_y: f32) -> f32 {
        self.top_buffer + relative_y
    }
}
