use wasm_bindgen::JsValue;

use crate::bridge::target_invoke;
use crate::editor::list_format::reconcile_numbering_patches;
use crate::models::PersistableRegionPatch;
use crate::page::page_store::with_page_state;
use crate::ui_state_store::{
    record_patch, clear_persistable_patches as core_clear_persistable_patches,
    collect_persistable_patches,
};

/// Direct Rust-to-Rust patch application (no JsValue roundtrip).
/// Use this from internal Rust callers; only `apply_document_patch` should
/// be used when receiving a `JsValue` from the JS bridge.
pub fn apply_document_patch_direct(patch: PersistableRegionPatch) {
    crate::chain_trace!(
        "commit.persist",
        "regionId" => &patch.region_id,
        "source" => &patch.source,
        "pageIndex" => patch.page_index,
        "newLen" => patch.new_text.chars().count(),
    );
    record_patch(patch);
}

pub fn apply_document_patch(patch_js: JsValue) {
    match serde_wasm_bindgen::from_value::<PersistableRegionPatch>(patch_js) {
        Ok(patch) => {
            crate::chain_trace!(
                "commit.persist",
                "regionId" => &patch.region_id,
                "source" => &patch.source,
                "pageIndex" => patch.page_index,
                "newLen" => patch.new_text.chars().count(),
            );
            record_patch(patch);
        }
        Err(err) => {
            crate::chain_trace!(
                "commit.persist-error",
                "error" => format!("{:?}", err),
            );
        }
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
pub async fn save_persistable_patches(path: String, page_index: u16) -> Result<JsValue, JsValue> {
    let patches = with_page_state(|state| {
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
