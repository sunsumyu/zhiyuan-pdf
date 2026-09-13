//! 编辑器文档计划 — 数据结构与纯构建函数。
//!
//! 拆分（closeout #06，范式同 canvas/）：类型定义住 mod.rs，逻辑按职责分布：
//! - `geometry`:      bbox 工具 + 图形 marker 检测（vector 对象 bbox、候选判定）
//! - `session_split`: marker/body 会话切分（run 级拆分、symbolic 字体检测、几何合成）
//! - `line_plans`:    body 行计划与草稿模板 run 选择
//! - `plan_builder`:  构建入口（build_editor_document_plan* / collect_*）
//! - `tests`:         原内联测试模块（原样搬移）

#[cfg(test)]
mod tests;

mod geometry;
mod line_plans;
mod plan_builder;
mod session_split;

use crate::models::BoundingBox;
use crate::models::{
    GlyphPaintRun, LayoutParagraph, LayoutRun, ParagraphEditContext, VisualMarker,
};
use crate::text::glyph_layout::EditorSessionTextPlan;
use crate::text::list_semantics::ListMarkerKind;
use serde::{Deserialize, Serialize};

pub use plan_builder::{
    build_editor_document_plan, build_editor_document_plan_for_target,
    build_editor_document_plan_from_session, collect_editor_document_target_plans,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphEditorMarker {
    pub kind: ListMarkerKind,
    pub text: String,
    pub advance: f32,
    #[serde(default)]
    pub runs: Vec<LayoutRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorDocumentPlan {
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub base_paragraph_id: String,
    pub shell_bbox: BoundingBox,
    pub body_session: ParagraphEditContext,
    pub source_body_text: String,
    pub body_text_plan: EditorSessionTextPlan,
    #[serde(default)]
    pub draft_template_run: LayoutRun,
    #[serde(default)]
    pub body_lines: Vec<EditorDocumentLinePlan>,
    #[serde(default)]
    pub body_initial_caret: usize,
    #[serde(default)]
    pub marker: Option<ParagraphEditorMarker>,
    /// 图形 marker 列表（新增）
    /// 存储 VectorPageModel.objects 中的 Image/Path 引用
    /// 用于渲染抑制时跳过有意义的图形 marker
    #[serde(default)]
    pub graphic_markers: Vec<VisualMarker>,
    #[serde(default)]
    pub original_runs: Vec<GlyphPaintRun>,
}

impl Default for EditorDocumentPlan {
    fn default() -> Self {
        Self {
            target_id: String::new(),
            base_paragraph_id: String::new(),
            shell_bbox: BoundingBox::default(),
            body_session: ParagraphEditContext {
                anchor_bbox: BoundingBox::default(),
                paragraph: LayoutParagraph::default(),
            },
            source_body_text: String::new(),
            body_text_plan: EditorSessionTextPlan::default(),
            draft_template_run: LayoutRun::default(),
            body_lines: Vec::new(),
            body_initial_caret: 0,
            marker: None,
            graphic_markers: Vec::new(),
            original_runs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorDocumentLinePlan {
    #[serde(default)]
    pub template_runs: Vec<LayoutRun>,
    #[serde(default)]
    pub source_runs: Vec<LayoutRun>,
    #[serde(default)]
    pub reconstructed_char_count: usize,
}

impl EditorDocumentPlan {
    pub fn source_body_text(&self) -> &str {
        &self.source_body_text
    }

    pub fn body_char_count(&self) -> usize {
        self.source_body_text.chars().count()
    }
}
