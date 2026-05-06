use crate::editor::session::ActiveEditorTarget;
use crate::models::{ParagraphRegionSnapshot, PersistableRegionPatch};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::sync::RwLock;

// --- Patch State ---

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

pub static GLOBAL_PATCH_STATE: OnceLock<RwLock<GlobalPatchState>> = OnceLock::new();

pub fn get_patch_state() -> &'static RwLock<GlobalPatchState> {
    GLOBAL_PATCH_STATE.get_or_init(|| RwLock::new(GlobalPatchState::default()))
}

fn bump_patch_revision(state: &mut GlobalPatchState) {
    state.patch_revision = state.patch_revision.saturating_add(1);
}

fn has_visible_patches(state: &GlobalPatchState) -> bool {
    !state.paragraph_texts.is_empty()
        || !state.paragraph_snapshots.is_empty()
        || !state.paragraph_patches.is_empty()
        || !state.field_group_texts.is_empty()
        || !state.field_group_snapshots.is_empty()
        || !state.field_group_patches.is_empty()
}

pub fn current_patch_revision() -> u64 {
    get_patch_state()
        .read()
        .map(|state| state.patch_revision)
        .unwrap_or(0)
}

pub fn current_paragraph_patch_text(paragraph_id: &str) -> Option<String> {
    get_patch_state()
        .read()
        .ok()
        .and_then(|state| state.paragraph_texts.get(paragraph_id).cloned())
}

