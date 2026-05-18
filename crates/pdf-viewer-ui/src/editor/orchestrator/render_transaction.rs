use serde::{Deserialize, Serialize};

use crate::editor::activation::{
    activate_editor_from_client_point, activate_region_editor,
    OpenEditorAtClientPointRequest,
};
use crate::editor::command::{
    apply_editor_input_command, apply_input_with_host, EditorInputCommand,
};
use crate::editor::orchestrator::commit::commit_active_editor_text;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::editor::mode::close_active_editor;
use crate::editor::editor_controller::{
    apply_active_editor_format_action, sync_editor_input, EditorFormatAction,
};
use crate::editor::session::{active_editor_draft_text, active_editor_has_session_changes};
use crate::present::plan_builder::FramePlanRequest;
use crate::present::present_store::schedule_render_frame_request;
use crate::render::workflow::RenderFrameEnvelope;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorRenderTransactionResult {
    pub changed: bool,
    pub render_frame: Option<RenderFrameEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorInputRenderTransactionResult {
    pub text_changed: bool,
    pub caret_changed: bool,
    pub scene_changed: bool,
    pub caret_index: usize,
    pub render_frame: Option<RenderFrameEnvelope>,
}

fn schedule_editor_render(
    frame_request: &FramePlanRequest,
    should_render: bool,
) -> Option<RenderFrameEnvelope> {
    schedule_editor_render_with_reason(frame_request, should_render, "editorVisibility")
}

fn schedule_editor_render_with_reason(
    frame_request: &FramePlanRequest,
    should_render: bool,
    render_reason: &str,
) -> Option<RenderFrameEnvelope> {
    if !should_render {
        return None;
    }

    let mut render_request = frame_request.clone();
    render_request.render_reason = render_reason.to_string();
    schedule_render_frame_request(&render_request)
}

pub fn open_editor_tx(
    request: OpenEditorAtClientPointRequest,
    _frame_request: FramePlanRequest,
) -> EditorRenderTransactionResult {
    // NOTE: We intentionally do NOT call schedule_editor_render here. Previously
    // this leaked a frame_token (allocated in RENDER_STATE) that the JS callers
    // never consumed, deadlocking subsequent schedule_render_frame calls during
    // editing. The JS host is responsible for invoking renderCurrentPage when
    // it needs the page canvas to refresh (e.g. to suppress source text runs
    // for the active editor).
    let action = activate_editor_from_client_point(request);
    EditorRenderTransactionResult {
        changed: action.changed,
        render_frame: None,
    }
}

pub fn open_region_editor_tx(
    page_index: u16,
    region_id: String,
    kind: String,
    original_text: String,
    _frame_request: FramePlanRequest,
) -> EditorRenderTransactionResult {
    let action = activate_region_editor(page_index, &region_id, &kind, &original_text);
    EditorRenderTransactionResult {
        changed: action.changed,
        render_frame: None,
    }
}

pub fn sync_input_tx(
    new_text: String,
    caret_index: usize,
    _frame_request: FramePlanRequest,
) -> EditorInputRenderTransactionResult {
    let text_len = new_text.chars().count();
    let result = sync_editor_input(new_text, caret_index);
    crate::chain_trace!("sync_input_tx",
        "inCaret" => caret_index,
        "inTextLen" => text_len,
        "outCaret" => result.caret_index,
        "textChanged" => result.text_changed,
        "caretChanged" => result.caret_changed
    );
    EditorInputRenderTransactionResult {
        text_changed: result.text_changed,
        caret_changed: result.caret_changed,
        scene_changed: result.scene_changed,
        caret_index: result.caret_index,
        render_frame: None,
    }
}

pub fn apply_input_tx(
    command: EditorInputCommand<'_>,
    _frame_request: FramePlanRequest,
) -> EditorInputRenderTransactionResult {
    let result = apply_editor_input_command(command);
    EditorInputRenderTransactionResult {
        text_changed: result.text_changed,
        caret_changed: result.caret_changed,
        scene_changed: result.scene_changed,
        caret_index: result.caret_index,
        render_frame: None,
    }
}

pub fn apply_host_input_tx(
    command: EditorInputCommand<'_>,
    host_text: Option<String>,
    host_caret_index: Option<usize>,
    _frame_request: FramePlanRequest,
) -> EditorInputRenderTransactionResult {
    let result = apply_input_with_host(command, host_text, host_caret_index);
    EditorInputRenderTransactionResult {
        text_changed: result.text_changed,
        caret_changed: result.caret_changed,
        scene_changed: result.scene_changed,
        caret_index: result.caret_index,
        render_frame: None,
    }
}

pub fn commit_editor_tx(
    new_text: String,
    caret_index: usize,
    _frame_request: FramePlanRequest,
) -> EditorRenderTransactionResult {
    let has_session_changes = active_editor_has_session_changes();
    let commit_text = if has_session_changes {
        let _sync_result = sync_editor_input(new_text.clone(), caret_index);
        new_text
    } else {
        dbg_event(
            "render-tx.commit",
            "clean-session-skip-host-sync",
            vec![
                dbg_field("caretIndex", caret_index),
                dbg_field("hostText", &new_text),
            ],
        );
        active_editor_draft_text().unwrap_or(new_text)
    };
    let action = commit_active_editor_text(commit_text);
    // NOTE: We intentionally do NOT call schedule_editor_render here.
    // The JS host calls renderCurrentPage('documentMutation') after commit,
    // which triggers refreshMutatedDocument(). Scheduling a frame here would
    // leak a token that JS never consumes, deadlocking ALL subsequent
    // schedule_render_frame calls. (Same pattern as open_editor_tx.)
    EditorRenderTransactionResult {
        changed: action.changed,
        render_frame: None,
    }
}

pub fn commit_editor_silent_tx(
    new_text: String,
    caret_index: usize,
) -> EditorRenderTransactionResult {
    let has_session_changes = active_editor_has_session_changes();
    let commit_text = if has_session_changes {
        let _sync_result = sync_editor_input(new_text.clone(), caret_index);
        new_text
    } else {
        dbg_event(
            "render-tx.commit",
            "clean-session-skip-host-sync-silent",
            vec![
                dbg_field("caretIndex", caret_index),
                dbg_field("hostText", &new_text),
            ],
        );
        active_editor_draft_text().unwrap_or(new_text)
    };
    let action = commit_active_editor_text(commit_text);
    EditorRenderTransactionResult {
        changed: action.changed,
        render_frame: None,
    }
}

pub fn close_editor_tx(_frame_request: FramePlanRequest) -> EditorRenderTransactionResult {
    // P0 止血：原实现直接丢弃 live state，走该路径的退出（外部点击 / ESC / blur /
    // Ctrl+R 前清理）会丢失编辑。当存在 live draft 时强制走 commit，保证
    // "Editing → Idle" 的迁移必经 Persisting（见 docs/edit-save-architecture.md §4.1）。
    if let Some(draft_text) = active_editor_draft_text() {
        dbg_event(
            "render-tx.close",
            "force-commit-pending-edit",
            vec![dbg_field("draftLen", draft_text.chars().count())],
        );
        let action = commit_active_editor_text(draft_text);
        // NOTE: Do NOT schedule a render frame here — same rationale as
        // commit_editor_tx and open_editor_tx. The JS host handles the
        // post-close render via renderCurrentPage(). Scheduling here leaks
        // a token that JS never consumes, deadlocking the render pipeline.
        return EditorRenderTransactionResult {
            changed: action.changed,
            render_frame: None,
        };
    }
    let changed = close_active_editor();
    // Same: let JS drive the render.
    EditorRenderTransactionResult {
        changed,
        render_frame: None,
    }
}

fn format_render_tx(
    frame_request: FramePlanRequest,
    changed: bool,
) -> EditorRenderTransactionResult {
    EditorRenderTransactionResult {
        changed,
        render_frame: schedule_editor_render(&frame_request, changed),
    }
}

pub fn apply_format_action_tx(
    action: EditorFormatAction,
    frame_request: FramePlanRequest,
) -> EditorRenderTransactionResult {
    format_render_tx(
        frame_request,
        apply_active_editor_format_action(action).changed,
    )
}
