use std::collections::HashMap;
use std::sync::RwLock;
use lazy_static::lazy_static;
use crate::models::{PaginationAction, PaginationCommand};
use crate::persistence::models::PersistableRegionPatch;
use crate::geometry::layout_engine::ParagraphLayout;

#[derive(Debug, Default)]
pub struct GlobalPatchState {
    pub field_group_texts: HashMap<String, String>,
    pub field_group_snapshots: HashMap<String, serde_json::Value>,
    pub paragraph_texts: HashMap<String, String>,
    pub paragraph_snapshots: HashMap<String, ParagraphLayout>,
    pub patched_run_texts: HashMap<String, String>,
    pub patched_texts: HashMap<String, String>,
}

impl GlobalPatchState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_paragraph_snapshot(&self, key: &str) -> Option<&ParagraphLayout> {
        self.paragraph_snapshots.get(key)
    }
}

lazy_static! {
    pub static ref GLOBAL_PATCH_STATE: RwLock<GlobalPatchState> = RwLock::new(GlobalPatchState::new());
}

pub fn apply_patch(patch: PersistableRegionPatch) {
    if let Ok(mut state) = GLOBAL_PATCH_STATE.write() {
        if patch.source == "field-row" {
            state.field_group_texts.insert(patch.patch_key.clone(), patch.new_text.clone());
            if let Some(snapshot) = patch.snapshot {
                state.field_group_snapshots.insert(patch.patch_key, snapshot);
            }
        } else {
            state.paragraph_texts.insert(patch.patch_key.clone(), patch.new_text.clone());
            // Note: Paragraph snapshots (ParagraphLayout) are usually set via specialized WASM calls 
            // but we might need to deserialize them if present in patch. 
            // For now we keep the existing logic where they are updated independently.
        }
    }
}

pub fn should_prefetch_page(
    current_page: usize,
    target_page: usize,
    buffer: usize,
) -> bool {
    if target_page > current_page {
        target_page - current_page <= buffer
    } else if current_page > target_page {
        current_page - target_page <= buffer
    } else {
        false
    }
}

pub fn get_pagination_commands(
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
