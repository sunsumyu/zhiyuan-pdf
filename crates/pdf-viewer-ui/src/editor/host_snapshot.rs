use serde::{Deserialize, Serialize};

use crate::editor::debug_trace::{resolve_editor_debug_trace, EditorDebugTraceEvent};
use crate::editor::host_runtime::get_state as get_editor_host_state;
use crate::editor::mode::get_active_editor_state;
use crate::editor::mode::is_text_edit_mode_enabled;
use crate::editor::projection::{
    project_active_editor_shell, project_paragraph_interaction_targets,
    ProjectedEditorShell, ProjectedParagraphInteractionTarget,
};
use crate::document::patch_persistence::has_persistable_patches;
use crate::zoom::runtime::get_zoom_state;
use pdf_viewer_core::glyph_layout::EditorGlyphSlotKind;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEditorRunDiagnostic {
    pub text: String,
    pub origin_x: f32,
    pub origin_y: f32,
    pub bbox_left: f32,
    pub bbox_right: f32,
    pub char_origins: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEditorDiagnostics {
    pub paragraph_id: String,
    pub draft_text: String,
    pub scene_body_text: String,
    pub text_plan_text: String,
    pub marker_text: Option<String>,
    pub initial_caret_index: usize,
    #[serde(default)]
    pub live_caret_index: usize,
    pub runs: Vec<ActiveEditorRunDiagnostic>,
    pub slots: Vec<ActiveEditorSlotDiagnostic>,
    #[serde(default)]
    pub debug_trace: Vec<EditorDebugTraceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveEditorSlotDiagnostic {
    pub kind: String,
    pub ch: String,
    pub raw_char_index: Option<usize>,
    pub left: f32,
    pub right: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorHostSnapshot {
    pub enabled: bool,
    pub active_target: Option<ProjectedEditorShell>,
    pub draft_text: Option<String>,
    pub caret_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<ActiveEditorDiagnostics>,
    pub targets: Vec<ProjectedParagraphInteractionTarget>,
    pub has_persistable_patches: bool,
}

fn sanitize_projection_zoom(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else if fallback.is_finite() && fallback > 0.0 {
        fallback
    } else {
        1.0
    }
}

fn resolve_editor_projection_zoom(requested_display_zoom: f32) -> f32 {
    let runtime_zoom = get_editor_host_state().last_display_zoom;
    let zoom_state = get_zoom_state();
    if zoom_state.preview_host.preview_active
        || (zoom_state.target_zoom - zoom_state.visual_zoom).abs() >= 0.001
    {
        return sanitize_projection_zoom(zoom_state.last_rendered_zoom, runtime_zoom);
    }

    sanitize_projection_zoom(requested_display_zoom, runtime_zoom)
}

pub fn resolve_editor_host_snapshot(display_zoom: f32) -> EditorHostSnapshot {
    let projection_zoom = resolve_editor_projection_zoom(display_zoom);
    let enabled = is_text_edit_mode_enabled();
    let active_state = get_active_editor_state();
    let page_height = crate::page::runtime::HOST_PAGE_STATE.with(|state: &crate::page::runtime::HostPageState| {
        let state = state.borrow();
        state
            .vector_model
            .as_ref()
            .map(|m| m.height)
            .or_else(|| state.paint_plan.as_ref().map(|p| p.height))
            .unwrap_or(842.0)
    });

    let active_target = if enabled {
        project_active_editor_shell(projection_zoom, page_height)
    } else {
        None
    };
    let targets = if enabled {
        project_paragraph_interaction_targets(projection_zoom, page_height)
    } else {
        Vec::new()
    };

    EditorHostSnapshot {
        enabled,
        active_target,
        draft_text: active_state
            .as_ref()
            .map(|state| state.current_text().to_string()),
        caret_index: active_state.map(|state| state.caret_index).unwrap_or(0),
        diagnostics: resolve_active_editor_diagnostics(),
        targets,
        has_persistable_patches: has_persistable_patches(),
    }
}

pub fn resolve_active_editor_diagnostics() -> Option<ActiveEditorDiagnostics> {
    let active_state = get_active_editor_state()?;
    let target = active_state.target.clone();
    let text_plan = target.scene.document_plan.body_text_plan.clone();
    let initial_caret_index = target.initial_body_caret_index();

    Some(ActiveEditorDiagnostics {
        paragraph_id: target.paragraph_id,
        draft_text: active_state.current_text().to_string(),
        scene_body_text: target.scene.document_plan.source_body_text().to_string(),
        text_plan_text: text_plan.text,
        marker_text: target
            .scene
            .document_plan
            .marker
            .as_ref()
            .map(|marker| marker.text.clone()),
        initial_caret_index,
        live_caret_index: active_state.caret_index,
        runs: target
            .scene
            .body_session
            .paragraph
            .runs
            .iter()
            .map(|run| ActiveEditorRunDiagnostic {
                text: run.text.clone(),
                origin_x: run.origin_x,
                origin_y: run.origin_y,
                bbox_left: run.bbox.left,
                bbox_right: run.bbox.right,
                char_origins: run.char_origins.clone(),
            })
            .collect(),
        slots: text_plan
            .slots
            .iter()
            .map(|slot| ActiveEditorSlotDiagnostic {
                kind: match slot.kind {
                    EditorGlyphSlotKind::Glyph => "glyph".to_string(),
                    EditorGlyphSlotKind::Gap => "gap".to_string(),
                },
                ch: slot.ch.to_string(),
                raw_char_index: slot.raw_char_index,
                left: slot.left,
                right: slot.right,
            })
            .collect(),
        debug_trace: resolve_editor_debug_trace(),
    })
}
