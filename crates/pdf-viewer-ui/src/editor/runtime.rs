use wasm_bindgen::prelude::JsValue;

use super::target_resolution::{
    is_supported_region_kind as host_is_supported_region_kind,
    resolve_region_target_from_page_state as host_resolve_region_target_from_page_state,
};
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::commit::commit_pending_edit_if_any;
use crate::editor::list_format::resolve_active_marker_text;
use crate::editor::replacement_snapshot::build_edit_replacement_snapshot;
use crate::editor::session::{
    active_edit_paragraph_id as host_active_edit_paragraph_id,
    is_text_edit_enabled as host_is_text_edit_enabled,
    open_paragraph_editor as host_open_paragraph_editor,
    set_active_editor_caret_index as host_set_active_editor_caret_index,
    sync_active_editor_input as host_sync_active_editor_input, ActiveEditorInputSyncResult,
};
use crate::editor::workflow::{
    build_region_text_patch as host_build_region_text_patch,
    get_paragraph_interaction_targets as host_get_paragraph_interaction_targets,
    open_paragraph_editor as host_open_paragraph_editor_workflow,
    resolve_paragraph_shell_bbox as host_resolve_paragraph_shell_bbox_workflow,
};
use crate::models::PersistableRegionPatch;
use crate::page::runtime::HOST_PAGE_STATE;
use crate::utils::sanitize::sanitize_positive;
use crate::zoom::state::HOST_ZOOM_STATE;
use pdf_viewer_core::list_semantics::ListMarkerKind;
use pdf_viewer_core::models::BoundingBox;
use pdf_viewer_core::models::LayoutAlignment;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEditorFormatState {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: String,
    pub font_family: String,
    pub font_size: f32,
    pub char_spacing: f32,
    pub line_height: f32,
    pub paragraph_mode: String,
    pub alignment: String,
    pub list_kind: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EditorFormatAction {
    ToggleBold,
    ToggleItalic,
    ToggleUnderline,
    IncreaseFontSize,
    DecreaseFontSize,
    SetParagraphMode { mode: String },
    SetColor { color: String },
    SetFontFamily { font_family: String },
    SetFontSize { font_size: f32 },
    SetCharSpacing { char_spacing: f32 },
    SetLineHeight { line_height: f32 },
    SetAlignment { alignment: String },
    SetListKind { list_kind: String },
}

fn build_active_editor_format_state(
    live_state: &crate::editor::engine_state::LiveEditorParagraphState,
    changed: bool,
) -> ActiveEditorFormatState {
    ActiveEditorFormatState {
        bold: live_state.is_bold_active(),
        italic: live_state.is_italic_active(),
        underline: live_state.is_underline_active(),
        color: live_state.active_color(),
        font_family: live_state.active_font_family(),
        font_size: live_state.active_font_size(),
        char_spacing: live_state.active_char_spacing(),
        line_height: live_state.active_line_height(),
        paragraph_mode: live_state.active_paragraph_mode_label(),
        alignment: live_state.active_alignment_label(),
        list_kind: live_state.active_list_kind_label(),
        changed,
    }
}

fn resolve_font_size_step(current: f32, increase: bool) -> f32 {
    const STEPS: [f32; 14] = [
        8.0, 9.0, 10.0, 10.5, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0,
    ];
    let normalized = current.clamp(1.0, 288.0);
    if increase {
        STEPS
            .into_iter()
            .find(|step| *step > normalized + 0.01)
            .unwrap_or_else(|| (normalized + 1.0).min(288.0))
    } else {
        STEPS
            .into_iter()
            .rev()
            .find(|step| *step < normalized - 0.01)
            .unwrap_or_else(|| (normalized - 1.0).max(1.0))
    }
}

fn parse_alignment(value: &str) -> Option<LayoutAlignment> {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" => Some(LayoutAlignment::Left),
        "center" => Some(LayoutAlignment::Center),
        "right" => Some(LayoutAlignment::Right),
        "justify" => Some(LayoutAlignment::Justify),
        _ => None,
    }
}

