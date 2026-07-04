//! Region-patch application command.

use super::ipc_converters::execute_region_patches;
use crate::log_step;
use pdf_viewer_core::persistence::models::{PersistableRegionPatch, PersistableSemanticOperation};
use tauri::command;

#[command]
pub async fn apply_region_patches(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
    patches: Vec<PersistableRegionPatch>,
    semantic_ops: Option<Vec<PersistableSemanticOperation>>,
) -> Result<(), String> {
    println!(
        ">>>>> [ENTRY] apply_region_patches | path={} | count={}",
        path,
        patches.len()
    );
    log_step!(
        "[V3-SAVE-CMD] Applying region patches: path={} page={} count={}",
        path,
        page_index,
        patches.len()
    );
    execute_region_patches(
        &state,
        path,
        page_index,
        patches,
        semantic_ops.unwrap_or_default(),
    )
    .await
}
