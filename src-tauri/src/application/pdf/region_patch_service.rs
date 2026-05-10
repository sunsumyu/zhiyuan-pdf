use std::collections::BTreeMap;
use pdf_viewer_core::persistence::models::PersistableRegionPatch;
use crate::interfaces::pdf::execute_region_patches;

#[derive(Debug, Clone, Default)]
pub(crate)
struct RegionPatchBatchApplyResult {
pub(crate) applied_patch_count: usize,
pub(crate) touched_pages: Vec<u16>,
}
pub(crate) async fn apply_region_patch_batch(
    state: &crate::AppState,
    path: &str,
    patches: Vec<PersistableRegionPatch>,
) -> Result<RegionPatchBatchApplyResult, String> {
    if patches.is_empty() {
        return Ok(RegionPatchBatchApplyResult::default());
    }

    let mut grouped = BTreeMap::<u16, Vec<PersistableRegionPatch>>::new();
    for patch in patches {
        grouped.entry(patch.page_index).or_default().push(patch);
    }

    let mut touched_pages = Vec::with_capacity(grouped.len());
    let mut applied_patch_count = 0usize;

    for (page_index, page_patches) in grouped {
        applied_patch_count += page_patches.len();
        execute_region_patches(state, path.to_string(), page_index, page_patches).await?;
        touched_pages.push(page_index);
    }

    Ok(RegionPatchBatchApplyResult {
        applied_patch_count,
        touched_pages,
    })
}
/*
pub(crate)
fn is_font_unsupported_error(err: &str) -> bool {
    err.contains("unsupported") || err.contains("missing")
}
*/
