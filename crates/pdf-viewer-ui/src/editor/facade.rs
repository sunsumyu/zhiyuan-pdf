use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;

use crate::editor::activation::{MoveCaretToClientPointRequest, OpenEditorAtClientPointRequest};
use crate::editor::host_mode::set_text_edit_mode as host_set_text_edit_mode;
use crate::editor::mode::is_text_edit_mode_enabled;
use crate::editor::host_runtime::{
    begin_commit as host_begin_commit,
    finish_commit as host_finish_commit,
    get_state as host_get_runtime_state,
    reset_state as host_reset_runtime_state,
    set_display_zoom as host_set_display_zoom,
};
use crate::editor::host_mode::toggle_text_edit_mode as host_toggle_edit_mode;
use crate::editor::host_snapshot::{
    resolve_active_editor_diagnostics as host_resolve_diagnostics,
    resolve_editor_host_snapshot as host_resolve_snapshot,
};
use crate::editor::runtime::active_editor_format_state as host_active_format_state;
use crate::editor::visual::render_active_editor_canvas as host_paint_canvas;
use crate::editor::host_workflow::{
    move_caret_to_client_point as host_move_caret_to_client_point,
    save_editor_session as host_save_session,
};
use crate::editor::render_transaction::{
    apply_format_action_tx as host_apply_format_action_tx,
    apply_input_tx as host_apply_input_tx,
    close_editor_tx as host_close_editor_tx,
    commit_editor_silent_tx as host_commit_silent_tx,
    commit_editor_tx as host_commit_tx,
    open_editor_tx as host_open_editor_tx,
    open_region_editor_tx as host_open_region_editor_tx,
    sync_input_tx as host_sync_input_tx,
};
use crate::editor::runtime::EditorFormatAction;
use crate::editor::command::EditorInputCommand;
use crate::editor::session::active_editor_has_session_changes as host_has_session_changes;
use crate::editor::text_index::{
    char_index_to_utf16_offset as host_char_to_utf16,
    utf16_offset_to_char_index as host_utf16_to_char,
};
use crate::present::plan_builder::FramePlanRequest;
use crate::viewer::session::HOST_VIEWER_SESSION;
use crate::zoom::state::HOST_ZOOM_STATE;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorOpenRequest {
    pub paragraph_id: String,
    pub client_x: f32,
    pub client_y: f32,
    pub reference_left: f32,
    pub reference_top: f32,
    pub reference_width: f32,
    pub reference_height: f32,
    pub page_width: f32,
    pub page_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorOpenRegionRequest {
    pub page_index: u16,
    pub region_id: String,
    pub kind: String,
    pub original_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSyncInputRequest {
    pub text: String,
    pub caret_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorCommitRequest {
    pub draft_text: String,
    pub caret_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorCommandRequest {
    pub command: String,
    pub inserted_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorMoveCaretRequest {
    pub client_x: f32,
    pub client_y: f32,
    pub reference_left: f32,
    pub reference_top: f32,
    pub reference_width: f32,
    pub reference_height: f32,
    pub page_width: f32,
    pub page_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorFacadeResult {
    pub changed: bool,
    pub enabled: bool,
    pub caret_index: Option<u32>,
    pub draft_text: Option<String>,
    pub render_frame: Option<crate::render::workflow::RenderFrameEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditorSnapshotResult {
    pub enabled: bool,
    pub has_active_target: bool,
    pub paragraph_id: Option<String>,
    pub draft_text: Option<String>,
    pub caret_index: u32,
    pub has_persistable_patches: bool,
    pub target_count: u32,
}

fn build_frame_request() -> FramePlanRequest {
    let zoom_state = HOST_ZOOM_STATE.with(|s| s.borrow().clone());
    let viewer_session = HOST_VIEWER_SESSION.with(|s| s.borrow().clone());
    FramePlanRequest {
        display_zoom: zoom_state.target_zoom.max(0.1),
        render_reason: String::new(),
        page_width: viewer_session.page_width.max(1.0),
        page_height: viewer_session.page_height.max(1.0),
        viewport_width: 0.0,
        viewport_height: 0.0,
        scroll_left: 0.0,
        scroll_top: 0.0,
        device_pixel_ratio: 1.0,
        max_zoom: 8.0,
        max_canvas_dim: 4096.0,
        timestamp_ms: 0.0,
        force_static_render_scale: None,
    }
}

#[wasm_bindgen(js_name = "editorFacadeOpen")]
pub fn facade_open_editor(request_js: JsValue) -> JsValue {
    let request: EditorOpenRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    let open_request = OpenEditorAtClientPointRequest {
        paragraph_id: request.paragraph_id,
        client_x: request.client_x,
        client_y: request.client_y,
        reference_left: request.reference_left,
        reference_top: request.reference_top,
        reference_width: request.reference_width,
        reference_height: request.reference_height,
        page_width: request.page_width,
        page_height: request.page_height,
        fallback_page_x: 0.0,
        fallback_page_y: 0.0,
    };

    let frame_request = build_frame_request();
    let result = host_open_editor_tx(open_request, frame_request);

    let snapshot = host_resolve_snapshot(1.0);
    let caret_index = if snapshot.caret_index > 0 {
        snapshot.caret_index
    } else {
        snapshot.active_target.as_ref().map(|t| t.initial_caret_index).unwrap_or(0)
    };

    let facade_result = EditorFacadeResult {
        changed: result.changed,
        enabled: snapshot.enabled,
        caret_index: Some(caret_index as u32),
        draft_text: snapshot.draft_text,
        render_frame: result.render_frame,
    };

    to_value(&facade_result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeOpenRegion")]
pub fn facade_open_region_editor(request_js: JsValue) -> JsValue {
    let request: EditorOpenRegionRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    let frame_request = build_frame_request();
    let result = host_open_region_editor_tx(
        request.page_index,
        request.region_id,
        request.kind,
        request.original_text,
        frame_request,
    );

    let snapshot = host_resolve_snapshot(1.0);
    let caret_index = if snapshot.caret_index > 0 {
        snapshot.caret_index
    } else {
        snapshot.active_target.as_ref().map(|t| t.initial_caret_index).unwrap_or(0)
    };

    let facade_result = EditorFacadeResult {
        changed: result.changed,
        enabled: snapshot.enabled,
        caret_index: Some(caret_index as u32),
        draft_text: snapshot.draft_text,
        render_frame: result.render_frame,
    };

    to_value(&facade_result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeSyncInput")]
pub fn facade_sync_input(request_js: JsValue) -> JsValue {
    let request: EditorSyncInputRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    let frame_request = build_frame_request();
    let result = host_sync_input_tx(request.text, request.caret_index as usize, frame_request);

    let facade_result = EditorFacadeResult {
        changed: result.text_changed || result.caret_changed || result.scene_changed,
        enabled: is_text_edit_mode_enabled(),
        caret_index: Some(result.caret_index as u32),
        draft_text: None,
        render_frame: result.render_frame,
    };

    to_value(&facade_result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeCommit")]
pub fn facade_commit_editor(request_js: JsValue) -> JsValue {
    let request: EditorCommitRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    if !host_begin_commit() {
        return to_value(&EditorFacadeResult::default()).unwrap_or(JsValue::NULL);
    }

    let frame_request = build_frame_request();
    let result = host_commit_tx(request.draft_text, request.caret_index as usize, frame_request);

    host_finish_commit();

    let facade_result = EditorFacadeResult {
        changed: result.changed,
        enabled: is_text_edit_mode_enabled(),
        caret_index: None,
        draft_text: None,
        render_frame: result.render_frame,
    };

    to_value(&facade_result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeCommitSilent")]
pub fn facade_commit_silent(request_js: JsValue) -> JsValue {
    let request: EditorCommitRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    if !host_begin_commit() {
        return to_value(&EditorFacadeResult::default()).unwrap_or(JsValue::NULL);
    }

    let result = host_commit_silent_tx(request.draft_text, request.caret_index as usize);

    host_finish_commit();

    let facade_result = EditorFacadeResult {
        changed: result.changed,
        enabled: is_text_edit_mode_enabled(),
        caret_index: None,
        draft_text: None,
        render_frame: None,
    };

    to_value(&facade_result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeApplyCommand")]
pub fn facade_apply_command(request_js: JsValue) -> JsValue {
    let request: EditorCommandRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    let command = match request.command.as_str() {
        "backspace" => EditorInputCommand::DeleteBackward,
        "delete" => EditorInputCommand::DeleteForward,
        "insert" => EditorInputCommand::InsertText(request.inserted_text.as_deref().unwrap_or("")),
        _ => return JsValue::NULL,
    };

    let frame_request = build_frame_request();
    let result = host_apply_input_tx(command, frame_request);

    let snapshot = host_resolve_snapshot(1.0);

    let facade_result = EditorFacadeResult {
        changed: result.text_changed || result.caret_changed || result.scene_changed,
        enabled: is_text_edit_mode_enabled(),
        caret_index: Some(result.caret_index as u32),
        draft_text: snapshot.draft_text,
        render_frame: result.render_frame,
    };

    to_value(&facade_result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeClose")]
pub fn facade_close_editor() -> JsValue {
    let frame_request = build_frame_request();
    let result = host_close_editor_tx(frame_request);

    let facade_result = EditorFacadeResult {
        changed: result.changed,
        enabled: is_text_edit_mode_enabled(),
        caret_index: None,
        draft_text: None,
        render_frame: result.render_frame,
    };

    to_value(&facade_result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeMoveCaret")]
pub fn facade_move_caret(request_js: JsValue) -> JsValue {
    let request: EditorMoveCaretRequest = match serde_wasm_bindgen::from_value(request_js) {
        Ok(r) => r,
        Err(_) => return JsValue::NULL,
    };

    let move_request = MoveCaretToClientPointRequest {
        client_x: request.client_x,
        client_y: request.client_y,
        reference_left: request.reference_left,
        reference_top: request.reference_top,
        reference_width: request.reference_width,
        reference_height: request.reference_height,
        page_width: request.page_width,
        page_height: request.page_height,
    };

    let caret = host_move_caret_to_client_point(move_request);
    let caret_value = caret.unwrap_or(0) as u32;

    to_value(&EditorFacadeResult {
        changed: true,
        enabled: is_text_edit_mode_enabled(),
        caret_index: Some(caret_value),
        draft_text: None,
        render_frame: None,
    }).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeApplyFormat")]
pub fn facade_apply_format(action_js: JsValue) -> JsValue {
    let action: EditorFormatAction = match serde_wasm_bindgen::from_value(action_js) {
        Ok(a) => a,
        Err(_) => return JsValue::NULL,
    };

    let frame_request = build_frame_request();
    let result = host_apply_format_action_tx(action, frame_request);

    let facade_result = EditorFacadeResult {
        changed: result.changed,
        enabled: is_text_edit_mode_enabled(),
        caret_index: None,
        draft_text: None,
        render_frame: result.render_frame,
    };

    to_value(&facade_result).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeReadSnapshot")]
pub fn facade_read_snapshot(display_zoom: f32) -> JsValue {
    // Return the full EditorHostSnapshot so TS can access active_target / targets / draft_text /
    // caret_index / has_persistable_patches as nested objects. The previously flattened
    // EditorSnapshotResult lost active_target which broke editor_host.ts open paths.
    let snapshot = host_resolve_snapshot(display_zoom);
    to_value(&snapshot).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeSetEditMode")]
pub fn facade_set_edit_mode(enabled: bool) -> JsValue {
    let result = host_set_text_edit_mode(enabled);

    to_value(&EditorFacadeResult {
        changed: result.changed,
        enabled: result.enabled,
        caret_index: None,
        draft_text: None,
        render_frame: None,
    }).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeHasSessionChanges")]
pub fn facade_has_session_changes() -> bool {
    host_has_session_changes()
}

#[wasm_bindgen(js_name = "editorFacadeUtf16ToCharIndex")]
pub fn facade_utf16_to_char_index(text: &str, utf16_offset: u32) -> u32 {
    host_utf16_to_char(text, utf16_offset as usize) as u32
}

#[wasm_bindgen(js_name = "editorFacadeCharToUtf16Offset")]
pub fn facade_char_to_utf16_offset(text: &str, char_index: u32) -> u32 {
    host_char_to_utf16(text, char_index as usize) as u32
}

// ─────────────────────────────────────────────────────────────────────────────
// Editor facade — runtime / diagnostics / format state
// (Stable, additions only)
// ─────────────────────────────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "editorFacadePaintCanvas")]
pub fn facade_paint_canvas(
    canvas_js: JsValue,
    display_zoom: f32,
    draft_text: String,
    caret_index: u32,
) -> bool {
    host_paint_canvas(canvas_js, display_zoom, draft_text, caret_index)
}

#[wasm_bindgen(js_name = "editorFacadeReadDiagnostics")]
pub fn facade_read_diagnostics() -> JsValue {
    to_value(&host_resolve_diagnostics()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeReadRuntime")]
pub fn facade_read_runtime() -> JsValue {
    to_value(&host_get_runtime_state()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeResetRuntime")]
pub fn facade_reset_runtime() {
    host_reset_runtime_state();
}

#[wasm_bindgen(js_name = "editorFacadeSetDisplayZoom")]
pub fn facade_set_display_zoom(display_zoom: f32) {
    host_set_display_zoom(display_zoom);
}

#[wasm_bindgen(js_name = "editorFacadeBeginCommit")]
pub fn facade_begin_commit() -> bool {
    host_begin_commit()
}

#[wasm_bindgen(js_name = "editorFacadeFinishCommit")]
pub fn facade_finish_commit() {
    host_finish_commit();
}

#[wasm_bindgen(js_name = "editorFacadeToggleMode")]
pub fn facade_toggle_mode() -> JsValue {
    to_value(&host_toggle_edit_mode()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeReadFormatState")]
pub fn facade_read_format_state() -> JsValue {
    to_value(&host_active_format_state()).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen(js_name = "editorFacadeSaveSession")]
pub async fn facade_save_session(path: String, page_index: u16) -> JsValue {
    to_value(&host_save_session(path, page_index).await).unwrap_or(JsValue::NULL)
}

// ─────────────────────────────────────────────────────────────────────────────
// Editor facade — STUB API (reserved, returns standardized "not_implemented")
// These names are FROZEN. Implementations will land in subsequent releases.
// Do NOT inline-implement here; create dedicated workflow modules.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StubResult {
    pub implemented: bool,
    pub error: String,
}

fn stub(api: &str) -> JsValue {
    let result = StubResult {
        implemented: false,
        error: format!("{} is reserved but not yet implemented", api),
    };
    to_value(&result).unwrap_or(JsValue::NULL)
}

/// Reserved: select a range in the active editor.
#[wasm_bindgen(js_name = "editorFacadeSelectRange")]
pub fn facade_select_range(_start_char: u32, _end_char: u32) -> JsValue {
    stub("editor.selectRange")
}

/// Reserved: cut current selection to clipboard payload.
#[wasm_bindgen(js_name = "editorFacadeCut")]
pub fn facade_cut() -> JsValue {
    stub("editor.cut")
}

/// Reserved: copy current selection.
#[wasm_bindgen(js_name = "editorFacadeCopy")]
pub fn facade_copy() -> JsValue {
    stub("editor.copy")
}

/// Reserved: paste plain text at caret.
#[wasm_bindgen(js_name = "editorFacadePaste")]
pub fn facade_paste(_text: String) -> JsValue {
    stub("editor.paste")
}

/// Reserved: editor-level undo (within active session).
#[wasm_bindgen(js_name = "editorFacadeUndo")]
pub fn facade_undo() -> JsValue {
    stub("editor.undo")
}

/// Reserved: editor-level redo.
#[wasm_bindgen(js_name = "editorFacadeRedo")]
pub fn facade_redo() -> JsValue {
    stub("editor.redo")
}

/// Reserved: find substring within active paragraph.
#[wasm_bindgen(js_name = "editorFacadeFindInActive")]
pub fn facade_find_in_active(_query: String, _case_sensitive: bool) -> JsValue {
    stub("editor.findInActive")
}

/// Reserved: replace substring within active paragraph.
#[wasm_bindgen(js_name = "editorFacadeReplaceInActive")]
pub fn facade_replace_in_active(
    _query: String,
    _replacement: String,
    _case_sensitive: bool,
    _replace_all: bool,
) -> JsValue {
    stub("editor.replaceInActive")
}

