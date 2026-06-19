//! 段落编辑器场景 — 数据结构与构建函数。

use crate::edit::document_plan::{
    from_paragraph, from_target_id, EditContext,
    ParagraphEditorMarker,
};
use crate::models::{
    BoundingBox, GlyphPaintParagraph, GlyphPaintRun, LayoutParagraph, ParagraphEditContext,
    VectorPageModel,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphEditorScene {
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub base_paragraph_id: String,
    pub shell_bbox: BoundingBox,
    pub document_plan: EditContext,
    pub body_text: String,
    pub body_session: ParagraphEditContext,
    #[serde(default)]
    pub body_initial_caret: usize,
    #[serde(default)]
    pub marker: Option<ParagraphEditorMarker>,
    #[serde(default)]
    pub original_runs: Vec<GlyphPaintRun>,
}

impl Default for ParagraphEditorScene {
    fn default() -> Self {
        Self {
            target_id: String::new(),
            base_paragraph_id: String::new(),
            shell_bbox: BoundingBox::default(),
            document_plan: EditContext::default(),
            body_text: String::new(),
            body_session: ParagraphEditContext {
                anchor_bbox: BoundingBox::default(),
                paragraph: LayoutParagraph::default(),
            },
            body_initial_caret: 0,
            marker: None,
            original_runs: Vec::new(),
        }
    }
}

/// 从 EditContext 构造 ParagraphEditorScene（纯数据组装，无副作用）。
pub fn paragraph_editor_scene_from_plan(
    document_plan: EditContext,
) -> Option<ParagraphEditorScene> {
    Some(ParagraphEditorScene {
        target_id: document_plan.target_id.clone(),
        base_paragraph_id: document_plan.base_paragraph_id.clone(),
        shell_bbox: document_plan.shell_bbox,
        body_text: document_plan.source_body_text().to_string(),
        body_session: document_plan.body_session.clone(),
        body_initial_caret: document_plan.body_initial_caret,
        marker: document_plan.marker.clone(),
        original_runs: document_plan.original_runs.clone(),
        document_plan,
    })
}

pub fn build_paragraph_editor_scene(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    click_page_point: Option<(f32, f32)>,
) -> Option<ParagraphEditorScene> {
    let document_plan: EditContext =
        from_paragraph(paragraph, vector_model, click_page_point)?;
    paragraph_editor_scene_from_plan(document_plan)
}

pub fn build_target_scene(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    target_id: &str,
    click_page_point: Option<(f32, f32)>,
) -> Option<ParagraphEditorScene> {
    let document_plan: EditContext = from_target_id(
        paragraph,
        vector_model,
        target_id,
        click_page_point,
    )?;
    paragraph_editor_scene_from_plan(document_plan)
}
