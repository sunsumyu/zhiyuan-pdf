//! PDF 物理空间语义拓扑推断分析器 (Spatial Layout Typography Graph Analyzer)
//!
//! Moved from `pdf_viewer_core::analysis::analyzer` (architecture-review §2.2 /
//! Phase 2: P3 single-side module relocation). Only the Tauri backend invokes
//! this analyser; it does not belong in the cross-side `pdf-viewer-core`.
//!
//! # Overview (架构总览)
//! 从 PDF 反序列化层提取出来的初始数据通常是毫无业务语义的离散墨迹块序列（即 `LayoutRun`）。
//! 本分析器引擎的核心目标在于实施 **逆向排版推演 (Reverse Layout Inference)** 控制流。
//!
//! # Implementation Logic (核心算法细节)
//! 本分析器重度依赖基于近似聚类的**稀疏连通中心搜索算法**。
//!
//! 主要经历两道工序：
//! 1. **引力合并场 (Spatial Adjacency Clustering)**: 基于启发式几何容差碰撞，建立有向边的连通分量 (Connected Components) 的拓扑结构。
//! 2. **语法角色的强行降格与特征提取 (Pattern Matching Strategy)**: 将聚合后的块 (Boxes) 解析为段落主体或者含有标识符的复合列表。
//!
//! # Invariants (不变式约束)
//! * **依赖坐标系稳定**: 进出本图状分析器的所有原始流点 `LayoutRun` 均要求强依赖于 `Y-Down` 的极性取反倒转。反向注入 `Y-Up` 将引发不可逆转排版堆栈灾难。
//! * **单调顺序遍历流**: 基于文档天然阅读惯性，必须符合（纵向优先、侧边其次）的从上至下由左至右。

use pdf_viewer_core::models::{
    BoundingBox, LayoutAlignment, LayoutInferenceResult, LayoutMode, LayoutParagraph, LayoutRole,
    LayoutRun, ParagraphStyle, SemanticRegion, SemanticRole,
};

use super::spatial_graph::SpatialGraph;

/// 基于物理渲染图网解析语义流的核心门面调度器。管理特定单页界限下的全套执行上下文。
///
/// # Thread Safety & Memory Cost (内存释放与存留界线)
/// 整个分析周期建立在不可重入的一次性快照闭包环境。
/// 算法执行期间会在栈和专用区域爆发性分配极大规模的中间表结构图（通过 `Vec` 收拾 Graph），
/// `resolve_regions` 折叠计算完毕后，图的实例应伴随其生命周期主动走向销毁，不存在跨长效异步挂起的驻留池。
pub struct LayoutGraphAnalyzer {
    pub page_index: u16,
    pub width: f32,
    pub height: f32,
}

impl LayoutGraphAnalyzer {
    /// 注入页面的几何基本描述参数构建解析调度器上下文。
    pub fn new(page_index: u16, width: f32, height: f32) -> Self {
        Self {
            page_index,
            width,
            height,
        }
    }

    /// 【调度图神经骨干的核心管道】分析原始离散游程列表，重建高维页面语义（Region 与 Paragraph的父子级嵌套块结构）。
    ///
    /// # Arguments
    /// * `runs` - 源 PDF 内该页面被抽取的无状态墨迹段 (Layout Runs)。
    ///
    /// # Returns
    /// 经过结构封箱的 `LayoutInferenceResult` 装载，映射出阅读型页面的骨干（如多栏结构、列表结构）。
    pub fn resolve_regions(&self, runs: Vec<LayoutRun>) -> LayoutInferenceResult {
        let mut graph = SpatialGraph::new(runs.clone());
        // 模式识别的第一步：建立空间引力邻域 (以物理邻边测算距离投射线)
        // [V4.1] 降低水平阈值 (从 30.0 降到 8.0)，防止跨列合并
        graph.build_adjacency(8.0, 16.0);

        let components = graph.find_components();
        let mut regions = Vec::new();

        for (idx, run_indices) in components.into_iter().enumerate() {
            let mut component_runs: Vec<LayoutRun> = run_indices
                .iter()
                .map(|&i| runs[i].clone())
                .collect();

            // 阅读流强制对齐：基于标准的视觉从上至下扫描
            component_runs.sort_by(|a, b| {
                a.bbox.top.partial_cmp(&b.bbox.top).unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.bbox.left.partial_cmp(&b.bbox.left).unwrap_or(std::cmp::Ordering::Equal))
            });

            // 【模式判定】：探测这串合并好的几何簇符合哪类结构
            let (role, mode) = self.detect_layout_pattern(&component_runs);

            let region = self.create_semantic_region(
                format!("v4-node-{}", idx),
                role,
                mode,
                component_runs,
            );
            regions.push(region);
        }

