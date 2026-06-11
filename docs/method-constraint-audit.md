# 方法命名约束审查

> 由 `node scripts/generate-method-inventory.mjs` 与 `docs/method-inventory.md` 一起生成。

## 范围和规则来源

- `docs/architecture-principles.md`：单一渲染链、单一 owner、TS 作为宿主适配层、命名禁忌。
- `docs/architecture-overview.md`：Rust core / Rust WASM / Tauri / TS 分层边界、命令命名。
- `docs/development-guide.md`：Rust `fn` 使用 snake_case，Tauri command 使用 snake_case，WASM `js_name` 使用 camelCase，TS facade 命名。
- 本生成文档遵循的 Codex 工作约束：优先项目现有模式，生成物放在 `docs/`，避免无关重写。

## 摘要

- 提取方法/函数总数：2894
- 提取类型/类总数：730
- Tauri commands：30
- 显式 WASM js_name 导出：144
- WASM 推断 JS 名导出：68
- raw invoke/targetInvoke 命令字符串：23
- Rust 命名违规：0
- Tauri command 命名违规：0
- WASM js_name 命名违规：0
- WASM 推断名违规：57
- TS/JS 命名异常：0
- 长/句子式方法名：0
- 生产代码长/句子式方法名：0
- 测试代码长/句子式方法名：0
- 类型/类名违规：0
- 长/句子式类型/类名：0
- 历史标签/版本标签命中：4
- helper/manager/utils 命中：11

## 命令约束检查

### Tauri Commands

- 通过：提取到的 Tauri command 函数名全部是 snake_case。

| 文件 | 行 | Command |
|---|---:|---|
| `src-tauri/src/interfaces/pdf/annotation.rs` | 10 | `read_annotation_targets` |
| `src-tauri/src/interfaces/pdf/annotation.rs` | 22 | `read_highlights` |
| `src-tauri/src/interfaces/pdf/annotation.rs` | 31 | `apply_highlight` |
| `src-tauri/src/interfaces/pdf/annotation.rs` | 40 | `delete_annotation` |
| `src-tauri/src/interfaces/pdf/comment.rs` | 11 | `read_comments` |
| `src-tauri/src/interfaces/pdf/comment.rs` | 20 | `read_comment_review` |
| `src-tauri/src/interfaces/pdf/comment.rs` | 29 | `apply_comment` |
| `src-tauri/src/interfaces/pdf/comment.rs` | 38 | `apply_comment_update` |
| `src-tauri/src/interfaces/pdf/document.rs` | 8 | `open_pdf` |
| `src-tauri/src/interfaces/pdf/document.rs` | 24 | `clear_cache` |
| `src-tauri/src/interfaces/pdf/document.rs` | 44 | `save_pdf` |
| `src-tauri/src/interfaces/pdf/document.rs` | 66 | `undo` |
| `src-tauri/src/interfaces/pdf/document.rs` | 78 | `redo` |
| `src-tauri/src/interfaces/pdf/page.rs` | 15 | `read_preview` |
| `src-tauri/src/interfaces/pdf/render.rs` | 18 | `read_page_asset_bundle` |
| `src-tauri/src/interfaces/pdf/render.rs` | 73 | `read_vector` |
| `src-tauri/src/interfaces/pdf/render.rs` | 110 | `read_glyph_plan` |
| `src-tauri/src/interfaces/pdf/render.rs` | 144 | `read_images` |
| `src-tauri/src/interfaces/pdf/render.rs` | 149 | `diagnose_page` |
| `src-tauri/src/interfaces/pdf/replace.rs` | 9 | `apply_region_patches` |
| `src-tauri/src/interfaces/pdf/search.rs` | 10 | `find_in_page` |
| `src-tauri/src/interfaces/pdf/search.rs` | 35 | `find_in_document` |
| `src-tauri/src/interfaces/pdf/system.rs` | 7 | `create_demo_pdf` |
| `src-tauri/src/interfaces/pdf/system.rs` | 12 | `set_log_level` |
| `src-tauri/src/interfaces/pdf/system.rs` | 17 | `clear_pdf_event_log` |
| `src-tauri/src/interfaces/pdf/system.rs` | 22 | `read_pdf_event_log` |
| `src-tauri/src/interfaces/pdf/system.rs` | 27 | `set_page_asset_test_delay_ms` |
| `src-tauri/src/interfaces/pdf/system.rs` | 32 | `terminal_log` |
| `src-tauri/src/interfaces/pdf/system.rs` | 37 | `resolve_asset_url` |
| `src-tauri/src/interfaces/pdf/system.rs` | 44 | `pick_file` |

### Raw Invoke 命令字符串

- 通过：提取到的 raw invoke 命令字符串全部符合小写 snake/kebab 兼容模式。

| 文件 | 行 | Command | 状态 |
|---|---:|---|---|
| `crates/pdf-viewer-ui/src/document/comment.rs` | 42 | `read_comments` | ok |
| `crates/pdf-viewer-ui/src/document/comment.rs` | 49 | `read_annotation_targets` | ok |
| `crates/pdf-viewer-ui/src/document/comment.rs` | 56 | `read_comment_review` | ok |
| `crates/pdf-viewer-ui/src/document/comment.rs` | 298 | `apply_comment` | ok |
| `crates/pdf-viewer-ui/src/document/comment.rs` | 305 | `delete_annotation` | ok |
| `crates/pdf-viewer-ui/src/document/comment.rs` | 312 | `apply_comment_update` | ok |
| `crates/pdf-viewer-ui/src/document/io.rs` | 21 | `open_pdf` | ok |
| `crates/pdf-viewer-ui/src/document/io.rs` | 35 | `pick_file` | ok |
| `crates/pdf-viewer-ui/src/document/io.rs` | 50 | `save_pdf` | ok |
| `crates/pdf-viewer-ui/src/document/patch_persistence.rs` | 87 | `apply_region_patches` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 145 | `delete_annotation` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 166 | `apply_highlight` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 229 | `read_highlights` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 236 | `read_annotation_targets` | ok |
| `src/bridge/find/find_facade.ts` | 91 | `find_in_page` | ok |
| `src/bridge/find/find_facade.ts` | 104 | `find_in_document` | ok |
| `src/bridge/render/render_flow.ts` | 96 | `read_preview` | ok |
| `src/bridge/render/render_flow.ts` | 339 | `read_preview` | ok |
| `src/bridge/render/vector_page_bundle.ts` | 262 | `read_page_asset_bundle` | ok |
| `src/bridge/shared/diagnostics.ts` | 182 | `terminal_log` | ok |
| `src/bridge/viewer/pdf_runtime.ts` | 407 | `read_preview` | ok |
| `src/bridge/viewer/pdf_viewer_api.ts` | 302 | `undo` | ok |
| `src/bridge/viewer/pdf_viewer_api.ts` | 315 | `redo` | ok |

### WASM js_name 导出

- 通过：显式 WASM `js_name` 导出全部符合 camelCase/PascalCase。

| 文件 | 行 | Rust fn | js_name | 状态 |
|---|---:|---|---|---|
| `crates/pdf-viewer-ui/src/annotation/annotation_api.rs` | 80 | `list` | `list` | ok |
| `crates/pdf-viewer-ui/src/annotation/annotation_api.rs` | 97 | `delete` | `delete` | ok |
| `crates/pdf-viewer-ui/src/app_controller.rs` | 8 | `target_invoke` | `targetInvoke` | ok |
| `crates/pdf-viewer-ui/src/app_controller.rs` | 11 | `on_debug` | `onDebug` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 110 | `open` | `open` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 141 | `close` | `close` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 160 | `reset_all` | `resetAll` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 173 | `read_state` | `readState` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 179 | `get_state` | `getState` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 194 | `add_event_listener` | `addEventListener` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 202 | `remove_event_listener` | `removeEventListener` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 208 | `remove_all_event_listeners` | `removeAllEventListeners` | ok |
| `crates/pdf-viewer-ui/src/bridge.rs` | 6 | `on_debug` | `onDebug` | ok |
| `crates/pdf-viewer-ui/src/bridge.rs` | 8 | `on_input` | `onInput` | ok |
| `crates/pdf-viewer-ui/src/bridge.rs` | 10 | `on_open` | `onOpen` | ok |
| `crates/pdf-viewer-ui/src/bridge.rs` | 12 | `on_commit` | `onCommit` | ok |
| `crates/pdf-viewer-ui/src/bridge.rs` | 14 | `on_cancel` | `onCancel` | ok |
| `crates/pdf-viewer-ui/src/bridge.rs` | 16 | `target_invoke` | `invoke` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 43 | `clear_review_session` | `clearReviewSession` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 48 | `read_review_session` | `readReviewSession` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 55 | `load_review` | `loadReview` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 61 | `load_overlay` | `loadOverlay` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 67 | `load_target_overlay` | `loadTargetOverlay` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 77 | `set_panel_open_and_load` | `setPanelOpenAndLoad` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 88 | `toggle_panel_and_load` | `togglePanelAndLoad` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 98 | `set_scope_and_load` | `setScopeAndLoad` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 110 | `set_query_and_load` | `setQueryAndLoad` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 121 | `select_and_load` | `selectAndLoad` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 135 | `add_region_comment` | `addRegionComment` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 146 | `delete_annotation` | `deleteAnnotation` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 157 | `update_comment` | `updateComment` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 40 | `open` | `open` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 48 | `close` | `close` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 60 | `undo` | `undo` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 66 | `redo` | `redo` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 74 | `rotate` | `rotate` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 85 | `has_unsaved_changes` | `hasUnsavedChanges` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 95 | `patch_count` | `patchCount` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 102 | `can_undo` | `canUndo` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 108 | `can_redo` | `canRedo` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 119 | `request_refresh` | `requestRefresh` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 80 | `begin` | `begin` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 97 | `hit_test` | `hitTest` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 149 | `open_block` | `openBlock` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 231 | `move_caret` | `moveCaret` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 276 | `close_block` | `closeBlock` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 300 | `commit` | `commit` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 352 | `end` | `end` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 368 | `discard` | `discard` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 399 | `read_snapshot` | `readSnapshot` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 419 | `get_snapshot` | `getSnapshot` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 425 | `is_active` | `isActive` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 434 | `has_unsaved_changes` | `hasUnsavedChanges` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 442 | `sync_input` | `syncInput` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 477 | `apply_command` | `applyCommand` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 532 | `set_edit_mode` | `setEditMode` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 558 | `read_legacy_snapshot` | `readLegacySnapshot` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 567 | `paint_canvas` | `paintCanvas` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 580 | `utf16_to_char_index` | `utf16ToCharIndex` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 587 | `char_to_utf16_offset` | `charToUtf16Offset` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 594 | `has_session_changes` | `hasSessionChanges` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 601 | `open_region` | `openRegion` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 681 | `set_display_zoom` | `setDisplayZoom` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 688 | `read_diagnostics` | `readDiagnostics` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 695 | `save_session` | `saveSession` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 704 | `insert_text` | `insertText` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 727 | `delete_text` | `deleteText` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 760 | `apply_format` | `applyFormat` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 795 | `read_text_blocks` | `readTextBlocks` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 818 | `get_text_blocks` | `getTextBlocks` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 824 | `read_format_state` | `readFormatState` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 834 | `get_format_state` | `getFormatState` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 844 | `on_state_change` | `onStateChange` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 865 | `on_change` | `onChange` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 90 | `facade_search_page` | `searchFacadePage` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 108 | `facade_search_document` | `searchFacadeDocument` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 125 | `facade_replace` | `searchFacadeReplace` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 150 | `facade_batch_replace` | `searchFacadeBatchReplace` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 167 | `facade_set_session` | `searchFacadeSetSession` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 194 | `facade_clear_session` | `searchFacadeClearSession` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 205 | `facade_move_match` | `searchFacadeMoveMatch` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 219 | `facade_get_session` | `searchFacadeGetSession` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 12 | `facade_open` | `findControllerOpen` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 17 | `facade_close` | `findControllerClose` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 22 | `facade_toggle` | `findControllerToggle` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 27 | `facade_set_result` | `findControllerSetResult` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 37 | `facade_clear` | `findControllerClear` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 42 | `facade_move_active` | `findControllerMoveActive` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 47 | `facade_set_current_page` | `findControllerSetCurrentPage` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 52 | `facade_get_toolbar_state` | `findControllerGetToolbarState` | ok |
| `crates/pdf-viewer-ui/src/find/controller_facade.rs` | 57 | `facade_get_replace_requests` | `findControllerGetReplaceRequests` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 32 | `open` | `open` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 38 | `close` | `close` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 44 | `toggle` | `toggle` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 50 | `clear` | `clear` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 56 | `set_current_page` | `setCurrentPage` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 66 | `read_state` | `readState` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 72 | `get_state` | `getState` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 75 | `client_to_page` | `clientToPage` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 94 | `page_to_client` | `pageToClient` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 110 | `page_to_raw` | `pageToRaw` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 121 | `raw_to_page` | `rawToPage` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 136 | `client_to_raw` | `clientToRaw` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 157 | `measure_scale` | `measureScale` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 171 | `project_rect` | `projectRect` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 21 | `request_page_turn` | `requestPageTurn` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 26 | `read_page_turn` | `readPageTurn` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 31 | `is_latest_page_turn` | `isLatestPageTurn` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 36 | `mark_page_visible` | `markPageVisible` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 41 | `can_prefetch` | `canPrefetch` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 46 | `admit_page_asset` | `admitPageAsset` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 51 | `decide_adjacent_prefetch` | `decideAdjacentPrefetch` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 56 | `resolve_render_queue_action` | `resolveRenderQueueAction` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 73 | `reset` | `reset` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 43 | `facade_start_progressive` | `renderFacadeStartProgressive` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 48 | `facade_step_progressive` | `renderFacadeStepProgressive` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 64 | `facade_cancel_progressive` | `renderFacadeCancelProgressive` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 69 | `facade_render_page` | `renderFacadeRenderPage` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 76 | `facade_commit_result` | `renderFacadeCommitResult` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 92 | `facade_abort_frame` | `renderFacadeAbortFrame` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 98 | `facade_is_frame_current` | `renderFacadeIsFrameCurrent` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 105 | `facade_touch_cache` | `renderFacadeTouchCache` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 110 | `facade_store_cache` | `renderFacadeStoreCache` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 115 | `facade_reset_cache` | `renderFacadeResetCache` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 123 | `facade_snapshot_png` | `renderFacadeSnapshotPng` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 129 | `facade_prewarm_cache` | `renderFacadePrewarmCache` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 135 | `facade_set_quality` | `renderFacadeSetQuality` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 141 | `facade_set_debug_overlay` | `renderFacadeSetDebugOverlay` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 71 | `read_feed` | `readFeed` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 77 | `accept` | `accept` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 83 | `reject` | `reject` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 89 | `accept_all` | `acceptAll` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 95 | `reject_all` | `rejectAll` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 104 | `read_state` | `readState` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 110 | `get_state` | `getState` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 32 | `read` | `read` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 38 | `set_document` | `setDocument` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 44 | `reset` | `reset` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 50 | `set_current_page` | `setCurrentPage` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 56 | `set_current_zoom` | `setCurrentZoom` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 62 | `set_page_dimensions` | `setPageDimensions` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 71 | `read_state` | `readState` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 77 | `get_state` | `getState` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 97 | `set_state` | `setState` | ok |

