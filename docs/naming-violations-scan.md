# 命名违规扫描报告

> 扫描日期：2026-06-18
> 规则依据：docs/naming-conventions.md

## 命名原则

函数名应**精炼表达核心行为**，而非用自然语言描述调用场景：

```rust
// ❌ 描述调用场景（for/with/from 后缀）
resolve_layout_inference_from_app_state
resolve_layout_inference_revisioned
build_plan_for_target_session

// ✅ 精炼核心行为
resolve_layout_inference          // 模块/impl 已限定上下文
resolve_layout_inference_revisioned  // 区分"带版本校验"的变体
from_target_id                    // 构造器 from_ 模式
```

## 冲突风险说明

同名函数在不同调用路径下**不会冲突**：
- 模块函数：`crate::module::resolve_layout_inference()`
- impl 方法：`service.resolve_layout_inference()`

但 impl 方法去后缀后，若同一 impl 块内出现同名方法，需加区分词。

---

## P0: `_for_` / `_with_` 后缀（13 项）

### core

| 文件 | 当前名 | 建议名 | 冲突风险 |
|------|--------|--------|---------|
| geometry/source_geometry.rs | `source_line_visual_bbox_for_caret` | `caret_line_bbox` | 无 |
| typography/matcher.rs | `build_descriptor_request` | `build_descriptor_request` | 无 |
| edit/paragraph_scene.rs | `build_target_scene` | `build_target_scene` | 无 |
| edit/bridge.rs | `build_rich_patch` | `build_rich_patch` | 无 |
| edit/source_runs.rs | `target_paint_runs` | `target_paint_runs` | 无 |
| edit/engine_state.rs | `resolved_marker_text` | `resolved_marker_text` | 无 |
| edit/engine_state.rs | `source_marker_text` | `source_marker_text` | 无 |
| edit/target_resolution.rs | `resolve_region_target` | `resolve_region_target` | 无 |
| edit/replacement_region.rs | `row_suppression_bbox` | `row_suppression_bbox` | 无 |
| edit/replacement_region.rs | `viewport_cull_bbox` | `viewport_cull_bbox` | 无 |
| edit/replacement_region.rs | `cache_invalidation_bbox` | `cache_invalidation_bbox` | 无 |

### ui

| 文件 | 当前名 | 建议名 | 冲突风险 |
|------|--------|--------|---------|
| editor/command.rs | `apply_host_input` | `apply_host_input` | 无 |
| ui_state_store.rs | `record_patch` | `record_patch` | 无 |

### tauri

| 文件 | 当前名 | 建议名 | 冲突风险 |
|------|--------|--------|---------|
| infrastructure/pdf/geometry_service.rs | `resolve_layout_inference_revisioned` | `resolve_layout_inference_revisioned` | 无（impl 方法，与模块函数不冲突） |

---

## P1: 冗余上下文 / 4+ 下划线段（~100 项）

> 函数名重复了模块路径已提供的信息。模块 `editor::format` 下的函数无需再写 `active_editor_` 前缀。

### core

| 文件 | 当前名 | 建议名 |
|------|--------|--------|
| edit/source_suppression.rs | `text_run_spatially_matches_replacement_region` | `text_matches_region` |
| edit/source_suppression.rs | `glyph_run_spatially_matches_replacement_region` | `glyph_matches_region` |
| edit/source_suppression.rs | `text_object_matches_overlay_source_text` | `text_matches_overlay` |
| edit/source_suppression.rs | `glyph_paragraph_matches_overlay_source_text` | `glyph_matches_overlay` |
| edit/engine_state.rs | `restore_list_kind_from_marker_text` | `restore_list_kind` |
| edit/source_identity.rs | `collect_target_source_object_indices_set` | `collect_object_index_set` |
| edit/source_identity.rs | `collect_object_indices_from_runs` | `collect_run_indices` |

### ui — editor/format

