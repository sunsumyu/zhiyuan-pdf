use wasm_bindgen::JsValue;

use crate::bridge::target_invoke;
use crate::editor::list_format::reconcile_numbering_patches;
use crate::models::PersistableRegionPatch;
use crate::page::runtime::HOST_PAGE_STATE;
use crate::state_manager::{
    apply_patch_with_history, clear_persistable_patches as core_clear_persistable_patches, collect_persistable_patches,
};

pub fn apply_document_patch(patch_js: JsValue) {
    if let Ok(patch) = serde_wasm_bindgen::from_value::<PersistableRegionPatch>(patch_js) {
        apply_patch_with_history(patch);
    }
}

pub fn has_persistable_patches() -> bool {
    !collect_persistable_patches().is_empty()
}

pub fn collect_persistable_patches_js() -> JsValue {
    serde_wasm_bindgen::to_value(&collect_persistable_patches()).unwrap_or(JsValue::NULL)
}

pub fn clear_persistable_patches(clear_history: bool) {
    core_clear_persistable_patches(clear_history);
}

/// Persists queued document patches through the host save command.
pub async fn save_persistable_patches(
    path: String,
    page_index: u16,
) -> Result<JsValue, JsValue> {
    let patches = HOST_PAGE_STATE.with(|state: &crate::page::runtime::HostPageState| {
        let state = state.borrow();
        let base_patches = collect_persistable_patches()
            .into_iter()
            .map(|mut patch| {
                patch.page_index = page_index;
                patch
            })
            .collect::<Vec<_>>();
        match state.paint_plan.as_ref() {
            Some(plan) => {
                reconcile_numbering_patches(plan, state.vector_model.as_ref(), base_patches)
            }
            None => base_patches,
        }
    });

    if patches.is_empty() {
        return Ok(JsValue::TRUE);
    }

    let args = serde_json::json!({
        "path": path,
        "pageIndex": page_index,
        "patches": patches,
    });

    let result = target_invoke(
        "apply_region_patches".into(),
        serde_wasm_bindgen::to_value(&args).unwrap_or(JsValue::NULL),
    )
    .await?;
    clear_persistable_patches(true);
    Ok(result)
}