### WASM 推断名导出

- 警告：裸 `#[wasm_bindgen]` 函数会把 Rust 名推断到 JS；snake_case 项应加显式 camelCase `js_name`，或作为 legacy 导出退役。

| 文件 | 行 | Rust fn | 推断 JS 名 | 状态 |
|---|---:|---|---|---|
| `crates/pdf-viewer-ui/src/annotation/annotation_api.rs` | 70 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 95 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 36 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 32 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 16 | `undo_document_pipeline` | `undo_document_pipeline` | check |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 21 | `redo_document_pipeline` | `redo_document_pipeline` | check |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 26 | `open_document_pipeline` | `open_document_pipeline` | check |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 33 | `pick_document_pipeline` | `pick_document_pipeline` | check |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 39 | `rotate_document_pipeline` | `rotate_document_pipeline` | check |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 45 | `close_document_pipeline` | `close_document_pipeline` | check |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 54 | `read_viewer_session` | `read_viewer_session` | check |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 60 | `get_viewer_session` | `get_viewer_session` | check |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | 65 | `set_viewer_document` | `set_viewer_document` | check |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 71 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 24 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 64 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/lib.rs` | 37 | `start` | `start` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 16 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/render/canvas.rs` | 46 | `render_run_standalone` | `render_run_standalone` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 64 | `resolve_frame_plan` | `resolve_frame_plan` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 70 | `take_frame_plan` | `take_frame_plan` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 78 | `schedule_render_frame` | `schedule_render_frame` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 87 | `commit_render_result` | `commit_render_result` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 103 | `settle_render_frame` | `settle_render_frame` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 108 | `abort_render_frame` | `abort_render_frame` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 113 | `is_render_frame_current` | `is_render_frame_current` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 118 | `schedule_render_follow_up` | `schedule_render_follow_up` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 129 | `queue_render_loop_frame` | `queue_render_loop_frame` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 138 | `advance_render_loop_frame` | `advance_render_loop_frame` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 149 | `step_zoom_frame_plan` | `step_zoom_frame_plan` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 155 | `resolve_viewport_refresh` | `resolve_viewport_refresh` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 161 | `resolve_host_scroll_refresh` | `resolve_host_scroll_refresh` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 167 | `clear_zoom_preview_host_state` | `clear_zoom_preview_host_state` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 172 | `resolve_wheel_render_decision` | `resolve_wheel_render_decision` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 178 | `resolve_preview_tick_decision` | `resolve_preview_tick_decision` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 184 | `handle_wheel_zoom_host` | `handle_wheel_zoom_host` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 190 | `step_preview_host` | `step_preview_host` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 198 | `resolve_render_execution_plan` | `resolve_render_execution_plan` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 208 | `resolve_layer_execution_plan` | `resolve_layer_execution_plan` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 218 | `resolve_layer_present_decision` | `resolve_layer_present_decision` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 230 | `update_page_viewport` | `update_page_viewport` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 251 | `render_page` | `render_page` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 256 | `render_page_offscreen` | `render_page_offscreen` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 261 | `start_progressive_render` | `start_progressive_render` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 266 | `step_progressive_render` | `step_progressive_render` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 282 | `step_progressive_render_offscreen` | `step_progressive_render_offscreen` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 300 | `cancel_progressive_render` | `cancel_progressive_render` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 305 | `resolve_progressive_render_policy` | `resolve_progressive_render_policy` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 313 | `touch_frame_cache_entry` | `touch_frame_cache_entry` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 319 | `store_frame_cache_entry` | `store_frame_cache_entry` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 324 | `reset_frame_cache` | `reset_frame_cache` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 331 | `set_wheel_render_pending` | `set_wheel_render_pending` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 336 | `get_wheel_render_pending` | `get_wheel_render_pending` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 341 | `queue_committed_frame` | `queue_committed_frame` | check |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | 348 | `take_ready_committed_frame` | `take_ready_committed_frame` | check |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 65 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/viewer/free_api.rs` | 13 | `init_page_context` | `init_page_context` | check |
| `crates/pdf-viewer-ui/src/viewer/free_api.rs` | 70 | `set_current_page` | `set_current_page` | check |
| `crates/pdf-viewer-ui/src/viewer/free_api.rs` | 78 | `dump_editor_debug_trace` | `dump_editor_debug_trace` | check |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 26 | `new` | `new` | ok |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | 14 | `resolve_wheel_zoom` | `resolve_wheel_zoom` | check |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | 21 | `reset_zoom_state` | `reset_zoom_state` | check |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | 26 | `read_zoom_state` | `read_zoom_state` | check |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | 32 | `get_zoom_state` | `get_zoom_state` | check |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | 37 | `set_target_zoom` | `set_target_zoom` | check |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | 42 | `mark_rendered_zoom` | `mark_rendered_zoom` | check |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | 47 | `clear_pending_anchor` | `clear_pending_anchor` | check |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | 52 | `apply_zoom_selection` | `apply_zoom_selection` | check |

## 类型和类名

- 通过：提取到的 Rust/TS 类型和类名全部使用 PascalCase。

| 文件 | 行 | 类型 | 长度 | 分段数 | 名称 | 状态 |
|---|---:|---|---:|---:|---|---|
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 23 | rust_enum | 14 | 2 | `AnnotationKind` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 53 | rust_struct | 10 | 1 | `Annotation` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 79 | rust_struct | 14 | 3 | `AnnotationBBox` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 90 | rust_enum | 15 | 2 | `AnnotationError` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 107 | rust_struct | 18 | 2 | `AnnotationResponse` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 117 | rust_struct | 14 | 3 | `CommentBoxRect` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 124 | rust_type | 20 | 4 | `PdfPageAnnotationBox` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 128 | rust_struct | 19 | 3 | `CommentPercentFrame` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 137 | rust_struct | 18 | 4 | `PdfPageCommentItem` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 149 | rust_struct | 18 | 4 | `PdfPageCommentList` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 158 | rust_struct | 23 | 4 | `PdfPageAnnotationTarget` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 170 | rust_struct | 29 | 5 | `PdfPageAnnotationTargetResult` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 179 | rust_struct | 29 | 5 | `PdfCommentTargetOverlayMarker` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 190 | rust_struct | 30 | 5 | `PdfCommentTargetOverlayDisplay` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 196 | rust_struct | 27 | 5 | `PdfCommentReviewPageSummary` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 204 | rust_struct | 23 | 4 | `PdfCommentReviewRequest` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 212 | rust_struct | 22 | 4 | `PdfCommentReviewResult` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 222 | rust_struct | 27 | 5 | `PdfCommentReviewSummaryChip` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 229 | rust_struct | 26 | 5 | `PdfCommentReviewCardAction` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 237 | rust_struct | 20 | 4 | `PdfCommentReviewCard` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 250 | rust_struct | 21 | 4 | `PdfCommentReviewPanel` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 259 | rust_struct | 23 | 4 | `PdfCommentOverlayMarker` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 268 | rust_struct | 24 | 4 | `PdfCommentOverlayDisplay` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 274 | rust_struct | 23 | 4 | `PdfCommentReviewDisplay` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 283 | rust_struct | 23 | 4 | `PdfRegionCommentRequest` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 293 | rust_struct | 22 | 4 | `PdfRegionCommentResult` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 301 | rust_struct | 26 | 4 | `PdfDeleteAnnotationRequest` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 308 | rust_struct | 25 | 4 | `PdfDeleteAnnotationResult` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 316 | rust_struct | 23 | 4 | `PdfUpdateCommentRequest` | ok |
| `crates/pdf-viewer-core/src/annotation/annotation_types.rs` | 324 | rust_struct | 22 | 4 | `PdfUpdateCommentResult` | ok |
| `crates/pdf-viewer-core/src/document/document_types.rs` | 15 | rust_enum | 13 | 2 | `DocumentError` | ok |
| `crates/pdf-viewer-core/src/document/document_types.rs` | 30 | rust_struct | 16 | 2 | `DocumentResponse` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 6 | rust_struct | 27 | 4 | `ParagraphRegionSnapshotLine` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 40 | rust_struct | 23 | 3 | `ParagraphRegionSnapshot` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 52 | rust_struct | 18 | 3 | `FieldGroupSnapshot` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 67 | rust_struct | 17 | 3 | `BoundingBoxOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 76 | rust_struct | 25 | 3 | `ParagraphProjectionOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 86 | rust_struct | 29 | 4 | `ParagraphLineProjectionOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 97 | rust_struct | 26 | 4 | `FieldGroupProjectionOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 107 | rust_struct | 11 | 2 | `StyleSource` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 129 | rust_struct | 16 | 3 | `StyleRunSnapshot` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 146 | rust_struct | 19 | 3 | `ParagraphLineOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 175 | rust_struct | 21 | 3 | `ParagraphRegionOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 200 | rust_struct | 20 | 4 | `ListItemRegionOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 239 | rust_struct | 6 | 2 | `KeyBox` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 248 | rust_struct | 18 | 4 | `KeyValuePairOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 268 | rust_struct | 25 | 5 | `FieldRowRegionGroupOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 295 | rust_struct | 20 | 4 | `FieldRowRegionOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 311 | rust_struct | 20 | 3 | `LineProjectionOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 321 | rust_struct | 21 | 4 | `LineRegionModelOutput` | ok |
| `crates/pdf-viewer-core/src/document/page_region_models.rs` | 335 | rust_struct | 23 | 4 | `PageRegionContextOutput` | ok |
| `crates/pdf-viewer-core/src/edit/active_target.rs` | 10 | rust_struct | 18 | 3 | `ActiveEditorTarget` | ok |
| `crates/pdf-viewer-core/src/edit/bridge.rs` | 15 | rust_struct | 26 | 3 | `ParagraphInteractionTarget` | ok |
| `crates/pdf-viewer-core/src/edit/debug_trace.rs` | 11 | rust_struct | 16 | 3 | `EditorDebugField` | ok |
| `crates/pdf-viewer-core/src/edit/debug_trace.rs` | 18 | rust_struct | 21 | 4 | `EditorDebugTraceEvent` | ok |
| `crates/pdf-viewer-core/src/edit/debug_trace.rs` | 26 | rust_struct | 21 | 4 | `EditorDebugTraceState` | ok |
| `crates/pdf-viewer-core/src/edit/document_edit_ops.rs` | 8 | rust_struct | 18 | 3 | `EditorTextMutation` | ok |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 25 | rust_struct | 21 | 3 | `ParagraphEditorMarker` | ok |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 35 | rust_struct | 18 | 3 | `EditorDocumentPlan` | ok |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 79 | rust_struct | 22 | 4 | `EditorDocumentLinePlan` | ok |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 137 | rust_struct | 12 | 2 | `SessionSplit` | ok |
| `crates/pdf-viewer-core/src/edit/document_runtime.rs` | 7 | rust_struct | 27 | 4 | `EditorResolvedDocumentState` | ok |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 15 | rust_struct | 14 | 3 | `DraftCaretStop` | ok |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 22 | rust_struct | 14 | 3 | `DraftCaretLine` | ok |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 30 | rust_struct | 21 | 4 | `EditorDraftRenderPlan` | ok |
| `crates/pdf-viewer-core/src/edit/edit_target.rs` | 12 | rust_struct | 16 | 3 | `EditorEditTarget` | ok |
| `crates/pdf-viewer-core/src/edit/edit_target.rs` | 117 | rust_struct | 13 | 2 | `VisualSegment` | ok |
| `crates/pdf-viewer-core/src/edit/edit_target.rs` | 122 | rust_type | 13 | 3 | `IndexedRunRef` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 7 | rust_enum | 12 | 2 | `SessionState` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 33 | rust_enum | 11 | 2 | `EditorError` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 50 | rust_struct | 14 | 2 | `EditorResponse` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 64 | rust_struct | 13 | 3 | `HitTestResult` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 72 | rust_struct | 15 | 3 | `OpenBlockResult` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 80 | rust_struct | 15 | 3 | `MoveCaretResult` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 86 | rust_struct | 12 | 2 | `CommitResult` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 92 | rust_struct | 14 | 2 | `SnapshotResult` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 102 | rust_struct | 13 | 3 | `TextBlockInfo` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 112 | rust_struct | 11 | 2 | `FormatState` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 121 | rust_struct | 15 | 3 | `SyncInputResult` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 128 | rust_struct | 18 | 3 | `ApplyCommandResult` | ok |
| `crates/pdf-viewer-core/src/edit/editor_types.rs` | 136 | rust_struct | 17 | 4 | `SetEditModeResult` | ok |
| `crates/pdf-viewer-core/src/edit/engine_state.rs` | 14 | rust_struct | 24 | 4 | `LiveEditorParagraphState` | ok |
| `crates/pdf-viewer-core/src/edit/paragraph_overlay.rs` | 7 | rust_enum | 27 | 4 | `ParagraphRenderOverlayOwner` | ok |
| `crates/pdf-viewer-core/src/edit/paragraph_overlay.rs` | 13 | rust_struct | 22 | 3 | `ParagraphRenderOverlay` | ok |
| `crates/pdf-viewer-core/src/edit/paragraph_scene.rs` | 15 | rust_struct | 20 | 3 | `ParagraphEditorScene` | ok |
| `crates/pdf-viewer-core/src/edit/replacement_region.rs` | 10 | rust_struct | 26 | 3 | `ParagraphReplacementRegion` | ok |
| `crates/pdf-viewer-core/src/edit/replacement_snapshot.rs` | 11 | rust_struct | 23 | 3 | `EditReplacementSnapshot` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 21 | rust_struct | 13 | 3 | `PageViewPoint` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 31 | rust_struct | 16 | 3 | `EditorLocalPoint` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 38 | rust_struct | 17 | 3 | `HostReferenceRect` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 47 | rust_struct | 11 | 2 | `ClientPoint` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 54 | rust_struct | 8 | 2 | `PageSize` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 61 | rust_struct | 9 | 2 | `PageScale` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 68 | rust_struct | 17 | 3 | `HostPageTransform` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 145 | rust_struct | 22 | 5 | `PdfToPageViewTransform` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 175 | rust_struct | 18 | 3 | `PdfCoordinateSpace` | ok |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 209 | rust_struct | 23 | 3 | `EditorViewportTransform` | ok |
| `crates/pdf-viewer-core/src/geometry/dom_projection.rs` | 6 | rust_struct | 11 | 3 | `DomRectLike` | ok |
| `crates/pdf-viewer-core/src/geometry/dom_projection.rs` | 15 | rust_struct | 12 | 3 | `DomPointLike` | ok |
| `crates/pdf-viewer-core/src/geometry/dom_projection.rs` | 24 | rust_struct | 9 | 2 | `ScalePair` | ok |
| `crates/pdf-viewer-core/src/geometry/layout_engine.rs` | 26 | rust_struct | 10 | 2 | `VisualLine` | ok |
| `crates/pdf-viewer-core/src/geometry/layout_engine.rs` | 38 | rust_struct | 15 | 2 | `ParagraphLayout` | ok |
| `crates/pdf-viewer-core/src/geometry/reflow_engine.rs` | 13 | rust_struct | 10 | 2 | `ReflowUnit` | ok |
| `crates/pdf-viewer-core/src/history/history_types.rs` | 15 | rust_enum | 12 | 2 | `HistoryError` | ok |
| `crates/pdf-viewer-core/src/history/history_types.rs` | 29 | rust_struct | 12 | 2 | `HistoryState` | ok |
| `crates/pdf-viewer-core/src/history/history_types.rs` | 47 | rust_struct | 17 | 3 | `HistoryStepResult` | ok |
| `crates/pdf-viewer-core/src/history/history_types.rs` | 58 | rust_struct | 15 | 2 | `HistoryResponse` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 6 | rust_struct | 9 | 2 | `PageState` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 28 | rust_struct | 14 | 3 | `BaseEditIntent` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 36 | rust_enum | 10 | 2 | `EditIntent` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 49 | rust_enum | 13 | 3 | `LightPageKind` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 59 | rust_struct | 14 | 3 | `LightPageModel` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 69 | rust_enum | 15 | 3 | `PdfDocumentKind` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 79 | rust_enum | 20 | 2 | `ClassificationReason` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 91 | rust_struct | 16 | 3 | `ReadDocumentMeta` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 103 | rust_enum | 16 | 2 | `PaginationAction` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 111 | rust_struct | 17 | 2 | `PaginationCommand` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 120 | rust_struct | 17 | 3 | `DeletePageCommand` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 126 | rust_struct | 17 | 3 | `RotatePageCommand` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 133 | rust_struct | 17 | 3 | `InsertPageCommand` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 139 | rust_struct | 19 | 3 | `AddHighlightCommand` | ok |
| `crates/pdf-viewer-core/src/models/document_runtime.rs` | 147 | rust_struct | 21 | 3 | `UpdateMetadataCommand` | ok |
| `crates/pdf-viewer-core/src/models/font.rs` | 5 | rust_struct | 9 | 2 | `FontHints` | ok |
| `crates/pdf-viewer-core/src/models/font.rs` | 26 | rust_enum | 14 | 3 | `FontSourceKind` | ok |
| `crates/pdf-viewer-core/src/models/font.rs` | 36 | rust_enum | 11 | 2 | `SymbolClass` | ok |
| `crates/pdf-viewer-core/src/models/font.rs` | 45 | rust_struct | 20 | 3 | `ResolvedFontIdentity` | ok |
| `crates/pdf-viewer-core/src/models/font.rs` | 57 | rust_struct | 16 | 3 | `ResolvedFontFace` | ok |
| `crates/pdf-viewer-core/src/models/geometry.rs` | 12 | rust_struct | 11 | 2 | `BoundingBox` | ok |
| `crates/pdf-viewer-core/src/models/glyph.rs` | 15 | rust_struct | 13 | 3 | `GlyphPaintRun` | ok |
| `crates/pdf-viewer-core/src/models/glyph.rs` | 44 | rust_struct | 18 | 3 | `EditorControlStyle` | ok |
| `crates/pdf-viewer-core/src/models/glyph.rs` | 56 | rust_struct | 19 | 3 | `GlyphPaintParagraph` | ok |
| `crates/pdf-viewer-core/src/models/glyph.rs` | 71 | rust_enum | 14 | 2 | `ExternalObject` | ok |
| `crates/pdf-viewer-core/src/models/glyph.rs` | 92 | rust_struct | 16 | 3 | `GlyphPaintRegion` | ok |
| `crates/pdf-viewer-core/src/models/glyph.rs` | 105 | rust_struct | 14 | 3 | `GlyphPaintPlan` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 6 | rust_struct | 7 | 2 | `RectBox` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 15 | rust_struct | 15 | 2 | `FieldProjection` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 25 | rust_struct | 22 | 3 | `FieldProjectionRequest` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 43 | rust_enum | 13 | 3 | `FieldPartKind` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 51 | rust_struct | 15 | 3 | `FieldHitRequest` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 64 | rust_struct | 18 | 3 | `FieldHitResolution` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 73 | rust_struct | 14 | 3 | `FieldHitTarget` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 85 | rust_struct | 20 | 4 | `FieldHitBatchRequest` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 93 | rust_struct | 13 | 3 | `FieldHitMatch` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 100 | rust_struct | 24 | 4 | `FieldEditorParamsRequest` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 110 | rust_struct | 17 | 3 | `FieldEditorParams` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 119 | rust_struct | 21 | 2 | `InteractionProjection` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 129 | rust_struct | 17 | 2 | `InteractionTarget` | ok |
| `crates/pdf-viewer-core/src/models/interaction.rs` | 140 | rust_struct | 21 | 3 | `FieldEditorProjection` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 9 | rust_enum | 9 | 2 | `FieldKind` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 17 | rust_enum | 12 | 2 | `SemanticRole` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 34 | rust_struct | 18 | 3 | `EditableFieldGroup` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 50 | rust_struct | 15 | 2 | `EditableSegment` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 84 | rust_enum | 10 | 2 | `LayoutRole` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 98 | rust_enum | 15 | 2 | `LayoutAlignment` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 108 | rust_enum | 10 | 2 | `LayoutMode` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 121 | rust_struct | 8 | 2 | `RunStyle` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 137 | rust_struct | 9 | 2 | `LayoutRun` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 190 | rust_struct | 14 | 2 | `ParagraphStyle` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 201 | rust_struct | 15 | 2 | `LayoutParagraph` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 229 | rust_struct | 20 | 3 | `ParagraphEditContext` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 236 | rust_struct | 14 | 2 | `SemanticRegion` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 259 | rust_struct | 21 | 3 | `LayoutInferenceResult` | ok |
| `crates/pdf-viewer-core/src/models/layout.rs` | 278 | rust_enum | 9 | 2 | `PaintMode` | ok |
| `crates/pdf-viewer-core/src/models/styled_run.rs` | 27 | rust_struct | 9 | 2 | `StyledRun` | ok |
| `crates/pdf-viewer-core/src/models/styled_run.rs` | 137 | rust_struct | 15 | 3 | `NativeTextModel` | ok |
| `crates/pdf-viewer-core/src/models/styled_run.rs` | 307 | rust_struct | 16 | 3 | `NativePathObject` | ok |
| `crates/pdf-viewer-core/src/models/styled_run.rs` | 311 | rust_struct | 17 | 3 | `NativeImageObject` | ok |
| `crates/pdf-viewer-core/src/models/styled_run.rs` | 315 | rust_enum | 16 | 3 | `NativePageObject` | ok |
| `crates/pdf-viewer-core/src/models/styled_run.rs` | 323 | rust_struct | 15 | 3 | `NativePageModel` | ok |
| `crates/pdf-viewer-core/src/models/vector.rs` | 7 | rust_struct | 17 | 3 | `VectorPathSegment` | ok |
| `crates/pdf-viewer-core/src/models/vector.rs` | 15 | rust_struct | 16 | 3 | `VectorPathObject` | ok |
| `crates/pdf-viewer-core/src/models/vector.rs` | 33 | rust_struct | 17 | 3 | `VectorImageObject` | ok |
| `crates/pdf-viewer-core/src/models/vector.rs` | 45 | rust_struct | 16 | 3 | `VectorTextObject` | ok |
| `crates/pdf-viewer-core/src/models/vector.rs` | 55 | rust_enum | 18 | 3 | `VectorRenderObject` | ok |
| `crates/pdf-viewer-core/src/models/vector.rs` | 68 | rust_struct | 15 | 3 | `VectorPageModel` | ok |
| `crates/pdf-viewer-core/src/persistence/history_store.rs` | 26 | rust_struct | 12 | 2 | `HistoryStore` | ok |
| `crates/pdf-viewer-core/src/persistence/models.rs` | 6 | rust_struct | 22 | 3 | `PersistableRegionPatch` | ok |
| `crates/pdf-viewer-core/src/persistence/models.rs` | 51 | rust_struct | 16 | 3 | `RegionTextReflow` | ok |
| `crates/pdf-viewer-core/src/persistence/models.rs` | 60 | rust_struct | 19 | 3 | `PersistableSavePlan` | ok |
| `crates/pdf-viewer-core/src/persistence/patch_store.rs` | 13 | rust_struct | 16 | 3 | `GlobalPatchState` | ok |
| `crates/pdf-viewer-core/src/persistence/patch_store.rs` | 47 | rust_struct | 12 | 2 | `PatchCommand` | ok |
| `crates/pdf-viewer-core/src/persistence/patch_store.rs` | 55 | rust_struct | 17 | 3 | `ReviewChangeEntry` | ok |
| `crates/pdf-viewer-core/src/persistence/patch_store.rs` | 67 | rust_struct | 22 | 4 | `ReviewBulkChangeResult` | ok |
| `crates/pdf-viewer-core/src/persistence/review_types.rs` | 7 | rust_struct | 16 | 3 | `ReviewFeedResult` | ok |
| `crates/pdf-viewer-core/src/persistence/review_types.rs` | 15 | rust_struct | 24 | 4 | `RejectReviewChangeResult` | ok |
| `crates/pdf-viewer-core/src/persistence/review_types.rs` | 23 | rust_struct | 24 | 4 | `AcceptReviewChangeResult` | ok |
| `crates/pdf-viewer-core/src/render/comment_review_state.rs` | 5 | rust_enum | 22 | 4 | `HostCommentReviewScope` | ok |
| `crates/pdf-viewer-core/src/render/comment_review_state.rs` | 13 | rust_struct | 24 | 4 | `HostCommentReviewSession` | ok |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 30 | rust_enum | 26 | 4 | `EffectiveVectorRenderEntry` | ok |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 39 | rust_struct | 17 | 3 | `GlyphParagraphRef` | ok |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 47 | rust_enum | 25 | 4 | `EffectiveGlyphRenderEntry` | ok |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 52 | rust_struct | 15 | 2 | `PreparedOverlay` | ok |
| `crates/pdf-viewer-core/src/render/facade_types.rs` | 5 | rust_struct | 21 | 3 | `ViewportLayoutRequest` | ok |
| `crates/pdf-viewer-core/src/render/facade_types.rs` | 14 | rust_struct | 19 | 3 | `ViewportTileRequest` | ok |
| `crates/pdf-viewer-core/src/render/find_state.rs` | 5 | rust_enum | 13 | 3 | `HostFindScope` | ok |
| `crates/pdf-viewer-core/src/render/find_state.rs` | 13 | rust_struct | 15 | 3 | `HostFindSession` | ok |
| `crates/pdf-viewer-core/src/render/find_state.rs` | 23 | rust_struct | 24 | 4 | `HostFindNavigationResult` | ok |
| `crates/pdf-viewer-core/src/render/layer.rs` | 15 | rust_struct | 18 | 3 | `LayerExecutionPlan` | ok |
| `crates/pdf-viewer-core/src/render/layer.rs` | 25 | rust_struct | 20 | 3 | `LayerPresentDecision` | ok |
| `crates/pdf-viewer-core/src/render/layer.rs` | 32 | rust_struct | 22 | 4 | `RenderLayerRuntimePlan` | ok |
| `crates/pdf-viewer-core/src/render/layer.rs` | 43 | rust_struct | 19 | 3 | `RenderExecutionPlan` | ok |
| `crates/pdf-viewer-core/src/render/plan_builder.rs` | 9 | rust_struct | 17 | 3 | `RenderZoomRequest` | ok |
| `crates/pdf-viewer-core/src/render/plan_builder.rs` | 20 | rust_struct | 16 | 3 | `RenderZoomResult` | ok |
| `crates/pdf-viewer-core/src/render/plan_builder.rs` | 30 | rust_struct | 16 | 3 | `FramePlanRequest` | ok |
| `crates/pdf-viewer-core/src/render/plan_builder.rs` | 48 | rust_struct | 15 | 3 | `FramePlanResult` | ok |
| `crates/pdf-viewer-core/src/render/plan_builder.rs` | 84 | rust_struct | 20 | 3 | `ViewportLayoutResult` | ok |
| `crates/pdf-viewer-core/src/render/plan_builder.rs` | 93 | rust_struct | 18 | 3 | `ViewportTileResult` | ok |
| `crates/pdf-viewer-core/src/render/plan_builder.rs` | 102 | rust_struct | 26 | 4 | `AnchorViewportLayoutResult` | ok |
| `crates/pdf-viewer-core/src/render/prepared_scene.rs` | 13 | rust_struct | 17 | 3 | `PreparedPageScene` | ok |
| `crates/pdf-viewer-core/src/render/present_plan.rs` | 9 | rust_struct | 13 | 2 | `PresentPolicy` | ok |
| `crates/pdf-viewer-core/src/render/preview.rs` | 5 | rust_struct | 18 | 3 | `PreviewPresentPlan` | ok |
| `crates/pdf-viewer-core/src/render/progressive.rs` | 21 | rust_struct | 22 | 3 | `ProgressiveRenderStart` | ok |
| `crates/pdf-viewer-core/src/render/progressive.rs` | 27 | rust_struct | 21 | 3 | `ProgressiveRenderStep` | ok |
| `crates/pdf-viewer-core/src/render/progressive.rs` | 36 | rust_struct | 23 | 3 | `ProgressiveRenderPolicy` | ok |
| `crates/pdf-viewer-core/src/render/progressive.rs` | 43 | rust_struct | 27 | 4 | `ProgressiveVectorRenderTask` | ok |
| `crates/pdf-viewer-core/src/render/progressive.rs` | 115 | rust_struct | 30 | 4 | `ProgressiveRenderPolicyRequest` | ok |
| `crates/pdf-viewer-core/src/render/renderer.rs` | 5 | rust_enum | 11 | 2 | `DrawCommand` | ok |
| `crates/pdf-viewer-core/src/render/renderer.rs` | 33 | rust_trait | 11 | 2 | `PdfRenderer` | ok |
| `crates/pdf-viewer-core/src/render/scheduler.rs` | 5 | rust_struct | 15 | 3 | `HostRenderState` | ok |
| `crates/pdf-viewer-core/src/render/scheduler.rs` | 33 | rust_struct | 19 | 3 | `RenderFrameEnvelope` | ok |
| `crates/pdf-viewer-core/src/render/scheduler.rs` | 40 | rust_struct | 21 | 3 | `RenderFrameTransition` | ok |
| `crates/pdf-viewer-core/src/render/snapshot_paint_plan.rs` | 123 | rust_struct | 9 | 2 | `RunLayout` | ok |
| `crates/pdf-viewer-core/src/render/source_suppression.rs` | 16 | rust_struct | 24 | 4 | `SuppressedVectorTextRuns` | ok |
| `crates/pdf-viewer-core/src/render/tile_cache.rs` | 15 | rust_struct | 19 | 4 | `BaseLayerCacheEntry` | ok |
| `crates/pdf-viewer-core/src/render/tile_cache.rs` | 24 | rust_struct | 20 | 4 | `DetailTileCacheEntry` | ok |
| `crates/pdf-viewer-core/src/render/tile_cache.rs` | 36 | rust_struct | 16 | 3 | `HostPresentState` | ok |
| `crates/pdf-viewer-core/src/render/tile_cache.rs` | 44 | rust_struct | 19 | 4 | `HostFrameCacheState` | ok |
| `crates/pdf-viewer-core/src/render/tile_cache.rs` | 51 | rust_struct | 21 | 4 | `FrameCacheStoreResult` | ok |
| `crates/pdf-viewer-core/src/render/viewer_session.rs` | 5 | rust_struct | 17 | 3 | `HostViewerSession` | ok |
| `crates/pdf-viewer-core/src/render/viewport_refresh.rs` | 7 | rust_struct | 24 | 4 | `HostViewportRefreshState` | ok |
| `crates/pdf-viewer-core/src/render/viewport_refresh.rs` | 13 | rust_struct | 23 | 3 | `ViewportRefreshDecision` | ok |
| `crates/pdf-viewer-core/src/render/workflow.rs` | 6 | rust_type | 19 | 3 | `RenderFrameEnvelope` | ok |
| `crates/pdf-viewer-core/src/render/workflow.rs` | 7 | rust_type | 21 | 3 | `RenderFrameTransition` | ok |
| `crates/pdf-viewer-core/src/render/workflow.rs` | 11 | rust_struct | 28 | 4 | `ProgressiveRenderStartResult` | ok |
| `crates/pdf-viewer-core/src/render/workflow.rs` | 18 | rust_struct | 27 | 4 | `ProgressiveRenderStepResult` | ok |
| `crates/pdf-viewer-core/src/render/zoom_host.rs` | 5 | rust_struct | 26 | 4 | `WheelRenderDecisionRequest` | ok |
| `crates/pdf-viewer-core/src/render/zoom_host.rs` | 15 | rust_struct | 19 | 3 | `WheelRenderDecision` | ok |
| `crates/pdf-viewer-core/src/render/zoom_host.rs` | 24 | rust_struct | 26 | 4 | `PreviewTickDecisionRequest` | ok |
| `crates/pdf-viewer-core/src/render/zoom_host.rs` | 34 | rust_struct | 19 | 3 | `PreviewTickDecision` | ok |
| `crates/pdf-viewer-core/src/render/zoom_host.rs` | 43 | rust_struct | 22 | 4 | `RenderFollowUpDecision` | ok |
| `crates/pdf-viewer-core/src/render/zoom_interaction.rs` | 12 | rust_struct | 16 | 3 | `WheelZoomRequest` | ok |
| `crates/pdf-viewer-core/src/render/zoom_interaction.rs` | 35 | rust_struct | 15 | 3 | `WheelZoomResult` | ok |
| `crates/pdf-viewer-core/src/render/zoom_interaction.rs` | 47 | rust_struct | 19 | 3 | `AnchorScrollRequest` | ok |
| `crates/pdf-viewer-core/src/render/zoom_interaction.rs` | 60 | rust_struct | 18 | 3 | `AnchorScrollResult` | ok |
| `crates/pdf-viewer-core/src/render/zoom_interaction.rs` | 67 | rust_struct | 17 | 3 | `ZoomLimitsRequest` | ok |
| `crates/pdf-viewer-core/src/render/zoom_interaction.rs` | 77 | rust_struct | 16 | 3 | `ZoomLimitsResult` | ok |
| `crates/pdf-viewer-core/src/render/zoom_interaction.rs` | 83 | rust_struct | 16 | 3 | `ZoomPreviewFrame` | ok |
| `crates/pdf-viewer-core/src/render/zoom_state.rs` | 5 | rust_struct | 15 | 3 | `ZoomAnchorState` | ok |
| `crates/pdf-viewer-core/src/render/zoom_state.rs` | 16 | rust_struct | 17 | 3 | `VisualLayoutState` | ok |
| `crates/pdf-viewer-core/src/render/zoom_state.rs` | 24 | rust_struct | 21 | 3 | `PreviewTransformState` | ok |
| `crates/pdf-viewer-core/src/render/zoom_state.rs` | 32 | rust_struct | 21 | 3 | `PendingCommittedFrame` | ok |
| `crates/pdf-viewer-core/src/render/zoom_state.rs` | 45 | rust_struct | 16 | 3 | `PreviewHostState` | ok |
| `crates/pdf-viewer-core/src/render/zoom_state.rs` | 53 | rust_struct | 13 | 3 | `HostZoomState` | ok |
| `crates/pdf-viewer-core/src/render/zoom_state.rs` | 83 | rust_struct | 17 | 3 | `ZoomAnimationStep` | ok |
| `crates/pdf-viewer-core/src/text/caret_geometry.rs` | 9 | rust_struct | 25 | 4 | `EditorCaretVisualPosition` | ok |
| `crates/pdf-viewer-core/src/text/caret_geometry.rs` | 16 | rust_struct | 9 | 2 | `CaretStop` | ok |
| `crates/pdf-viewer-core/src/text/caret_geometry.rs` | 22 | rust_struct | 9 | 2 | `CaretLine` | ok |
| `crates/pdf-viewer-core/src/text/editable_segments.rs` | 6 | rust_struct | 16 | 3 | `FieldLabelAnchor` | ok |
| `crates/pdf-viewer-core/src/text/editable_segments.rs` | 13 | rust_struct | 10 | 2 | `FieldGroup` | ok |
| `crates/pdf-viewer-core/src/text/glyph_layout.rs` | 22 | rust_struct | 22 | 3 | `DecorativePrefixLayout` | ok |
| `crates/pdf-viewer-core/src/text/glyph_layout.rs` | 37 | rust_struct | 21 | 4 | `EditorSessionTextPlan` | ok |
| `crates/pdf-viewer-core/src/text/glyph_layout.rs` | 47 | rust_enum | 19 | 4 | `EditorGlyphSlotKind` | ok |
| `crates/pdf-viewer-core/src/text/glyph_layout.rs` | 57 | rust_struct | 15 | 3 | `EditorGlyphSlot` | ok |
| `crates/pdf-viewer-core/src/text/list_semantics.rs` | 6 | rust_enum | 14 | 3 | `ListMarkerKind` | ok |
| `crates/pdf-viewer-core/src/text/list_semantics.rs` | 17 | rust_struct | 16 | 3 | `ListTextSemantic` | ok |
| `crates/pdf-viewer-core/src/text/search_replace.rs` | 2 | rust_struct | 20 | 3 | `SearchReplaceOptions` | ok |
| `crates/pdf-viewer-core/src/text/semantic_axiom.rs` | 3 | rust_struct | 11 | 2 | `AxiomEngine` | ok |
| `crates/pdf-viewer-core/src/text/style_mapper.rs` | 7 | rust_struct | 9 | 2 | `StyleSpan` | ok |
| `crates/pdf-viewer-core/src/text/style_mapper.rs` | 15 | rust_struct | 11 | 2 | `StyleMapper` | ok |
| `crates/pdf-viewer-core/src/text/text_model.rs` | 5 | rust_struct | 15 | 3 | `EditorTextModel` | ok |
| `crates/pdf-viewer-core/src/typography/engine.rs` | 6 | rust_struct | 16 | 2 | `TypographyEngine` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 7 | rust_enum | 17 | 4 | `PdfFontSourceKind` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 16 | rust_enum | 14 | 3 | `RenderFontKind` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 24 | rust_enum | 19 | 4 | `PdfEmbeddedFontKind` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 35 | rust_struct | 25 | 4 | `NormalizedPdfFontIdentity` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 46 | rust_struct | 17 | 3 | `PdfFontDescriptor` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 62 | rust_struct | 19 | 4 | `PdfFontMatchRequest` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 70 | rust_struct | 19 | 3 | `SystemFontCandidate` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 85 | rust_struct | 11 | 2 | `MatchReason` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 93 | rust_struct | 21 | 4 | `SystemFontMatchResult` | ok |
| `crates/pdf-viewer-core/src/typography/models.rs` | 101 | rust_struct | 15 | 3 | `ResolvedPdfFont` | ok |
| `crates/pdf-viewer-ui/src/annotation/annotation_api.rs` | 65 | rust_struct | 17 | 2 | `AnnotationManager` | ok |
| `crates/pdf-viewer-ui/src/app_controller.rs` | 57 | rust_struct | 9 | 2 | `PdfLogger` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 47 | rust_struct | 16 | 2 | `ApplicationState` | ok |
| `crates/pdf-viewer-ui/src/application.rs` | 90 | rust_struct | 11 | 1 | `Application` | ok |
| `crates/pdf-viewer-ui/src/commands.rs` | 5 | rust_enum | 14 | 3 | `PdfEditCommand` | ok |
| `crates/pdf-viewer-ui/src/comment/comment_api.rs` | 31 | rust_struct | 14 | 2 | `CommentManager` | ok |
| `crates/pdf-viewer-ui/src/document/comment.rs` | 21 | rust_type | 23 | 4 | `PdfCommentReviewDisplay` | ok |
| `crates/pdf-viewer-ui/src/document/comment.rs` | 26 | rust_struct | 12 | 3 | `PathPageArgs` | ok |
| `crates/pdf-viewer-ui/src/document/comment.rs` | 33 | rust_struct | 15 | 3 | `PathRequestArgs` | ok |
| `crates/pdf-viewer-ui/src/document/document_api.rs` | 27 | rust_struct | 15 | 2 | `DocumentSession` | ok |
| `crates/pdf-viewer-ui/src/document/host_pipeline.rs` | 15 | rust_struct | 27 | 4 | `OpenDocumentPipelineRequest` | ok |
| `crates/pdf-viewer-ui/src/document/host_pipeline.rs` | 24 | rust_struct | 26 | 4 | `OpenDocumentPipelineResult` | ok |
| `crates/pdf-viewer-ui/src/document/host_pipeline.rs` | 34 | rust_struct | 27 | 4 | `CloseDocumentPipelineResult` | ok |
| `crates/pdf-viewer-ui/src/document/host_pipeline.rs` | 42 | rust_struct | 27 | 4 | `PickDocumentPipelineRequest` | ok |
| `crates/pdf-viewer-ui/src/document/host_pipeline.rs` | 50 | rust_struct | 28 | 4 | `RotateDocumentPipelineResult` | ok |
| `crates/pdf-viewer-ui/src/document/host_pipeline.rs` | 56 | rust_struct | 30 | 4 | `DocumentMutationPipelineResult` | ok |
| `crates/pdf-viewer-ui/src/document/io.rs` | 9 | rust_struct | 17 | 4 | `OpenPdfFileResult` | ok |
| `crates/pdf-viewer-ui/src/document/io.rs` | 16 | rust_struct | 23 | 4 | `RotateCurrentPageResult` | ok |
| `crates/pdf-viewer-ui/src/document/mutation_pipeline.rs` | 10 | rust_struct | 29 | 4 | `DocumentRefreshPipelineResult` | ok |
| `crates/pdf-viewer-ui/src/editor/activation.rs` | 24 | rust_struct | 30 | 6 | `OpenEditorAtClientPointRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/activation.rs` | 51 | rust_struct | 29 | 6 | `MoveCaretToClientPointRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/activation.rs` | 72 | rust_struct | 23 | 4 | `SaveEditorSessionResult` | ok |
| `crates/pdf-viewer-ui/src/editor/command.rs` | 14 | rust_enum | 18 | 3 | `EditorInputCommand` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 14 | rust_struct | 14 | 3 | `HitTestRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 27 | rust_struct | 16 | 3 | `OpenBlockRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 45 | rust_struct | 16 | 3 | `MoveCaretRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 58 | rust_struct | 13 | 2 | `CommitRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 66 | rust_struct | 13 | 2 | `EditorSession` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 447 | rust_struct | 16 | 3 | `SyncInputRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 482 | rust_struct | 14 | 2 | `CommandRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_api.rs` | 619 | rust_struct | 17 | 3 | `OpenRegionRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_controller.rs` | 37 | rust_struct | 22 | 3 | `EditorVisibilityAction` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_format.rs` | 7 | rust_struct | 23 | 4 | `ActiveEditorFormatState` | ok |
| `crates/pdf-viewer-ui/src/editor/editor_format.rs` | 24 | rust_enum | 18 | 3 | `EditorFormatAction` | ok |
| `crates/pdf-viewer-ui/src/editor/format/list_format.rs` | 15 | rust_struct | 18 | 3 | `EffectiveListState` | ok |
| `crates/pdf-viewer-ui/src/editor/format/list_format.rs` | 21 | rust_struct | 20 | 3 | `ParagraphListContext` | ok |
| `crates/pdf-viewer-ui/src/editor/host_mode.rs` | 8 | rust_struct | 22 | 4 | `ToggleEditorModeResult` | ok |
| `crates/pdf-viewer-ui/src/editor/host_runtime.rs` | 7 | rust_struct | 22 | 4 | `EditorHostRuntimeState` | ok |
| `crates/pdf-viewer-ui/src/editor/host_snapshot.rs` | 17 | rust_struct | 25 | 4 | `ActiveEditorRunDiagnostic` | ok |
| `crates/pdf-viewer-ui/src/editor/host_snapshot.rs` | 28 | rust_struct | 23 | 3 | `ActiveEditorDiagnostics` | ok |
| `crates/pdf-viewer-ui/src/editor/host_snapshot.rs` | 45 | rust_struct | 26 | 4 | `ActiveEditorSlotDiagnostic` | ok |
| `crates/pdf-viewer-ui/src/editor/host_snapshot.rs` | 55 | rust_struct | 18 | 3 | `EditorHostSnapshot` | ok |
| `crates/pdf-viewer-ui/src/editor/orchestrator/render_transaction.rs` | 24 | rust_struct | 29 | 4 | `EditorRenderTransactionResult` | ok |
| `crates/pdf-viewer-ui/src/editor/orchestrator/render_transaction.rs` | 31 | rust_struct | 34 | 5 | `EditorInputRenderTransactionResult` | ok |
| `crates/pdf-viewer-ui/src/editor/orchestrator/replace_pipeline.rs` | 12 | rust_struct | 24 | 4 | `RegionTextReplaceRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/orchestrator/replace_pipeline.rs` | 25 | rust_struct | 23 | 4 | `RegionTextReplaceResult` | ok |
| `crates/pdf-viewer-ui/src/editor/overlay/projection.rs` | 11 | rust_struct | 35 | 4 | `ProjectedParagraphInteractionTarget` | ok |
| `crates/pdf-viewer-ui/src/editor/overlay/projection.rs` | 31 | rust_struct | 20 | 3 | `ProjectedEditorShell` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 15 | rust_struct | 17 | 3 | `SearchPageRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 24 | rust_struct | 21 | 3 | `SearchDocumentRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 37 | rust_struct | 15 | 3 | `FindSessionData` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 47 | rust_struct | 13 | 2 | `ReplaceResult` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 54 | rust_struct | 19 | 3 | `BatchReplaceRequest` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 64 | rust_struct | 18 | 3 | `BatchReplaceResult` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 72 | rust_struct | 18 | 3 | `SearchFacadeResult` | ok |
| `crates/pdf-viewer-ui/src/editor/search_facade.rs` | 80 | rust_struct | 14 | 2 | `FindNavigation` | ok |
| `crates/pdf-viewer-ui/src/editor/session/session.rs` | 19 | rust_struct | 15 | 3 | `EditorModeState` | ok |
| `crates/pdf-viewer-ui/src/editor/session/session.rs` | 27 | rust_struct | 27 | 5 | `ActiveEditorInputSyncResult` | ok |
| `crates/pdf-viewer-ui/src/events.rs` | 56 | rust_struct | 13 | 3 | `EventBusInner` | ok |
| `crates/pdf-viewer-ui/src/find/find_api.rs` | 19 | rust_struct | 11 | 2 | `FindSession` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 12 | rust_struct | 11 | 2 | `SearchMatch` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 28 | rust_struct | 9 | 2 | `SearchBox` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 37 | rust_struct | 12 | 2 | `SearchResult` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 45 | rust_enum | 9 | 2 | `FindScope` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 73 | rust_enum | 16 | 3 | `FindSessionState` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 118 | rust_struct | 19 | 3 | `FindControllerState` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 131 | rust_struct | 15 | 3 | `FindStateUpdate` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 141 | rust_struct | 16 | 3 | `CurrentPageMatch` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 156 | rust_struct | 14 | 2 | `ReplaceRequest` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 173 | rust_struct | 16 | 3 | `FindToolbarState` | ok |
| `crates/pdf-viewer-ui/src/find/find_store.rs` | 188 | rust_struct | 19 | 3 | `FindControllerInner` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 32 | rust_struct | 11 | 2 | `PointResult` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 39 | rust_struct | 10 | 2 | `RectResult` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 48 | rust_struct | 16 | 2 | `TransformContext` | ok |
| `crates/pdf-viewer-ui/src/geometry_api.rs` | 59 | rust_struct | 11 | 2 | `GeometryApi` | ok |
| `crates/pdf-viewer-ui/src/host/command.rs` | 10 | rust_struct | 26 | 4 | `OpenDocumentSessionRequest` | ok |
| `crates/pdf-viewer-ui/src/host/command.rs` | 20 | rust_struct | 16 | 3 | `HostActionResult` | ok |
| `crates/pdf-viewer-ui/src/host/layout.rs` | 9 | rust_struct | 18 | 3 | `HostLayoutOverride` | ok |
| `crates/pdf-viewer-ui/src/host/layout.rs` | 18 | rust_struct | 21 | 4 | `SyncHostLayoutRequest` | ok |
| `crates/pdf-viewer-ui/src/host/layout.rs` | 29 | rust_struct | 20 | 4 | `SyncHostLayoutResult` | ok |
| `crates/pdf-viewer-ui/src/page/page_store.rs` | 14 | rust_type | 13 | 3 | `HostPageState` | ok |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 18 | rust_enum | 13 | 3 | `PageTurnPhase` | ok |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 29 | rust_struct | 16 | 3 | `PageTurnSnapshot` | ok |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 63 | rust_struct | 16 | 3 | `PageTurnDecision` | ok |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 76 | rust_struct | 19 | 3 | `PageVisibleDecision` | ok |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 88 | rust_struct | 18 | 3 | `PageAssetAdmission` | ok |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 100 | rust_struct | 18 | 3 | `PagePrefetchTarget` | ok |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 109 | rust_struct | 20 | 3 | `PagePrefetchDecision` | ok |
| `crates/pdf-viewer-ui/src/presentation/presentation_api.rs` | 11 | rust_struct | 23 | 3 | `PagePresentationRuntime` | ok |
| `crates/pdf-viewer-ui/src/presentation/render_queue.rs` | 8 | rust_struct | 17 | 3 | `RenderQueueAction` | ok |
| `crates/pdf-viewer-ui/src/render/canvas.rs` | 31 | rust_enum | 14 | 2 | `CoordinateMode` | ok |
| `crates/pdf-viewer-ui/src/render/canvas.rs` | 36 | rust_struct | 14 | 2 | `CanvasRenderer` | ok |
| `crates/pdf-viewer-ui/src/render/canvas.rs` | 81 | rust_struct | 19 | 3 | `TextMetricsSnapshot` | ok |
| `crates/pdf-viewer-ui/src/render/commit.rs` | 9 | rust_struct | 18 | 3 | `RenderCommitResult` | ok |
| `crates/pdf-viewer-ui/src/render/host_runtime.rs` | 6 | rust_struct | 19 | 4 | `HostRenderLoopState` | ok |
| `crates/pdf-viewer-ui/src/render/wasm_facade.rs` | 27 | rust_struct | 10 | 2 | `StubResult` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 30 | rust_enum | 18 | 3 | `ReviewSessionState` | ok |
| `crates/pdf-viewer-ui/src/review/review_api.rs` | 60 | rust_struct | 13 | 2 | `ReviewSession` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_api.rs` | 21 | rust_struct | 13 | 2 | `ViewerSession` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_controller.rs` | 21 | rust_struct | 25 | 4 | `ViewerRuntimeResetOptions` | ok |
| `crates/pdf-viewer-ui/src/viewer/viewer_store.rs` | 20 | rust_enum | 18 | 3 | `ViewerSessionState` | ok |
| `crates/pdf-viewer-ui/src/zoom/event.rs` | 16 | rust_struct | 20 | 4 | `WheelZoomHostRequest` | ok |
| `crates/pdf-viewer-ui/src/zoom/event.rs` | 23 | rust_struct | 19 | 4 | `WheelZoomHostResult` | ok |
| `crates/pdf-viewer-ui/src/zoom/event.rs` | 31 | rust_struct | 22 | 4 | `PreviewHostStepRequest` | ok |
| `crates/pdf-viewer-ui/src/zoom/event.rs` | 37 | rust_struct | 21 | 4 | `PreviewHostStepResult` | ok |
| `crates/pdf-viewer-ui/src/zoom/zoom_store.rs` | 22 | rust_enum | 16 | 3 | `ZoomSessionState` | ok |
| `src-tauri/src/app_state.rs` | 29 | rust_struct | 13 | 2 | `DocumentStore` | ok |
| `src-tauri/src/app_state.rs` | 46 | rust_struct | 10 | 2 | `CacheStore` | ok |
| `src-tauri/src/app_state.rs` | 72 | rust_struct | 12 | 2 | `HistoryStore` | ok |
| `src-tauri/src/app_state.rs` | 87 | rust_struct | 13 | 2 | `RendererState` | ok |
| `src-tauri/src/app_state.rs` | 100 | rust_struct | 8 | 2 | `AppState` | ok |
| `src-tauri/src/application/pdf/page_annotation.rs` | 20 | rust_struct | 20 | 4 | `PdfPageHighlightItem` | ok |
| `src-tauri/src/application/pdf/page_annotation.rs` | 31 | rust_struct | 20 | 4 | `PdfPageHighlightList` | ok |
| `src-tauri/src/application/pdf/page_annotation.rs` | 40 | rust_struct | 25 | 4 | `PdfRegionHighlightRequest` | ok |
| `src-tauri/src/application/pdf/page_annotation.rs` | 50 | rust_struct | 24 | 4 | `PdfRegionHighlightResult` | ok |
| `src-tauri/src/application/pdf/page_asset.rs` | 6 | rust_enum | 13 | 3 | `PageAssetRole` | ok |
| `src-tauri/src/application/pdf/page_asset.rs` | 239 | rust_enum | 13 | 3 | `PageAssetKind` | ok |
| `src-tauri/src/application/pdf/page_asset.rs` | 257 | rust_struct | 25 | 4 | `PageAssetAdmissionService` | ok |
| `src-tauri/src/application/pdf/page_search.rs` | 8 | rust_struct | 20 | 4 | `PdfPageSearchRequest` | ok |
| `src-tauri/src/application/pdf/page_search.rs` | 16 | rust_struct | 16 | 4 | `PdfPageSearchBox` | ok |
| `src-tauri/src/application/pdf/page_search.rs` | 25 | rust_struct | 18 | 4 | `PdfPageSearchMatch` | ok |
| `src-tauri/src/application/pdf/page_search.rs` | 41 | rust_struct | 19 | 4 | `PdfPageSearchResult` | ok |
| `src-tauri/src/application/pdf/page_search.rs` | 52 | rust_struct | 23 | 4 | `PdfDocumentSearchResult` | ok |
| `src-tauri/src/error.rs` | 50 | rust_enum | 8 | 2 | `PdfError` | ok |
| `src-tauri/src/error.rs` | 104 | rust_type | 9 | 2 | `PdfResult` | ok |
| `src-tauri/src/infrastructure/pdf_read/backend.rs` | 2 | rust_trait | 14 | 3 | `PdfReadBackend` | ok |
| `src-tauri/src/infrastructure/pdf_read/facade.rs` | 5 | rust_struct | 13 | 3 | `PdfReadFacade` | ok |
| `src-tauri/src/infrastructure/pdf_read/scanned_backend.rs` | 20 | rust_struct | 18 | 3 | `ScannedReadBackend` | ok |
| `src-tauri/src/infrastructure/pdf_read/scanned_backend.rs` | 26 | rust_struct | 22 | 2 | `ClassificationDecision` | ok |
| `src-tauri/src/infrastructure/pdf_read/types.rs` | 6 | rust_struct | 11 | 2 | `PagePreview` | ok |
| `src-tauri/src/infrastructure/pdf_read/vector_backend.rs` | 5 | rust_struct | 17 | 3 | `VectorReadBackend` | ok |
| `src-tauri/src/infrastructure/pdf/annotation_store.rs` | 4 | rust_struct | 18 | 3 | `StoredPdfHighlight` | ok |
| `src-tauri/src/infrastructure/pdf/annotation_store.rs` | 11 | rust_struct | 16 | 3 | `StoredPdfComment` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 10 | rust_trait | 14 | 3 | `PdfEditCommand` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 17 | rust_struct | 18 | 3 | `ReplaceTextCommand` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 35 | rust_struct | 29 | 4 | `PersistableRegionPatchCommand` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 66 | rust_struct | 17 | 3 | `TextReflowCommand` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 95 | rust_struct | 22 | 4 | `BatchTextReflowCommand` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 127 | rust_struct | 19 | 3 | `ReplaceImageCommand` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 167 | rust_struct | 17 | 3 | `AddCommentCommand` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 181 | rust_struct | 20 | 3 | `UpdateCommentCommand` | ok |
| `src-tauri/src/infrastructure/pdf/commands.rs` | 194 | rust_struct | 23 | 3 | `DeleteAnnotationCommand` | ok |
| `src-tauri/src/infrastructure/pdf/document_service.rs` | 41 | rust_struct | 18 | 3 | `PdfDocumentService` | ok |
| `src-tauri/src/infrastructure/pdf/font/matching.rs` | 12 | rust_struct | 20 | 4 | `PdfSystemFontMatcher` | ok |
| `src-tauri/src/infrastructure/pdf/font/ttc.rs` | 105 | rust_struct | 15 | 3 | `SfntTableRecord` | ok |
| `src-tauri/src/infrastructure/pdf/geometry_service.rs` | 6 | rust_struct | 24 | 4 | `PdfEditorGeometryService` | ok |
| `src-tauri/src/infrastructure/pdf/layout_analyzer.rs` | 5 | rust_struct | 19 | 3 | `LayoutGraphAnalyzer` | ok |
| `src-tauri/src/infrastructure/pdf/layout_engine.rs` | 35 | rust_struct | 19 | 3 | `LayoutGraphAnalyzer` | ok |
| `src-tauri/src/infrastructure/pdf/log_service.rs` | 158 | rust_struct | 12 | 3 | `PdfEventSpan` | ok |
| `src-tauri/src/infrastructure/pdf/log_service.rs` | 226 | rust_struct | 11 | 2 | `ProfileSpan` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 15 | rust_struct | 16 | 3 | `EmbeddedGlyphMap` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 40 | rust_struct | 9 | 2 | `PageModel` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 51 | rust_struct | 12 | 3 | `PageTextInfo` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 66 | rust_struct | 14 | 3 | `TextObjectInfo` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 74 | rust_struct | 15 | 3 | `TextReflowPatch` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 100 | rust_struct | 32 | 4 | `PdfMaterializationDecisionReport` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 109 | rust_struct | 29 | 4 | `PdfMaterializationSourceStats` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 117 | rust_struct | 24 | 3 | `PdfMaterializationReport` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 130 | rust_struct | 16 | 2 | `PdfModifications` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 142 | rust_struct | 11 | 2 | `PathSegment` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 149 | rust_struct | 15 | 3 | `NativePathModel` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 211 | rust_struct | 16 | 3 | `NativeImageModel` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 231 | rust_struct | 13 | 2 | `VectorPalette` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 238 | rust_struct | 9 | 2 | `TextPatch` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 248 | rust_enum | 12 | 2 | `RenderObject` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 255 | rust_struct | 15 | 3 | `PageDisplayList` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 265 | rust_struct | 21 | 4 | `NativeVectorPageModel` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 292 | rust_enum | 13 | 3 | `LightPageKind` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 301 | rust_struct | 14 | 3 | `LightPageModel` | ok |
| `src-tauri/src/infrastructure/pdf/models.rs` | 311 | rust_struct | 11 | 2 | `PdfMetadata` | ok |
| `src-tauri/src/infrastructure/pdf/page_intermediate_service.rs` | 8 | rust_struct | 22 | 3 | `PageIntermediateBundle` | ok |
| `src-tauri/src/infrastructure/pdf/page_intermediate_service.rs` | 13 | rust_struct | 26 | 4 | `PdfPageIntermediateService` | ok |
| `src-tauri/src/infrastructure/pdf/page_model_service.rs` | 7 | rust_struct | 19 | 4 | `PdfPageModelService` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_font.rs` | 8 | rust_struct | 4 | 2 | `CMap` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_font.rs` | 33 | rust_struct | 10 | 2 | `ParsedFont` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_font.rs` | 386 | rust_struct | 11 | 2 | `ParsedImage` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_font.rs` | 392 | rust_struct | 13 | 2 | `ResourceCache` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_read_service.rs` | 16 | rust_struct | 14 | 3 | `PdfReadService` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_read.rs` | 10 | rust_struct | 13 | 2 | `GraphicsState` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_read.rs` | 70 | rust_type | 13 | 2 | `FlatResources` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_write_font_resolver.rs` | 12 | rust_struct | 16 | 4 | `PdfTextWriteFont` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_write_font_resolver.rs` | 20 | rust_enum | 20 | 4 | `PdfTextWriteEncoding` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_write_font_resolver.rs` | 41 | rust_struct | 19 | 3 | `ResolvedFontProgram` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_write_service.rs` | 8 | rust_struct | 15 | 3 | `PdfWriteService` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_write.rs` | 15 | rust_trait | 9 | 3 | `PdfDocExt` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_write.rs` | 762 | rust_struct | 12 | 3 | `PdfTextState` | ok |
| `src-tauri/src/infrastructure/pdf/pdf_write.rs` | 778 | rust_struct | 13 | 2 | `ReflowCluster` | ok |
| `src-tauri/src/infrastructure/pdf/region_materializer.rs` | 9 | rust_struct | 29 | 3 | `RegionMaterializationDecision` | ok |
| `src-tauri/src/infrastructure/pdf/region_materializer.rs` | 17 | rust_struct | 25 | 3 | `RegionMaterializationPlan` | ok |
| `src-tauri/src/infrastructure/pdf/region_materializer.rs` | 106 | rust_struct | 18 | 3 | `SnapshotLineReflow` | ok |
| `src-tauri/src/infrastructure/pdf/save_text_write_plan.rs` | 2 | rust_struct | 21 | 4 | `PersistedTextLinePlan` | ok |
| `src-tauri/src/infrastructure/pdf/spatial_graph.rs` | 14 | rust_struct | 12 | 2 | `SpatialGraph` | ok |
| `src-tauri/src/infrastructure/pdf/vello_renderer.rs` | 17 | rust_struct | 13 | 2 | `VelloRenderer` | ok |
| `src-tauri/src/interfaces/pdf/render.rs` | 12 | rust_struct | 15 | 3 | `PageAssetBundle` | ok |
| `src-tauri/src/state.rs` | 4 | rust_enum | 13 | 2 | `LoadingStatus` | ok |
| `src/bridge/ai/resume_ai_apply.ts` | 21 | ts_type | 12 | 2 | `ApplyContext` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 24 | ts_interface | 19 | 4 | `PlanResumeAiRequest` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 33 | ts_interface | 20 | 4 | `ApplyResumeAiRequest` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 39 | ts_interface | 26 | 5 | `SyncResumeAiSessionRequest` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 45 | ts_interface | 27 | 5 | `SubmitResumeAiPromptRequest` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 53 | ts_interface | 30 | 5 | `ApplyResumeAiSuggestionRequest` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 60 | ts_interface | 12 | 2 | `SessionState` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 102 | ts_interface | 10 | 2 | `GeminiPart` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 103 | ts_interface | 13 | 2 | `GeminiContent` | ok |
| `src/bridge/ai/resume_ai_client.ts` | 104 | ts_interface | 14 | 2 | `GeminiResponse` | ok |
| `src/bridge/ai/resume_ai_controller.ts` | 25 | ts_type | 12 | 2 | `ApplyContext` | ok |
| `src/bridge/ai/resume_ai_controller.ts` | 28 | ts_type | 10 | 2 | `StatusTone` | ok |
| `src/bridge/ai/resume_ai_controller.ts` | 30 | ts_type | 22 | 4 | `ResumeAiControllerDeps` | ok |
| `src/bridge/ai/resume_ai_controller.ts` | 38 | ts_type | 18 | 3 | `ResumeAiController` | ok |
| `src/bridge/ai/resume_ai_controller.ts` | 81 | ts_class | 21 | 4 | `PdfResumeAiController` | ok |
| `src/bridge/ai/resume_ai_diff_preview.ts` | 1 | ts_type | 9 | 2 | `DiffToken` | ok |
| `src/bridge/ai/resume_ai_panel_state_view.ts` | 3 | ts_type | 10 | 2 | `StatusTone` | ok |
| `src/bridge/ai/resume_ai_panel_state_view.ts` | 5 | ts_type | 13 | 3 | `BusyStateArgs` | ok |
| `src/bridge/ai/resume_ai_panel_view.ts` | 4 | ts_type | 21 | 3 | `ApplySuggestionSource` | ok |
| `src/bridge/ai/resume_ai_panel_view.ts` | 6 | ts_type | 30 | 5 | `RenderResumeAiConversationArgs` | ok |
| `src/bridge/ai/resume_ai_panel_view.ts` | 15 | ts_type | 23 | 5 | `SyncResumeAiSummaryArgs` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 1 | ts_type | 16 | 3 | `ResumeRegionKind` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 3 | ts_type | 14 | 3 | `ResumeChatRole` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 5 | ts_type | 13 | 3 | `ResumeAiScope` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 7 | ts_interface | 14 | 3 | `ResumeChatTurn` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 12 | ts_interface | 25 | 4 | `PdfPersistableRegionPatch` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 28 | ts_interface | 17 | 4 | `ResumeAiEditDraft` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 36 | ts_interface | 12 | 3 | `ResumeAiPlan` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 41 | ts_interface | 18 | 4 | `ResumeAiPlanResult` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 47 | ts_interface | 18 | 4 | `ResumeAiThreadView` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 58 | ts_interface | 18 | 3 | `ResumeAiSuggestion` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 73 | ts_interface | 22 | 4 | `RawParagraphRegionLine` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 80 | ts_interface | 18 | 3 | `RawParagraphRegion` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 89 | ts_interface | 17 | 4 | `RawListItemRegion` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 99 | ts_interface | 20 | 4 | `RawPageRegionContext` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 105 | ts_interface | 20 | 3 | `ResumeEditableRegion` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 119 | ts_interface | 17 | 3 | `ResumePageContext` | ok |
| `src/bridge/ai/resume_ai_types.ts` | 126 | ts_interface | 21 | 3 | `ResumeDocumentContext` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 5 | ts_type | 21 | 3 | `ViewerSessionSnapshot` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 10 | ts_type | 23 | 4 | `PdfPageAnnotationTarget` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 25 | ts_type | 29 | 5 | `PdfPageAnnotationTargetResult` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 29 | ts_type | 20 | 4 | `PdfPageHighlightItem` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 43 | ts_type | 20 | 4 | `PdfPageHighlightList` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 47 | ts_type | 33 | 5 | `CreatePdfAnnotationControllerDeps` | ok |
| `src/bridge/annotation/pdf_annotation_controller.ts` | 52 | ts_type | 23 | 3 | `PdfAnnotationController` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 1 | ts_type | 21 | 3 | `ViewerSessionSnapshot` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 6 | ts_type | 23 | 4 | `PdfPageAnnotationTarget` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 13 | ts_type | 29 | 5 | `PdfCommentTargetOverlayMarker` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 27 | ts_type | 30 | 5 | `PdfCommentTargetOverlayDisplay` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 31 | ts_type | 18 | 4 | `PdfPageCommentItem` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 46 | ts_type | 23 | 4 | `PdfCommentOverlayMarker` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 58 | ts_type | 24 | 4 | `PdfCommentOverlayDisplay` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 62 | ts_type | 27 | 5 | `PdfCommentReviewPageSummary` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 68 | ts_type | 22 | 4 | `PdfCommentReviewResult` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 76 | ts_type | 27 | 5 | `PdfCommentReviewSummaryChip` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 81 | ts_type | 26 | 5 | `PdfCommentReviewCardAction` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 87 | ts_type | 20 | 4 | `PdfCommentReviewCard` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 98 | ts_type | 21 | 4 | `PdfCommentReviewPanel` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 105 | ts_type | 18 | 3 | `CommentReviewScope` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 107 | ts_type | 20 | 3 | `CommentReviewSession` | ok |
| `src/bridge/comment/pdf_comment_contracts.ts` | 114 | ts_type | 23 | 4 | `PdfCommentReviewDisplay` | ok |
| `src/bridge/comment/pdf_comment_controller.ts` | 20 | ts_type | 30 | 5 | `CreatePdfCommentControllerDeps` | ok |
| `src/bridge/comment/pdf_comment_controller.ts` | 27 | ts_type | 20 | 3 | `PdfCommentController` | ok |
| `src/bridge/comment/pdf_comment_dom.ts` | 4 | ts_type | 18 | 4 | `PdfCommentDomNodes` | ok |
| `src/bridge/comment/pdf_comment_host_actions.ts` | 10 | ts_type | 31 | 6 | `CreatePdfCommentHostActionsDeps` | ok |
| `src/bridge/comment/pdf_comment_host_actions.ts` | 23 | ts_type | 21 | 4 | `PdfCommentHostActions` | ok |
| `src/bridge/comment/pdf_comment_review_view.ts` | 6 | ts_type | 15 | 3 | `ReviewViewNodes` | ok |
| `src/bridge/comment/pdf_comment_review_view.ts` | 13 | ts_type | 18 | 3 | `ReviewViewHandlers` | ok |
| `src/bridge/comment/pdf_comment_wasm_bridge.ts` | 16 | ts_type | 30 | 6 | `CreatePdfCommentWasmBridgeDeps` | ok |
| `src/bridge/comment/pdf_comment_wasm_bridge.ts` | 46 | ts_type | 20 | 4 | `PdfCommentWasmBridge` | ok |
| `src/bridge/document/document_edit_api.ts` | 5 | ts_type | 24 | 4 | `RegionTextReplaceRequest` | ok |
| `src/bridge/document/document_edit_api.ts` | 6 | ts_type | 23 | 4 | `RegionTextReplaceResult` | ok |
| `src/bridge/document/document_edit_api.ts` | 7 | ts_type | 24 | 4 | `AcceptReviewChangeResult` | ok |
| `src/bridge/document/document_edit_api.ts` | 8 | ts_type | 24 | 4 | `RejectReviewChangeResult` | ok |
| `src/bridge/document/document_edit_api.ts` | 9 | ts_type | 22 | 4 | `ReviewBulkChangeResult` | ok |
| `src/bridge/document/document_edit_api.ts` | 10 | ts_type | 16 | 3 | `ReviewFeedResult` | ok |
| `src/bridge/document/document_edit_api.ts` | 14 | ts_type | 13 | 3 | `PdfEditSource` | ok |
| `src/bridge/document/document_edit_api.ts` | 28 | ts_type | 13 | 3 | `PdfSaveResult` | ok |
| `src/bridge/document/document_edit_api.ts` | 34 | ts_type | 17 | 4 | `PdfRegionTextEdit` | ok |
| `src/bridge/document/document_edit_api.ts` | 43 | ts_type | 20 | 4 | `PdfRegionTextReplace` | ok |
| `src/bridge/document/document_edit_api.ts` | 45 | ts_type | 19 | 4 | `DocumentEditApiDeps` | ok |
| `src/bridge/document/document_edit_api.ts` | 56 | ts_type | 15 | 3 | `DocumentEditApi` | ok |
| `src/bridge/document/pdf_document_runtime.ts` | 23 | ts_type | 28 | 5 | `CreatePdfDocumentRuntimeDeps` | ok |
| `src/bridge/document/pdf_document_runtime.ts` | 44 | ts_type | 18 | 3 | `PdfDocumentRuntime` | ok |
| `src/bridge/editor/editor_host_view.ts` | 32 | ts_type | 26 | 3 | `ParagraphInteractionTarget` | ok |
| `src/bridge/editor/editor_host_view.ts` | 49 | ts_type | 18 | 3 | `ActiveEditorTarget` | ok |
| `src/bridge/editor/editor_host_view.ts` | 67 | ts_type | 16 | 3 | `HostReferenceBox` | ok |
| `src/bridge/editor/editor_host_view.ts` | 74 | ts_type | 15 | 3 | `EditorHostNodes` | ok |
| `src/bridge/editor/editor_host_view.ts` | 83 | ts_type | 18 | 3 | `BeforeInputCommand` | ok |
| `src/bridge/editor/editor_host_view.ts` | 85 | ts_type | 18 | 4 | `EditorHostViewDeps` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 11 | ts_type | 10 | 3 | `GetWasmApi` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 40 | ts_type | 24 | 4 | `RegionTextReplaceRequest` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 50 | ts_type | 23 | 4 | `RegionTextReplaceResult` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 56 | ts_type | 21 | 3 | `DocumentRefreshResult` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 61 | ts_type | 17 | 3 | `ReviewChangeEntry` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 71 | ts_type | 16 | 3 | `ReviewFeedResult` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 77 | ts_type | 24 | 4 | `AcceptReviewChangeResult` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 83 | ts_type | 24 | 4 | `RejectReviewChangeResult` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 85 | ts_type | 22 | 4 | `ReviewBulkChangeResult` | ok |
| `src/bridge/editor/editor_wasm_api.ts` | 93 | ts_type | 13 | 3 | `EditorWasmApi` | ok |
| `src/bridge/editor/index.ts` | 20 | ts_type | 18 | 3 | `ActiveEditorTarget` | ok |
| `src/bridge/editor/index.ts` | 21 | ts_type | 15 | 3 | `EditorHostNodes` | ok |
| `src/bridge/editor/index.ts` | 22 | ts_type | 16 | 3 | `HostReferenceBox` | ok |
| `src/bridge/editor/index.ts` | 23 | ts_type | 26 | 3 | `ParagraphInteractionTarget` | ok |
| `src/bridge/editor/index.ts` | 31 | ts_type | 14 | 3 | `EditorHostDeps` | ok |
| `src/bridge/editor/index.ts` | 48 | ts_type | 10 | 2 | `EditorHost` | ok |
| `src/bridge/editor/types.ts` | 3 | ts_type | 12 | 2 | `SessionState` | ok |
| `src/bridge/editor/types.ts` | 5 | ts_type | 11 | 2 | `EditorError` | ok |
| `src/bridge/editor/types.ts` | 15 | ts_type | 14 | 2 | `EditorResponse` | ok |
| `src/bridge/editor/types.ts` | 24 | ts_type | 13 | 3 | `HitTestResult` | ok |
| `src/bridge/editor/types.ts` | 30 | ts_type | 15 | 3 | `OpenBlockResult` | ok |
| `src/bridge/editor/types.ts` | 36 | ts_type | 15 | 3 | `MoveCaretResult` | ok |
| `src/bridge/editor/types.ts` | 40 | ts_type | 12 | 2 | `CommitResult` | ok |
| `src/bridge/editor/types.ts` | 44 | ts_type | 14 | 2 | `SnapshotResult` | ok |
| `src/bridge/editor/types.ts` | 52 | ts_type | 13 | 3 | `TextBlockInfo` | ok |
| `src/bridge/editor/types.ts` | 62 | ts_type | 14 | 3 | `HitTestRequest` | ok |
| `src/bridge/editor/types.ts` | 73 | ts_type | 16 | 3 | `OpenBlockRequest` | ok |
| `src/bridge/editor/types.ts` | 87 | ts_type | 16 | 3 | `MoveCaretRequest` | ok |
| `src/bridge/editor/types.ts` | 98 | ts_type | 13 | 2 | `CommitRequest` | ok |
| `src/bridge/editor/types.ts` | 103 | ts_type | 16 | 3 | `SyncInputRequest` | ok |
| `src/bridge/editor/types.ts` | 108 | ts_type | 19 | 3 | `ApplyCommandRequest` | ok |
| `src/bridge/editor/types.ts` | 115 | ts_type | 15 | 3 | `SyncInputResult` | ok |
| `src/bridge/editor/types.ts` | 120 | ts_type | 18 | 3 | `ApplyCommandResult` | ok |
| `src/bridge/editor/types.ts` | 126 | ts_type | 17 | 4 | `SetEditModeResult` | ok |
| `src/bridge/editor/types.ts` | 134 | ts_type | 18 | 3 | `LegacyActiveTarget` | ok |
| `src/bridge/editor/types.ts` | 152 | ts_type | 23 | 3 | `LegacyInteractionTarget` | ok |
| `src/bridge/editor/types.ts` | 169 | ts_type | 14 | 2 | `LegacySnapshot` | ok |
| `src/bridge/editor/types.ts` | 178 | ts_type | 18 | 3 | `EditorFormatAction` | ok |
| `src/bridge/find/find_facade.ts` | 5 | ts_type | 11 | 2 | `SearchMatch` | ok |
| `src/bridge/find/find_facade.ts` | 24 | ts_type | 12 | 2 | `SearchResult` | ok |
| `src/bridge/find/find_facade.ts` | 30 | ts_type | 17 | 3 | `SearchPageRequest` | ok |
| `src/bridge/find/find_facade.ts` | 37 | ts_type | 21 | 3 | `SearchDocumentRequest` | ok |
| `src/bridge/find/find_facade.ts` | 44 | ts_type | 14 | 2 | `ReplaceRequest` | ok |
| `src/bridge/find/find_facade.ts` | 55 | ts_type | 13 | 2 | `ReplaceResult` | ok |
| `src/bridge/find/find_facade.ts` | 60 | ts_type | 19 | 3 | `BatchReplaceRequest` | ok |
| `src/bridge/find/find_facade.ts` | 68 | ts_type | 18 | 3 | `BatchReplaceResult` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 13 | ts_type | 12 | 2 | `SearchResult` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 14 | ts_type | 11 | 2 | `SearchMatch` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 18 | ts_type | 21 | 3 | `ViewerSessionSnapshot` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 24 | ts_type | 9 | 2 | `FindScope` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 26 | ts_type | 27 | 5 | `CreatePdfFindControllerDeps` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 40 | ts_type | 17 | 3 | `PdfFindController` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 61 | ts_type | 15 | 3 | `FindStateUpdate` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 75 | ts_type | 16 | 3 | `CurrentPageMatch` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 88 | ts_type | 16 | 3 | `FindToolbarState` | ok |
| `src/bridge/find/pdf_find_controller.ts` | 120 | ts_type | 9 | 2 | `FindNodes` | ok |
| `src/bridge/presentation/page_presenter.ts` | 6 | ts_type | 17 | 3 | `RasterSurfaceRole` | ok |
| `src/bridge/presentation/page_presenter.ts` | 8 | ts_type | 20 | 3 | `RasterSurfaceOptions` | ok |
| `src/bridge/presentation/page_presenter.ts` | 14 | ts_type | 21 | 3 | `PreparedRasterSurface` | ok |
| `src/bridge/presentation/page_presenter.ts` | 24 | ts_type | 17 | 3 | `PagePresenterDeps` | ok |
| `src/bridge/render/frame_plan.ts` | 4 | ts_type | 13 | 3 | `RustFramePlan` | ok |
| `src/bridge/render/frame_plan.ts` | 37 | ts_type | 16 | 3 | `RustPreviewFrame` | ok |
| `src/bridge/render/frame_plan.ts` | 50 | ts_type | 15 | 3 | `RustRenderFrame` | ok |
| `src/bridge/render/frame_plan.ts` | 55 | ts_type | 20 | 3 | `RustRenderTransition` | ok |
| `src/bridge/render/frame_plan.ts` | 60 | ts_type | 22 | 4 | `RustRenderCommitResult` | ok |
| `src/bridge/render/frame_plan.ts` | 67 | ts_type | 27 | 4 | `RustViewportRefreshDecision` | ok |
| `src/bridge/render/frame_plan.ts` | 72 | ts_type | 23 | 4 | `RustWheelRenderDecision` | ok |
| `src/bridge/render/frame_plan.ts` | 78 | ts_type | 23 | 4 | `RustPreviewTickDecision` | ok |
| `src/bridge/render/frame_plan.ts` | 85 | ts_type | 23 | 5 | `RustWheelZoomHostResult` | ok |
| `src/bridge/render/frame_plan.ts` | 93 | ts_type | 25 | 5 | `RustPreviewHostStepResult` | ok |
| `src/bridge/render/frame_plan.ts` | 98 | ts_type | 22 | 4 | `RustLayerExecutionPlan` | ok |
| `src/bridge/render/frame_plan.ts` | 106 | ts_type | 24 | 4 | `RustLayerPresentDecision` | ok |
| `src/bridge/render/frame_plan.ts` | 111 | ts_type | 18 | 3 | `RustCommittedFrame` | ok |
| `src/bridge/render/frame_plan.ts` | 122 | ts_type | 20 | 4 | `FramePlanAdapterDeps` | ok |
| `src/bridge/render/frame_plan.ts` | 131 | ts_type | 12 | 2 | `RenderReason` | ok |
| `src/bridge/render/frame_plan.ts` | 133 | ts_type | 16 | 3 | `FramePlanAdapter` | ok |
| `src/bridge/render/layout_trace.ts` | 3 | ts_type | 15 | 2 | `ElementSnapshot` | ok |
| `src/bridge/render/layout_trace.ts` | 24 | ts_type | 17 | 3 | `LayoutKeySnapshot` | ok |
| `src/bridge/render/raster_image_cache.ts` | 10 | ts_type | 17 | 3 | `RasterWarmOptions` | ok |
| `src/bridge/render/render_flow.ts` | 9 | ts_type | 14 | 3 | `RenderFlowDeps` | ok |
| `src/bridge/render/render_flow.ts` | 30 | ts_type | 14 | 2 | `VisibleSurface` | ok |
| `src/bridge/render/render_scheduler.ts` | 5 | ts_type | 12 | 2 | `RenderSource` | ok |
| `src/bridge/render/render_scheduler.ts` | 7 | ts_type | 20 | 3 | `RenderRequestContext` | ok |
| `src/bridge/render/render_scheduler.ts` | 12 | ts_type | 13 | 2 | `RenderRequest` | ok |
| `src/bridge/render/render_scheduler.ts` | 18 | ts_type | 19 | 3 | `RenderSchedulerDeps` | ok |
| `src/bridge/render/render_scheduler.ts` | 23 | ts_type | 15 | 2 | `RenderScheduler` | ok |
| `src/bridge/render/render_scheduler.ts` | 29 | ts_type | 19 | 3 | `QueuedRenderRequest` | ok |
| `src/bridge/render/render_wasm_api.ts` | 15 | ts_type | 10 | 3 | `GetWasmApi` | ok |
| `src/bridge/render/render_wasm_api.ts` | 17 | ts_type | 22 | 3 | `ProgressiveRenderStart` | ok |
| `src/bridge/render/render_wasm_api.ts` | 22 | ts_type | 23 | 3 | `ProgressiveRenderPolicy` | ok |
| `src/bridge/render/render_wasm_api.ts` | 28 | ts_type | 22 | 4 | `RenderLayerRuntimePlan` | ok |
| `src/bridge/render/render_wasm_api.ts` | 37 | ts_type | 19 | 3 | `RenderExecutionPlan` | ok |
| `src/bridge/render/render_wasm_api.ts` | 43 | ts_type | 21 | 3 | `ProgressiveRenderStep` | ok |
| `src/bridge/render/render_wasm_api.ts` | 48 | ts_type | 21 | 4 | `FrameCacheStoreResult` | ok |
| `src/bridge/render/render_wasm_api.ts` | 52 | ts_type | 13 | 3 | `RenderWasmApi` | ok |
| `src/bridge/render/vector_canvas_host.ts` | 12 | ts_type | 14 | 3 | `VectorHostRefs` | ok |
| `src/bridge/render/vector_canvas_host.ts` | 20 | ts_type | 28 | 4 | `PresentViewportCanvasOptions` | ok |
| `src/bridge/render/vector_canvas_host.ts` | 25 | ts_type | 19 | 3 | `ViewportCanvasFrame` | ok |
| `src/bridge/render/vector_canvas_pool.ts` | 7 | ts_class | 10 | 2 | `CanvasPool` | ok |
| `src/bridge/render/vector_host.ts` | 12 | ts_type | 14 | 3 | `VectorHostRefs` | ok |
| `src/bridge/render/vector_host.ts` | 58 | ts_type | 18 | 3 | `VectorRenderResult` | ok |
| `src/bridge/render/vector_host.ts` | 65 | ts_type | 19 | 3 | `VectorCommitOptions` | ok |
| `src/bridge/render/vector_host.ts` | 69 | ts_type | 18 | 3 | `VectorLayerPresent` | ok |
| `src/bridge/render/vector_host.ts` | 80 | ts_type | 14 | 3 | `RenderZoomPlan` | ok |
| `src/bridge/render/vector_page_bundle.ts` | 9 | ts_type | 16 | 3 | `VectorPageBundle` | ok |
| `src/bridge/render/vector_page_bundle.ts` | 18 | ts_type | 26 | 4 | `VectorPageBundleResolution` | ok |
| `src/bridge/render/vector_worker.ts` | 4 | ts_type | 19 | 3 | `VectorWorkerRequest` | ok |
| `src/bridge/render/vector_worker.ts` | 25 | ts_type | 20 | 3 | `VectorWorkerResponse` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 9 | ts_type | 17 | 3 | `ReviewChangeEntry` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 10 | ts_type | 16 | 3 | `ReviewFeedResult` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 11 | ts_type | 18 | 3 | `ReviewLocateResult` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 15 | ts_type | 21 | 3 | `ViewerSessionSnapshot` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 20 | ts_type | 29 | 5 | `CreatePdfReviewControllerDeps` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 32 | ts_type | 11 | 2 | `ReviewScope` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 34 | ts_type | 11 | 2 | `ReviewNodes` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 48 | ts_type | 13 | 3 | `ReviewUiState` | ok |
| `src/bridge/review/pdf_review_controller.ts` | 55 | ts_type | 19 | 3 | `PdfReviewController` | ok |
| `src/bridge/review/review_wasm_facade.ts` | 21 | ts_type | 17 | 3 | `ReviewChangeEntry` | ok |
| `src/bridge/review/review_wasm_facade.ts` | 31 | ts_type | 16 | 3 | `ReviewFeedResult` | ok |
| `src/bridge/review/review_wasm_facade.ts` | 37 | ts_type | 18 | 3 | `ReviewLocateResult` | ok |
| `src/bridge/review/review_wasm_facade.ts` | 44 | ts_type | 18 | 3 | `ReviewFacadeResult` | ok |
| `src/bridge/shared/diagnostics.ts` | 3 | ts_type | 16 | 2 | `DiagnosticFields` | ok |
| `src/bridge/shared/diagnostics.ts` | 5 | ts_type | 17 | 2 | `DiagnosticOptions` | ok |
| `src/bridge/shared/diagnostics.ts` | 12 | ts_type | 15 | 2 | `DiagnosticLevel` | ok |
| `src/bridge/viewer/page_presentation_runtime.ts` | 1 | ts_type | 16 | 3 | `PageTurnDecision` | ok |
| `src/bridge/viewer/page_presentation_runtime.ts` | 15 | ts_type | 19 | 3 | `PageVisibleDecision` | ok |
| `src/bridge/viewer/page_presentation_runtime.ts` | 25 | ts_type | 18 | 3 | `PageAssetAdmission` | ok |
| `src/bridge/viewer/page_presentation_runtime.ts` | 35 | ts_type | 18 | 3 | `PagePrefetchTarget` | ok |
| `src/bridge/viewer/page_presentation_runtime.ts` | 42 | ts_type | 20 | 3 | `PagePrefetchDecision` | ok |
| `src/bridge/viewer/page_presentation_runtime.ts` | 51 | ts_type | 17 | 3 | `RenderQueueAction` | ok |
| `src/bridge/viewer/page_presentation_runtime.ts` | 65 | ts_type | 30 | 4 | `PagePresentationRuntimeAdapter` | ok |
| `src/bridge/viewer/page_presentation_runtime.ts` | 82 | ts_type | 27 | 4 | `PagePresentationRuntimeDeps` | ok |
| `src/bridge/viewer/pdf_keyboard.ts` | 3 | ts_type | 23 | 4 | `PdfKeyboardShortcutDeps` | ok |
| `src/bridge/viewer/pdf_layout_sync.ts` | 9 | ts_type | 14 | 2 | `LayoutOverride` | ok |
| `src/bridge/viewer/pdf_layout_sync.ts` | 18 | ts_type | 14 | 3 | `LayoutSyncDeps` | ok |
| `src/bridge/viewer/pdf_runtime.ts` | 43 | ts_type | 17 | 3 | `ZoomStateSnapshot` | ok |
| `src/bridge/viewer/pdf_runtime.ts` | 50 | ts_type | 16 | 3 | `PdfViewerRuntime` | ok |
| `src/bridge/viewer/pdf_viewer_api.ts` | 10 | ts_type | 16 | 4 | `PdfViewerApiDeps` | ok |
| `src/bridge/viewer/pdf_viewer_api.ts` | 60 | ts_class | 12 | 3 | `PdfViewerAPI` | ok |
| `src/bridge/viewer/pdf_viewer_api.ts` | 328 | ts_type | 20 | 4 | `PageTurnBenchOptions` | ok |
| `src/bridge/viewer/pdf_viewer_dom.ts` | 9 | ts_type | 15 | 3 | `PdfZoomSnapshot` | ok |
| `src/bridge/viewer/viewer_geometry_probe.ts` | 6 | ts_type | 17 | 3 | `GeometryProbeDeps` | ok |
| `src/bridge/viewer/viewer_geometry_probe.ts` | 35 | ts_type | 21 | 3 | `GeometryProbeSnapshot` | ok |
| `src/bridge/viewer/viewer_geometry_probe.ts` | 45 | ts_type | 16 | 3 | `GeometryProbeApi` | ok |
| `src/bridge/viewer/viewer_session.ts` | 14 | ts_type | 21 | 3 | `ViewerSessionSnapshot` | ok |
| `src/bridge/viewer/viewer_session.ts` | 24 | ts_type | 20 | 3 | `ViewerSessionAdapter` | ok |
| `src/bridge/viewer/viewer_session.ts` | 33 | ts_type | 17 | 3 | `ViewerSessionDeps` | ok |
| `src/bridge/zoom/zoom_controller.ts` | 3 | ts_type | 20 | 3 | `AnchorViewportLayout` | ok |
| `src/bridge/zoom/zoom_controller.ts` | 12 | ts_type | 19 | 4 | `RustAnchorFramePlan` | ok |
| `src/bridge/zoom/zoom_controller.ts` | 20 | ts_type | 23 | 4 | `RustWheelRenderDecision` | ok |
| `src/bridge/zoom/zoom_controller.ts` | 26 | ts_type | 23 | 4 | `RustPreviewTickDecision` | ok |
| `src/bridge/zoom/zoom_controller.ts` | 33 | ts_type | 23 | 5 | `RustWheelZoomHostResult` | ok |
| `src/bridge/zoom/zoom_controller.ts` | 37 | ts_type | 25 | 5 | `RustPreviewHostStepResult` | ok |
| `src/bridge/zoom/zoom_controller.ts` | 53 | ts_type | 18 | 3 | `ZoomControllerDeps` | ok |
| `src/bridge/zoom/zoom_controller.ts` | 79 | ts_type | 14 | 2 | `ZoomController` | ok |
| `src/dev/verify_editor_bugs.ts` | 59 | ts_interface | 12 | 2 | `ParagraphBox` | ok |
| `src/dev/verify_editor_bugs.ts` | 87 | ts_interface | 12 | 2 | `VerifyResult` | ok |
| `tests/e2e/specs/page_presentation_runtime.spec.ts` | 21 | ts_type | 16 | 2 | `DiagnosticWindow` | ok |
| `tests/e2e/specs/page_presentation_runtime.spec.ts` | 32 | ts_type | 16 | 3 | `PageSearchResult` | ok |
| `tests/e2e/specs/page_presentation_runtime.spec.ts` | 42 | ts_type | 22 | 3 | `AnnotationTargetResult` | ok |
| `tests/e2e/wdio.conf.ts` | 44 | ts_interface | 10 | 2 | `CustomCaps` | ok |
| `tests/e2e/wdio.conf.ts` | 50 | ts_type | 7 | 2 | `CapItem` | ok |
| `tests/e2e/wdio.conf.ts` | 51 | ts_type | 12 | 2 | `CustomConfig` | ok |
| `utils/ai-settings.ts` | 7 | ts_interface | 10 | 2 | `AiSettings` | ok |

## Rust 命名异常

未发现 Rust snake_case 异常。

| 文件 | 行 | 类型 | 上下文 | 名称 |
|---|---:|---|---|---|

## TS/JS 命名异常

未发现 TS/JS 命名异常。

| 文件 | 行 | 类型 | 上下文 | 名称 |
|---|---:|---|---|---|

## 长/句子式方法名

未发现长/句子式方法名。

| 文件 | 行 | 类型 | 测试 | 长度 | 分段数 | 名称 |
|---|---:|---|---|---:|---:|---|

## 历史标签或版本标签命中

文档反对 `v3`、`audit`、`sovereign` 等历史标签，临时日志 tag 除外。以下命中应清理或给出理由。

| 文件 | 行 | 类型 | 上下文 | 名称 |
|---|---:|---|---|---|
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | 190 | rust_method | PdfCoordinateSpace | `denormalize_y` |
| `crates/pdf-viewer-core/src/geometry/reflow_engine.rs` | 5 | rust_fn |  | `calculate_reflow_displacements` |
| `src/bridge/render/render_flow.ts` | 10 | object_arrow_method |  | `targetInvokeV3` |
| `src/bridge/shared/wasm_loader.ts` | 87 | function |  | `targetInvokeV3` |

## Helper/Manager/Utils 命名命中

文档反对模糊的 helper/manager/utils 命名，明确临时用途除外。以下位置需要审查；并非每一项都一定错误。

| 文件 | 行 | 类型 | 上下文 | 名称 |
|---|---:|---|---|---|
| `crates/pdf-viewer-core/src/utils/debug.rs` | 1 | rust_fn |  | `truncate_debug_text` |
| `crates/pdf-viewer-core/src/utils/sanitize.rs` | 1 | rust_fn |  | `sanitize_positive` |
| `crates/pdf-viewer-core/src/utils/sanitize.rs` | 9 | rust_fn |  | `sanitize_non_negative` |
| `crates/pdf-viewer-ui/src/utils/chain_trace.rs` | 34 | rust_fn |  | `set_chain_trace_enabled` |
| `crates/pdf-viewer-ui/src/utils/chain_trace.rs` | 38 | rust_fn |  | `is_chain_trace_enabled` |
| `crates/pdf-viewer-ui/src/utils/chain_trace.rs` | 43 | rust_fn |  | `trace_step` |
| `src/bridge/comment/pdf_comment_wasm_bridge.ts` | 26 | function |  | `getCommentManager` |
| `tests/e2e/helpers/app.js` | 9 | function |  | `waitForApp` |
| `tests/e2e/helpers/app.js` | 38 | function |  | `loadFixturePdf` |
| `utils/ai-settings.ts` | 13 | function |  | `loadAiSettings` |
| `utils/ai-settings.ts` | 22 | function |  | `saveAiSettings` |

## 架构边界观察

- TS 拥有大量 DOM/canvas 宿主函数，这是预期的。但任何 TS 方法如果负责页面准入、渲染队列、字体、glyph、PDF 语义决策，都应迁回或镜像到 Rust/WASM。
- `targetInvokeV3`/raw invoke 字符串仍是跨边界命令面。命令名本身大多合规，但类型安全弱于文档要求的 facade/session 方向。
- 清单故意包含测试和脚本，便于区分测试专用方法与生产方法。

## 建议后续动作

1. 将本脚本接入 CI，对高置信约束失败：长/句子式方法名、Tauri command snake_case、显式 WASM `js_name` camelCase、新增裸 WASM snake_case 导出。
2. 先缩短或迁移长测试名，尤其是 `draft_layout.rs` 和生产模块内联 `#[cfg(test)] mod tests`。
3. 审查历史/版本标签命中，把活跃运行时命名改为中性名称；只保留兼容 alias 或日志 tag。
4. 用 typed bridge 方法或现有领域 facade 包装 raw invoke 字符串，逐步减少裸调用。
5. 单独审查 TS render/presentation 方法，确认每个方法只是宿主适配，还是持有了应迁到 WASM 的决策。