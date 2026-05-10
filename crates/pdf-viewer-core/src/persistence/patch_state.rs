use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::document::page_region_models::ParagraphRegionSnapshot;
use crate::edit::active_target::ActiveEditorTarget;
use crate::persistence::models::PersistableRegionPatch;

// ── Patch State ─────────────────────────────────────────────────

#[derive(Default)]
pub struct GlobalPatchState {
    pub paragraph_texts: HashMap<String, String>,
    pub paragraph_snapshots: HashMap<String, ParagraphRegionSnapshot>,
    pub paragraph_patches: HashMap<String, PersistableRegionPatch>,
    pub paragraph_replacement_targets: HashMap<String, ActiveEditorTarget>,
    pub field_group_texts: HashMap<String, String>,
    pub field_group_snapshots: HashMap<String, serde_json::Value>,
    pub field_group_patches: HashMap<String, PersistableRegionPatch>,
    pub history: Vec<PatchCommand>,
    pub redo_stack: Vec<PatchCommand>,
    pub accepted_patch_keys: HashSet<String>,
    pub patch_revision: u64,
}

#[derive(Clone, Debug)]
pub struct PatchCommand {
    pub patch_key: String,
    pub old_patch: Option<PersistableRegionPatch>,
    pub new_patch: PersistableRegionPatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReviewChangeEntry {
    pub patch_key: String,
    pub page_index: u16,
    pub region_id: String,
    pub source: String,
    pub kind: Option<String>,
    pub original_text: String,
    pub current_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReviewBulkChangeResult {
    pub changed: bool,
    pub revision: u64,
    pub affected_patch_count: usize,
}

// ── Pure helper functions (no global state access) ──────────────

pub fn bump_patch_revision(state: &mut GlobalPatchState) {
    state.patch_revision = state.patch_revision.saturating_add(1);
}

pub fn has_visible_patches(state: &GlobalPatchState) -> bool {
    !state.paragraph_texts.is_empty()
        || !state.paragraph_snapshots.is_empty()
        || !state.paragraph_patches.is_empty()
        || !state.field_group_texts.is_empty()
        || !state.field_group_snapshots.is_empty()
        || !state.field_group_patches.is_empty()
}

pub fn apply_patch_maps(state: &mut GlobalPatchState, patch: &PersistableRegionPatch) {
    if patch.source == "paragraph-region" || patch.source == "list-item-region" {
        state
            .paragraph_texts
            .insert(patch.region_id.clone(), patch.new_text.clone());
        if let Some(snap_val) = patch.snapshot.clone() {
            if let Ok(snap) = serde_json::from_value::<ParagraphRegionSnapshot>(snap_val) {
                state
                    .paragraph_snapshots
                    .insert(patch.region_id.clone(), snap);
            }
        }
        state
            .paragraph_patches
            .insert(patch.region_id.clone(), patch.clone());
        if let Some(target) = patch
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.get("replacementTarget"))
            .cloned()
            .and_then(|value| serde_json::from_value::<ActiveEditorTarget>(value).ok())
        {
            state
                .paragraph_replacement_targets
                .insert(patch.region_id.clone(), target);
        }
    } else if patch.source == "field-group" || patch.source == "field-row" {
        state
            .field_group_texts
            .insert(patch.region_id.clone(), patch.new_text.clone());
        if let Some(snap) = patch.snapshot.clone() {
            state
                .field_group_snapshots
                .insert(patch.region_id.clone(), snap);
        }
        state
            .field_group_patches
            .insert(patch.region_id.clone(), patch.clone());
    }
}

pub fn remove_patch_maps(state: &mut GlobalPatchState, patch: &PersistableRegionPatch) {
    if patch.source == "paragraph-region" || patch.source == "list-item-region" {
        state.paragraph_texts.remove(&patch.region_id);
        state.paragraph_snapshots.remove(&patch.region_id);
        state.paragraph_patches.remove(&patch.region_id);
        state.paragraph_replacement_targets.remove(&patch.region_id);
    } else if patch.source == "field-group" || patch.source == "field-row" {
        state.field_group_texts.remove(&patch.region_id);
        state.field_group_snapshots.remove(&patch.region_id);
        state.field_group_patches.remove(&patch.region_id);
    }
}

pub fn capture_existing_patch(
    state: &GlobalPatchState,
    patch: &PersistableRegionPatch,
) -> Option<PersistableRegionPatch> {
    if patch.source == "paragraph-region" || patch.source == "list-item-region" {
        let existing_patch = state.paragraph_patches.get(&patch.region_id)?;
        let mut rollback_patch = existing_patch.clone();
        rollback_patch.new_text = state.paragraph_texts.get(&patch.region_id)?.clone();
        return Some(rollback_patch);
    }

    if patch.source == "field-group" || patch.source == "field-row" {
        let existing_patch = state.field_group_patches.get(&patch.region_id)?;
        let mut rollback_patch = existing_patch.clone();
        rollback_patch.new_text = state.field_group_texts.get(&patch.region_id)?.clone();
        return Some(rollback_patch);
    }

    None
}
