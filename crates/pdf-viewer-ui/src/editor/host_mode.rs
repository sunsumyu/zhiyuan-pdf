use serde::{Deserialize, Serialize};

use crate::editor::mode::{is_text_edit_mode_enabled, set_text_edit_mode_enabled};
use crate::editor::orchestrator::commit::commit_pending_edit_if_any;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToggleEditorModeResult {
    pub enabled: bool,
    pub changed: bool,
}

pub fn toggle_text_edit_mode() -> ToggleEditorModeResult {
    let currently_enabled = is_text_edit_mode_enabled();
    // 关闭编辑模式前必须强制 commit 任何未提交的 live state（架构 §4.1 不变量）
    if currently_enabled {
        let _ = commit_pending_edit_if_any();
    }
    let next_enabled = !currently_enabled;
    set_text_edit_mode_enabled(next_enabled);
    ToggleEditorModeResult {
        enabled: next_enabled,
        changed: true,
    }
}

pub fn set_text_edit_mode(enabled: bool) -> ToggleEditorModeResult {
    let previous = is_text_edit_mode_enabled();
    // 从 enabled → disabled 前先强制 commit
    if previous && !enabled {
        let _ = commit_pending_edit_if_any();
    }
    set_text_edit_mode_enabled(enabled);
    ToggleEditorModeResult {
        enabled,
        changed: previous != enabled,
    }
}
