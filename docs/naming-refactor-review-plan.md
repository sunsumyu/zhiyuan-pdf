# Naming Refactor Review Plan

> Status: proposal for review only. No source rename has been applied.
> Source: `docs/method-constraint-audit.md`, `docs/method-inventory.md`, `docs/architecture-principles.md`, `docs/api-contract.md`, and `docs/development-guide.md`.

## Non-Negotiable Naming Rules

- Naming quality is an architecture constraint, not a style preference.
- Names must be short, stable, searchable, and semantically focused.
- Module path should carry domain context; function names should not repeat the whole path.
- Prefixes/suffixes must keep project-wide meaning:
  - `build_*`: create pure value.
  - `find_*`: return optional match.
  - `resolve_*`: derive decision from input and fallback rules.
  - `sync_*`: copy cross-boundary state.
  - `set_*`: mutate one field.
  - `read_*`: query state or IO-backed data.
  - `get_*`: cheap synchronous pure read.
  - `open/close/save/commit/undo/redo`: user-level use-case actions.
- 禁止把测试场景写成方法名长句；场景细节放到测试数据、注释或子模块名里。
- 避免 `workflow`、`runtime`、`host`、`pipeline`、`with_policy`、`with_revision`、`from_app_state`、`manager`、`helper`、`utils`、历史标签、版本标签，除非模块边界确实需要且已文档化。
- 公开 API 重命名必须保留兼容别名，除非已证明符号无人使用。

## 当前命名债

| 类别 | 数量 | 严重级别 | 主要处理方式 |
|---|---:|---|---|
| 长/句子式方法名 | 78 | P0 | 缩短名称，把场景细节移出方法名 |
| 生产代码长/句子式方法名 | 13 | P0 | 原地重命名为聚焦动作 |
| 测试代码长/句子式方法名 | 65 | P0 | 拆测试模块，缩短 case 名 |
| 裸 WASM 推断出的 snake_case 导出 | 57 | P0 | 加 camelCase `js_name`、迁移或删除 |
| 历史/版本标签 | 方法命中 4 处，另有日志/全局命名 | P1 | 中性命名，加临时兼容别名 |
| helper/manager/utils 命名 | 11 | P1/P2 | 移到领域模块或 diagnostics |
| 类型/类名问题 | 0 | 持续监控 | 保持 PascalCase 和短语义名 |

## 阶段计划

| 阶段 | 范围 | 规则 | 验证 |
|---|---|---|---|
| 0 | 固化审查规则 | 新增长命名、裸 WASM snake_case、历史标签必须失败 | `node scripts/generate-method-inventory.mjs` |
| 1 | 长方法名/测试名 | 先缩短，必要时移动测试 | Rust 定向测试 + inventory |
| 2 | 公开 WASM 命名 | 增加稳定 camelCase facade/session 名，保留兼容 wrapper | `npm run wasm:pdf-viewer-ui`、`npm run build` |
| 3 | 历史标签 | 替换 V3/Sovereign 命名和日志，临时保留 alias | build + E2E smoke |
| 4 | helper/manager/utils | 迁移到领域模块或 diagnostics | build + 定向单测 |
| 5 | typed command boundary | raw invoke 字符串改 typed wrapper | inventory raw 调用数下降 |
| 6 | 打包应用回归 | release exe 验证文本/扫描/vector PDF | packaged smoke checklist |

## P0 重命名对照：长/句子式方法名

### 生产方法

