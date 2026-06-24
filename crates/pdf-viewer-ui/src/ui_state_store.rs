use crate::editor::session::ActiveEditorTarget;
use pdf_viewer_core::persistence::models::PersistableRegionPatch;
use std::sync::OnceLock;
use std::sync::RwLock;

// Re-export pure data structures and helpers from core.
pub use pdf_viewer_core::persistence::patch_store::*;

pub static GLOBAL_PATCH_STATE: OnceLock<RwLock<GlobalPatchState>> = OnceLock::new();

pub fn read_patch_state() -> &'static RwLock<GlobalPatchState> {
    GLOBAL_PATCH_STATE.get_or_init(|| RwLock::new(GlobalPatchState::default()))
}

pub fn current_patch_revision() -> u64 {
    read_patch_state()
        .read()
        .map(|state| state.patch_revision)
        .unwrap_or(0)
}

pub fn patch_text(paragraph_id: &str) -> Option<String> {
    read_patch_state()
        .read()
        .ok()
        .and_then(|state| state.paragraph_texts.get(paragraph_id).cloned())
}

pub fn current_paragraph_patch(paragraph_id: &str) -> Option<PersistableRegionPatch> {
    read_patch_state()
        .read()
        .ok()
        .and_then(|state| state.paragraph_patches.get(paragraph_id).cloned())
}

pub fn remember_target(paragraph_id: &str, target: ActiveEditorTarget) {
    let mut state = read_patch_state().write().unwrap();
    state
        .paragraph_replacement_targets
        .insert(paragraph_id.to_string(), target);
}

pub fn record_patch(patch: PersistableRegionPatch) {
    let mut state = read_patch_state().write().unwrap();
    let old_patch = capture_existing_patch(&state, &patch);
    state.accepted_patch_keys.remove(&patch.patch_key);
    crate::chain_trace!(
        "patch.apply",
        "regionId" => &patch.region_id,
        "patchKey" => &patch.patch_key,
        "pageIndex" => patch.page_index,
        "originalLen" => patch.original_text.chars().count(),
        "newLen" => patch.new_text.chars().count(),
    );
    apply_patch_maps(&mut state, &patch);
    let total_paragraph_patches = state.paragraph_patches.len();
    bump_patch_revision(&mut state);
    state.history.push(PatchCommand {
        patch_key: patch.patch_key.clone(),
        old_patch,
        new_patch: patch,
    });
    state.redo_stack.clear();
    crate::chain_trace!(
        "commit.persist",
        "totalPatches" => total_paragraph_patches,
    );
}

pub fn collect_persistable_patches() -> Vec<PersistableRegionPatch> {
    let state = read_patch_state().read().unwrap();
    let mut patches = state
        .paragraph_patches
        .values()
        .cloned()
        .collect::<Vec<_>>();
    patches.extend(state.field_group_patches.values().cloned());
    patches
}

pub fn clear_persistable_patches(clear_history: bool) {
    let mut state = read_patch_state().write().unwrap();
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

pub fn can_undo() -> bool {
    read_patch_state()
        .read()
        .map(|state| !state.history.is_empty())
        .unwrap_or(false)
}

pub fn can_redo() -> bool {
    read_patch_state()
        .read()
        .map(|state| !state.redo_stack.is_empty())
        .unwrap_or(false)
}

pub fn undo_depth() -> usize {
    read_patch_state()
        .read()
        .map(|state| state.history.len())
        .unwrap_or(0)
}

pub fn redo_depth() -> usize {
    read_patch_state()
        .read()
        .map(|state| state.redo_stack.len())
        .unwrap_or(0)
}

/// Clear undo + redo stacks without touching patch data.
///
/// Differs from `clear_persistable_patches(true)` which also wipes the
/// applied patch maps. Use this when you only want to forget edit history
/// (e.g. after explicit user action "clear history").
pub fn clear_history_stacks() {
    let mut state = read_patch_state().write().unwrap();
    state.history.clear();
    state.redo_stack.clear();
}

pub fn undo() -> bool {
    let mut state = read_patch_state().write().unwrap();
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
    let mut state = read_patch_state().write().unwrap();
    if let Some(cmd) = state.redo_stack.pop() {
        apply_patch_maps(&mut state, &cmd.new_patch);
        bump_patch_revision(&mut state);
        state.history.push(cmd);
        return true;
    }
    false
}

pub fn collect_review_changes() -> Vec<ReviewChangeEntry> {
    let state = read_patch_state().read().unwrap();
    let mut entries = state
        .paragraph_patches
        .values()
        .chain(state.field_group_patches.values())
        .filter(|patch: &&PersistableRegionPatch| {
            !state.accepted_patch_keys.contains(&patch.patch_key)
        })
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
    let mut state = read_patch_state().write().unwrap();

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
        state
            .history
            .retain(|cmd: &PatchCommand| cmd.patch_key != patch_key);
        state
            .redo_stack
            .retain(|cmd: &PatchCommand| cmd.patch_key != patch_key);
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
        state
            .history
            .retain(|cmd: &PatchCommand| cmd.patch_key != patch_key);
        state
            .redo_stack
            .retain(|cmd: &PatchCommand| cmd.patch_key != patch_key);
        bump_patch_revision(&mut state);
        return true;
    }

    false
}

pub fn accept_review_change(patch_key: &str) -> bool {
    let mut state = read_patch_state().write().unwrap();
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

pub fn accept_all_changes() -> ReviewBulkChangeResult {
    let mut state = read_patch_state().write().unwrap();
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

pub fn reject_all_changes() -> ReviewBulkChangeResult {
    let mut state = read_patch_state().write().unwrap();
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
