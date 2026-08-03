//! 编辑器活动目标数据 — 从 ui::editor::session::session 迁入纯数据部分。
//! thread_local / state 管理仍位于 ui 侧。

use crate::edit::paragraph_scene::ParagraphEditorScene;
use crate::models::{BoundingBox, LayoutParagraph, ParagraphEditContext, SemanticBlock};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEditorTarget {
    pub paragraph_id: String,
    pub region_id: String,
    pub page_index: u16,
    pub text: String,
    pub bbox_left: f32,
    pub bbox_top: f32,
    pub bbox_right: f32,
    pub bbox_bottom: f32,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: String,
    pub font_style: String,
    pub color: String,
    #[serde(default)]
    pub text_decoration: String,
    #[serde(default)]
    pub initial_caret_index: usize,
    pub editor_session: ParagraphEditContext,
    #[serde(default)]
    pub scene: ParagraphEditorScene,
}

impl Default for ActiveEditorTarget {
    fn default() -> Self {
        Self {
            paragraph_id: String::new(),
            region_id: String::new(),
            page_index: 0,
            text: String::new(),
            bbox_left: 0.0,
            bbox_top: 0.0,
            bbox_right: 0.0,
            bbox_bottom: 0.0,
            font_family: String::new(),
            font_size: 0.0,
            font_weight: String::new(),
            font_style: String::new(),
            color: String::new(),
            text_decoration: String::new(),
            initial_caret_index: 0,
            editor_session: ParagraphEditContext {
                anchor_bbox: BoundingBox::default(),
                paragraph: LayoutParagraph::default(),
            },
            scene: ParagraphEditorScene::default(),
        }
    }
}

impl ActiveEditorTarget {
    pub fn source_body_text(&self) -> &str {
        self.scene.body_text()
    }

    pub fn initial_body_caret_index(&self) -> usize {
        self.scene.body_initial_caret()
    }

    pub fn semantic_block(&self) -> SemanticBlock {
        self.scene.semantic_block()
    }
}
