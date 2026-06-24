use wasm_bindgen::prelude::JsValue;

use super::target_resolution::{is_supported_region_kind, resolve_region_target};
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::list_format::resolve_marker_text;
use crate::editor::orchestrator::commit::commit_pending;
use crate::editor::replacement_snapshot::build_edit_replacement_snapshot;
use crate::editor::session::{
    paragraph_id, is_edit_enabled, open_paragraph_editor,
    set_caret, sync_input, ActiveEditorInputSyncResult,
};
use crate::editor::workflow::{
    build_interaction_targets,
    build_text_patch as workflow_build_text_patch,
    open_paragraph_editor as workflow_open_paragraph_editor, resolve_shell_bbox,
};
use pdf_viewer_core::persistence::models::PersistableRegionPatch;
use crate::page::page_store::with_page_state;
use crate::common::sanitize::sanitize_positive;
use crate::zoom::zoom_store;
use pdf_viewer_core::models::BoundingBox;
use pdf_viewer_core::text::list_semantics::ListMarkerKind;

fn summarize_object_ids<'a>(ids: impl Iterator<Item = &'a String>) -> String {
    let ids = ids.take(6).map(|id| id.as_str()).collect::<Vec<_>>();
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.join(",")
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorVisibilityAction {
    pub changed: bool,
    pub request_visibility_render: bool,
}

pub use crate::editor::editor_format::{
    format_state, apply_format, ActiveEditorFormatState,
    EditorFormatAction,
};

pub fn collect_paragraph_targets() -> JsValue {
    let editing_enabled = is_edit_enabled();
    with_page_state(|state| build_interaction_targets(state, editing_enabled))
}

pub fn open_at_point(
    target_paragraph_id: &str,
    click_page_x: f32,
    click_page_y: f32,
) -> EditorVisibilityAction {
    // 切换段落 / 同段落点击重新定位光标 — 都会替换 live_state。
    // 替换前必须 commit 旧的 dirty edit，避免编辑丢失。
    // 见 docs/edit-save-architecture.md §4.1。
    let prev_paragraph_id = paragraph_id();
    if prev_paragraph_id.as_deref() != Some(target_paragraph_id) {
        let committed = commit_pending();
        crate::chain_trace!(
            "open.flush-prev",
            "prev" => prev_paragraph_id.as_deref().unwrap_or(""),
            "next" => target_paragraph_id,
            "committed" => committed,
        );
    }
    let zoom = zoom_store::with_zoom_state(|state| sanitize_positive(state.visual_zoom, 1.0));
    let active_target = with_page_state(|state| {
        workflow_open_paragraph_editor(state, target_paragraph_id, click_page_x, click_page_y, zoom)
    });
    let Some(mut active_target) = active_target else {
        dbg_event(
            "open.runtime",
            "target-not-found",
            vec![
                dbg_field("paragraphId", target_paragraph_id),
                dbg_field("clickPageX", click_page_x),
                dbg_field("clickPageY", click_page_y),
            ],
        );
        return EditorVisibilityAction::default();
    };

    // 统一首次点击 (Open) 与后续点击 (Move) 的光标解析路径：
    // 用 Move 路径同款的 `caret_at_page_point` 重新计算 body_initial_caret，
    // 确保 caret stop 构造算法一致（均基于 build_unified_draft_caret_lines），
    // 修复首次点击位置偏差、后续点击位置准确的不对称问题。
    {
        let source_text = active_target.source_body_text().to_string();
        let unified_caret = crate::editor::text_geometry::caret_at_page_point(
            &active_target,
            &source_text,
            click_page_x,
            click_page_y,
        );
        active_target.initial_caret_index = unified_caret;
        active_target.scene.document_plan.body_initial_caret = unified_caret;
    }

    let body_object_id_count = active_target
        .scene
        .body_session()
        .paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_ids.iter())
        .count();
    let original_object_id_count = active_target
        .scene
        .original_runs()
        .iter()
        .flat_map(|run| run.object_ids.iter())
        .count();
    let body_object_ids = summarize_object_ids(
        active_target
            .scene
            .body_session()
            .paragraph
            .runs
            .iter()
            .flat_map(|run| run.object_ids.iter()),
    );
    let original_object_ids = summarize_object_ids(
        active_target
            .scene
            .original_runs()
            .iter()
            .flat_map(|run| run.object_ids.iter()),
    );
    dbg_event(
        "open.runtime",
        "target-built",
        vec![
            dbg_field("paragraphId", target_paragraph_id),
            dbg_field("targetId", active_target.paragraph_id.as_str()),
            dbg_field("clickPageX", click_page_x),
            dbg_field("clickPageY", click_page_y),
            dbg_field("sourceText", active_target.source_body_text()),
            dbg_field(
                "initialCaretIndex",
                active_target.initial_body_caret_index(),
            ),
            dbg_field(
                "bodyCharCount",
                active_target.scene.document_plan.body_char_count(),
            ),
            dbg_field(
                "slotCount",
                active_target.scene.document_plan.body_text_plan.slots.len(),
            ),
            dbg_field(
                "lineCount",
                active_target.scene.document_plan.body_lines.len(),
            ),
            dbg_field("targetColor", active_target.color.as_str()),
            dbg_field(
                "targetTextDecoration",
                active_target.text_decoration.as_str(),
            ),
            dbg_field(
                "targetWidth",
                active_target.bbox_right - active_target.bbox_left,
            ),
            dbg_field(
                "targetHeight",
                active_target.bbox_bottom - active_target.bbox_top,
            ),
            dbg_field(
                "targetAlignment",
                format!(
                    "{:?}",
                    active_target.scene.body_session().paragraph.style.align
                ),
            ),
            dbg_field(
                "targetListKind",
                active_target
                    .scene
                    .marker()
                    .map(|marker| format!("{:?}", marker.kind))
                    .unwrap_or_else(|| "None".to_string()),
            ),
            dbg_field("bodyObjectIdCount", body_object_id_count),
            dbg_field("originalObjectIdCount", original_object_id_count),
            dbg_field("bodyObjectIds", body_object_ids),
            dbg_field("originalObjectIds", original_object_ids),
            dbg_field(
                "bodyBBox",
                format!(
                    "[{:.2},{:.2},{:.2},{:.2}]",
                    active_target.scene.body_session().anchor_bbox.left,
                    active_target.scene.body_session().anchor_bbox.top,
                    active_target.scene.body_session().anchor_bbox.right,
                    active_target.scene.body_session().anchor_bbox.bottom
                ),
            ),
        ],
    );

    let opened = open_paragraph_editor(active_target.paragraph_id.clone(), active_target);
    EditorVisibilityAction {
        changed: opened,
        request_visibility_render: opened,
    }
}