| 文件 | 行 | 重命名前 | 重命名后 (Rust 极简风) | 动作/理由 |
|---|---:|---|---|---|
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 309 | `resolve_draft_template_run_with_policy` | `resolve_template` | 模块已在 draft_layout， policy 是参数，run 由返回类型体现 |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 395 | `select_insert_style_run_with_policy` | `select_style` | 同上，基于签名去冗余 |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 487 | `build_style_runs_for_draft_text_with_policy` | `build_styles` | 复数已表达 runs，纯构造用 build |
| `crates/pdf-viewer-core/src/edit/replacement_region.rs` | 130 | `target_with_baseline_down_body_run` | `find_target` | 若返回 Option 用 find，细节用注释解释 |
| `crates/pdf-viewer-core/src/edit/source_runs.rs` | 408 | `layout_run_from_styled_with_owner` | `build_layout` | 参数带有 owner，类型是 styled，无需写在方法名 |
| `crates/pdf-viewer-core/src/render/path_suppression.rs` | 8 | `decorative_object_should_be_suppressed_by_overlay` | `should_suppress` | 若为 DecorativeObject 的方法，极简即可 |
| `crates/pdf-viewer-core/src/text/caret_geometry.rs` | 73 | `caret_index_at_page_point_with_plan` | `resolve_index` | 结合参数 point 和 plan 推导，使用 resolve |
| `crates/pdf-viewer-core/src/text/glyph_layout.rs` | 600 | `should_insert_visual_gap_space_with_context` | `needs_gap` | 简短的谓词 |
| `src-tauri/src/infrastructure/pdf/geometry_service.rs` | 40 | `resolve_glyph_paint_plan_with_revision` | `resolve_plan` | 在 geometry_service 中推导 plan |
| `src-tauri/src/infrastructure/pdf/page_model_service.rs` | 87 | `resolve_vector_page_model_from_app_state_with_revision` | `resolve_model` | 剔除所有参数来源描述 |
| `src-tauri/src/infrastructure/pdf/page_model_service.rs` | 113 | `resolve_vector_page_model_with_revision` | `resolve_model` | 保持统一 |
| `src-tauri/src/infrastructure/pdf/vector_engine.rs` | 8 | `resolve_page_display_list_with_doc` | `resolve_display_list` | doc 显然是参数 |
| `src-tauri/src/infrastructure/pdf/vector_engine.rs` | 35 | `resolve_vector_page_model_with_doc` | `resolve_model` | - |
| `src-tauri/src/interfaces/pdf/ipc_converters.rs` | 164 | `execute_pdf_commands_with_app_state` | `execute_commands` | - |

### 测试方法和测试辅助函数

> 规则：测试名应当是短促的断言，场景前置条件（When/Given）通过 `mod` 层级或内部注释解决。

