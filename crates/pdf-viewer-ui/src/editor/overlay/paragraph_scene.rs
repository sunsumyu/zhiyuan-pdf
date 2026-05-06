use pdf_viewer_core::models::{
    BoundingBox, EditorSession, GlyphPaintParagraph, GlyphPaintRun, VectorPageModel,
};
use serde::{Deserialize, Serialize};

use crate::editor::document_plan::{
    build_editor_document_plan_for_target, build_editor_document_plan, EditorDocumentPlan,
    ParagraphEditorMarker,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphEditorScene {
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub base_paragraph_id: String,
    pub shell_bbox: BoundingBox,
    pub document_plan: EditorDocumentPlan,
    pub body_text: String,
    pub body_session: EditorSession,
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
            document_plan: EditorDocumentPlan::default(),
            body_text: String::new(),
            body_session: EditorSession {
                anchor_bbox: BoundingBox::default(),
                paragraph: pdf_viewer_core::models::LayoutParagraph::default(),
            },
            body_initial_caret: 0,
            marker: None,
            original_runs: Vec::new(),
        }
    }
}

pub fn build_paragraph_editor_scene(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    click_page_point: Option<(f32, f32)>,
) -> Option<ParagraphEditorScene> {
    let document_plan = build_editor_document_plan(paragraph, vector_model, click_page_point)?;
    paragraph_editor_scene_from_plan(document_plan)
}

pub fn build_paragraph_editor_scene_for_target(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    target_id: &str,
    click_page_point: Option<(f32, f32)>,
) -> Option<ParagraphEditorScene> {
    let document_plan = build_editor_document_plan_for_target(
        paragraph,
        vector_model,
        target_id,
        click_page_point,
    )?;
    paragraph_editor_scene_from_plan(document_plan)
}

fn paragraph_editor_scene_from_plan(
    document_plan: EditorDocumentPlan,
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
