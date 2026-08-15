use serde::Deserialize;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::editor::editor_store;
use crate::editor::editor_types::*;
use crate::guard_state;

// ── Incoming request DTOs (JS → Rust) ───────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HitTestRequest {
    client_x: f32,
    client_y: f32,
    reference_left: f32,
    reference_top: f32,
    reference_width: f32,
    reference_height: f32,
    page_width: f32,
    page_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenBlockRequest {
    block_id: String,
    client_x: f32,
    client_y: f32,
    reference_left: f32,
    reference_top: f32,
    reference_width: f32,
    reference_height: f32,
    page_width: f32,
    page_height: f32,
    #[serde(default)]
    fallback_page_x: f32,
    #[serde(default)]
    fallback_page_y: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveCaretRequest {
    client_x: f32,
    client_y: f32,
    reference_left: f32,
    reference_top: f32,
    reference_width: f32,
    reference_height: f32,
    page_width: f32,
    page_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitRequest {
    draft_text: String,
    caret_index: u32,
}

// ── EditorSession ───────────────────────────────────────────────

#[wasm_bindgen]
pub struct EditorSession;

#[wasm_bindgen]
impl EditorSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        EditorSession
    }

    // ── P0: Lifecycle ───────────────────────────────────────────

    /// Activate edit mode: Viewing → Editing.
    /// Returns list of editable text blocks on the current page.
    #[wasm_bindgen(js_name = "begin")]
    pub fn begin(&self) -> JsValue {
        guard_state!(SessionState::Viewing, "begin");

        // Enable edit mode in existing infrastructure
        use crate::editor::host_mode::set_text_edit_mode;
        let _mode_result = set_text_edit_mode(true);

        editor_store::transition_to_editing();

        // Collect available text blocks
        let blocks = collect_text_blocks();
        ok_response(blocks, true)
    }

    /// Hit-test: find which block (if any) is at the given client point.
    /// Allowed in Editing or EditingBlock state.
    #[wasm_bindgen(js_name = "hitTest")]
    pub fn hit_test(&self, request_js: JsValue) -> JsValue {
        guard_state!(
            SessionState::Editing | SessionState::EditingBlock,
            "hit_test"
        );

        let request: HitTestRequest = match parse_request(request_js, "hit_test") {
            Ok(r) => r,
            Err(js) => return js,
        };

        use pdf_viewer_core::geometry::coordinate_transform::{
            ClientPoint, HostPageTransform, HostReferenceRect, PageSize,
        };

        let transform = HostPageTransform::new(
            HostReferenceRect {
                left: request.reference_left,
                top: request.reference_top,
                width: request.reference_width,
                height: request.reference_height,
            },
            PageSize {
                width: request.page_width,
                height: request.page_height,
            },
        );
        let page_point = transform.client_to_page(ClientPoint {
            x: request.client_x,
            y: request.client_y,
        });

        // Delegate to existing hit-test logic
        let target = resolve_target_at_page_point(page_point.x, page_point.y);

        ok_response(
            HitTestResult {
                block_id: target.map(|t| t.paragraph_id),
                page_x: page_point.x,
                page_y: page_point.y,
            },
            false,
        )
    }

    /// Open a specific block for editing: Editing → EditingBlock.
    /// If called from EditingBlock, auto-commits current block first (block switch).
    #[wasm_bindgen(js_name = "openBlock")]
    pub fn open_block(&self, request_js: JsValue) -> JsValue {
        let current = editor_store::read_state();
        match current {
            SessionState::Editing => {}
            SessionState::EditingBlock => {
                // Auto-commit current block before switching
                self.commit_draft_internal();
                editor_store::set_state(SessionState::Editing);
            }
            _ => {
                return err_response(EditorError::InvalidState {
                    expected: "Editing | EditingBlock".to_string(),
                    actual: current.as_str().to_string(),
                });
            }
        }

        let request: OpenBlockRequest = match parse_request(request_js, "open_block") {
            Ok(r) => r,
            Err(js) => return js,
        };

        // Delegate to existing activation logic
        use crate::editor::activation::OpenEditorAtClientPointRequest;
        use crate::editor::orchestrator::render_transaction::open_editor_tx;

        let open_request = OpenEditorAtClientPointRequest {
            paragraph_id: request.block_id.clone(),
            client_x: request.client_x,
            client_y: request.client_y,
            reference_left: request.reference_left,
            reference_top: request.reference_top,
            reference_width: request.reference_width,
            reference_height: request.reference_height,
            page_width: request.page_width,
            page_height: request.page_height,
            fallback_page_x: request.fallback_page_x,
            fallback_page_y: request.fallback_page_y,
        };

        let frame_request = build_frame_request();
        let result = open_editor_tx(open_request, frame_request);

        if !result.changed {
            return err_response(EditorError::NotFound {
                entity: "block".to_string(),
                id: request.block_id,
            });
        }

        // Read snapshot to get caret + draft text
        use crate::editor::host_snapshot::resolve_editor_host_snapshot;
        let snapshot = resolve_editor_host_snapshot(1.0);
        let caret_index = if snapshot.caret_index > 0 {
            snapshot.caret_index
        } else {
            snapshot
                .active_target
                .as_ref()
                .map(|t| t.initial_caret_index)
                .unwrap_or(0)
        };

        editor_store::transition_to_editing_block(request.block_id.clone());

        ok_response(
            OpenBlockResult {
                block_id: request.block_id,
                caret_index: caret_index as u32,
                draft_text: snapshot.draft_text.unwrap_or_default(),
            },
            true,
        )
    }

    /// Move caret within the active block.
    /// Returns None (null caret) if click is on blank space → caller should close_block.
    #[wasm_bindgen(js_name = "moveCaret")]
    pub fn move_caret(&self, request_js: JsValue) -> JsValue {
        guard_state!(SessionState::EditingBlock, "move_caret");

        let request: MoveCaretRequest = match parse_request(request_js, "move_caret") {
            Ok(r) => r,
            Err(js) => return js,
        };

        use crate::editor::activation::{MoveCaretToClientPointRequest, move_caret_to_client_point};

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

        let caret = move_caret_to_client_point(move_request);

        match caret {
            Some(index) => ok_response(
                MoveCaretResult {
                    caret_index: index as u32,
                },
                false,
            ),
            None => {
                // Blank click within shell — return null data so TS knows to close
                ok_empty(false)
            }
        }
    }

    /// Close the active block without committing: EditingBlock → Viewing.
    /// If there are pending edits, they are force-committed (P0 safety).
    #[wasm_bindgen(js_name = "closeBlock")]
    pub fn close_block(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "close_block");

        use crate::editor::orchestrator::render_transaction::close_editor_tx;
        let frame_request = build_frame_request();
        let result = close_editor_tx(frame_request);

        // close = close = back to Viewing
        editor_store::transition_to_viewing();

        // Also disable edit mode in old infrastructure
        use crate::editor::host_mode::set_text_edit_mode;
        let _ = set_text_edit_mode(false);

        ok_response(
            CommitResult {
                changed: result.changed,
            },
            true,
        )
    }

    /// Commit the active block's edits: EditingBlock → Viewing.
    #[wasm_bindgen(js_name = "commit")]
    pub fn commit(&self, request_js: JsValue) -> JsValue {
        guard_state!(SessionState::EditingBlock, "commit");

        let request: CommitRequest = match parse_request(request_js, "commit") {
            Ok(r) => r,
            Err(js) => return js,
        };

        use crate::editor::host_runtime::{begin_commit, finish_commit};
        use crate::editor::orchestrator::render_transaction::commit_editor_tx;

        if !begin_commit() {
            return err_response(EditorError::InvalidState {
                expected: "not committing".to_string(),
                actual: "already committing".to_string(),
            });
        }

        let frame_request = build_frame_request();
        let result = commit_editor_tx(
            request.draft_text,
            request.caret_index as usize,
            frame_request,
        );
        finish_commit();

        // commit = done = back to Viewing
        editor_store::transition_to_viewing();

        use crate::editor::host_mode::set_text_edit_mode;
        let _ = set_text_edit_mode(false);

        ok_response(
            CommitResult {
                changed: result.changed,
            },
            true,
        )
    }

    /// End the editing session, auto-committing any pending block edits.
    /// Allowed from `Editing` or `EditingBlock`; both end at `Viewing`.
    ///
    /// Distinct from `commit()` (which requires `EditingBlock`) and
    /// `discard()` (which throws away unsaved edits). `end()` is the
    /// "save and exit" exit point — equivalent to Nutrient's `session.commit()`.
    /// See architecture proposal §14.3.
    #[wasm_bindgen(js_name = "end")]
    pub fn end(&self) -> JsValue {
        guard_state!(SessionState::Editing | SessionState::EditingBlock, "end");

        if editor_store::read_state() == SessionState::EditingBlock {
            self.commit_draft_internal();
        }
        editor_store::transition_to_viewing();

        use crate::editor::host_mode::set_text_edit_mode;
        let _ = set_text_edit_mode(false);

        ok_empty(true)
    }

    /// Discard all edits and exit: any state → Viewing.
    #[wasm_bindgen(js_name = "discard")]
    pub fn discard(&self) -> JsValue {
        let current = editor_store::read_state();
        match current {
            SessionState::Viewing => {
                return ok_empty(false);
            }
            SessionState::Saving => {
                return err_response(EditorError::InvalidState {
                    expected: "not Saving".to_string(),
                    actual: "Saving".to_string(),
                });
            }
            _ => {}
        }

        // Close active editor if any
        use crate::editor::mode::close_active_editor;
        close_active_editor();

        editor_store::transition_to_viewing();

        use crate::editor::host_mode::set_text_edit_mode;
        let _ = set_text_edit_mode(false);

        ok_empty(true)
    }

    // ── P0: Query ───────────────────────────────────────────────

    /// Read current session snapshot.
    #[wasm_bindgen(js_name = "readSnapshot")]
    pub fn read_snapshot(&self, display_zoom: f32) -> JsValue {
        use crate::document::patch_persistence::has_persistable_patches;
        use crate::editor::host_snapshot::resolve_editor_host_snapshot;

        let snapshot = resolve_editor_host_snapshot(display_zoom);

        ok_response(
            SnapshotResult {
                state: editor_store::read_state(),
                block_id: editor_store::read_active_block_id(),
                draft_text: snapshot.draft_text,
                caret_index: snapshot.caret_index as u32,
                has_unsaved_changes: has_persistable_patches(),
            },
            false,
        )
    }

    #[wasm_bindgen(js_name = "getSnapshot")]
    #[deprecated(since = "0.2.0", note = "Use readSnapshot instead")]
    pub fn get_snapshot(&self, display_zoom: f32) -> JsValue {
        self.read_snapshot(display_zoom)
    }

    /// Check if the session is in an active editing state.
    #[wasm_bindgen(js_name = "isActive")]
    pub fn is_active(&self) -> bool {
        matches!(
            editor_store::read_state(),
            SessionState::Editing | SessionState::EditingBlock
        )
    }

    /// Check if there are unsaved changes.
    #[wasm_bindgen(js_name = "hasUnsavedChanges")]
    pub fn has_unsaved_changes(&self) -> bool {
        crate::document::patch_persistence::has_persistable_patches()
    }

    // ── P0: Bridge methods (replace legacy TS facade calls) ────

    /// Sync textarea content into Rust state (composition / IME).
    #[wasm_bindgen(js_name = "syncInput")]
    pub fn sync_input(&self, request_js: JsValue) -> JsValue {
        guard_state!(SessionState::EditingBlock, "sync_input");

        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SyncInputRequest {
            text: String,
            caret_index: u32,
        }

        let request: SyncInputRequest = match parse_request(request_js, "sync_input") {
            Ok(r) => r,
            Err(js) => return js,
        };

        use crate::editor::orchestrator::render_transaction::sync_input_tx;
        let frame_request = build_frame_request();
        let result = sync_input_tx(request.text, request.caret_index as usize, frame_request);

        ok_response(
            SyncInputResult {
                changed: result.text_changed || result.caret_changed || result.scene_changed,
                caret_index: result.caret_index as u32,
            },
            result.render_frame.is_some(),
        )
    }

    /// Apply an editor input command (insert, backspace, delete, navigation).
    /// Returns updated draft text + caret.
    #[wasm_bindgen(js_name = "applyCommand")]
    pub fn apply_command(&self, request_js: JsValue) -> JsValue {
        guard_state!(SessionState::EditingBlock, "apply_command");

        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CommandRequest {
            command: String,
            inserted_text: Option<String>,
        }

        let request: CommandRequest = match parse_request(request_js, "apply_command") {
            Ok(r) => r,
            Err(js) => return js,
        };

        use crate::editor::command::EditorInputCommand;
        use crate::editor::host_snapshot::resolve_editor_host_snapshot;
        use crate::editor::orchestrator::render_transaction::apply_input_tx;

        let command = match request.command.as_str() {
            "backspace" => EditorInputCommand::DeleteBackward,
            "delete" => EditorInputCommand::DeleteForward,
            "insert" => {
                EditorInputCommand::InsertText(request.inserted_text.as_deref().unwrap_or(""))
            }
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Home" | "End" => {
                EditorInputCommand::Navigation(&request.command)
            }
            other => {
                return err_response(EditorError::Internal {
                    message: format!("unknown command: {other}"),
                });
            }
        };

        let frame_request = build_frame_request();
        let result = apply_input_tx(command, frame_request);
        let snapshot = resolve_editor_host_snapshot(1.0);

        ok_response(
            ApplyCommandResult {
                changed: result.text_changed || result.caret_changed || result.scene_changed,
                caret_index: result.caret_index as u32,
                draft_text: snapshot.draft_text,
            },
            result.render_frame.is_some(),
        )
    }

    /// Enable or disable text edit mode.
    #[wasm_bindgen(js_name = "setEditMode")]
    pub fn set_edit_mode(&self, enabled: bool) -> JsValue {
        use crate::editor::host_mode::set_text_edit_mode;
        let result = set_text_edit_mode(enabled);

        // Sync our state machine
        if enabled {
            let current = editor_store::read_state();
            if current == SessionState::Viewing {
                editor_store::transition_to_editing();
            }
        } else {
            editor_store::transition_to_viewing();
        }

        ok_response(
            SetEditModeResult {
                enabled: result.enabled,
                changed: result.changed,
            },
            false,
        )
    }

    /// Read the full legacy snapshot (with activeTarget, targets, DOM coords).
    /// This is needed by TS for positioning the editor shell.
    #[wasm_bindgen(js_name = "readLegacySnapshot")]
    pub fn read_legacy_snapshot(&self, display_zoom: f32) -> JsValue {
        use crate::editor::host_snapshot::resolve_editor_host_snapshot;
        let snapshot = resolve_editor_host_snapshot(display_zoom);
        to_value(&snapshot).unwrap_or(JsValue::NULL)
    }

    /// Paint the editor canvas via the Rust glyph backend.
    /// Must stay as a direct WASM call because it takes an HTMLCanvasElement.
    #[wasm_bindgen(js_name = "paintCanvas")]
    pub fn paint_canvas(
        &self,
        canvas_js: JsValue,
        display_zoom: f32,
        draft_text: String,
        caret_index: u32,
    ) -> bool {
        use crate::editor::visual::render_active_editor_canvas;
        render_active_editor_canvas(canvas_js, display_zoom, draft_text, caret_index)
    }

    /// Convert a UTF-16 offset to a Rust char index.
    #[wasm_bindgen(js_name = "utf16ToCharIndex")]
    pub fn utf16_to_char_index(&self, text: &str, utf16_offset: u32) -> u32 {
        use crate::editor::text_index::utf16_offset_to_char_index;
        utf16_offset_to_char_index(text, utf16_offset as usize) as u32
    }

    /// Convert a Rust char index to a UTF-16 offset.
    #[wasm_bindgen(js_name = "charToUtf16Offset")]
    pub fn char_to_utf16_offset(&self, text: &str, char_index: u32) -> u32 {
        use crate::editor::text_index::char_index_to_utf16_offset;
        char_index_to_utf16_offset(text, char_index as usize) as u32
    }

    /// Check if there are uncommitted session changes in the active editor.
    #[wasm_bindgen(js_name = "hasSessionChanges")]
    pub fn has_session_changes(&self) -> bool {
        use crate::editor::session::active_editor_has_session_changes;
        active_editor_has_session_changes()
    }

    /// Open a region-based editor (used by document review flows).
    #[wasm_bindgen(js_name = "openRegion")]
    pub fn open_region(&self, request_js: JsValue) -> JsValue {
        let current = editor_store::read_state();
        match current {
            SessionState::Viewing | SessionState::Editing => {}
            SessionState::EditingBlock => {
                self.commit_draft_internal();
                editor_store::set_state(SessionState::Editing);
            }
            _ => {
                return err_response(EditorError::InvalidState {
                    expected: "Viewing | Editing | EditingBlock".to_string(),
                    actual: current.as_str().to_string(),
                });
            }
        }

        #[derive(Debug, Clone, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct OpenRegionRequest {
            page_index: u16,
            region_id: String,
            kind: String,
            original_text: String,
        }

        let request: OpenRegionRequest = match parse_request(request_js, "open_region") {
            Ok(r) => r,
            Err(js) => return js,
        };

        use crate::editor::orchestrator::render_transaction::open_region_editor_tx;
        let frame_request = build_frame_request();
        let result = open_region_editor_tx(
            request.page_index,
            request.region_id.clone(),
            request.kind,
            request.original_text,
            frame_request,
        );

        if !result.changed {
            return err_response(EditorError::NotFound {
                entity: "region".to_string(),
                id: request.region_id,
            });
        }

        use crate::editor::host_snapshot::resolve_editor_host_snapshot;
        let snapshot = resolve_editor_host_snapshot(1.0);
        let caret_index = if snapshot.caret_index > 0 {
            snapshot.caret_index
        } else {
            snapshot
                .active_target
                .as_ref()
                .map(|t| t.initial_caret_index)
                .unwrap_or(0)
        };

        // Ensure edit mode is on
        use crate::editor::host_mode::set_text_edit_mode;
        let _ = set_text_edit_mode(true);
        editor_store::transition_to_editing_block(request.region_id);

        ok_response(
            OpenBlockResult {
                block_id: "region".to_string(),
                caret_index: caret_index as u32,
                draft_text: snapshot.draft_text.unwrap_or_default(),
            },
            true,
        )
    }

    /// Set the display zoom level for the editor.
    #[wasm_bindgen(js_name = "setDisplayZoom")]
    pub fn set_display_zoom(&self, display_zoom: f32) {
        use crate::editor::host_runtime::set_display_zoom;
        set_display_zoom(display_zoom);
    }

    /// Read active editor diagnostics (debug info).
    #[wasm_bindgen(js_name = "readDiagnostics")]
    pub fn read_diagnostics(&self) -> JsValue {
        use crate::editor::host_snapshot::resolve_active_editor_diagnostics;
        to_value(&resolve_active_editor_diagnostics()).unwrap_or(JsValue::NULL)
    }

    /// Save the editor session to disk.
    #[wasm_bindgen(js_name = "saveSession")]
    pub async fn save_session(&self, path: String, page_index: u16) -> JsValue {
        use crate::editor::activation::save_editor_session;
        to_value(&save_editor_session(path, page_index).await).unwrap_or(JsValue::NULL)
    }

    // ── P1: Real implementations ────────────────────────────────

    /// Insert text at the current caret position.
    #[wasm_bindgen(js_name = "insertText")]
    pub fn insert_text(&self, text: &str) -> JsValue {
        guard_state!(SessionState::EditingBlock, "insert_text");

        use crate::editor::command::EditorInputCommand;
        use crate::editor::host_snapshot::resolve_editor_host_snapshot;
        use crate::editor::orchestrator::render_transaction::apply_input_tx;

        let frame_request = build_frame_request();
        let result = apply_input_tx(EditorInputCommand::InsertText(text), frame_request);
        let snapshot = resolve_editor_host_snapshot(1.0);

        ok_response(
            ApplyCommandResult {
                changed: result.text_changed || result.scene_changed,
                caret_index: result.caret_index as u32,
                draft_text: snapshot.draft_text,
            },
            result.render_frame.is_some(),
        )
    }

    /// Delete text in the given direction ("forward" or "backward").
    #[wasm_bindgen(js_name = "deleteText")]
    pub fn delete_text(&self, direction: &str) -> JsValue {
        guard_state!(SessionState::EditingBlock, "delete_text");

        use crate::editor::command::EditorInputCommand;
        use crate::editor::host_snapshot::resolve_editor_host_snapshot;
        use crate::editor::orchestrator::render_transaction::apply_input_tx;

        let command = match direction {
            "forward" => EditorInputCommand::DeleteForward,
            "backward" => EditorInputCommand::DeleteBackward,
            other => {
                return err_response(EditorError::Internal {
                    message: format!("unknown delete direction: {other}"),
                });
            }
        };

        let frame_request = build_frame_request();
        let result = apply_input_tx(command, frame_request);
        let snapshot = resolve_editor_host_snapshot(1.0);

        ok_response(
            ApplyCommandResult {
                changed: result.text_changed || result.scene_changed,
                caret_index: result.caret_index as u32,
                draft_text: snapshot.draft_text,
            },
            result.render_frame.is_some(),
        )
    }

    /// Apply a format action to the active block.
    #[wasm_bindgen(js_name = "applyFormat")]
    pub fn apply_format(&self, action_js: JsValue) -> JsValue {
        guard_state!(SessionState::EditingBlock, "apply_format");

        use crate::editor::editor_controller::EditorFormatAction;
        use crate::editor::orchestrator::render_transaction::apply_format_action_tx;

        let action: EditorFormatAction = match serde_wasm_bindgen::from_value(action_js) {
            Ok(a) => a,
            Err(e) => {
                return err_response(EditorError::Internal {
                    message: format!("failed to parse format action: {e}"),
                });
            }
        };

        let frame_request = build_frame_request();
        let result = apply_format_action_tx(action, frame_request);

        ok_response(
            CommitResult {
                changed: result.changed,
            },
            result.render_frame.is_some(),
        )
    }

    /// Undo the last text edit in the active block.
    #[wasm_bindgen(js_name = "undo")]
    pub fn undo(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "undo");

        use crate::editor::orchestrator::render_transaction::undo_active_editor_tx;
        use crate::editor::host_snapshot::resolve_editor_host_snapshot;

        let frame_request = build_frame_request();
        let result = undo_active_editor_tx(frame_request);
        let snapshot = resolve_editor_host_snapshot(1.0);

        ok_response(
            ApplyCommandResult {
                changed: result.text_changed || result.caret_changed || result.scene_changed,
                caret_index: result.caret_index as u32,
                draft_text: snapshot.draft_text,
            },
            result.render_frame.is_some(),
        )
    }

    /// Redo the last undone text edit in the active block.
    #[wasm_bindgen(js_name = "redo")]
    pub fn redo(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "redo");

        use crate::editor::orchestrator::render_transaction::redo_active_editor_tx;
        use crate::editor::host_snapshot::resolve_editor_host_snapshot;

        let frame_request = build_frame_request();
        let result = redo_active_editor_tx(frame_request);
        let snapshot = resolve_editor_host_snapshot(1.0);

        ok_response(
            ApplyCommandResult {
                changed: result.text_changed || result.caret_changed || result.scene_changed,
                caret_index: result.caret_index as u32,
                draft_text: snapshot.draft_text,
            },
            result.render_frame.is_some(),
        )
    }

    /// Check if undo is available.
    #[wasm_bindgen(js_name = "canUndo")]
    pub fn can_undo(&self) -> bool {
        use crate::editor::session::can_undo;
        can_undo()
    }

    /// Check if redo is available.
    #[wasm_bindgen(js_name = "canRedo")]
    pub fn can_redo(&self) -> bool {
        use crate::editor::session::can_redo;
        can_redo()
    }

    /// Get the list of editable text blocks on the given page.
    ///
    /// Currently only the active page's blocks are kept in memory by the
    /// page store, so callers must pass the same `page_index` as the viewer
    /// session's `currentPage`. Mismatched indices return an empty list
    /// (with a warning log) instead of an error — that lets cross-page
    /// queries fail gracefully while we wait for multi-page caching.
    /// See architecture proposal §14.6.
    #[wasm_bindgen(js_name = "readTextBlocks")]
    pub fn read_text_blocks(&self, page_index: u16) -> JsValue {
        guard_state!(
            SessionState::Editing | SessionState::EditingBlock,
            "read_text_blocks"
        );

        let active_page = crate::viewer::viewer_store::read_viewer_session().current_page;
        if page_index != active_page {
            log::warn!(
                "[EditorSession::read_text_blocks] page_index={} but only the active page (={}) is currently cached; returning empty list",
                page_index,
                active_page,
            );
            let empty: Vec<TextBlockInfo> = Vec::new();
            return ok_response(empty, false);
        }

        let blocks = collect_text_blocks();
        ok_response(blocks, false)
    }

    #[wasm_bindgen(js_name = "getTextBlocks")]
    #[deprecated(since = "0.2.0", note = "Use readTextBlocks instead")]
    pub fn get_text_blocks(&self, page_index: u16) -> JsValue {
        self.read_text_blocks(page_index)
    }

    /// Read the format state of the active editor.
    #[wasm_bindgen(js_name = "readFormatState")]
    pub fn read_format_state(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "read_format_state");

        use crate::editor::editor_controller::active_editor_format_state;
        let state = active_editor_format_state();
        to_value(&state).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen(js_name = "getFormatState")]
    #[deprecated(since = "0.2.0", note = "Use readFormatState instead")]
    pub fn get_format_state(&self) -> JsValue {
        self.read_format_state()
    }

    // ── P1: Event callbacks (§14.7) ─────────────────────────────

    /// Register a callback fired on every `SessionState` transition.
    /// The callback receives the new state as a camelCase string
    /// (matches `getSnapshot().state`). Pass `null` to unregister.
    #[wasm_bindgen(js_name = "onStateChange")]
    pub fn on_state_change(&self, callback: JsValue) -> JsValue {
        if callback.is_null() || callback.is_undefined() {
            editor_store::set_state_change_callback(None);
            return ok_empty(false);
        }
        let func: js_sys::Function = match callback.dyn_into() {
            Ok(f) => f,
            Err(_) => {
                return err_response(EditorError::Internal {
                    message: "onStateChange: callback must be a function".into(),
                });
            }
        };
        editor_store::set_state_change_callback(Some(func));
        ok_empty(false)
    }

    /// Register a callback fired on any session mutation (state or active block).
    /// Arity-0 callback; observers should re-read state via `getSnapshot()`.
    /// Pass `null` to unregister.
    #[wasm_bindgen(js_name = "onChange")]
    pub fn on_change(&self, callback: JsValue) -> JsValue {
        if callback.is_null() || callback.is_undefined() {
            editor_store::set_change_callback(None);
            return ok_empty(false);
        }
        let func: js_sys::Function = match callback.dyn_into() {
            Ok(f) => f,
            Err(_) => {
                return err_response(EditorError::Internal {
                    message: "onChange: callback must be a function".into(),
                });
            }
        };
        editor_store::set_change_callback(Some(func));
        ok_empty(false)
    }

    // ── Stubs for future features ────────────────────────────────

    #[wasm_bindgen(js_name = "setCaret")]
    pub fn set_caret(&self, char_index: u32) -> JsValue {
        guard_state!(SessionState::EditingBlock, "setCaret");
        use crate::editor::session::set_active_editor_caret_index;
        let changed = set_active_editor_caret_index(char_index as usize);
        ok_empty(changed)
    }

    #[wasm_bindgen(js_name = "setSelection")]
    pub fn set_selection(&self, start: u32, end: u32) -> JsValue {
        guard_state!(SessionState::EditingBlock, "setSelection");
        use crate::editor::session::set_active_editor_selection;
        let changed = set_active_editor_selection(start as usize, end as usize);
        ok_empty(changed)
    }

    #[wasm_bindgen(js_name = "selectAll")]
    pub fn select_all(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "selectAll");
        use crate::editor::session::{active_editor_state, set_active_editor_selection};
        let len = active_editor_state()
            .map(|state| state.text_char_count())
            .unwrap_or(0);
        let changed = set_active_editor_selection(0, len);
        ok_empty(changed)
    }

    #[wasm_bindgen(js_name = "getSelection")]
    pub fn read_selection(&self) -> JsValue {
        guard_state!(SessionState::EditingBlock, "getSelection");
        use crate::editor::session::active_editor_selection;
        match active_editor_selection() {
            Some((start, end, text)) => ok_response(
                TextSelection {
                    start: start as u32,
                    end: end as u32,
                    text,
                },
                false,
            ),
            None => ok_empty(false),
        }
    }

    #[wasm_bindgen(js_name = "cut")]
    pub fn cut(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "cut".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "copy")]
    pub fn copy(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "copy".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "paste")]
    pub fn paste(&self, _text: &str) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "paste".to_string(),
        })
    }


    #[wasm_bindgen(js_name = "getTextContent")]
    pub fn read_text_content(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "getTextContent".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "getTextLines")]
    pub fn read_text_lines(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "getTextLines".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "getCharRects")]
    pub fn read_char_rects(&self, _start: u32, _end: u32) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "getCharRects".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "clientToPage")]
    pub fn client_to_page(
        &self,
        _client_x: f32,
        _client_y: f32,
        _reference_left: f32,
        _reference_top: f32,
        _reference_width: f32,
        _reference_height: f32,
        _page_width: f32,
        _page_height: f32,
    ) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "clientToPage".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "pageToClient")]
    pub fn page_to_client(
        &self,
        _page_x: f32,
        _page_y: f32,
        _reference_left: f32,
        _reference_top: f32,
        _reference_width: f32,
        _reference_height: f32,
        _page_width: f32,
        _page_height: f32,
    ) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "pageToClient".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "addTextBlock")]
    pub fn add_text_block(&self, _x: f32, _y: f32, _max_width: f32, _text: &str) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "addTextBlock".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "deleteTextBlock")]
    pub fn delete_text_block(&self, _block_id: &str) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "deleteTextBlock".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "resizeTextBlock")]
    pub fn resize_text_block(&self, _block_id: &str, _max_width: f32) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "resizeTextBlock".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "moveTextBlock")]
    pub fn move_text_block(&self, _block_id: &str, _x: f32, _y: f32) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "moveTextBlock".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "exportPatch")]
    pub fn export_patch(&self) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "exportPatch".to_string(),
        })
    }

    #[wasm_bindgen(js_name = "importPatch")]
    pub fn import_patch(&self, _patch_js: JsValue) -> JsValue {
        err_response(EditorError::NotImplemented {
            method: "importPatch".to_string(),
        })
    }
}

