//! 段落编辑器场景 — 数据结构与构建函数。
//!
//! **架构原则：document_plan 是唯一状态源。**
//! scene.body_session / scene.marker / scene.original_runs 是序列化兼容字段，
//! 运行时所有读取必须通过 accessor 方法走 document_plan，消除状态分叉。

use crate::edit::document_plan::{
    from_paragraph, from_target_id, EditContext, ParagraphEditorMarker,
};
use crate::models::{
    BoundingBox, GlyphPaintParagraph, GlyphPaintRun, LayoutParagraph, ParagraphEditContext,
    SemanticBlock, SemanticListItem, VectorPageModel,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct ParagraphEditorScene {
    pub target_id: String,
    pub base_paragraph_id: String,
    pub shell_bbox: BoundingBox,
    /// 唯一状态源 — 运行时所有读取必须走这里
    pub document_plan: EditContext,
}

impl Serialize for ParagraphEditorScene {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ParagraphEditorScene", 4)?;
        state.serialize_field("targetId", &self.target_id)?;
        state.serialize_field("baseParagraphId", &self.base_paragraph_id)?;
        state.serialize_field("shellBbox", &self.shell_bbox)?;
        state.serialize_field("documentPlan", &self.document_plan)?;
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParagraphEditorSceneHelper {
    #[serde(default)]
    target_id: String,
    #[serde(default)]
    base_paragraph_id: String,
    shell_bbox: BoundingBox,
    document_plan: Option<EditContext>,
    body_text: Option<String>,
    body_session: Option<ParagraphEditContext>,
    body_initial_caret: Option<usize>,
    marker: Option<ParagraphEditorMarker>,
    original_runs: Option<Vec<GlyphPaintRun>>,
}

impl<'de> Deserialize<'de> for ParagraphEditorScene {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = ParagraphEditorSceneHelper::deserialize(deserializer)?;

        let document_plan = if let Some(plan) = helper.document_plan {
            plan
        } else {
            EditContext {
                target_id: helper.target_id.clone(),
                base_paragraph_id: helper.base_paragraph_id.clone(),
                shell_bbox: helper.shell_bbox,
                body_session: helper.body_session.unwrap_or_else(|| ParagraphEditContext {
                    anchor_bbox: helper.shell_bbox,
                    paragraph: LayoutParagraph::default(),
                }),
                source_body_text: helper.body_text.unwrap_or_default(),
                body_text_plan: crate::text::glyph_layout::EditorSessionTextPlan::default(),
                draft_template_run: crate::models::LayoutRun::default(),
                body_lines: Vec::new(),
                body_initial_caret: helper.body_initial_caret.unwrap_or(0),
                marker: helper.marker,
                graphic_markers: Vec::new(),
                original_runs: helper.original_runs.unwrap_or_default(),
            }
        };

        Ok(ParagraphEditorScene {
            target_id: helper.target_id,
            base_paragraph_id: helper.base_paragraph_id,
            shell_bbox: helper.shell_bbox,
            document_plan,
        })
    }
}

impl ParagraphEditorScene {
    // ── 统一 accessor：所有读取走 document_plan ──

    pub fn body_session(&self) -> &ParagraphEditContext {
        &self.document_plan.body_session
    }

    pub fn body_session_mut(&mut self) -> &mut ParagraphEditContext {
        &mut self.document_plan.body_session
    }

    pub fn body_text(&self) -> &str {
        self.document_plan.source_body_text()
    }

    pub fn body_initial_caret(&self) -> usize {
        self.document_plan.body_initial_caret
    }

    pub fn set_body_initial_caret(&mut self, caret: usize) {
        self.document_plan.body_initial_caret = caret;
    }

    pub fn marker(&self) -> Option<&ParagraphEditorMarker> {
        self.document_plan.marker.as_ref()
    }

    pub fn marker_mut(&mut self) -> &mut Option<ParagraphEditorMarker> {
        &mut self.document_plan.marker
    }

    pub fn original_runs(&self) -> &[GlyphPaintRun] {
        &self.document_plan.original_runs
    }

    pub fn graphic_markers(&self) -> &[crate::models::VisualMarker] {
        &self.document_plan.graphic_markers
    }

    pub fn semantic_block(&self) -> SemanticBlock {
        self.document_plan.semantic_block()
    }

    pub fn semantic_list_item(&self) -> Option<SemanticListItem> {
        self.semantic_block().list_item_ref().cloned()
    }
}

impl Default for ParagraphEditorScene {
    fn default() -> Self {
        Self {
            target_id: String::new(),
            base_paragraph_id: String::new(),
            shell_bbox: BoundingBox::default(),
            document_plan: EditContext::default(),
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
        document_plan,
    })
}

pub fn build_paragraph_editor_scene(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    click_page_point: Option<(f32, f32)>,
) -> Option<ParagraphEditorScene> {
    let document_plan: EditContext = from_paragraph(paragraph, vector_model, click_page_point)?;
    paragraph_editor_scene_from_plan(document_plan)
}

pub fn build_target_scene(
    paragraph: &GlyphPaintParagraph,
    vector_model: Option<&VectorPageModel>,
    target_id: &str,
    click_page_point: Option<(f32, f32)>,
) -> Option<ParagraphEditorScene> {
    let document_plan: EditContext =
        from_target_id(paragraph, vector_model, target_id, click_page_point)?;
    paragraph_editor_scene_from_plan(document_plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::document_plan::{EditContext, ParagraphEditorMarker};
    use crate::models::BoundingBox;
    use crate::text::list_semantics::ListMarkerKind;

    #[test]
    fn test_paragraph_editor_scene_serde_roundtrip() {
        let mut plan = EditContext::default();
        plan.target_id = "test_target".to_string();
        plan.base_paragraph_id = "test_base".to_string();
        plan.source_body_text = "Hello world".to_string();
        plan.body_initial_caret = 5;
        plan.marker = Some(ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 10.0,
            runs: Vec::new(),
        });

        let scene = ParagraphEditorScene {
            target_id: plan.target_id.clone(),
            base_paragraph_id: plan.base_paragraph_id.clone(),
            shell_bbox: BoundingBox {
                left: 0.0,
                top: 0.0,
                right: 100.0,
                bottom: 50.0,
            },
            document_plan: plan,
        };

        // Serialize to JSON string
        let json_str = serde_json::to_string(&scene).unwrap();

        // Assert that the JSON contains documentPlan (single source of truth)
        assert!(json_str.contains("\"documentPlan\""));
        // Flat fields should NOT appear at the top level — they are only inside documentPlan.
        // Verify by parsing the JSON and checking top-level keys.
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let obj = parsed.as_object().unwrap();
        assert!(
            obj.contains_key("documentPlan"),
            "documentPlan must be present"
        );
        assert!(
            !obj.contains_key("bodyText"),
            "bodyText must not be a top-level key"
        );
        assert!(
            !obj.contains_key("bodySession"),
            "bodySession must not be a top-level key"
        );
        assert!(
            !obj.contains_key("bodyInitialCaret"),
            "bodyInitialCaret must not be a top-level key"
        );
        assert!(
            !obj.contains_key("marker"),
            "marker must not be a top-level key"
        );
        assert!(
            !obj.contains_key("originalRuns"),
            "originalRuns must not be a top-level key"
        );
        // Verify data via documentPlan content
        assert!(json_str.contains("\"bodyInitialCaret\":5"));
        // Deserialize back
        let deserialized: ParagraphEditorScene = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.target_id, "test_target");
        assert_eq!(deserialized.base_paragraph_id, "test_base");
        assert_eq!(deserialized.document_plan.source_body_text(), "Hello world");
        assert_eq!(deserialized.document_plan.body_initial_caret, 5);
        assert_eq!(
            deserialized.document_plan.marker.as_ref().unwrap().text,
            "•"
        );
    }

    #[test]
    fn test_paragraph_editor_scene_deserialize_flat_only() {
        // Flat-only JSON representation mimicking legacy format
        let legacy_json = r#"{
            "targetId": "legacy_target",
            "baseParagraphId": "legacy_base",
            "shellBbox": {"left": 0.0, "top": 0.0, "right": 100.0, "bottom": 50.0},
            "bodyText": "Legacy flat text",
            "bodyInitialCaret": 3,
            "bodySession": {
                "anchorBbox": {"left": 0.0, "top": 0.0, "right": 100.0, "bottom": 50.0},
                "paragraph": {
                    "id": "legacy_para",
                    "bbox": {"left": 0.0, "top": 0.0, "right": 100.0, "bottom": 50.0},
                    "style": {
                        "align": "LEFT",
                        "lineHeight": 12.0,
                        "firstLineIndent": 0.0,
                        "leftIndent": 0.0,
                        "tabStops": []
                    },
                    "runs": []
                }
            },
            "marker": {
                "kind": "BULLET",
                "text": "•",
                "advance": 12.5,
                "runs": []
            },
            "originalRuns": []
        }"#;

        let deserialized: ParagraphEditorScene = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(deserialized.target_id, "legacy_target");
        assert_eq!(deserialized.base_paragraph_id, "legacy_base");
        assert_eq!(
            deserialized.document_plan.source_body_text(),
            "Legacy flat text"
        );
        assert_eq!(deserialized.document_plan.body_initial_caret, 3);
        assert_eq!(
            deserialized.document_plan.marker.as_ref().unwrap().text,
            "•"
        );
        assert_eq!(
            deserialized.document_plan.marker.as_ref().unwrap().advance,
            12.5
        );
    }
}