pub fn build_text_patch(
    page_index: u16,
    region_id: &str,
    kind: &str,
    original_text: &str,
    new_text: String,
) -> Option<PersistableRegionPatch> {
    with_page_state(|state| {
        workflow_build_text_patch(
            state,
            page_index,
            region_id,
            kind,
            original_text,
            new_text,
        )
    })
}

pub fn open_region_editor(
    page_index: u16,
    region_id: &str,
    kind: &str,
    original_text: &str,
) -> EditorVisibilityAction {
    if !is_supported_region_kind(kind) {
        return EditorVisibilityAction::default();
    }

    // 与 open_at_point 同样的不变量：替换 live_state 前先 commit 旧 dirty。
    let prev_paragraph_id = paragraph_id();
    if prev_paragraph_id.as_deref() != Some(region_id) {
        let committed = commit_pending();
        crate::chain_trace!(
            "open-region.flush-prev",
            "prev" => prev_paragraph_id.as_deref().unwrap_or(""),
            "next" => region_id,
            "committed" => committed,
        );
    }

    let zoom = zoom_store::with_zoom_state(|state| sanitize_positive(state.visual_zoom, 1.0));
    let active_target = with_page_state(|state| {
        let target = resolve_region_target(
            state,
            page_index,
            region_id,
            kind,
            original_text,
        )?;
        workflow_open_paragraph_editor(
            state,
            &target.paragraph_id,
            target.bbox.left + ((target.bbox.right - target.bbox.left).max(0.0) * 0.5),
            target.bbox.top + ((target.bbox.bottom - target.bbox.top).max(0.0) * 0.5),
            zoom,
        )
    });

    let Some(active_target) = active_target else {
        return EditorVisibilityAction::default();
    };

    let opened = open_paragraph_editor(active_target.paragraph_id.clone(), active_target);
    EditorVisibilityAction {
        changed: opened,
        request_visibility_render: opened,
    }
}