| 文件 | 行 | 重命名前 | 重命名后 | 动作 |
|---|---:|---|---|---|
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 798 | `test_layout_run_with_char_gaps` | `layout_with_gaps` | Helper 函数极简 |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 846 | `mixed_text_runs_with_pdf_split_words` | `mixed_runs` | Fixture 数据命名 |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 867 | `source_text_stays_canonical_when_text_plan_has_synthetic_gap_slots` | `preserves_canonical_source` | 断言核心行为 |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 884 | `source_text_restores_pdf_visual_word_gaps_without_intra_word_noise` | `restores_visual_gaps` | 断言核心行为 |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 907 | `source_text_restores_visual_spaces_inside_single_pdf_run` | `restores_run_spaces` | - |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 1013 | `editor_prefers_vector_source_over_paint_projection_text` | `prefers_vector_source` | - |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 1055 | `patched_display_runs_keep_original_vector_source_for_overlay_target` | `keeps_overlay_source` | - |
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 1112 | `vector_geometry_source_is_used_when_object_ids_are_missing` | `uses_vector_geometry` | - |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 1126 | `source_layout_sanitizes_partial_underlines_for_editor_canvas` | `sanitizes_underlines` | - |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 1162 | `draft_layout_renders_compact_pdf_text_when_runs_have_no_spaces` | `renders_compact_runs` | - |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 1210 | `changed_active_draft_layout_preserves_source_geometry_for_unchanged_parts` | `preserves_active_geometry` | - |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 1231 | `changed_persisted_overlay_layout_preserves_source_geometry_for_unchanged_parts` | `preserves_overlay_geometry` | - |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 1250 | `edited_draft_preserves_origins_when_runs_lack_synthetic_spaces` | `preserves_origins` | - |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 1302 | `active_draft_layout_keeps_source_geometry_for_unchanged_split_words` | `keeps_split_word_geometry` | - |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 1345 | `runs_to_source_index_map_accounts_for_synthetic_spaces` | `maps_synthetic_spaces` | - |
| `crates/pdf-viewer-core/src/edit/draft_layout.rs` | 1360 | `runs_to_source_index_map_clamps_when_runs_has_chars_missing_in_source` | `clamps_missing_source_chars` | - |
| `crates/pdf-viewer-core/src/edit/replacement_region.rs` | 184 | `path_suppression_is_tighter_than_source_replacement` | `tightens_path_suppression` | - |
| `crates/pdf-viewer-core/src/edit/replacement_region.rs` | 201 | `viewport_cull_region_covers_whole_row_for_tiled_path_suppression` | `covers_tiled_row` | - |
| `crates/pdf-viewer-core/src/edit/replacement_region.rs` | 219 | `replacement_region_uses_baseline_font_source_geometry` | `uses_baseline_geometry` | - |
| `crates/pdf-viewer-core/src/edit/source_text.rs` | 220 | `compact_pdf_text_does_not_split_technical_names_without_geometry` | `keeps_technical_names` | - |
| `crates/pdf-viewer-core/src/geometry/source_geometry.rs` | 186 | `visual_bbox_uses_baseline_font_geometry_when_stored_bbox_is_baseline_down` | `uses_baseline_bbox` | - |
| `crates/pdf-viewer-core/src/geometry/source_geometry.rs` | 198 | `caret_line_bbox_uses_same_source_visual_geometry` | `uses_source_geometry` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 916 | `active_editor_suppresses_zero_height_stroked_row_path` | `suppresses_zero_height_path` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 951 | `active_editor_keeps_section_divider_path_outside_text_row` | `keeps_section_divider` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 986 | `active_editor_keeps_nearby_divider_below_text_row` | `keeps_nearby_divider` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1018 | `active_editor_suppresses_row_path_touching_text_descenders` | `suppresses_descender_path` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1050 | `active_editor_suppresses_text_object_when_runs_have_no_object_id` | `suppresses_text_without_ids` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1083 | `active_editor_spatially_suppresses_text_run_when_source_ids_are_missing` | `spatially_suppresses_text` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1115 | `clean_active_editor_keeps_spatially_matching_text_visible` | `keeps_matching_text` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1151 | `clean_active_editor_keeps_source_text_object_visible` | `keeps_source_text` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1182 | `clean_active_editor_suppresses_row_path_without_hiding_text` | `suppresses_path_only` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1231 | `active_editor_spatially_suppresses_glyph_run_when_source_ids_are_missing` | `spatially_suppresses_glyphs` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1265 | `clean_active_editor_keeps_spatially_matching_glyph_run_visible` | `keeps_matching_glyphs` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1297 | `persisted_overlay_spatially_suppresses_glyph_run_when_source_ids_are_missing` | `overlay_suppresses_glyphs` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1336 | `persisted_overlay_renders_after_later_page_paths` | `overlay_renders_last` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1363 | `persisted_overlay_suppresses_row_path_after_commit` | `overlay_suppresses_path` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1393 | `replacement_region_keeps_right_tile_row_path_suppressed` | `keeps_right_tile_suppressed` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1431 | `list_item_marker_run_is_not_suppressed_when_body_is_replaced` | `keeps_list_marker` | - |
| `crates/pdf-viewer-core/src/render/effective_page_plan.rs` | 1549 | `suppression_works_when_z_index_differs_from_array_position` | `handles_z_index_order` | - |
| `crates/pdf-viewer-core/src/render/path_suppression.rs` | 213 | `suppresses_thin_horizontal_image_decoration_on_source_row` | `suppresses_thin_decoration` | - |
| `crates/pdf-viewer-core/src/render/path_suppression.rs` | 228 | `keeps_normal_image_near_source_row` | `keeps_normal_image` | - |
| `crates/pdf-viewer-core/src/render/zoom_interaction.rs` | 519 | `anchor_layout_preserves_cursor_point_when_page_is_centered` | `preserves_cursor_anchor` | - |
| `crates/pdf-viewer-core/src/text/index_convert.rs` | 22 | `converts_between_dom_utf16_offsets_and_rust_char_indexes` | `converts_utf16_indexes` | - |
| `crates/pdf-viewer-core/src/text/style_mapper.rs` | 422 | `canonical_gap_reconstruction_is_not_a_style_change` | `ignores_canonical_gaps` | - |
| `crates/pdf-viewer-core/src/typography/matcher.rs` | 521 | `embedded_font_with_cmap_marks_embedded_attempt_viable` | `accepts_embedded_cmap` | - |
| `crates/pdf-viewer-ui/src/editor/overlay/paragraph_overlay.rs` | 318 | `persisted_patch_yields_overlay_with_new_text_after_commit` | `patch_yields_overlay` | - |
| `crates/pdf-viewer-ui/src/editor/overlay/paragraph_overlay.rs` | 390 | `production_commit_flow_preserves_edit_after_exit` | `commit_preserves_edit` | - |
| `crates/pdf-viewer-ui/src/editor/overlay/paragraph_overlay.rs` | 434 | `persisted_patch_skipped_when_page_index_mismatches` | `skips_mismatched_page` | - |
| `crates/pdf-viewer-ui/src/editor/session/session.rs` | 104 | `text_edit_mode_starts_disabled_until_toolbar_enables_it` | `starts_disabled` | - |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 589 | `page_turn_tracks_latest_intent_and_rejects_stale_visible_page` | `rejects_stale_page` | - |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 616 | `prefetch_decision_prefers_turn_direction_with_preview_runway_and_nearby_vector` | `prefers_turn_direction` | - |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 642 | `asset_admission_rejects_stale_current_and_out_of_window_prefetch` | `rejects_stale_assets` | - |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 678 | `page_turn_rejects_without_document_or_out_of_range_target` | `rejects_invalid_turn` | - |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 695 | `fast_flip_mode_activates_when_turns_are_rapid` | `activates_fast_flip` | - |
| `crates/pdf-viewer-ui/src/presentation/page_turn.rs` | 716 | `fast_flip_pauses_vector_prefetch_and_reduces_reverse_preview` | `throttles_fast_flip` | - |
| `src-tauri/src/application/pdf/page_asset.rs` | 34 | `same_asset_key_waits_for_existing_inflight_work` | `waits_for_inflight_key` | - |
| `src-tauri/src/application/pdf/page_asset.rs` | 98 | `different_document_revision_uses_distinct_inflight_lock` | `separates_revision_locks` | - |
| `src-tauri/src/application/pdf/page_asset.rs` | 130 | `invalidating_page_cache_removes_document_asset_locks` | `clears_asset_locks` | - |
| `src-tauri/src/application/pdf/page_asset.rs` | 199 | `preview_prefetch_uses_wider_runway_than_vector_assets` | `widens_preview_runway` | - |
| `src-tauri/src/error.rs` | 121 | `page_out_of_range_renders_indices` | `renders_page_indices` | - |
| `src-tauri/src/infrastructure/pdf/font/embedded_program.rs` | 64 | `sorts_unsorted_sfnt_records_without_touching_payload` | `sorts_sfnt_records` | - |
| `src-tauri/src/infrastructure/pdf/font/matching.rs` | 222 | `matcher_resolves_native_text_with_descriptor_cache` | `uses_descriptor_cache` | - |
| `src-tauri/src/infrastructure/pdf/page_intermediate_service.rs` | 307 | `vector_and_layout_derive_from_seeded_display_list_cache` | `uses_seeded_display_list` | - |
| `src-tauri/src/infrastructure/pdf/page_intermediate_service.rs` | 384 | `annotation_and_search_can_use_display_list_derived_page_model` | `shares_derived_page_model` | - |

