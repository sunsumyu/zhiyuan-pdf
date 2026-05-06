use crate::editor::mode::{close_active_editor, get_active_editor_state};
use crate::editor::runtime::build_active_editor_patch;
use crate::editor::runtime::EditorVisibilityAction;
use crate::document::patch_persistence::apply_document_patch;
use crate::state_manager::remember_paragraph_replacement_target;

pub fn commit_active_editor_text(new_text: String) -> EditorVisibilityAction {
    let active_state = get_active_editor_state();
    let Some(patch) = build_active_editor_patch(new_text) else {
        let changed = close_active_editor();
        return EditorVisibilityAction {
            changed: false,
            request_visibility_render: changed,
        };
    };
    if let Some(active_state) = active_state {
        let active_paragraph_id = active_state.paragraph_id().to_string();
        let replacement_target = active_state.target;
        remember_paragraph_replacement_target(&patch.region_id, replacement_target.clone());
        if active_paragraph_id != patch.region_id {
            remember_paragraph_replacement_target(&active_paragraph_id, replacement_target);
        }
    }
    let patch_js = serde_wasm_bindgen::to_value(&patch).unwrap_or(wasm_bindgen::JsValue::NULL);
    apply_document_patch(patch_js);
    let changed = close_active_editor();
    EditorVisibilityAction {
        changed: true,
        request_visibility_render: changed,
    }
}
