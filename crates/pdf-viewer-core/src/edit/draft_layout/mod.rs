//! 草稿排版 — EditorDraftRenderPlan 数据结构与构建管线。
//!
//! 拆分（closeout #06，范式同 canvas/）：类型定义住 mod.rs，逻辑按职责分布：
//! - `text_mapping`:  source/runs 字符索引映射与 caret 索引重映射
//! - `styles`:        run 样式规范化、style 选择、diff 切片（build_styles）
//! - `source_layout`: 未编辑态的 source 布局重建
//! - `caret_plan`:    DraftCaretLine/Stop 构建
//! - `plan_builder`:  build_draft_render_plan / build_persisted_overlay_render_plan
//! - `tests`:         原内联测试模块（原样搬移）

#[cfg(test)]
mod tests;

mod caret_plan;
mod plan_builder;
mod source_layout;
mod styles;
mod text_mapping;

pub use plan_builder::{build_draft_render_plan, build_persisted_overlay_render_plan};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftCaretStop {
    pub index: usize,
    pub left: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftCaretLine {
    pub baseline_y: f32,
    pub height: f32,
    pub stops: Vec<DraftCaretStop>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorDraftRenderPlan {
    pub layout: crate::geometry::layout_engine::ParagraphLayout,
    pub caret_lines: Vec<DraftCaretLine>,
}