fn parse_list_kind(value: &str) -> Option<ListMarkerKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Some(ListMarkerKind::None),
        "bullet" => Some(ListMarkerKind::Bullet),
        "numbering" => Some(ListMarkerKind::Numbering),
        "symbol" => Some(ListMarkerKind::Symbol),
        "custom" => Some(ListMarkerKind::Custom),
        _ => None,
    }
}

pub fn collect_paragraph_targets() -> JsValue {
    let editing_enabled = host_is_text_edit_enabled();
    HOST_PAGE_STATE.with(|state: &std::cell::RefCell<pdf_viewer_core::models::PageState>| host_get_paragraph_interaction_targets(&state.borrow(), editing_enabled))
}

pub fn open_editor_at_page_point(
    paragraph_id: &str,
    click_page_x: f32,
    click_page_y: f32,
) -> EditorVisibilityAction {
    // 切换段落 / 同段落点击重新定位光标 — 都会替换 live_state。
    // 替换前必须 commit 旧的 dirty edit，避免编辑丢失。
    // 见 docs/edit-save-architecture.md §4.1。
    let prev_paragraph_id = host_active_edit_paragraph_id();
    if prev_paragraph_id.as_deref() != Some(paragraph_id) {
        let committed = commit_pending_edit_if_any();
        crate::chain_trace!(
            "open.flush-prev",
            "prev" => prev_paragraph_id.as_deref().unwrap_or(""),
            "next" => paragraph_id,
            "committed" => committed,
        );
    }
    let zoom = HOST_ZOOM_STATE.with(|state| sanitize_positive(state.borrow().visual_zoom, 1.0));
    let active_target = HOST_PAGE_STATE.with(|state: &std::cell::RefCell<pdf_viewer_core::models::PageState>| {
        host_open_paragraph_editor_workflow(
            &state.borrow(),
            paragraph_id,
            click_page_x,
            click_page_y,
            zoom,
        )
    });
    let Some(active_target) = active_target else {
        dbg_event(
            "open.runtime",
            "target-not-found",
            vec![
                dbg_field("paragraphId", paragraph_id),
                dbg_field("clickPageX", click_page_x),
                dbg_field("clickPageY", click_page_y),
            ],
        );
        return EditorVisibilityAction::default();
    };
    let body_object_id_count = active_target
        .scene
        .body_session
        .paragraph
        .runs
        .iter()
        .flat_map(|run| run.object_ids.iter())
        .count();
    let original_object_id_count = active_target
        .scene
        .original_runs
        .iter()
        .flat_map(|run| run.object_ids.iter())
        .count();
    let body_object_ids = summarize_object_ids(
        active_target
            .scene
            .body_session
            .paragraph
            .runs
            .iter()
            .flat_map(|run| run.object_ids.iter()),
    );
    let original_object_ids = summarize_object_ids(
        active_target
            .scene
            .original_runs
            .iter()
            .flat_map(|run| run.object_ids.iter()),
    );
    dbg_event(
        "open.runtime",
        "target-built",
        vec![
            dbg_field("paragraphId", paragraph_id),
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
                    active_target.scene.body_session.paragraph.style.align
                ),
            ),
            dbg_field(
                "targetListKind",
                active_target
                    .scene
                    .marker
                    .as_ref()
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
                    active_target.scene.body_session.anchor_bbox.left,
                    active_target.scene.body_session.anchor_bbox.top,
                    active_target.scene.body_session.anchor_bbox.right,
                    active_target.scene.body_session.anchor_bbox.bottom
                ),
            ),
        ],
    );

    let opened = host_open_paragraph_editor(active_target.paragraph_id.clone(), active_target);
    EditorVisibilityAction {
        changed: opened,
        request_visibility_render: opened,
    }
}