| 当前名 | 建议名 | 备注 |
|--------|--------|------|
| `toggle_active_editor_bold` | `toggle_bold` | |
| `toggle_active_editor_italic` | `toggle_italic` | |
| `toggle_active_editor_underline` | `toggle_underline` | |
| `set_active_editor_color` | `set_color` | |
| `set_active_editor_font_family` | `set_font_family` | |
| `set_active_editor_font_size` | `set_font_size` | |
| `step_active_editor_font_size` | `step_font_size` | |
| `set_active_editor_char_spacing` | `set_char_spacing` | |
| `set_active_editor_line_height` | `set_line_height` | |
| `set_active_editor_paragraph_mode` | `set_paragraph_mode` | |
| `set_active_editor_alignment` | `set_alignment` | |
| `set_active_editor_list_kind` | `set_list_kind` | |
| `active_editor_format_state` | `format_state` | |
| `apply_active_editor_format_action` | `apply_format` | |

### ui — editor/mode + session

| 当前名 | 建议名 | 备注 |
|--------|--------|------|
| `read_active_edit_paragraph` | `read_paragraph` | |
| `read_active_editor_target` | `read_target` | |
| `read_active_editor_state` | `read_state` | |
| `is_text_edit_mode_enabled` | `is_edit_enabled` | |
| `set_text_edit_mode_enabled` | `set_edit_enabled` | |
| `set_active_edit_paragraph` | `set_paragraph` | |
| `is_text_edit_enabled` | `is_edit_enabled` | mode vs session 同名→不同 impl |
| `set_text_edit_enabled` | `set_edit_enabled` | 同上 |
| `active_edit_paragraph_id` | `paragraph_id` | |
| `set_active_edit_paragraph` | `set_paragraph` | |
| `active_editor_draft_text` | `draft_text` | |
| `active_editor_has_session_changes` | `has_changes` | |
| `active_editor_caret_index` | `caret_index` | |
| `set_active_editor_caret_index` | `set_caret` | |
| `set_active_editor_selection` | `set_selection` | |
| `clear_active_editor_selection` | `clear_selection` | |
| `sync_active_editor_input` | `sync_input` | |

### ui — editor/controller, host, workflow

| 当前名 | 建议名 | 备注 |
|--------|--------|------|
| `activate_editor_from_client_point` | `activate_from_client` | |
| `move_caret_to_client_point` | `move_caret_to_client` | |
| `open_editor_at_page_point` | `open_at_point` | |
| `build_region_text_patch` | `build_text_patch` | |
| `build_active_editor_patch` | `build_patch` | |
| `find_paragraph_shell_bbox` | `find_shell_bbox` | |
| `read_paragraph_shell_bbox` | `read_shell_bbox` | |
| `resolve_editor_host_snapshot` | `resolve_snapshot` | |
| `resolve_active_editor_diagnostics` | `resolve_diagnostics` | |
| `build_paragraph_interaction_targets` | `build_interaction_targets` | |
| `resolve_paragraph_shell_bbox` | `resolve_shell_bbox` | |
| `build_active_editor_patch` | `build_editor_patch` | 与 build_text_patch 区分 |
| `resolve_active_marker_text` | `resolve_marker_text` | |
| `measure_editor_layout_text_width` | `measure_text_width` | |
| `active_caret_index_at_page_point` | `caret_at_page_point` | |
| `active_caret_index_at_shell_point` | `caret_at_shell_point` | |
| `execute_editor_navigation_key` | `execute_navigation` | |
| `render_active_editor_canvas` | `render_canvas` | |
| `collect_paragraph_render_overlays` | `collect_overlays` | |
| `project_paragraph_interaction_targets` | `project_targets` | |
| `project_active_editor_shell` | `project_shell` | |
| `commit_pending_edit_if_any` | `commit_pending` | |
| `commit_active_editor_text` | `commit_text` | |
| `open_region_editor_tx` | `open_region_tx` | |
| `apply_host_input_tx` | `apply_input_tx` | |
| `commit_editor_silent_tx` | `commit_silent_tx` | |
| `apply_format_action_tx` | `apply_format_tx` | |
| `undo_active_editor_tx` | `undo_tx` | |
| `redo_active_editor_tx` | `redo_tx` | |
| `apply_region_text_replacements_tx` | `apply_replacements_tx` | |

