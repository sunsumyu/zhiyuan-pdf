use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use crate::document::page_region_models::ParagraphRegionSnapshot;
use crate::edit::active_target::ActiveEditorTarget;
use crate::geometry::layout_engine::ParagraphLayout;
use crate::models::{PaginationAction, PaginationCommand};
use crate::persistence::models::{PersistableRegionPatch, PersistableSemanticOperation};

#[derive(Default)]
pub struct GlobalPatchState {
    pub paragraph_texts: HashMap<String, String>,
    pub paragraph_snapshots: HashMap<String, ParagraphRegionSnapshot>,
    pub paragraph_layout_snapshots: HashMap<String, ParagraphLayout>,
    pub paragraph_patches: HashMap<String, PersistableRegionPatch>,
    pub semantic_ops: HashMap<String, Vec<PersistableSemanticOperation>>,
    pub paragraph_replacement_targets: HashMap<String, ActiveEditorTarget>,
    pub field_group_texts: HashMap<String, String>,
    pub field_group_snapshots: HashMap<String, serde_json::Value>,
    pub field_group_patches: HashMap<String, PersistableRegionPatch>,
    pub history: Vec<PatchCommand>,
    pub redo_stack: Vec<PatchCommand>,
    pub accepted_patch_keys: HashSet<String>,
    pub patch_revision: u64,
    pub patched_run_texts: HashMap<String, String>,
    pub patched_texts: HashMap<String, String>,
}

impl GlobalPatchState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_paragraph_snapshot(&self, key: &str) -> Option<&ParagraphLayout> {
        self.paragraph_layout_snapshots.get(key)
    }
}

lazy_static! {
    pub static ref GLOBAL_PATCH_STATE: RwLock<GlobalPatchState> =
        RwLock::new(GlobalPatchState::new());
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

pub fn bump_patch_revision(state: &mut GlobalPatchState) {
    state.patch_revision = state.patch_revision.saturating_add(1);
}

pub fn has_visible_patches(state: &GlobalPatchState) -> bool {
    !state.paragraph_texts.is_empty()
        || !state.paragraph_snapshots.is_empty()
        || !state.paragraph_patches.is_empty()
        || !state.semantic_ops.is_empty()
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
        if patch.semantic_ops.is_empty() {
            state.semantic_ops.remove(&patch.region_id);
        } else {
            state
                .semantic_ops
                .insert(patch.region_id.clone(), patch.semantic_ops.clone());
        }
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
        state.semantic_ops.remove(&patch.region_id);
        state.paragraph_replacement_targets.remove(&patch.region_id);
    } else if patch.source == "field-group" || patch.source == "field-row" {
        state.field_group_texts.remove(&patch.region_id);
        state.field_group_snapshots.remove(&patch.region_id);
        state.field_group_patches.remove(&patch.region_id);
    }
}

pub fn collect_semantic_ops(state: &GlobalPatchState) -> Vec<PersistableSemanticOperation> {
    state
        .semantic_ops
        .values()
        .flat_map(|ops| ops.iter().cloned())
        .collect()
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

pub fn apply_patch(patch: PersistableRegionPatch) {
    if let Ok(mut state) = GLOBAL_PATCH_STATE.write() {
        if patch.source == "field-row" {
            state
                .field_group_texts
                .insert(patch.patch_key.clone(), patch.new_text.clone());
            if let Some(snapshot) = patch.snapshot {
                state
                    .field_group_snapshots
                    .insert(patch.patch_key, snapshot);
            }
        } else {
            state
                .paragraph_texts
                .insert(patch.patch_key.clone(), patch.new_text.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::list_semantics::ListMarkerKind;

    fn semantic_patch() -> PersistableRegionPatch {
        PersistableRegionPatch {
            patch_key: "list:p1".to_string(),
            region_id: "p1".to_string(),
            source: "list-item-region".to_string(),
            original_text: "Body".to_string(),
            new_text: "Body edited".to_string(),
            semantic_ops: vec![PersistableSemanticOperation::SetListKind {
                block_id: "p1".to_string(),
                list_kind: ListMarkerKind::Bullet,
                marker_text: Some("●".to_string()),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn apply_and_remove_patch_maps_tracks_semantic_ops() {
        let mut state = GlobalPatchState::new();
        let patch = semantic_patch();

        apply_patch_maps(&mut state, &patch);
        assert_eq!(collect_semantic_ops(&state).len(), 1);
        assert!(state.semantic_ops.contains_key("p1"));

        remove_patch_maps(&mut state, &patch);
        assert!(collect_semantic_ops(&state).is_empty());
        assert!(!state.semantic_ops.contains_key("p1"));
    }

    #[test]
    fn applying_patch_without_semantic_ops_clears_previous_region_ops() {
        let mut state = GlobalPatchState::new();
        let patch = semantic_patch();
        apply_patch_maps(&mut state, &patch);

        let replacement = PersistableRegionPatch {
            patch_key: "list:p1".to_string(),
            region_id: "p1".to_string(),
            source: "list-item-region".to_string(),
            original_text: "Body".to_string(),
            new_text: "Body edited again".to_string(),
            ..Default::default()
        };
        apply_patch_maps(&mut state, &replacement);

        assert!(collect_semantic_ops(&state).is_empty());
        assert!(!state.semantic_ops.contains_key("p1"));
    }
}

pub fn should_prefetch_page(current_page: usize, target_page: usize, buffer: usize) -> bool {
    if target_page > current_page {
        target_page - current_page <= buffer
    } else if current_page > target_page {
        current_page - target_page <= buffer
    } else {
        false
    }
}

pub fn build_pagination_commands(
    current_page: usize,
    total_pages: usize,
    path: &str,
    zoom: f32,
) -> Vec<crate::models::PaginationCommand> {
    let mut commands = Vec::new();

    // 计算滑动窗口 [current - 1, current + 1]
    let start = current_page.saturating_sub(1);
    let end = (current_page + 1).min(total_pages.saturating_sub(1));

    for page in start..=end {
        commands.push(PaginationCommand {
            action: PaginationAction::Prefetch,
            page_index: page,
            path: path.to_string(),
            zoom,
        });
    }

    commands
}