pub fn build_region_text_patch(
    page_index: u16,
    region_id: &str,
    kind: &str,
    original_text: &str,
    new_text: String,
) -> Option<PersistableRegionPatch> {
    HOST_PAGE_STATE.with(|state: &std::cell::RefCell<pdf_viewer_core::models::PageState>| {
        host_build_region_text_patch(
            &state.borrow(),
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
    if !host_is_supported_region_kind(kind) {
        return EditorVisibilityAction::default();
    }

    // 与 open_editor_at_page_point 同样的不变量：替换 live_state 前先 commit 旧 dirty。
    let prev_paragraph_id = host_active_edit_paragraph_id();
    if prev_paragraph_id.as_deref() != Some(region_id) {
        let committed = commit_pending_edit_if_any();
        crate::chain_trace!(
            "open-region.flush-prev",
            "prev" => prev_paragraph_id.as_deref().unwrap_or(""),
            "next" => region_id,
            "committed" => committed,
        );
    }

    let zoom = HOST_ZOOM_STATE.with(|state| sanitize_positive(state.borrow().visual_zoom, 1.0));
    let active_target = HOST_PAGE_STATE.with(|state: &std::cell::RefCell<pdf_viewer_core::models::PageState>| {
        let state = state.borrow();
        let target = host_resolve_region_target_from_page_state(
            &state,
            page_index,
            region_id,
            kind,
            original_text,
        )?;
        host_open_paragraph_editor_workflow(
            &state,
            &target.paragraph_id,
            target.bbox.left + ((target.bbox.right - target.bbox.left).max(0.0) * 0.5),
            target.bbox.top + ((target.bbox.bottom - target.bbox.top).max(0.0) * 0.5),
            zoom,
        )
    });

    let Some(active_target) = active_target else {
        return EditorVisibilityAction::default();
    };

    let opened = host_open_paragraph_editor(active_target.paragraph_id.clone(), active_target);
    EditorVisibilityAction {
        changed: opened,
        request_visibility_render: opened,
    }
}

pub fn build_active_editor_patch(new_text: String) -> Option<PersistableRegionPatch> {
    let active_state = crate::editor::session::active_editor_state()?;
    let new_runs = if active_state.has_style_changes() {
        Some(active_state.draft_runs())
    } else {
        None
    };
    HOST_PAGE_STATE.with(|state: &std::cell::RefCell<pdf_viewer_core::models::PageState>| {
        let page_state = state.borrow();
        let paragraph_id = host_active_edit_paragraph_id()?;
        let mut patch = crate::editor::bridge::build_paragraph_patch_with_runs(
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
        patch.new_marker_text = resolve_active_marker_text(&active_state, &page_state);
        web_sys::console::log_1(&format!(
            "[AREN_LIST-MARKER] resolved='{}' source_marker='{}' listKind={:?}",
            patch.new_marker_text.as_deref().unwrap_or("<empty>"),
            patch.marker_text.as_deref().unwrap_or("<empty>"),
            active_state.active_list_kind()
        ).into());
        let active_list_kind = active_state.active_list_kind();
        if patch.source == "paragraph-region" && active_list_kind != ListMarkerKind::None {
            patch.source = "list-item-region".to_string();
            patch.kind = Some("list-item".to_string());
            patch.full_target_indices = patch.target_indices.clone();
            web_sys::console::log_1(&format!(
                "[AREN_LIST-CONVERT] switched to list-item-region listKind={:?} target_indices={:?}",
                active_list_kind, patch.target_indices
            ).into());
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
                        active_state.source_marker_text_for_patch().unwrap_or(""),
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
        .source_marker_text_for_patch()
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
    web_sys::console::log_1(&format!(
        "[AREN_PATCH-NOOP-DBG] noop={} text_u={} style_u={} align_u={} lh_u={} marker_u={} sourceMarker='{}' nextMarker='{}' origTextLen={} newTextLen={}",
        noop, text_unchanged, style_unchanged, alignment_unchanged, line_height_unchanged, marker_unchanged,
        source_marker, next_marker,
        patch.original_text.chars().count(), patch.new_text.chars().count()
    ).into());
    noop
}

pub fn toggle_active_editor_bold() -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.toggle_bold_all();
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn toggle_active_editor_italic() -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.toggle_italic_all();
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn toggle_active_editor_underline() -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.toggle_underline_all();
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn set_active_editor_color(color: &str) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_color_all(color);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn set_active_editor_font_family(font_family: &str) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_font_family_all(font_family);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn set_active_editor_font_size(font_size: f32) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_font_size_all(font_size);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn step_active_editor_font_size(increase: bool) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let next_size = resolve_font_size_step(live_state.active_font_size(), increase);
        let changed = live_state.set_font_size_all(next_size);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn set_active_editor_char_spacing(char_spacing: f32) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_char_spacing_all(char_spacing);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn set_active_editor_line_height(line_height: f32) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_line_height(line_height);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn set_active_editor_paragraph_mode(mode: &str) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|editor_mode| {
        let mut editor_mode = editor_mode.borrow_mut();
        let Some(live_state) = editor_mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let changed = live_state.set_paragraph_mode(mode);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn set_active_editor_alignment(alignment: &str) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let Some(next_alignment) = parse_alignment(alignment) else {
            return build_active_editor_format_state(live_state, false);
        };
        let changed = live_state.set_alignment(next_alignment);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn set_active_editor_list_kind(list_kind: &str) -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mut mode = mode.borrow_mut();
        let Some(live_state) = mode.live_state.as_mut() else {
            return ActiveEditorFormatState::default();
        };
        let Some(next_list_kind) = parse_list_kind(list_kind) else {
            return build_active_editor_format_state(live_state, false);
        };
        let changed = live_state.set_list_kind(next_list_kind);
        build_active_editor_format_state(live_state, changed)
    })
}

pub fn active_editor_format_state() -> ActiveEditorFormatState {
    crate::editor::session::HOST_EDITOR_MODE.with(|mode| {
        let mode = mode.borrow();
        let Some(live_state) = mode.live_state.as_ref() else {
            return ActiveEditorFormatState::default();
        };
        build_active_editor_format_state(live_state, false)
    })
}

pub fn apply_active_editor_format_action(
    action: EditorFormatAction,
) -> ActiveEditorFormatState {
    match action {
        EditorFormatAction::ToggleBold => toggle_active_editor_bold(),
        EditorFormatAction::ToggleItalic => toggle_active_editor_italic(),
        EditorFormatAction::ToggleUnderline => toggle_active_editor_underline(),
        EditorFormatAction::IncreaseFontSize => step_active_editor_font_size(true),
        EditorFormatAction::DecreaseFontSize => step_active_editor_font_size(false),
        EditorFormatAction::SetParagraphMode { mode } => {
            set_active_editor_paragraph_mode(&mode)
        }
        EditorFormatAction::SetColor { color } => set_active_editor_color(&color),
        EditorFormatAction::SetFontFamily { font_family } => {
            set_active_editor_font_family(&font_family)
        }
        EditorFormatAction::SetFontSize { font_size } => {
            set_active_editor_font_size(font_size)
        }
        EditorFormatAction::SetCharSpacing { char_spacing } => {
            set_active_editor_char_spacing(char_spacing)
        }
        EditorFormatAction::SetLineHeight { line_height } => {
            set_active_editor_line_height(line_height)
        }
        EditorFormatAction::SetAlignment { alignment } => {
            set_active_editor_alignment(&alignment)
        }
        EditorFormatAction::SetListKind { list_kind } => {
            set_active_editor_list_kind(&list_kind)
        }
    }
}

pub fn find_paragraph_shell_bbox(paragraph_id: &str) -> Option<BoundingBox> {
    HOST_PAGE_STATE
        .with(|state: &std::cell::RefCell<pdf_viewer_core::models::PageState>| host_resolve_paragraph_shell_bbox_workflow(&state.borrow(), paragraph_id))
}

pub fn set_editor_caret(caret_index: usize) -> bool {
    host_set_active_editor_caret_index(caret_index)
}

pub fn sync_editor_input(new_text: String, caret_index: usize) -> ActiveEditorInputSyncResult {
    host_sync_active_editor_input(new_text, caret_index)
}
