//! Region-patch application command.

use crate::application::pdf::edit_commands::execute_region_patches;
use crate::log_step;
use pdf_viewer_core::persistence::models::PersistableRegionPatch;
use tauri::command;

#[command]
pub async fn apply_region_patches(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    patches: Vec<PersistableRegionPatch>,
) -> Result<(), String> {
    log_step!(
        "[V3-SAVE-CMD] Applying region patches: path={} page={} count={}",
        path,
        page_index,
        patches.len()
    );
    execute_region_patches(&state, path, page_index, patches).await
}