## P0 重命名对照：裸 WASM 推断 snake_case 导出

> 原则：面向 TS 的公开接口直接用 camelCase，如果需要提供回退兼容，则使用 `#[wasm_bindgen(js_name = "xxx")]` 以明确暴露名称，内部 Rust 函数名依然保持简短。剔除 `pipeline`, `compat` 等泄露内部架构的废话。

| 文件 | 重命名前 | 重命名后 (优雅导出) | 动作 |
|---|---|---|---|
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `undo_document_pipeline` | `undo` | 绑定到 DocumentSession 后就是 `session.undo()` |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `redo_document_pipeline` | `redo` | 同上 |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `open_document_pipeline` | `open_document` | `openDocument` 足够清晰 |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `pick_document_pipeline` | `pick_document` | `pickDocument` |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `rotate_document_pipeline` | `rotate_document` | `rotateDocument` |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `close_document_pipeline` | `close_document` | `closeDocument` |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `read_viewer_session` | `read_session` | 已经知道是 viewer，暴露为 `readSession` |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `get_viewer_session` | 彻底删除 | 使用 read_session 替代 |
| `crates/pdf-viewer-ui/src/document/free_api.rs` | `set_viewer_document` | `set_document` | `setDocument` |
| `crates/pdf-viewer-ui/src/render/canvas.rs` | `render_run_standalone` | `render_standalone` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_frame_plan` | `resolve_frame` | plan 是细节，返回类型自会说明 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `take_frame_plan` | `take_frame` | 同上 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `schedule_render_frame` | `schedule_frame` | render_api 模块下，无需重复 render |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `commit_render_result` | `commit_frame` | 统一以 frame 为核心名词 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `settle_render_frame` | `settle_frame` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `abort_render_frame` | `abort_frame` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `is_render_frame_current` | `is_frame_current` | JS 导出 `isFrameCurrent` 非常优雅 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `schedule_render_follow_up` | `schedule_follow_up` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `queue_render_loop_frame` | `queue_loop_frame` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `advance_render_loop_frame` | `advance_loop_frame` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `step_zoom_frame_plan` | `step_zoom_frame` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_viewport_refresh` | `resolve_viewport` | 动作足够清晰 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_host_scroll_refresh` | `resolve_scroll` | host/refresh 是废话 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `clear_zoom_preview_host_state` | `clear_zoom_preview` | 去掉 host/state |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_wheel_render_decision` | `resolve_wheel_decision` | render 在模块上下文 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_preview_tick_decision` | `resolve_preview_tick` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `handle_wheel_zoom_host` | `handle_wheel_zoom` | 丢弃 host |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `step_preview_host` | `step_preview` | 丢弃 host |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_render_execution_plan` | `resolve_execution` | execution_plan 变成 execution 或 plan 皆可 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_layer_execution_plan` | `resolve_layer_execution` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_layer_present_decision` | `resolve_layer_present` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `update_page_viewport` | `update_viewport` | 默认指代 page |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `render_page` | `render_page` | 保持，足够好 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `render_page_offscreen` | `render_offscreen` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `start_progressive_render` | `start_progressive` | render 上下文已知 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `step_progressive_render` | `step_progressive` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `step_progressive_render_offscreen` | `step_progressive_offscreen` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `cancel_progressive_render` | `cancel_progressive` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `resolve_progressive_render_policy` | `resolve_progressive_policy` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `touch_frame_cache_entry` | `touch_frame_cache` | entry 多余 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `store_frame_cache_entry` | `store_frame_cache` | entry 多余 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `reset_frame_cache` | `reset_frame_cache` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `set_wheel_render_pending` | `set_wheel_pending` | - |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `get_wheel_render_pending` | `is_wheel_pending` | 布尔值读取用 is_ |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `queue_committed_frame` | `queue_frame` | 既然能 queue 肯定是被 commit 过了 |
| `crates/pdf-viewer-ui/src/render/free_api.rs` | `take_ready_committed_frame` | `take_ready_frame` | 减少形容词堆砌 |
| `crates/pdf-viewer-ui/src/viewer/free_api.rs` | `init_page_context` | `init_page` | context 常为废话 |
| `crates/pdf-viewer-ui/src/viewer/free_api.rs` | `set_current_page` | `set_page` | set 必定指代 current |
| `crates/pdf-viewer-ui/src/viewer/free_api.rs` | `dump_editor_debug_trace` | `dump_debug_trace` | editor 在 viewer 中不言而喻 |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | `resolve_wheel_zoom` | `resolve_wheel` | zoom_api 模块自带 zoom 含义 |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | `reset_zoom_state` | `reset` | - |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | `read_zoom_state` | `read_state` | - |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | `get_zoom_state` | 删除 | 统一使用 read_state |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | `set_target_zoom` | `set_target` | - |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | `mark_rendered_zoom` | `mark_rendered` | - |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | `clear_pending_anchor` | `clear_anchor` | 既然 clear，必定是 pending |
| `crates/pdf-viewer-ui/src/zoom/free_api.rs` | `apply_zoom_selection` | `apply_selection` | - |