        LayoutInferenceResult {
            page_index: self.page_index,
            width: self.width,
            height: self.height,
            regions,
            column_bands: vec![],
        }
    }

    fn detect_layout_pattern(&self, runs: &[LayoutRun]) -> (LayoutRole, LayoutMode) {
        if runs.is_empty() {
            return (LayoutRole::Paragraph, LayoutMode::Flow);
        }

        let first = &runs[0];
        let text: String = runs.iter().map(|r| r.text.as_str()).collect();
        let trimmed = text.trim();

        // 1. 结构化对模式 (Label-Value Pattern)
        if trimmed.contains(':') || trimmed.contains('：') {
            return (LayoutRole::KvField, LayoutMode::Anchored);
        }

        // 2. 标题模式 (Header Pattern)
        if first.style.font_size > 15.0 || first.style.is_bold {
            if trimmed.chars().count() < 20 {
                return (LayoutRole::SectionHeader, LayoutMode::Fixed);
            }
        }

        // 3. 列表项模式 (List Pattern)
        if trimmed.starts_with('•') || trimmed.starts_with('·')
            || (trimmed.chars().next().map_or(false, |c| c.is_digit(10)) && trimmed.contains('.'))
        {
            return (LayoutRole::ListItem, LayoutMode::Flow);
        }

        // 4. 默认安全回退态 (Paragraph Mode)
        (LayoutRole::Paragraph, LayoutMode::Flow)
    }

    fn create_semantic_region(
        &self,
        id: String,
        kind: LayoutRole,
        layout_mode: LayoutMode,
        runs: Vec<LayoutRun>,
    ) -> SemanticRegion {
        let mut sorted_runs = runs;

        sorted_runs.sort_by(|a, b| {
            a.bbox.top.partial_cmp(&b.bbox.top).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.bbox.left.partial_cmp(&b.bbox.left).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut bbox = BoundingBox {
            left: f32::INFINITY,
            top: f32::INFINITY,
            right: f32::NEG_INFINITY,
            bottom: f32::NEG_INFINITY,
        };

        for run in &sorted_runs {
            bbox.left = bbox.left.min(run.bbox.left);
            bbox.right = bbox.right.max(run.bbox.right);
            bbox.top = bbox.top.min(run.bbox.top);
            bbox.bottom = bbox.bottom.max(run.bbox.bottom);
        }

        if bbox.left >= bbox.right || bbox.top >= bbox.bottom {
            bbox = BoundingBox::default();
        }

        let paragraph = LayoutParagraph {
            id: format!("v4-p-{}", id),
            bbox: bbox.clone(),
            style: ParagraphStyle {
                align: LayoutAlignment::Left,
                line_height: 1.2,
                first_line_indent: 0.0,
                left_indent: 0.0,
                tab_stops: vec![],
            },
            runs: sorted_runs,
            object_ids: vec![],
            origin_x: bbox.left,
            origin_y: bbox.top,
            wrap_width: (bbox.right - bbox.left).max(0.0),
        };

        SemanticRegion {
            id,
            kind,
            layout_mode,
            bbox: bbox.clone(),
            paragraphs: vec![paragraph],
            semantic_role: SemanticRole::None,
            object_ids: vec![],
        }
    }
}