pub fn current_paragraph_patch(paragraph_id: &str) -> Option<PersistableRegionPatch> {
    get_patch_state()
        .read()
        .ok()
        .and_then(|state| state.paragraph_patches.get(paragraph_id).cloned())
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

fn apply_patch_maps(state: &mut GlobalPatchState, patch: &PersistableRegionPatch) {
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

fn remove_patch_maps(state: &mut GlobalPatchState, patch: &PersistableRegionPatch) {
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

pub fn remember_paragraph_replacement_target(paragraph_id: &str, target: ActiveEditorTarget) {
    let mut state = get_patch_state().write().unwrap();
    state
        .paragraph_replacement_targets
        .insert(paragraph_id.to_string(), target);
}

fn capture_existing_patch(
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

pub fn apply_patch_with_history(patch: PersistableRegionPatch) {
    let mut state = get_patch_state().write().unwrap();
    let old_patch = capture_existing_patch(&state, &patch);
    state.accepted_patch_keys.remove(&patch.patch_key);
    apply_patch_maps(&mut state, &patch);
    bump_patch_revision(&mut state);
    state.history.push(PatchCommand {
        patch_key: patch.patch_key.clone(),
        old_patch,
        new_patch: patch,
    });
    state.redo_stack.clear();
}

pub fn collect_persistable_patches() -> Vec<PersistableRegionPatch> {
    let state = get_patch_state().read().unwrap();
    let mut patches = state
        .paragraph_patches
        .values()
        .cloned()
        .collect::<Vec<_>>();
    patches.extend(state.field_group_patches.values().cloned());
    patches
}

pub fn clear_persistable_patches(clear_history: bool) {
    let mut state = get_patch_state().write().unwrap();
    let had_visible_patches = has_visible_patches(&state);
    state.paragraph_texts.clear();
    state.paragraph_snapshots.clear();
    state.paragraph_patches.clear();
    state.field_group_texts.clear();
    state.field_group_snapshots.clear();
    state.field_group_patches.clear();
    state.accepted_patch_keys.clear();
    if clear_history {
        state.history.clear();
        state.redo_stack.clear();
    }
    if had_visible_patches {
        bump_patch_revision(&mut state);
    }
}

pub fn undo() -> bool {
    let mut state = get_patch_state().write().unwrap();
    if let Some(cmd) = state.history.pop() {
        if let Some(old_patch) = &cmd.old_patch {
            apply_patch_maps(&mut state, old_patch);
        } else {
            remove_patch_maps(&mut state, &cmd.new_patch);
        }
        bump_patch_revision(&mut state);
        state.redo_stack.push(cmd);
        return true;
    }
    false
}

pub fn redo() -> bool {
    let mut state = get_patch_state().write().unwrap();
    if let Some(cmd) = state.redo_stack.pop() {
        apply_patch_maps(&mut state, &cmd.new_patch);
        bump_patch_revision(&mut state);
        state.history.push(cmd);
        return true;
    }
    false
}

pub fn collect_review_changes() -> Vec<ReviewChangeEntry> {
    let state = get_patch_state().read().unwrap();
    let mut entries = state
        .paragraph_patches
        .values()
        .chain(state.field_group_patches.values())
        .filter(|patch: &&PersistableRegionPatch| !state.accepted_patch_keys.contains(&patch.patch_key))
        .map(|patch| ReviewChangeEntry {
            patch_key: patch.patch_key.clone(),
            page_index: patch.page_index,
            region_id: patch.region_id.clone(),
            source: patch.source.clone(),
            kind: patch.kind.clone(),
            original_text: patch.original_text.clone(),
            current_text: patch.new_text.clone(),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.page_index
            .cmp(&right.page_index)
            .then_with(|| left.patch_key.cmp(&right.patch_key))
    });
    entries
}

pub fn reject_review_change(patch_key: &str) -> bool {
    let mut state = get_patch_state().write().unwrap();

    let paragraph_region_id = state
        .paragraph_patches
        .values()
        .find(|patch| patch.patch_key == patch_key)
        .map(|patch| patch.region_id.clone());
    if let Some(region_id) = paragraph_region_id {
        state.accepted_patch_keys.remove(patch_key);
        state.paragraph_texts.remove(&region_id);
        state.paragraph_snapshots.remove(&region_id);
        state.paragraph_patches.remove(&region_id);
        state.history.retain(|cmd: &PatchCommand| cmd.patch_key != patch_key);
        state.redo_stack.retain(|cmd: &PatchCommand| cmd.patch_key != patch_key);
        bump_patch_revision(&mut state);
        return true;
    }

    let field_region_id = state
        .field_group_patches
        .values()
        .find(|patch| patch.patch_key == patch_key)
        .map(|patch| patch.region_id.clone());
    if let Some(region_id) = field_region_id {
        state.accepted_patch_keys.remove(patch_key);
        state.field_group_texts.remove(&region_id);
        state.field_group_snapshots.remove(&region_id);
        state.field_group_patches.remove(&region_id);
        state.history.retain(|cmd: &PatchCommand| cmd.patch_key != patch_key);
        state.redo_stack.retain(|cmd: &PatchCommand| cmd.patch_key != patch_key);
        bump_patch_revision(&mut state);
        return true;
    }

    false
}

pub fn accept_review_change(patch_key: &str) -> bool {
    let mut state = get_patch_state().write().unwrap();
    let exists = state
        .paragraph_patches
        .values()
        .chain(state.field_group_patches.values())
        .any(|patch| patch.patch_key == patch_key);
    if !exists {
        return false;
    }
    if state.accepted_patch_keys.insert(patch_key.to_string()) {
        bump_patch_revision(&mut state);
        return true;
    }
    false
}

pub fn accept_all_review_changes() -> ReviewBulkChangeResult {
    let mut state = get_patch_state().write().unwrap();
    let patch_keys = state
        .paragraph_patches
        .values()
        .chain(state.field_group_patches.values())
        .map(|patch| patch.patch_key.clone())
        .collect::<Vec<_>>();
    let mut affected = 0usize;
    for patch_key in patch_keys {
        if state.accepted_patch_keys.insert(patch_key) {
            affected += 1;
        }
    }
    if affected > 0 {
        bump_patch_revision(&mut state);
    }
    ReviewBulkChangeResult {
        changed: affected > 0,
        revision: state.patch_revision,
        affected_patch_count: affected,
    }
}

pub fn reject_all_review_changes() -> ReviewBulkChangeResult {
    let mut state = get_patch_state().write().unwrap();
    let paragraph_region_ids = state.paragraph_patches.keys().cloned().collect::<Vec<_>>();
    let field_region_ids = state
        .field_group_patches
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let patch_keys = state
        .paragraph_patches
        .values()
        .chain(state.field_group_patches.values())
        .map(|patch| patch.patch_key.clone())
        .collect::<Vec<_>>();

    let affected = patch_keys.len();
    if affected == 0 {
        return ReviewBulkChangeResult {
            changed: false,
            revision: state.patch_revision,
            affected_patch_count: 0,
        };
    }

    for region_id in paragraph_region_ids {
        state.paragraph_texts.remove(&region_id);
        state.paragraph_snapshots.remove(&region_id);
        state.paragraph_patches.remove(&region_id);
    }
    for region_id in field_region_ids {
        state.field_group_texts.remove(&region_id);
        state.field_group_snapshots.remove(&region_id);
        state.field_group_patches.remove(&region_id);
    }
    for patch_key in &patch_keys {
        state.accepted_patch_keys.remove(patch_key);
    }
    state.history.retain(|cmd: &PatchCommand| {
        !patch_keys
            .iter()
            .any(|patch_key| patch_key == &cmd.patch_key)
    });
    state.redo_stack.retain(|cmd: &PatchCommand| {
        !patch_keys
            .iter()
            .any(|patch_key| patch_key == &cmd.patch_key)
    });
    bump_patch_revision(&mut state);
    ReviewBulkChangeResult {
        changed: true,
        revision: state.patch_revision,
        affected_patch_count: affected,
    }
}