pub fn build_patch(new_text: String) -> Option<PersistableRegionPatch> {
    let active_state = crate::editor::session::active_editor_state()?;
    let new_runs = if active_state.has_style_changes() {
        Some(active_state.draft_runs())
    } else {
        None
    };
    with_page_state(|page_state| {
        let paragraph_id = paragraph_id()?;
        let mut patch = crate::editor::bridge::build_rich_patch(
            page_state.paint_plan.as_ref()?,
            page_state.vector_model.as_ref(),
            &paragraph_id,
            new_text,
            new_runs,
        )?;
        patch.align = Some(active_state.active_alignment());
        patch.line_height =
            if (active_state.active_line_height() - active_state.source_line_height()).abs() > 0.01
            {
                Some(active_state.active_line_height())
            } else {
                None
            };
        patch.new_marker_text = resolve_marker_text(&active_state, &page_state);
        crate::chain_trace!(
            "commit.marker",
            "resolved" => patch.new_marker_text.as_deref().unwrap_or(""),
            "sourceMarker" => patch.marker_text.as_deref().unwrap_or(""),
            "listKind" => format!("{:?}", active_state.active_list_kind()),
        );
        let active_list_kind = active_state.active_list_kind();
        if patch.source == "paragraph-region" && active_list_kind != ListMarkerKind::None {
            patch.source = "list-item-region".to_string();
            patch.kind = Some("list-item".to_string());
            patch.full_target_indices = patch.target_indices.clone();
            crate::chain_trace!(
                "commit.list-convert",
                "listKind" => format!("{:?}", active_list_kind),
                "targetIndicesLen" => patch.target_indices.len(),
            );
        }
        patch.snapshot = Some(build_edit_replacement_snapshot(
            active_state.target.clone(),
            patch.kind.as_deref().unwrap_or("paragraph"),
            patch.new_text.clone(),
            patch.marker_text.clone(),
            patch.new_marker_text.clone(),
        ));
        if patch_is_noop(&active_state, &patch) {
            dbg_event(
                "patch.build",
                "active-editor.noop-suppressed",
                vec![
                    dbg_field("paragraphId", active_state.paragraph_id()),
                    dbg_field("source", patch.source.as_str()),
                    dbg_field("originalText", patch.original_text.as_str()),
                    dbg_field("newText", patch.new_text.as_str()),
                    dbg_field("hasNewRuns", patch.new_runs.is_some()),
                    dbg_field(
                        "sourceAlignment",
                        format!("{:?}", active_state.source_alignment()),
                    ),
                    dbg_field(
                        "activeAlignment",
                        format!("{:?}", active_state.active_alignment()),
                    ),
                    dbg_field(
                        "sourceMarker",
                        active_state.source_marker_text().unwrap_or(""),
                    ),
                    dbg_field("newMarker", patch.new_marker_text.as_deref().unwrap_or("")),
                ],
            );
            return None;
        }
        Some(patch)
    })
}

fn patch_is_noop(
    active_state: &crate::editor::engine_state::LiveEditorParagraphState,
    patch: &PersistableRegionPatch,
) -> bool {
    let text_unchanged = patch.new_text == patch.original_text;
    let style_unchanged = patch.new_runs.is_none();
    let alignment_unchanged = active_state.active_alignment() == active_state.source_alignment();
    let line_height_unchanged = patch
        .line_height
        .map(|line_height| (line_height - active_state.source_line_height()).abs() <= 0.01)
        .unwrap_or(true);
    let source_marker = active_state
        .source_marker_text()
        .unwrap_or("")
        .trim();
    let next_marker = patch
        .new_marker_text
        .as_deref()
        .or(patch.marker_text.as_deref())
        .unwrap_or("")
        .trim();
    let marker_unchanged = source_marker == next_marker;
    let noop = text_unchanged
        && style_unchanged
        && alignment_unchanged
        && line_height_unchanged
        && marker_unchanged;
    crate::chain_trace!(
        "commit.noop-check",
        "noop" => noop,
        "textUnchanged" => text_unchanged,
        "styleUnchanged" => style_unchanged,
        "markerUnchanged" => marker_unchanged,
    );
    noop
}

pub fn find_shell_bbox(paragraph_id: &str) -> Option<BoundingBox> {
    with_page_state(|state| resolve_shell_bbox(state, paragraph_id))
}

pub fn set_editor_caret(caret_index: usize) -> bool {
    set_caret(caret_index)
}

pub fn sync_editor_input(new_text: String, caret_index: usize) -> ActiveEditorInputSyncResult {
    sync_input(new_text, caret_index)
}