## P1 重命名对照：历史标签和版本标签

| 文件 | 重命名前 | 重命名后 | 动作 |
|---|---|---|---|
| `src/bridge/shared/wasm_loader.ts` | `targetInvokeV3` | `invokeTauriCommand` | 重命名内部函数；临时保留 `targetInvokeV3` window alias |
| `src/bridge/shared/wasm_loader.ts` | `__targetInvokeV3` | `__pdfViewerInvoke` | 增加中性 alias；E2E 更新后删除旧 alias |
| `src/bridge/shared/wasm_loader.ts` | `host.wasmv3` | `host.pdfViewerWasm` | 重命名全局调试挂载 |
| `src/bridge/shared/wasm_loader.ts` | `[V3-Sovereign]` | `[PDF-WASM]` | 重命名日志 |
| `src/bridge/shared/wasm_loader.ts` | `[Sovereignty]` | `[PDF-WASM]` | 重命名注释/日志 |
| `src/bridge/render/render_flow.ts` | `targetInvokeV3` 依赖字段 | `invokeTauriCommand` | 重命名依赖字段 |
| `src/main.ts` | `Initializing Sovereignty PDF Viewer...` | `Initializing PDF Viewer...` | 重命名日志 |
| `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs` | `v3_y` | `page_y_down` | 重命名参数 |
| `crates/pdf-viewer-core/src/geometry/reflow_engine.rs` | `v3_model` | `layout_model` | 重命名参数 |
| `crates/pdf-viewer-core/src/document/page_region_context.rs` | `backend-sovereign` | `backend-owned-region` | 重命名语义标签 |

