use crate::document::patch_persistence::apply_document_patch_direct;
use crate::editor::editor_controller::build_patch;
use crate::editor::editor_controller::EditorVisibilityAction;
use crate::editor::mode::{close_active_editor, read_state};
use crate::editor::session::draft_text;
use crate::ui_state_store::remember_target as remember_paragraph_replacement_target;

/// 高层入口：若当前有未提交的 live state，强制走 commit 持久化它。
/// 用于"退出 edit mode / 切换 edit mode / 关闭编辑器"等所有路径，
/// 保证 Editing → Idle 的迁移必经 Persisting（见 docs/edit-save-architecture.md §4.1）。
/// 返回 true 表示有 patch 被持久化。
pub fn commit_pending() -> bool {
    let Some(text) = draft_text() else {
        crate::chain_trace!("exit.no-live-state");
        return false;
    };
    crate::chain_trace!(
        "exit.force-commit",
        "draftLen" => text.chars().count(),
    );
    let action = commit_text(text);
    action.changed
}

pub fn commit_text(new_text: String) -> EditorVisibilityAction {
    let active_state = read_state();
    crate::chain_trace!(
        "commit.start",
        "newLen" => new_text.chars().count(),
        "hasActive" => active_state.is_some(),
    );
    let patch_opt = build_patch(new_text);
    let Some(patch) = patch_opt else {
        crate::chain_trace!("commit.build", "ok" => false, "reason" => "noop-or-no-state");
        let changed = close_active_editor();
        return EditorVisibilityAction {
            changed: false,
            request_visibility_render: changed,
        };
    };
    crate::chain_trace!(
        "commit.build",
        "ok" => true,
        "regionId" => &patch.region_id,
        "source" => &patch.source,
        "origLen" => patch.original_text.chars().count(),
        "newLen" => patch.new_text.chars().count(),
    );
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
