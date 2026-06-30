//! 段落渲染覆盖层数据 — 从 ui::editor::overlay::paragraph_overlay 迁入纯数据部分。
//! 构建/状态收集函数仍位于 ui 侧。

use crate::edit::active_target::ActiveEditorTarget;
use crate::models::VisualMarker;

#[derive(Debug, Clone)]
pub enum ParagraphRenderOverlayOwner {
    ActiveEditorShell,
    PersistedPageCanvas,
}

#[derive(Debug, Clone)]
pub struct ParagraphRenderOverlay {
    pub owner: ParagraphRenderOverlayOwner,
    pub target: ActiveEditorTarget,
    pub source_object_indices: Vec<usize>,
    pub graphic_markers: Vec<VisualMarker>,
    pub source_text: String,
    pub draft_text: String,
    pub replaces_source: bool,
    pub marker_text_override: Option<String>,
}