## P1/P2 重命名对照：helper / manager / utils

| 文件 | 重命名前 | 重命名后 | 动作 |
|---|---|---|---|
| `crates/pdf-viewer-core/src/utils/debug.rs` | `utils::debug::truncate_debug_text` | `diagnostics::text::truncate_debug_text` | 移动模块；保留 re-export 兼容 |
| `crates/pdf-viewer-core/src/utils/sanitize.rs` | `utils::sanitize::sanitize_positive` | `geometry::sanitize::sanitize_positive` | 移动模块；保留 re-export |
| `crates/pdf-viewer-core/src/utils/sanitize.rs` | `utils::sanitize::sanitize_non_negative` | `geometry::sanitize::sanitize_non_negative` | 移动模块；保留 re-export |
| `crates/pdf-viewer-ui/src/utils/chain_trace.rs` | `utils::chain_trace::set_chain_trace_enabled` | `diagnostics::chain_trace::set_chain_trace_enabled` | 移动模块 |
| `crates/pdf-viewer-ui/src/utils/chain_trace.rs` | `utils::chain_trace::is_chain_trace_enabled` | `diagnostics::chain_trace::is_chain_trace_enabled` | 移动模块 |
| `crates/pdf-viewer-ui/src/utils/chain_trace.rs` | `utils::chain_trace::trace_step` | `diagnostics::chain_trace::trace_step` | 移动模块 |
| `src/bridge/comment/pdf_comment_wasm_bridge.ts` | `getCommentManager` | `getCommentSessionApi` | 重命名 accessor；如 WASM class 是公开 API 则保留 |
| `tests/e2e/helpers/app.js` | `helpers/app.js` | 不改名 | 测试支撑命名可接受 |
| `tests/e2e/helpers/app.js` | `waitForApp` | 不改名 | 测试 helper 可接受 |
| `tests/e2e/helpers/app.js` | `loadFixturePdf` | 不改名 | 测试 helper 可接受 |
| `utils/ai-settings.ts` | `utils/ai-settings.ts` | `src/bridge/ai/ai_settings.ts` | 如果 AI 设置继续属于 app bridge，后续移动 |
| `utils/ai-settings.ts` | `loadAiSettings` | `readAiSettings` | 对齐 read/get 约定 |
| `utils/ai-settings.ts` | `saveAiSettings` | `saveAiSettings` | 保留，动作动词正确 |

