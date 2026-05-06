use std::collections::BTreeMap;

use pdf_viewer_core::models::{GlyphPaintPlan, VectorPageModel};

use crate::editor::bridge::build_paragraph_render_target;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::edit_target::edit_target_base_paragraph_id;
use crate::editor::list_format::{
    collect_marker_overrides, resolve_active_marker_text,
};
use crate::editor::mode::get_active_editor_state;
use crate::editor::replacement_snapshot::replacement_target_from_patch_snapshot;
use crate::editor::session::ActiveEditorTarget;
use crate::editor::source_identity::collect_target_source_object_indices;
use crate::page::runtime::HOST_PAGE_STATE;
use crate::state_manager::get_patch_state;

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
    pub source_text: String,
    pub draft_text: String,
    pub replaces_source: bool,
    pub marker_text_override: Option<String>,
}

fn target_source_object_indices(target: &ActiveEditorTarget) -> Vec<usize> {
    collect_target_source_object_indices(target)
}

fn persisted_patch_source_indices(
    patch_target_indices: &[usize],
    patch_full_target_indices: &[usize],
    target: &ActiveEditorTarget,
) -> Vec<usize> {
    if !patch_full_target_indices.is_empty() {
        return patch_full_target_indices.to_vec();
    }
    if !patch_target_indices.is_empty() {
        return patch_target_indices.to_vec();
    }
    target_source_object_indices(target)
}

pub fn collect_paragraph_render_overlays(
    plan: &GlyphPaintPlan,
    vector_model: Option<&VectorPageModel>,
) -> Vec<ParagraphRenderOverlay> {
    let mut overlays = BTreeMap::<String, ParagraphRenderOverlay>::new();
    let active_state = get_active_editor_state();
    let marker_overrides = collect_marker_overrides(Some(plan), active_state.as_ref());

    if let Ok(state) = get_patch_state().read() {
        for (paragraph_id, patch) in &state.paragraph_patches {
            if patch.page_index != plan.page_index {
                continue;
            }
            let Some(target) = state
                .paragraph_replacement_targets
                .get(paragraph_id)
                .cloned()
                .or_else(|| replacement_target_from_patch_snapshot(patch))
                .or_else(|| build_paragraph_render_target(plan, vector_model, paragraph_id))
            else {
                continue;
            };
            let base_paragraph_id =
                edit_target_base_paragraph_id(&target.paragraph_id).to_string();
            let source_object_indices = persisted_patch_source_indices(
                &patch.target_indices,
                &patch.full_target_indices,
                &target,
            );
            dbg_event(
                "overlay.source-indices",
                "persisted",
                vec![
                    dbg_field("paragraphId", paragraph_id),
                    dbg_field("patchTargetCount", patch.target_indices.len()),
                    dbg_field("patchFullTargetCount", patch.full_target_indices.len()),
                    dbg_field("resolvedCount", source_object_indices.len()),
                    dbg_field("resolvedIndices", format!("{:?}", source_object_indices)),
                    dbg_field("sourceText", patch.original_text.as_str()),
                    dbg_field("draftText", patch.new_text.as_str()),
                ],
            );
            overlays.insert(
                paragraph_id.clone(),
                ParagraphRenderOverlay {
                    owner: ParagraphRenderOverlayOwner::PersistedPageCanvas,
                    target,
                    source_object_indices,
                    source_text: patch.original_text.clone(),
                    draft_text: patch.new_text.clone(),
                    replaces_source: true,
                    marker_text_override: patch
                        .new_marker_text
                        .clone()
                        .or_else(|| marker_overrides.get(&base_paragraph_id).cloned().flatten()),
                },
            );
        }
    }

    if let Some(active_state) = active_state {
        let marker_text_override = marker_overrides
            .get(edit_target_base_paragraph_id(
                active_state.paragraph_id(),
            ))
            .cloned()
            .flatten()
            .or_else(|| {
                HOST_PAGE_STATE.with(|page_state: &crate::page::runtime::HostPageState| {
                    resolve_active_marker_text(&active_state, &page_state.borrow())
                })
            });
        overlays.insert(
            active_state.paragraph_id().to_string(),
            ParagraphRenderOverlay {
                owner: ParagraphRenderOverlayOwner::ActiveEditorShell,
                target: active_state.target.clone(),
                source_object_indices: target_source_object_indices(&active_state.target),
                source_text: active_state.target.source_body_text().to_string(),
                draft_text: active_state.current_text().to_string(),
                replaces_source: active_state.requires_source_replacement(),
                marker_text_override,
            },
        );
    }

    overlays.into_values().collect()
}