// ── Internal helpers (not exported to JS) ───────────────────────

impl EditorSession {
    fn commit_draft_internal(&self) {
        use crate::editor::orchestrator::commit::commit_pending_edit_if_any;
        commit_pending_edit_if_any();
    }
}

fn build_frame_request() -> crate::present::plan_builder::FramePlanRequest {
    let zoom_state = crate::zoom::zoom_store::read_zoom_state();
    let viewer_session = crate::viewer::viewer_store::read_viewer_session();
    crate::present::plan_builder::FramePlanRequest {
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

fn resolve_target_at_page_point(
    page_x: f32,
    page_y: f32,
) -> Option<pdf_viewer_core::edit::bridge::ParagraphInteractionTarget> {
    use pdf_viewer_core::edit::bridge::collect_paragraph_interaction_targets;
    use crate::page::page_store::with_page_state;

    let targets = with_page_state(|state| {
        state
            .paint_plan
            .as_ref()
            .map(|plan| collect_paragraph_interaction_targets(plan, state.vector_model.as_ref()))
            .unwrap_or_default()
    });

    if targets.is_empty() {
        return None;
    }

    // Direct hit only (with 4px tolerance).
    // No nearest-neighbor fallback — clicking blank area must NOT open a distant paragraph.
    targets
        .iter()
        .find(|t| {
            page_x >= t.bbox.left - 4.0
                && page_x <= t.bbox.right + 4.0
                && page_y >= t.bbox.top - 4.0
                && page_y <= t.bbox.bottom + 4.0
        })
        .cloned()
}

fn collect_text_blocks() -> Vec<TextBlockInfo> {
    use pdf_viewer_core::edit::bridge::collect_paragraph_interaction_targets;
    use crate::page::page_store::with_page_state;

    with_page_state(|state| {
        state
            .paint_plan
            .as_ref()
            .map(|plan| {
                collect_paragraph_interaction_targets(plan, state.vector_model.as_ref())
                    .into_iter()
                    .map(|t| TextBlockInfo {
                        id: t.paragraph_id,
                        bbox_left: t.bbox.left,
                        bbox_top: t.bbox.top,
                        bbox_right: t.bbox.right,
                        bbox_bottom: t.bbox.bottom,
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}