## typed boundary 命名对照

| 当前写法 | 目标写法 | 动作 |
|---|---|---|
| `targetInvokeV3('read_preview', args)` | `pdfCommands.readPreview(args)` | 替换 raw string 调用 |
| `targetInvokeV3('read_page_asset_bundle', args)` | `pdfCommands.readPageAssetBundle(args)` | 替换 raw string 调用 |
| `targetInvokeV3('find_in_page', args)` | `pdfCommands.findInPage(args)` | 替换 raw string 调用 |
| `targetInvokeV3('find_in_document', args)` | `pdfCommands.findInDocument(args)` | 替换 raw string 调用 |
| `targetInvokeV3('apply_highlight', args)` | `pdfCommands.applyHighlight(args)` | 替换 raw string 调用 |
| `targetInvokeV3('delete_annotation', args)` | `pdfCommands.deleteAnnotation(args)` | 替换 raw string 调用 |
| `targetInvokeV3('undo', args)` | `pdfCommands.undo(args)` | 替换 raw string 调用 |
| `targetInvokeV3('redo', args)` | `pdfCommands.redo(args)` | 替换 raw string 调用 |

## 类型和类名

当前审查结果：

- 提取类型/类符号：730。
- PascalCase 违规：0。
- 长/句子式类型或类名：0。

本阶段不提出类型/类名重命名，但保留审查规则，避免后续新增坏命名。

## 兼容规则

- 私有 Rust 测试和 helper 可以直接重命名。
- 公开 Rust/WASM 导出必须保留兼容 wrapper 或 `#[deprecated]` 说明。
- TS 公开 API 重命名必须补 JSDoc `@deprecated`。
- window 全局变量至少保留一个 E2E/release 周期的兼容 alias。
- 每一批源码重命名后都必须重新生成 inventory，并运行受影响测试。

## 审核清单

- [ ] 审核或修改每一个“重命名后”的名称
- [ ] 确认测试块是迁出生产模块，还是只缩短命名。
- [ ] 确认公开 WASM API 的兼容周期。
- [ ] 确认 raw invoke wrapper 命名是否采用 `pdfCommands.*`。
- [ ] 审核通过后，只执行 Phase 1，再重新生成审查文档。
