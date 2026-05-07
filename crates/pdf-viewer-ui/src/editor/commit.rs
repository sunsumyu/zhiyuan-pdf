use crate::editor::mode::{close_active_editor, get_active_editor_state};
use crate::editor::runtime::build_active_editor_patch;
use crate::editor::runtime::EditorVisibilityAction;
use crate::document::patch_persistence::apply_document_patch_direct;
use crate::state_manager::remember_paragraph_replacement_target;

pub fn commit_active_editor_text(new_text: String) -> EditorVisibilityAction {
    let active_state = get_active_editor_state();
    let new_text_preview: String = new_text.chars().take(40).collect();
    web_sys::console::log_1(&format!(
        "[COMMIT-TEXT] enter newTextLen={} newTextPreview='{}' hasActiveState={}",
        new_text.chars().count(), new_text_preview, active_state.is_some()
    ).into());
    let Some(patch) = build_active_editor_patch(new_text) else {
        web_sys::console::log_1(&"[COMMIT-TEXT] !!! build_active_editor_patch returned None (noop or no active state)".to_string().into());
        let changed = close_active_editor();
        return EditorVisibilityAction {
            changed: false,
            request_visibility_render: changed,
        };
    };
    web_sys::console::log_1(&format!(
        "[COMMIT-TEXT] patch built regionId={} source={} originalLen={} newLen={}",
        patch.region_id, patch.source,
        patch.original_text.chars().count(), patch.new_text.chars().count()
    ).into());
    if let Some(active_state) = active_state {
        let active_paragraph_id = active_state.paragraph_id().to_string();
        let replacement_target = active_state.target;
        remember_paragraph_replacement_target(&patch.region_id, replacement_target.clone());
        if active_paragraph_id != patch.region_id {
            remember_paragraph_replacement_target(&active_paragraph_id, replacement_target);
        }
    }
    apply_document_patch_direct(patch);
    let changed = close_active_editor();
    EditorVisibilityAction {
        changed: true,
        request_visibility_render: changed,
    }
}