### ui — editor/store, host_mode

| 当前名 | 建议名 | 备注 |
|--------|--------|------|
| `read_active_block_id` | `read_block_id` | |
| `set_active_block_id` | `set_block_id` | |
| `set_state_change_callback` | `set_change_callback` | |
| `transition_to_editing_block` | `transition_editing` | |
| `toggle_text_edit_mode` | `toggle_edit_mode` | |
| `set_text_edit_mode` | `set_edit_mode` | |

### ui — review, zoom, present, render, page

| 当前名 | 建议名 | 备注 |
|--------|--------|------|
| `clear_comment_review_session` | `clear_review_session` | |
| `read_comment_review_session` | `read_review_session` | |
| `set_comment_review_panel_open` | `set_panel_open` | |
| `toggle_comment_review_panel` | `toggle_panel` | |
| `set_comment_review_scope` | `set_scope` | |
| `set_comment_review_query` | `set_query` | |
| `select_comment_review_comment` | `select_comment` | |
| `reset_zoom_preview_host` | `reset_preview` | |
| `clear_zoom_preview_host_state` | `clear_preview_state` | |
| `settle_zoom_preview_at_target` | `settle_at_target` | |
| `set_wheel_render_pending` | `set_pending` | |
| `is_wheel_render_pending` | `is_pending` | |
| `take_ready_committed_frame` | `take_frame` | |
| `read_zoom_session_state` | `read_session_state` | |
| `step_zoom_frame_plan` | `step_frame_plan` | |
| `take_pending_anchor_scroll` | `take_anchor_scroll` | |
| `peek_pending_anchor_scroll` | `peek_anchor_scroll` | |
| `peek_pending_anchor_layout` | `peek_anchor_layout` | |
| `take_pending_anchor_layout` | `take_anchor_layout` | |
| `build_frame_plan_result` | `build_plan_result` | present_store 与 plan_builder 同名→不同调用路径 |
| `touch_frame_cache_entry` | `touch_cache_entry` | |
| `store_frame_cache_entry` | `store_cache_entry` | |
| `schedule_render_frame_request` | `schedule_request` | |
| `read_page_turn_snapshot` | `read_snapshot` | |
| `reset_page_turn_state` | `reset_state` | |
| `is_latest_page_turn` | `is_latest_turn` | |
| `resolve_render_queue_action` | `resolve_queue_action` | |
| `is_render_frame_current` | `is_frame_current` | |
| `queue_render_loop_frame` | `queue_frame` | |
| `advance_render_loop_frame` | `advance_frame` | |
| `reset_render_loop_runtime` | `reset_runtime` | |
| `resolve_render_follow_up_runtime` | `resolve_follow_up` | |
| `schedule_render_follow_up_runtime` | `schedule_follow_up` | |
| `step_progressive_render_offscreen` | `step_offscreen` | |
| `current_paragraph_patch_text` | `patch_text` | |
| `remember_paragraph_replacement_target` | `remember_target` | |
| `accept_all_review_changes` | `accept_all_changes` | |
| `reject_all_review_changes` | `reject_all_changes` | |
| `build_page_region_context` | `build_region_context` | |
| `measure_dom_to_page_scale` | `measure_page_scale` | |
| `init_page_context_from_models` | `init_context` | |
| `update_page_viewport_workflow` | `update_viewport` | |

### tauri

| 当前名 | 建议名 | 备注 |
|--------|--------|------|
| `resolve_vector_page_model_from_app_state` | `resolve_vector_page_model` | impl 方法，无冲突 |
| `resolve_layout_inference_from_app_state` | `resolve_layout_inference` | impl 方法，与 vector_engine 模块函数不冲突 |
| `resolve_glyph_paint_plan_from_app_state` | `resolve_glyph_paint_plan` | impl 方法，无冲突 |
| `resolve_page_display_list_from_app_state` | `resolve_page_display_list` | impl 方法，无冲突 |
| `build_page_region_context_from_vector_model` | `build_region_context` | impl 方法，无冲突 |
