# 方法与类型清单

> 由 `node scripts/generate-method-inventory.mjs` 生成。
> 范围：`.rs`、`.ts`、`.tsx`、`.js`、`.mjs`；排除 `node_modules/`、`target/`、`dist/`、生成的 `pkg/`、归档/origin 文档。
> 这是静态提取，宏生成方法和运行时动态创建函数不包含在内。

- 扫描源码文件：362
- 方法/函数数量：2894
- 类型/类数量：730
- Rust: 1796
- TS/JS: 1098

## 方法类型统计

| 类型 | 数量 |
|---|---:|
| rust_fn | 1325 |
| object_arrow_method | 510 |
| function | 504 |
| rust_method | 471 |
| class_method | 70 |
| arrow_fn | 14 |

## 类型/类统计

| 类型 | 数量 |
|---|---:|
| rust_struct | 413 |
| ts_type | 224 |
| rust_enum | 51 |
| ts_interface | 27 |
| rust_type | 8 |
| rust_trait | 4 |
| ts_class | 3 |

## 全部方法

### `crates/pdf-viewer-core/src/document/list_item_region_builder.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | rust_fn |  | `chars_count` |  |  |
| 13 | rust_fn |  | `split_runs_by_body_start` |  |  |
| 95 | rust_fn |  | `resolve_body_left` |  |  |
| 120 | rust_fn |  | `build_list_item_region` | yes |  |

### `crates/pdf-viewer-core/src/document/page_region_context.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 23 | rust_fn |  | `read_object_display_text` |  |  |
| 31 | rust_fn |  | `chars_count` |  |  |
| 35 | rust_fn |  | `resolve_run_visible_glyph_width` |  |  |
| 45 | rust_fn |  | `build_style_source` |  |  |
| 71 | rust_fn |  | `build_style_runs_from_text_object` |  |  |
| 234 | rust_fn |  | `build_paragraph_line_from_text_object` |  |  |
| 302 | rust_fn |  | `infer_scene_hint` |  |  |
| 324 | rust_fn |  | `is_standalone_paragraph_candidate` |  |  |
| 338 | rust_fn |  | `should_merge_paragraph_objects` |  |  |
| 355 | rust_fn |  | `build_paragraph_region_from_objects` |  |  |
| 468 | rust_fn |  | `split_key_value_text` |  |  |
| 479 | rust_fn |  | `build_page_region_context` | yes |  |

### `crates/pdf-viewer-core/src/document/page_region_models.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 123 | rust_fn |  | `default_scale_x_persistence` |  |  |

### `crates/pdf-viewer-core/src/edit/active_target.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 34 | rust_method | Default for ActiveEditorTarget | `default` |  |  |
| 61 | rust_method | ActiveEditorTarget | `source_body_text` | yes |  |
| 65 | rust_method | ActiveEditorTarget | `initial_body_caret_index` | yes |  |

### `crates/pdf-viewer-core/src/edit/bridge.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 32 | rust_fn |  | `collect_paragraph_interaction_targets` | yes |  |
| 64 | rust_fn |  | `build_paragraph_patch` | yes |  |
| 73 | rust_fn |  | `build_rich_patch` | yes |  |
| 178 | rust_fn |  | `active_editor_target_from_scene` |  |  |
| 204 | rust_fn |  | `build_active_editor_target` | yes |  |
| 247 | rust_fn |  | `build_paragraph_render_target` | yes |  |
| 288 | rust_fn |  | `resolve_paragraph_shell_bbox` | yes |  |
| 306 | rust_fn |  | `resolve_target_indices_from_runs` |  |  |

### `crates/pdf-viewer-core/src/edit/debug_trace.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 36 | rust_fn |  | `editor_debug_field` | yes |  |
| 43 | rust_fn |  | `record_editor_debug_event` | yes |  |
| 61 | rust_fn |  | `resolve_editor_debug_trace` | yes |  |

### `crates/pdf-viewer-core/src/edit/document_edit_ops.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_fn |  | `insert_text` | yes |  |
| 39 | rust_fn |  | `delete_backward` | yes |  |
| 87 | rust_fn |  | `delete_forward` | yes |  |

### `crates/pdf-viewer-core/src/edit/document_plan.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 57 | rust_method | Default for EditorDocumentPlan | `default` |  |  |
| 89 | rust_method | EditorDocumentPlan | `source_body_text` | yes |  |
| 93 | rust_method | EditorDocumentPlan | `body_char_count` | yes |  |
| 100 | rust_fn |  | `resolve_shell_bbox` |  |  |
| 115 | rust_fn |  | `build_editor_document_plan_from_session` | yes |  |
| 142 | rust_fn |  | `split_run_at_char_index` |  |  |
| 186 | rust_fn |  | `bbox_from_runs` |  |  |
| 201 | rust_fn |  | `normalize_draft_template_run` |  |  |
| 213 | rust_fn |  | `select_draft_template_run` |  |  |
| 256 | rust_fn |  | `split_editor_session` |  |  |
| 359 | rust_fn |  | `same_document_line` |  |  |
| 365 | rust_fn |  | `detect_symbolic_font_marker` |  |  |
| 406 | rust_fn |  | `synthesize_marker_from_paragraph` |  |  |
| 461 | rust_fn |  | `normalize_template_run_for_draft` |  |  |
| 473 | rust_fn |  | `build_body_line_plans` |  |  |
| 515 | rust_fn |  | `build_editor_document_plan` | yes |  |
| 523 | rust_fn |  | `collect_editor_document_target_plans` | yes |  |
| 535 | rust_fn |  | `build_editor_document_plan_for_target` | yes |  |
| 549 | rust_fn |  | `build_plan_for_target_session` |  |  |
| 761 | rust_fn |  | `test_style` |  |  |
| 774 | rust_fn |  | `test_bbox` |  |  |
| 783 | rust_fn |  | `test_layout_run` |  |  |
| 798 | rust_fn |  | `layout_with_gaps` |  |  |
| 816 | rust_fn |  | `session_from_runs` |  |  |
| 846 | rust_fn |  | `mixed_runs` |  |  |
| 867 | rust_fn |  | `preserves_canonical_source` |  |  |
| 884 | rust_fn |  | `restores_visual_gaps` |  |  |
| 907 | rust_fn |  | `restores_run_spaces` |  |  |
| 952 | rust_fn |  | `test_resolved_font` |  |  |
| 970 | rust_fn |  | `test_paint_run` |  |  |
| 994 | rust_fn |  | `test_styled_run` |  |  |
| 1013 | rust_fn |  | `prefers_vector_source` |  |  |
| 1055 | rust_fn |  | `keeps_overlay_source` |  |  |
| 1112 | rust_fn |  | `uses_vector_geometry` |  |  |

### `crates/pdf-viewer-core/src/edit/document_runtime.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 18 | rust_method | EditorResolvedDocumentState | `char_count` | yes |  |
| 23 | rust_fn |  | `chars_to_text` | yes |  |
| 27 | rust_fn |  | `resolve_document_state` | yes |  |

### `crates/pdf-viewer-core/src/edit/draft_layout.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 35 | rust_fn |  | `summarize_render_plan_lines` |  |  |
| 74 | rust_fn |  | `shell_width` |  |  |
| 78 | rust_fn |  | `paragraph_preserve_underline` |  |  |
| 82 | rust_fn |  | `body_runs_text` |  |  |
| 92 | rust_fn |  | `body_runs_match_source_text` |  |  |
| 107 | rust_fn |  | `build_source_to_runs_index_map` |  |  |
| 129 | rust_fn |  | `build_runs_to_source_index_map` |  |  |
| 156 | rust_fn |  | `remap_caret_indices_to_draft_space` |  |  |
| 246 | rust_fn |  | `same_existing_layout_line` |  |  |
| 252 | rust_fn |  | `build_source_layout` |  |  |
| 302 | rust_fn |  | `resolve_draft_template_run` |  |  |
| 309 | rust_fn |  | `resolve_template` |  |  |
| 339 | rust_fn |  | `sanitize_draft_run_style` |  |  |
| 348 | rust_fn |  | `normalize_style_run` |  |  |
| 361 | rust_fn |  | `normalize_preserved_geometry_run` |  |  |
| 372 | rust_fn |  | `find_source_run_index_at_char` |  |  |
| 389 | rust_fn |  | `is_good_body_style` |  |  |
| 395 | rust_fn |  | `select_style` |  |  |
| 435 | rust_fn |  | `slice_runs_by_char_range` |  |  |
| 487 | rust_fn |  | `build_styles` |  |  |
| 642 | rust_fn |  | `source_baseline_y` |  |  |
| 658 | rust_fn |  | `build_draft_paragraph` |  |  |
| 671 | rust_fn |  | `build_draft_paragraph_with_policy` |  |  |
| 744 | rust_fn |  | `align_layout_baseline` |  |  |
| 757 | rust_fn |  | `build_empty_render_plan` |  |  |
| 786 | rust_fn |  | `build_editor_draft_caret_plan_from_layout` |  |  |
| 887 | rust_fn |  | `build_draft_render_plan` | yes |  |
| 973 | rust_fn |  | `build_persisted_overlay_render_plan` | yes |  |
| 1040 | rust_fn |  | `test_run` |  |  |
| 1069 | rust_fn |  | `test_run_with_origins` |  |  |
| 1083 | rust_fn |  | `changed_text_document_plan` |  |  |
| 1109 | rust_fn |  | `rendered_text` |  |  |
| 1117 | rust_fn |  | `plan_has_source_char_origins` |  |  |
| 1126 | rust_fn |  | `sanitizes_underlines` |  |  |
| 1162 | rust_fn |  | `renders_compact_runs` |  |  |
| 1210 | rust_fn |  | `preserves_active_geometry` |  |  |
| 1231 | rust_fn |  | `preserves_overlay_geometry` |  |  |
| 1250 | rust_fn |  | `preserves_origins` |  |  |
| 1302 | rust_fn |  | `keeps_split_word_geometry` |  |  |
| 1345 | rust_fn |  | `maps_synthetic_spaces` |  |  |
| 1360 | rust_fn |  | `clamps_missing_source_chars` |  |  |

### `crates/pdf-viewer-core/src/edit/edit_target.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 20 | rust_fn |  | `make_edit_segment_target_id` | yes |  |
| 24 | rust_fn |  | `edit_target_base_paragraph_id` | yes |  |
| 31 | rust_fn |  | `edit_target_segment_key` | yes |  |
| 37 | rust_fn |  | `collect_edit_targets_from_session` | yes |  |
| 62 | rust_fn |  | `resolve_edit_target_from_session` | yes |  |
| 124 | rust_fn |  | `build_visual_segments` |  |  |
| 192 | rust_fn |  | `group_runs_by_visual_line` |  |  |
| 219 | rust_fn |  | `line_is_list_like` |  |  |
| 243 | rust_fn |  | `visual_segment_from_indices` |  |  |
| 252 | rust_fn |  | `build_segment_target` |  |  |
| 290 | rust_fn |  | `whole_session_target` |  |  |
| 311 | rust_fn |  | `normalize_paragraph_to_bbox` |  |  |
| 318 | rust_fn |  | `bbox_from_layout_runs` |  |  |
| 333 | rust_fn |  | `line_sort_key` |  |  |
| 338 | rust_fn |  | `same_visual_line` |  |  |
| 343 | rust_fn |  | `segment_break_gap` |  |  |
| 355 | rust_fn |  | `target_hit_score` |  |  |
| 380 | rust_fn |  | `test_run` |  |  |
| 411 | rust_fn |  | `segmented_targets_use_baseline_font_visual_bbox` |  |  |

### `crates/pdf-viewer-core/src/edit/editor_types.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 19 | rust_method | SessionState | `as_str` | yes |  |

### `crates/pdf-viewer-core/src/edit/engine_state.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 32 | rust_fn |  | `alignment_label` |  |  |
| 41 | rust_fn |  | `list_kind_label` |  |  |
| 51 | rust_fn |  | `derive_next_marker_text` |  |  |
| 88 | rust_method | LiveEditorParagraphState | `new` | yes |  |
| 119 | rust_method | LiveEditorParagraphState | `paragraph_id` | yes |  |
| 123 | rust_method | LiveEditorParagraphState | `text_char_count` | yes |  |
| 127 | rust_method | LiveEditorParagraphState | `normalize_caret` | yes |  |
| 131 | rust_method | LiveEditorParagraphState | `set_caret_index` | yes |  |
| 140 | rust_method | LiveEditorParagraphState | `set_draft_text` | yes |  |
| 153 | rust_method | LiveEditorParagraphState | `current_text` | yes |  |
| 157 | rust_method | LiveEditorParagraphState | `draft_text` | yes |  |
| 161 | rust_method | LiveEditorParagraphState | `source_text` | yes |  |
| 165 | rust_method | LiveEditorParagraphState | `normalized_caret_index` | yes |  |
| 169 | rust_method | LiveEditorParagraphState | `toggle_bold_all` | yes |  |
| 178 | rust_method | LiveEditorParagraphState | `toggle_italic_all` | yes |  |
| 187 | rust_method | LiveEditorParagraphState | `toggle_underline_all` | yes |  |
| 196 | rust_method | LiveEditorParagraphState | `is_bold_active` | yes |  |
| 200 | rust_method | LiveEditorParagraphState | `is_italic_active` | yes |  |
| 204 | rust_method | LiveEditorParagraphState | `is_underline_active` | yes |  |
| 208 | rust_method | LiveEditorParagraphState | `active_color` | yes |  |
| 212 | rust_method | LiveEditorParagraphState | `active_font_family` | yes |  |
| 216 | rust_method | LiveEditorParagraphState | `active_font_size` | yes |  |
| 220 | rust_method | LiveEditorParagraphState | `active_char_spacing` | yes |  |
| 224 | rust_method | LiveEditorParagraphState | `active_line_height` | yes |  |
| 234 | rust_method | LiveEditorParagraphState | `source_line_height` | yes |  |
| 238 | rust_method | LiveEditorParagraphState | `active_paragraph_mode_label` | yes |  |
| 251 | rust_method | LiveEditorParagraphState | `active_alignment` | yes |  |
| 255 | rust_method | LiveEditorParagraphState | `active_alignment_label` | yes |  |
| 259 | rust_method | LiveEditorParagraphState | `source_alignment` | yes |  |
| 263 | rust_method | LiveEditorParagraphState | `source_list_kind` | yes |  |
| 272 | rust_method | LiveEditorParagraphState | `active_list_kind` | yes |  |
| 276 | rust_method | LiveEditorParagraphState | `active_list_kind_label` | yes |  |
| 280 | rust_method | LiveEditorParagraphState | `has_style_changes` | yes |  |
| 285 | rust_method | LiveEditorParagraphState | `requires_source_replacement` | yes |  |
| 293 | rust_method | LiveEditorParagraphState | `has_session_changes` | yes |  |
| 297 | rust_method | LiveEditorParagraphState | `mark_session_clean` | yes |  |
| 301 | rust_method | LiveEditorParagraphState | `draft_runs` | yes |  |
| 305 | rust_method | LiveEditorParagraphState | `sync_target_control_style` | yes |  |
| 340 | rust_method | LiveEditorParagraphState | `set_alignment` | yes |  |
| 351 | rust_method | LiveEditorParagraphState | `set_list_kind` | yes |  |
| 366 | rust_method | LiveEditorParagraphState | `restore_list_kind_from_marker_text` | yes |  |
| 371 | rust_method | LiveEditorParagraphState | `resolved_marker_text` | yes |  |
| 390 | rust_method | LiveEditorParagraphState | `source_marker_text` | yes |  |
| 398 | rust_method | LiveEditorParagraphState | `set_color_all` | yes |  |
| 413 | rust_method | LiveEditorParagraphState | `set_font_family_all` | yes |  |
| 428 | rust_method | LiveEditorParagraphState | `set_font_size_all` | yes |  |
| 443 | rust_method | LiveEditorParagraphState | `set_char_spacing_all` | yes |  |
| 458 | rust_method | LiveEditorParagraphState | `set_line_height` | yes |  |
| 473 | rust_method | LiveEditorParagraphState | `set_paragraph_mode` | yes |  |

### `crates/pdf-viewer-core/src/edit/paragraph_scene.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 33 | rust_method | Default for ParagraphEditorScene | `default` |  |  |
| 52 | rust_fn |  | `paragraph_editor_scene_from_plan` | yes |  |
| 68 | rust_fn |  | `build_paragraph_editor_scene` | yes |  |
| 78 | rust_fn |  | `build_target_scene` | yes |  |

### `crates/pdf-viewer-core/src/edit/replacement_region.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 20 | rust_method | ParagraphReplacementRegion | `row_suppression_bbox` | yes |  |
| 34 | rust_method | ParagraphReplacementRegion | `viewport_cull_bbox` | yes |  |
| 41 | rust_method | ParagraphReplacementRegion | `cache_invalidation_bbox` | yes |  |
| 46 | rust_fn |  | `paragraph_replacement_region` | yes |  |
| 84 | rust_fn |  | `preferred_source_bbox` |  |  |
| 105 | rust_fn |  | `bbox_has_area` |  |  |
| 115 | rust_fn |  | `target_for_body` |  |  |
| 130 | rust_fn |  | `find_target` |  |  |
| 167 | rust_fn |  | `text_clear_region_stays_near_editable_text` |  |  |
| 184 | rust_fn |  | `tightens_path_suppression` |  |  |
| 201 | rust_fn |  | `covers_tiled_row` |  |  |
| 219 | rust_fn |  | `uses_baseline_geometry` |  |  |

### `crates/pdf-viewer-core/src/edit/replacement_snapshot.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 32 | rust_fn |  | `build_edit_replacement_snapshot` | yes |  |
| 73 | rust_fn |  | `replacement_target_from_patch_snapshot` | yes |  |
| 83 | rust_fn |  | `replacement_object_indices` |  |  |
| 120 | rust_fn |  | `replacement_snapshot_stays_lightweight` |  |  |

### `crates/pdf-viewer-core/src/edit/source_identity.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | rust_fn |  | `collect_target_source_object_ids` | yes |  |
| 48 | rust_fn |  | `collect_target_source_object_indices_set` | yes |  |
| 87 | rust_fn |  | `collect_target_source_object_indices` | yes |  |
| 94 | rust_fn |  | `collect_object_indices_from_runs` | yes |  |

### `crates/pdf-viewer-core/src/edit/source_runs.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_fn |  | `target_paint_runs` | yes |  |
| 90 | rust_fn |  | `summarize_layout_runs` |  |  |
| 119 | rust_fn |  | `resolve_preferred_editor_session` | yes |  |
| 214 | rust_fn |  | `resolve_vector_model_source_runs` |  |  |
| 225 | rust_fn |  | `resolve_vector_model_runs_by_object_id` |  |  |
| 277 | rust_fn |  | `bbox_intersection_width` |  |  |
| 281 | rust_fn |  | `bbox_intersection_height` |  |  |
| 285 | rust_fn |  | `expand_bbox` |  |  |
| 294 | rust_fn |  | `vector_run_matches_paragraph_geometry` |  |  |
| 311 | rust_fn |  | `resolve_vector_model_runs_by_geometry` |  |  |
| 358 | rust_fn |  | `resolve_vector_source_object_order` |  |  |
| 393 | rust_fn |  | `resolve_glyph_paint_runs` |  |  |
| 408 | rust_fn |  | `build_layout` |  |  |
| 423 | rust_fn |  | `layout_run_from_glyph_paint` |  |  |

### `crates/pdf-viewer-core/src/edit/source_text.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 3 | rust_fn |  | `run_gap` |  |  |
| 10 | rust_fn |  | `boundary_needs_visual_space` |  |  |
| 29 | rust_fn |  | `should_insert_run_space` |  |  |
| 59 | rust_fn |  | `char_gap_threshold` |  |  |
| 64 | rust_fn |  | `should_insert_char_space` |  |  |
| 81 | rust_fn |  | `is_ascii_word_char` |  |  |
| 85 | rust_fn |  | `is_pdf_text_separator` |  |  |
| 89 | rust_fn |  | `starts_with_compact_word_boundary` |  |  |
| 99 | rust_fn |  | `needs_compact_text_space` |  |  |
| 120 | rust_fn |  | `normalize_compact_pdf_text` |  |  |
| 132 | rust_fn |  | `run_text_with_visual_spaces` |  |  |
| 180 | rust_fn |  | `session_source_text` | yes |  |
| 202 | rust_fn |  | `compact_pdf_text_restores_resume_word_boundaries` |  |  |
| 212 | rust_fn |  | `compact_pdf_text_does_not_split_acronyms` |  |  |
| 220 | rust_fn |  | `keeps_technical_names` |  |  |

### `crates/pdf-viewer-core/src/edit/target_resolution.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 5 | rust_fn |  | `resolve_region_target` | yes |  |
| 22 | rust_fn |  | `is_supported_region_kind` | yes |  |
| 26 | rust_fn |  | `resolve_region_text_target` | yes |  |
| 62 | rust_fn |  | `normalize_target_text` |  |  |

### `crates/pdf-viewer-core/src/geometry/bbox_ops.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 3 | rust_fn |  | `bbox_width` | yes |  |
| 7 | rust_fn |  | `bbox_height` | yes |  |
| 11 | rust_fn |  | `bbox_intersects` | yes |  |
| 15 | rust_fn |  | `union_bbox` | yes |  |

### `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 74 | rust_method | HostPageTransform | `new` | yes |  |
| 78 | rust_method | HostPageTransform | `scale` | yes |  |
| 85 | rust_method | HostPageTransform | `client_to_page` | yes |  |
| 93 | rust_method | HostPageTransform | `client_to_page_in_box` | yes |  |
| 108 | rust_method | HostPageTransform | `client_to_local_in_box` | yes |  |
| 122 | rust_fn |  | `positive_ratio` |  |  |
| 154 | rust_method | PdfToPageViewTransform | `new` | yes |  |
| 166 | rust_method | PdfToPageViewTransform | `point` | yes |  |
| 184 | rust_method | PdfCoordinateSpace | `normalize_y` | yes |  |
| 190 | rust_method | PdfCoordinateSpace | `denormalize_y` | yes |  |
| 218 | rust_method | EditorViewportTransform | `new` | yes |  |
| 230 | rust_method | EditorViewportTransform | `point_from_pdf` | yes |  |
| 241 | rust_method | EditorViewportTransform | `x_from_pdf` | yes |  |
| 247 | rust_method | EditorViewportTransform | `baseline_y_from_pdf` | yes |  |
| 253 | rust_method | EditorViewportTransform | `baseline_y_from_anchor_relative` | yes |  |

### `crates/pdf-viewer-core/src/geometry/dom_projection.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 31 | rust_fn |  | `measure_dom_to_page_scale` | yes |  |

### `crates/pdf-viewer-core/src/geometry/field_projection.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 3 | rust_fn |  | `resolve_field_projection` | yes |  |

### `crates/pdf-viewer-core/src/geometry/layout_engine.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 49 | rust_fn |  | `is_no_start` |  |  |
| 53 | rust_fn |  | `is_no_end` |  |  |
| 57 | rust_fn |  | `is_forced_line_break_run` |  |  |
| 74 | rust_fn |  | `layout_paragraph` | yes |  |
| 247 | rust_fn |  | `finish_line` |  |  |
| 292 | rust_fn |  | `layout_anchored_pair` | yes |  |
| 315 | rust_fn |  | `mock_run` |  |  |
| 334 | rust_fn |  | `test_cjk_no_start_rule` |  |  |
| 364 | rust_fn |  | `test_justified_alignment` |  |  |
| 397 | rust_method | ParagraphLayout | `find_run_at_text_offset` | yes |  |
| 422 | rust_fn |  | `find_paragraph_at` | yes |  |
| 458 | rust_fn |  | `is_point_in_bbox` | yes |  |
| 462 | rust_fn |  | `bbox_area` |  |  |
| 468 | rust_fn |  | `resolve_editor_projection` | yes |  |

### `crates/pdf-viewer-core/src/geometry/reflow_engine.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 5 | rust_fn |  | `calculate_reflow_displacements` | yes |  |

### `crates/pdf-viewer-core/src/geometry/source_geometry.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 3 | rust_fn |  | `source_session_visual_bbox` | yes |  |
| 7 | rust_fn |  | `source_visual_bbox_from_runs` | yes |  |
| 21 | rust_fn |  | `source_line_visual_bbox_for_caret` | yes |  |
| 49 | rust_fn |  | `source_run_visual_bbox` | yes |  |
| 74 | rust_fn |  | `source_run_horizontal_span` |  |  |
| 98 | rust_fn |  | `inferred_run_width` |  |  |
| 127 | rust_fn |  | `bbox_width` |  |  |
| 132 | rust_fn |  | `bbox_height` |  |  |
| 137 | rust_fn |  | `bbox_has_area` |  |  |
| 142 | rust_fn |  | `union_bbox` |  |  |
| 156 | rust_fn |  | `test_run` |  |  |
| 186 | rust_fn |  | `uses_baseline_bbox` |  |  |
| 198 | rust_fn |  | `uses_source_geometry` |  |  |

### `crates/pdf-viewer-core/src/lib.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_fn |  | `read_core_version` | yes |  |

### `crates/pdf-viewer-core/src/models/geometry.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 27 | rust_method | BoundingBox | `flip_y` | yes |  |

### `crates/pdf-viewer-core/src/models/glyph.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | rust_fn |  | `default_scale_x` |  |  |
| 116 | rust_method | GlyphPaintPlan | `flip_y` | yes |  |

### `crates/pdf-viewer-core/src/models/layout.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 115 | rust_fn |  | `default_scale` |  |  |
| 155 | rust_method | LayoutRun | `from_styled` | yes |  |
| 217 | rust_method | LayoutParagraph | `flip_y` | yes |  |
| 249 | rust_method | SemanticRegion | `flip_y` | yes |  |
| 268 | rust_method | LayoutInferenceResult | `flip_y` | yes |  |

### `crates/pdf-viewer-core/src/models/styled_run.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | rust_fn |  | `default_horizontal_scaling` |  |  |
| 108 | rust_method | StyledRun | `flip_y` | yes |  |
| 113 | rust_fn |  | `is_zero` |  |  |
| 116 | rust_fn |  | `is_zero_i32` |  |  |
| 119 | rust_fn |  | `is_false` |  |  |
| 122 | rust_fn |  | `default_scale` |  |  |
| 125 | rust_fn |  | `is_default_scale` |  |  |
| 128 | rust_fn |  | `default_alpha` |  |  |
| 131 | rust_fn |  | `is_default_alpha` |  |  |
| 233 | rust_method | NativeTextModel | `flip_y` | yes |  |
| 247 | rust_method | Default for NativeTextModel | `default` |  |  |

### `crates/pdf-viewer-core/src/models/vector.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 82 | rust_method | VectorPageModel | `flip_y` | yes |  |

### `crates/pdf-viewer-core/src/persistence/engine.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | rust_fn |  | `collect_persistable_region_patches` | yes |  |
| 99 | rust_fn |  | `collect_legacy_text_reflows` | yes |  |
| 165 | rust_fn |  | `build_persistable_save_plan` | yes |  |
| 223 | rust_fn |  | `resolve_reflow_key` |  |  |

### `crates/pdf-viewer-core/src/persistence/history_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 5 | rust_method | PatchCommand | `execute` | yes |  |
| 9 | rust_method | PatchCommand | `undo` | yes |  |
| 32 | rust_method | HistoryStore | `new` | yes |  |
| 39 | rust_method | HistoryStore | `push` | yes |  |
| 50 | rust_method | HistoryStore | `undo` | yes |  |
| 60 | rust_method | HistoryStore | `redo` | yes |  |
| 70 | rust_method | HistoryStore | `clear` | yes |  |
| 80 | rust_fn |  | `push_command` | yes |  |
| 86 | rust_fn |  | `undo` | yes |  |
| 94 | rust_fn |  | `redo` | yes |  |
| 102 | rust_fn |  | `clear_history` | yes |  |

### `crates/pdf-viewer-core/src/persistence/models.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 45 | rust_fn |  | `default_scale_x_model` |  |  |

### `crates/pdf-viewer-core/src/persistence/patch_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 31 | rust_method | GlobalPatchState | `new` | yes |  |
| 35 | rust_method | GlobalPatchState | `find_paragraph_snapshot` | yes |  |
| 73 | rust_fn |  | `bump_patch_revision` | yes |  |
| 77 | rust_fn |  | `has_visible_patches` | yes |  |
| 86 | rust_fn |  | `apply_patch_maps` | yes |  |
| 127 | rust_fn |  | `remove_patch_maps` | yes |  |
| 140 | rust_fn |  | `capture_existing_patch` | yes |  |
| 161 | rust_fn |  | `apply_patch` | yes |  |
| 180 | rust_fn |  | `should_prefetch_page` | yes |  |
| 190 | rust_fn |  | `build_pagination_commands` | yes |  |

### `crates/pdf-viewer-core/src/render/effective_page_plan.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 73 | rust_fn |  | `overlay_paragraph_object_ids` |  |  |
| 77 | rust_fn |  | `overlay_paragraph_object_indices` |  |  |
| 83 | rust_fn |  | `overlay_renders_last` |  |  |
| 91 | rust_fn |  | `overlay_suppresses_text_source` |  |  |
| 95 | rust_fn |  | `overlay_suppresses_row_paths` |  |  |
| 103 | rust_fn |  | `overlay_intersects_viewport` |  |  |
| 116 | rust_fn |  | `vector_object_bbox` |  |  |
| 144 | rust_fn |  | `vector_object_summary` |  |  |
| 183 | rust_fn |  | `record_overlay_object_summary` |  |  |
| 193 | rust_fn |  | `build_effective_vector_render_plan` | yes |  |
| 645 | rust_fn |  | `build_effective_glyph_render_plan` | yes |  |
| 774 | rust_fn |  | `horizontal_stroked_path` |  |  |
| 778 | rust_fn |  | `horizontal_stroked_path_between` |  |  |
| 803 | rust_fn |  | `active_overlay_for_body` |  |  |
| 828 | rust_fn |  | `active_overlay_for_source_object` |  |  |
| 846 | rust_fn |  | `persisted_overlay_for_source_object` |  |  |
| 852 | rust_fn |  | `text_object_without_run_ids` |  |  |
| 868 | rust_fn |  | `glyph_plan_without_run_ids` |  |  |
| 916 | rust_fn |  | `suppresses_zero_height_path` |  |  |
| 951 | rust_fn |  | `keeps_section_divider` |  |  |
| 986 | rust_fn |  | `keeps_nearby_divider` |  |  |
| 1018 | rust_fn |  | `suppresses_descender_path` |  |  |
| 1050 | rust_fn |  | `suppresses_text_without_ids` |  |  |
| 1083 | rust_fn |  | `spatially_suppresses_text` |  |  |
| 1115 | rust_fn |  | `keeps_matching_text` |  |  |
| 1151 | rust_fn |  | `keeps_source_text` |  |  |
| 1182 | rust_fn |  | `suppresses_path_only` |  |  |
| 1231 | rust_fn |  | `spatially_suppresses_glyphs` |  |  |
| 1265 | rust_fn |  | `keeps_matching_glyphs` |  |  |
| 1297 | rust_fn |  | `overlay_suppresses_glyphs` |  |  |
| 1336 | rust_fn |  | `overlay_renders_last` |  |  |
| 1363 | rust_fn |  | `overlay_suppresses_path` |  |  |
| 1393 | rust_fn |  | `keeps_right_tile_suppressed` |  |  |
| 1431 | rust_fn |  | `keeps_list_marker` |  |  |
| 1549 | rust_fn |  | `handles_z_index_order` |  |  |

### `crates/pdf-viewer-core/src/render/frame_cache.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | rust_fn |  | `resolve_viewport_refresh` | yes |  |
| 24 | rust_fn |  | `touch_frame_cache_entry` | yes |  |
| 35 | rust_fn |  | `store_frame_cache_entry` | yes |  |
| 46 | rust_fn |  | `reset_frame_cache` | yes |  |

### `crates/pdf-viewer-core/src/render/layer.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 5 | rust_fn |  | `allow_detail_overlay_retention` |  |  |
| 49 | rust_fn |  | `resolve_layer_execution_plan` | yes |  |
| 66 | rust_fn |  | `resolve_layer_present_decision` | yes |  |
| 77 | rust_fn |  | `resolve_render_execution_plan` | yes |  |

### `crates/pdf-viewer-core/src/render/paint_plan.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 8 | rust_fn |  | `paint_mode_from_render_mode` |  |  |
| 16 | rust_fn |  | `resolve_run_font` |  |  |
| 34 | rust_fn |  | `build_paint_run` |  |  |
| 63 | rust_fn |  | `build_editor_session` |  |  |
| 94 | rust_fn |  | `is_decorative_text` |  |  |
| 102 | rust_fn |  | `build_control_style` |  |  |
| 146 | rust_fn |  | `build_field_editor_params` | yes |  |
| 212 | rust_fn |  | `build_glyph_paint_plan` | yes |  |

### `crates/pdf-viewer-core/src/render/path_suppression.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 8 | rust_fn |  | `should_suppress` | yes |  |
| 67 | rust_fn |  | `image_object_bbox` |  |  |
| 76 | rust_fn |  | `allowed_path_height` |  |  |
| 89 | rust_fn |  | `source_row_decoration_matches` |  |  |
| 126 | rust_fn |  | `source_row_decoration_summary` |  |  |
| 155 | rust_fn |  | `bbox_overlap_width` |  |  |
| 159 | rust_fn |  | `bbox_overlap_height` |  |  |
| 163 | rust_fn |  | `row_overlap_height` |  |  |
| 181 | rust_fn |  | `replacement_target` |  |  |
| 201 | rust_fn |  | `row_image` |  |  |
| 213 | rust_fn |  | `suppresses_thin_decoration` |  |  |
| 228 | rust_fn |  | `keeps_normal_image` |  |  |

### `crates/pdf-viewer-core/src/render/plan_builder.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 111 | rust_fn |  | `clamp_f32` | yes |  |
| 134 | rust_fn |  | `centered_offset` | yes |  |
| 138 | rust_fn |  | `cache_zoom_ratio_delta` | yes |  |
| 144 | rust_fn |  | `should_prepare_layout` | yes |  |
| 151 | rust_fn |  | `is_stable_document_frame` | yes |  |
| 158 | rust_fn |  | `compute_viewport_layout_result` | yes |  |
| 176 | rust_fn |  | `compute_viewport_tile_result` | yes |  |
| 220 | rust_fn |  | `resolve_tile_overscan` | yes |  |
| 235 | rust_fn |  | `compute_anchor_viewport_layout_result` | yes |  |
| 291 | rust_fn |  | `compute_visible_content_rect` | yes |  |
| 324 | rust_fn |  | `resolve_render_zoom_result` | yes |  |

### `crates/pdf-viewer-core/src/render/prepared_scene.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 21 | rust_method | PreparedPageScene | `build` | yes |  |
| 82 | rust_method | PreparedPageScene | `visible_vector_indices` | yes |  |
| 111 | rust_method | PreparedPageScene | `active_text_object_ids` | yes |  |
| 116 | rust_fn |  | `vector_object_bbox` |  |  |
| 144 | rust_fn |  | `resolve_bucket_keys` |  |  |

### `crates/pdf-viewer-core/src/render/present_plan.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 22 | rust_fn |  | `resolve_present_policy` | yes |  |
| 57 | rust_fn |  | `preview_is_settled` | yes |  |
| 61 | rust_fn |  | `preview_base_layer_reuse_ratio` | yes |  |
| 65 | rust_fn |  | `preview_detail_layer_reuse_ratio` | yes |  |
| 69 | rust_fn |  | `quantize_cache_zoom` | yes |  |

### `crates/pdf-viewer-core/src/render/preview.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | rust_fn |  | `resolve_preview_present_plan` | yes |  |

### `crates/pdf-viewer-core/src/render/progressive.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 50 | rust_method | ProgressiveVectorRenderTask | `build` | yes |  |
| 73 | rust_method | ProgressiveVectorRenderTask | `is_complete` | yes |  |
| 77 | rust_method | ProgressiveVectorRenderTask | `total_items` | yes |  |
| 82 | rust_fn |  | `resolve_progressive_render_policy` | yes |  |
| 121 | rust_fn |  | `resolve_progressive_render_policy_request` | yes |  |

### `crates/pdf-viewer-core/src/render/renderer.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 35 | rust_fn |  | `render` |  |  |
| 38 | rust_fn |  | `clear` |  |  |
| 41 | rust_fn |  | `name` |  |  |

### `crates/pdf-viewer-core/src/render/scheduler.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_method | Default for HostRenderState<TPlan> | `default` |  |  |
| 49 | rust_method | Default for RenderFrameTransition<TPlan> | `default` |  |  |
| 58 | rust_fn |  | `allocate_render_frame_token` | yes |  |

### `crates/pdf-viewer-core/src/render/snapshot_paint_plan.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | rust_fn |  | `to_paint_mode` |  |  |
| 17 | rust_fn |  | `build_resolved_font_face` | yes |  |
| 48 | rust_fn |  | `build_run_bbox` |  |  |
| 77 | rust_fn |  | `build_snapshot_paint_run` |  |  |
| 129 | rust_fn |  | `resolve_run_layout` | yes |  |
| 180 | rust_fn |  | `build_paragraph_snapshot_paint_runs` | yes |  |
| 278 | rust_fn |  | `build_field_group_snapshot_paint_runs` | yes |  |

### `crates/pdf-viewer-core/src/render/source_suppression.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 22 | rust_method | SuppressedVectorTextRuns | `is_empty` | yes |  |
| 26 | rust_method | SuppressedVectorTextRuns | `extend` | yes |  |
| 31 | rust_method | SuppressedVectorTextRuns | `suppressed_count_for_text_object` | yes |  |
| 39 | rust_method | SuppressedVectorTextRuns | `suppresses_run` | yes |  |
| 49 | rust_fn |  | `bbox_overlap_width` |  |  |
| 53 | rust_fn |  | `bbox_overlap_height` |  |  |
| 57 | rust_fn |  | `run_text_is_list_marker_only` | yes |  |
| 79 | rust_fn |  | `text_run_spatially_matches_replacement_region` | yes |  |
| 103 | rust_fn |  | `glyph_run_spatially_matches_replacement_region` | yes |  |
| 122 | rust_fn |  | `normalize_source_match_text` |  |  |
| 127 | rust_fn |  | `text_object_matches_overlay_source_text` | yes |  |
| 157 | rust_fn |  | `glyph_paragraph_matches_overlay_source_text` | yes |  |
| 185 | rust_fn |  | `matching_text_run_refs` | yes |  |
| 210 | rust_fn |  | `text_object_should_be_suppressed` | yes |  |

### `crates/pdf-viewer-core/src/render/tile_cache.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 55 | rust_fn |  | `build_base_cache_key` | yes |  |
| 68 | rust_fn |  | `build_detail_cache_key` | yes |  |
| 93 | rust_fn |  | `find_reusable_base_layer` | yes |  |
| 134 | rust_fn |  | `find_reusable_detail_tile` | yes |  |
| 199 | rust_fn |  | `remember_base_layer` | yes |  |
| 204 | rust_fn |  | `remember_detail_tile` | yes |  |
| 209 | rust_fn |  | `clear_detail_tiles` | yes |  |
| 214 | rust_fn |  | `touch_frame_cache_key` | yes |  |
| 234 | rust_fn |  | `store_frame_cache_key` | yes |  |
| 270 | rust_fn |  | `clear_frame_cache_keys` | yes |  |
| 277 | rust_fn |  | `detail_tile_covers_viewport` |  |  |
| 297 | rust_fn |  | `cache_zoom_matches` |  |  |
| 301 | rust_fn |  | `cache_zoom_ratio_delta` |  |  |
| 307 | rust_fn |  | `best_matching_base_layer` |  |  |
| 332 | rust_fn |  | `best_matching_detail_tile` |  |  |
| 368 | rust_fn |  | `push_recent_base_layer` |  |  |
| 376 | rust_fn |  | `push_recent_detail_tile` |  |  |
| 384 | rust_fn |  | `detail_tile_covers_viewport_geometry` |  |  |

### `crates/pdf-viewer-core/src/render/viewer_session.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_method | Default for HostViewerSession | `default` |  |  |

### `crates/pdf-viewer-core/src/render/viewport_culling.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | rust_fn |  | `resolve_page_viewport_bbox` | yes |  |
| 43 | rust_fn |  | `glyph_run_intersects_viewport` | yes |  |
| 47 | rust_fn |  | `paragraph_intersects_viewport` | yes |  |
| 54 | rust_fn |  | `region_intersects_viewport` | yes |  |
| 58 | rust_fn |  | `vector_object_intersects_viewport` | yes |  |
| 69 | rust_fn |  | `text_object_intersects_viewport` |  |  |
| 75 | rust_fn |  | `path_object_intersects_viewport` |  |  |
| 81 | rust_fn |  | `image_object_intersects_viewport` |  |  |
| 91 | rust_fn |  | `styled_run_bbox` | yes |  |
| 100 | rust_fn |  | `path_object_bbox` | yes |  |
| 134 | rust_fn |  | `resolves_viewport_bbox_from_display_space` |  |  |
| 152 | rust_fn |  | `detects_bbox_intersection` |  |  |

### `crates/pdf-viewer-core/src/render/viewport_refresh.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 18 | rust_fn |  | `note_viewport_render_commit` | yes |  |
| 27 | rust_fn |  | `resolve_viewport_refresh_decision` | yes |  |

### `crates/pdf-viewer-core/src/render/workflow.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 25 | rust_fn |  | `build_render_frame_envelope` | yes |  |
| 35 | rust_fn |  | `frame_plan_requires_render` | yes |  |
| 39 | rust_fn |  | `frame_plan_needs_viewport_refresh` | yes |  |
| 43 | rust_fn |  | `frame_plans_share_render_work` | yes |  |
| 52 | rust_fn |  | `progressive_start_result` | yes |  |
| 59 | rust_fn |  | `progressive_step_result` | yes |  |

### `crates/pdf-viewer-core/src/render/zoom_host.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 48 | rust_fn |  | `needs_render` |  |  |
| 52 | rust_fn |  | `wheel_render_idle_ms` |  |  |
| 62 | rust_fn |  | `preview_is_active` |  |  |
| 66 | rust_fn |  | `resolve_wheel_render_decision` | yes |  |
| 110 | rust_fn |  | `resolve_preview_tick_decision` | yes |  |
| 129 | rust_fn |  | `resolve_render_follow_up_decision` | yes |  |

### `crates/pdf-viewer-core/src/render/zoom_interaction.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 92 | rust_fn |  | `clamp_zoom` | yes |  |
| 101 | rust_fn |  | `clamp_f32` | yes |  |
| 124 | rust_fn |  | `clamp_unit` | yes |  |
| 128 | rust_fn |  | `centered_offset` | yes |  |
| 132 | rust_fn |  | `compute_anchor_scroll_result` | yes |  |
| 180 | rust_fn |  | `resolve_anchor_from_visible_preview_state` | yes |  |
| 214 | rust_fn |  | `compute_anchor_viewport_layout_result` | yes |  |
| 270 | rust_fn |  | `resolve_wheel_zoom_request` | yes |  |
| 365 | rust_fn |  | `resolve_zoom_limits_result` | yes |  |
| 380 | rust_fn |  | `advance_zoom_animation_state` | yes |  |
| 446 | rust_fn |  | `commit_rendered_zoom` | yes |  |
| 462 | rust_fn |  | `build_zoom_preview_frame` | yes |  |
| 519 | rust_fn |  | `preserves_cursor_anchor` |  |  |

### `crates/pdf-viewer-core/src/render/zoom_state.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 66 | rust_method | Default for HostZoomState | `default` |  |  |

### `crates/pdf-viewer-core/src/text/caret_geometry.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 28 | rust_fn |  | `same_existing_session_line` | yes |  |
| 38 | rust_fn |  | `resolve_caret_index_from_lines` | yes |  |
| 59 | rust_fn |  | `dedupe_caret_stops` | yes |  |
| 64 | rust_fn |  | `caret_index_at_page_point` | yes |  |
| 73 | rust_fn |  | `resolve_index` | yes |  |
| 85 | rust_fn |  | `caret_visual_for_session` | yes |  |
| 94 | rust_fn |  | `caret_visual_for_session_plan` | yes |  |
| 126 | rust_fn |  | `build_session_caret_lines` | yes |  |
| 185 | rust_fn |  | `populate_line_stops_from_text_plan` | yes |  |
| 221 | rust_fn |  | `resolve_navigation_from_lines` | yes |  |

### `crates/pdf-viewer-core/src/text/editable_segments.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 21 | rust_fn |  | `resolve_segment_patch_key` |  |  |
| 25 | rust_fn |  | `is_colon_token` |  |  |
| 32 | rust_fn |  | `looks_like_short_field_token` |  |  |
| 37 | rust_fn |  | `resolve_run_visible_glyph_width` |  |  |
| 48 | rust_fn |  | `resolve_run_style_signature` |  |  |
| 72 | rust_fn |  | `normalize_field_label` |  |  |
| 77 | rust_fn |  | `detect_field_label_anchors` |  |  |
| 131 | rust_fn |  | `build_field_groups` |  |  |
| 162 | rust_fn |  | `create_editable_segment` |  |  |
| 235 | rust_fn |  | `build_contiguous_segments_in_range` |  |  |
| 271 | rust_fn |  | `build_editable_segments` | yes |  |

### `crates/pdf-viewer-core/src/text/glyph_layout.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 67 | rust_method | EditorSessionTextPlan | `map_raw_to_reconstructed` | yes |  |
| 75 | rust_method | EditorSessionTextPlan | `reconstructed_char_count` | yes |  |
| 79 | rust_method | EditorSessionTextPlan | `map_reconstructed_to_raw` | yes |  |
| 88 | rust_fn |  | `is_decorative_glyph` | yes |  |
| 92 | rust_fn |  | `is_decorative_text` | yes |  |
| 97 | rust_fn |  | `is_cjk_unified` |  |  |
| 109 | rust_fn |  | `is_open_punctuation` |  |  |
| 113 | rust_fn |  | `is_close_punctuation` |  |  |
| 138 | rust_fn |  | `should_allow_synthetic_gap` |  |  |
| 153 | rust_fn |  | `is_spacing_punctuation` |  |  |
| 157 | rust_fn |  | `is_ascii_word_start` |  |  |
| 161 | rust_fn |  | `estimated_gap_source_advance` |  |  |
| 169 | rust_fn |  | `should_insert_gap_from_origin_delta` |  |  |
| 193 | rust_fn |  | `infer_run_advance` | yes |  |
| 218 | rust_fn |  | `compute_run_aware_caret_left` | yes |  |
| 264 | rust_fn |  | `resolve_caret_index_for_click` | yes |  |
| 314 | rust_fn |  | `resolve_field_hit_for_click` | yes |  |
| 365 | rust_fn |  | `rect_contains_point` |  |  |
| 380 | rust_fn |  | `resolve_field_hit_target_for_click` | yes |  |
| 416 | rust_fn |  | `extract_decorative_prefix` | yes |  |
| 451 | rust_fn |  | `glyph_left` |  |  |
| 460 | rust_fn |  | `glyph_right` |  |  |
| 478 | rust_fn |  | `glyph_visual_width` |  |  |
| 482 | rust_fn |  | `same_visual_line` |  |  |
| 487 | rust_fn |  | `typical_contiguous_advance` |  |  |
| 504 | rust_fn |  | `should_insert_internal_gap_space` |  |  |
| 529 | rust_fn |  | `should_insert_visual_gap_space` |  |  |
| 565 | rust_fn |  | `line_contextual_run_delta` |  |  |
| 600 | rust_fn |  | `needs_gap` |  |  |
| 644 | rust_fn |  | `build_editor_session_text_plan` | yes |  |
| 764 | rust_fn |  | `has_suspicious_run_geometry` | yes |  |

### `crates/pdf-viewer-core/src/text/index_convert.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 1 | rust_fn |  | `utf16_offset_to_char_index` | yes |  |
| 13 | rust_fn |  | `char_index_to_utf16_offset` | yes |  |
| 22 | rust_fn |  | `converts_utf16_indexes` |  |  |

### `crates/pdf-viewer-core/src/text/list_semantics.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 26 | rust_fn |  | `extract_numbering_prefix` |  |  |
| 68 | rust_fn |  | `parse_numbering_value` | yes |  |
| 92 | rust_fn |  | `format_numbering_marker` | yes |  |
| 131 | rust_fn |  | `derive_list_text_semantics` | yes |  |

### `crates/pdf-viewer-core/src/text/search_replace.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | rust_fn |  | `replace_query_matches` | yes |  |
| 59 | rust_fn |  | `matches_query_at` |  |  |
| 77 | rust_fn |  | `slice_chars` |  |  |

### `crates/pdf-viewer-core/src/text/semantic_axiom.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | rust_method | AxiomEngine | `infer_role` | yes |  |

### `crates/pdf-viewer-core/src/text/style_mapper.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 21 | rust_method | StyleMapper | `new_from_paragraph` |  |  |
| 38 | rust_method | StyleMapper | `new_from_paragraph_for_text` | yes |  |
| 47 | rust_method | StyleMapper | `read_full_text` | yes |  |
| 52 | rust_method | StyleMapper | `update_with_text` | yes |  |
| 143 | rust_method | StyleMapper | `set_bold_all` | yes |  |
| 150 | rust_method | StyleMapper | `set_italic_all` | yes |  |
| 157 | rust_method | StyleMapper | `set_underline_all` | yes |  |
| 164 | rust_method | StyleMapper | `set_color_all` | yes |  |
| 171 | rust_method | StyleMapper | `set_font_name_all` | yes |  |
| 178 | rust_method | StyleMapper | `set_font_size_all` | yes |  |
| 185 | rust_method | StyleMapper | `set_char_spacing_all` | yes |  |
| 192 | rust_method | StyleMapper | `is_bold_any` | yes |  |
| 196 | rust_method | StyleMapper | `is_italic_any` | yes |  |
| 200 | rust_method | StyleMapper | `is_bold_all` | yes |  |
| 204 | rust_method | StyleMapper | `is_italic_all` | yes |  |
| 208 | rust_method | StyleMapper | `is_underline_any` | yes |  |
| 212 | rust_method | StyleMapper | `is_underline_all` | yes |  |
| 216 | rust_method | StyleMapper | `dominant_style` | yes |  |
| 225 | rust_method | StyleMapper | `has_style_changes_against_paragraph` | yes |  |
| 232 | rust_method | StyleMapper | `to_layout_runs` | yes |  |
| 255 | rust_fn |  | `should_preserve_editor_underline` | yes |  |
| 277 | rust_fn |  | `compute_lcp_len` |  |  |
| 285 | rust_fn |  | `compute_lcs_len` |  |  |
| 295 | rust_fn |  | `merge_adjacent_spans` |  |  |
| 316 | rust_fn |  | `style_spans_have_same_paint_style` |  |  |
| 328 | rust_fn |  | `expand_style_signature_by_char` |  |  |
| 338 | rust_fn |  | `is_style_equal` |  |  |
| 346 | rust_fn |  | `is_decorative_text` |  |  |
| 361 | rust_fn |  | `create_test_mapper` |  |  |
| 376 | rust_fn |  | `create_test_run` |  |  |
| 395 | rust_fn |  | `test_deletion_at_head` |  |  |
| 404 | rust_fn |  | `test_deletion_multi_byte_chinese` |  |  |
| 413 | rust_fn |  | `test_full_deletion_protection` |  |  |
| 422 | rust_fn |  | `ignores_canonical_gaps` |  |  |

### `crates/pdf-viewer-core/src/text/style_preservation.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 5 | rust_fn |  | `make_style_run` | yes |  |
| 20 | rust_fn |  | `reindex_style_runs` | yes |  |
| 34 | rust_fn |  | `resolve_dominant_paragraph_style` | yes |  |
| 44 | rust_fn |  | `distribute_text_across_runs` | yes |  |
| 112 | rust_fn |  | `is_decorative_run_text` |  |  |
| 122 | rust_fn |  | `line_selection_range` |  |  |
| 146 | rust_fn |  | `preserve_changed_line_styles` | yes |  |

### `crates/pdf-viewer-core/src/text/text_model.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | rust_method | EditorTextModel | `new` | yes |  |
| 18 | rust_method | EditorTextModel | `source_text` | yes |  |
| 22 | rust_method | EditorTextModel | `current_text` | yes |  |
| 26 | rust_method | EditorTextModel | `current_char_count` | yes |  |
| 30 | rust_method | EditorTextModel | `is_pristine` | yes |  |
| 34 | rust_method | EditorTextModel | `set_current_text` | yes |  |

### `crates/pdf-viewer-core/src/typography/engine.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | rust_method | TypographyEngine<'a> | `new` | yes |  |
| 19 | rust_method | TypographyEngine<'a> | `resolve_pdf_font` | yes |  |

### `crates/pdf-viewer-core/src/typography/font_resolver.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 5 | rust_fn |  | `strip_subset_prefix` |  |  |
| 12 | rust_fn |  | `split_family_and_style` |  |  |
| 46 | rust_fn |  | `classify_symbol_family` |  |  |
| 59 | rust_fn |  | `resolve_render_family` |  |  |
| 186 | rust_fn |  | `resolve_font_face` | yes |  |
| 215 | rust_fn |  | `looks_like_symbolic_font` | yes |  |

### `crates/pdf-viewer-core/src/typography/matcher.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 8 | rust_fn |  | `build_match_request` | yes |  |
| 35 | rust_fn |  | `build_descriptor_request` | yes |  |
| 47 | rust_fn |  | `normalize_pdf_font_identity` | yes |  |
| 66 | rust_fn |  | `score_system_font_candidate` | yes |  |
| 276 | rust_fn |  | `choose_best_match` | yes |  |
| 285 | rust_fn |  | `choose_top_matches` | yes |  |
| 304 | rust_fn |  | `resolve_system_or_fallback_font` | yes |  |
| 358 | rust_fn |  | `strip_subset_prefix` |  |  |
| 365 | rust_fn |  | `split_family_name` |  |  |
| 390 | rust_fn |  | `extract_style_name` |  |  |
| 412 | rust_fn |  | `push_reason` |  |  |
| 425 | rust_fn |  | `normalized_font_key` |  |  |
| 460 | rust_fn |  | `strips_pdf_subset_prefix` |  |  |
| 468 | rust_fn |  | `exact_family_match_beats_unrelated_candidate` |  |  |
| 486 | rust_fn |  | `chinese_aliases_normalize_to_same_key` |  |  |
| 495 | rust_fn |  | `descriptor_postscript_match_boosts_candidate` |  |  |
| 521 | rust_fn |  | `accepts_embedded_cmap` |  |  |

### `crates/pdf-viewer-core/src/utils/debug.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 1 | rust_fn |  | `truncate_debug_text` | yes |  |

### `crates/pdf-viewer-core/src/utils/sanitize.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 1 | rust_fn |  | `sanitize_positive` | yes |  |
| 9 | rust_fn |  | `sanitize_non_negative` | yes |  |

### `crates/pdf-viewer-ui/src/annotation/annotation_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 26 | rust_fn |  | `parse_annotation_kind` |  |  |
| 43 | rust_fn |  | `target_to_annotation` |  |  |
| 70 | rust_method | AnnotationManager | `new` | yes | new |
| 80 | rust_method | AnnotationManager | `list` | yes | list |
| 97 | rust_method | AnnotationManager | `delete` | yes | delete |
| 112 | rust_method | Default for AnnotationManager | `default` |  |  |

### `crates/pdf-viewer-ui/src/annotation/annotation_types.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_fn |  | `response_to_js` |  |  |
| 18 | rust_fn |  | `ok_response` | yes |  |
| 29 | rust_fn |  | `ok_empty` | yes |  |
| 39 | rust_fn |  | `err_response` | yes |  |

### `crates/pdf-viewer-ui/src/app_controller.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 8 | rust_fn |  | `target_invoke` | yes | targetInvoke |
| 11 | rust_fn |  | `on_debug` | yes | onDebug |
| 61 | rust_method | PdfLogger | `info` | yes |  |
| 65 | rust_method | PdfLogger | `debug` | yes |  |
| 69 | rust_method | PdfLogger | `error` | yes |  |
| 73 | rust_method | PdfLogger | `trace` | yes |  |
| 79 | rust_fn |  | `format_structured_trace` |  |  |
| 112 | rust_fn |  | `format_value` |  |  |
| 123 | rust_fn |  | `smart_invoke` | yes |  |

### `crates/pdf-viewer-ui/src/application.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 56 | rust_fn |  | `snapshot_state` |  |  |
| 81 | rust_fn |  | `reset_find_controller` |  |  |
| 95 | rust_method | Application | `new` | yes | new |
| 110 | rust_method | Application | `open` | yes | open |
| 141 | rust_method | Application | `close` | yes | close |
| 160 | rust_method | Application | `reset_all` | yes | resetAll |
| 173 | rust_method | Application | `read_state` | yes | readState |
| 179 | rust_method | Application | `get_state` | yes | getState |
| 194 | rust_method | Application | `add_event_listener` | yes | addEventListener |
| 202 | rust_method | Application | `remove_event_listener` | yes | removeEventListener |
| 208 | rust_method | Application | `remove_all_event_listeners` | yes | removeAllEventListeners |
| 214 | rust_method | Default for Application | `default` |  |  |

### `crates/pdf-viewer-ui/src/bridge.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | rust_fn |  | `on_debug` | yes | onDebug |
| 8 | rust_fn |  | `on_input` | yes | onInput |
| 10 | rust_fn |  | `on_open` | yes | onOpen |
| 12 | rust_fn |  | `on_commit` | yes | onCommit |
| 14 | rust_fn |  | `on_cancel` | yes | onCancel |
| 16 | rust_fn |  | `target_invoke` | yes | invoke |
| 19 | rust_fn |  | `emit_debug_trace` | yes |  |

### `crates/pdf-viewer-ui/src/commands.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 20 | rust_method | From<DeletePageCommand> for PdfEditCommand | `from` |  |  |
| 25 | rust_method | From<RotatePageCommand> for PdfEditCommand | `from` |  |  |
| 30 | rust_method | From<InsertPageCommand> for PdfEditCommand | `from` |  |  |
| 35 | rust_method | From<AddHighlightCommand> for PdfEditCommand | `from` |  |  |
| 40 | rust_method | From<UpdateMetadataCommand> for PdfEditCommand | `from` |  |  |

### `crates/pdf-viewer-ui/src/comment/comment_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 21 | rust_fn |  | `parse_scope` |  |  |
| 36 | rust_method | CommentManager | `new` | yes | new |
| 43 | rust_method | CommentManager | `clear_review_session` | yes | clearReviewSession |
| 48 | rust_method | CommentManager | `read_review_session` | yes | readReviewSession |
| 55 | rust_method | CommentManager | `load_review` | yes | loadReview |
| 61 | rust_method | CommentManager | `load_overlay` | yes | loadOverlay |
| 67 | rust_method | CommentManager | `load_target_overlay` | yes | loadTargetOverlay |
| 77 | rust_method | CommentManager | `set_panel_open_and_load` | yes | setPanelOpenAndLoad |
| 88 | rust_method | CommentManager | `toggle_panel_and_load` | yes | togglePanelAndLoad |
| 98 | rust_method | CommentManager | `set_scope_and_load` | yes | setScopeAndLoad |
| 110 | rust_method | CommentManager | `set_query_and_load` | yes | setQueryAndLoad |
| 121 | rust_method | CommentManager | `select_and_load` | yes | selectAndLoad |
| 135 | rust_method | CommentManager | `add_region_comment` | yes | addRegionComment |
| 146 | rust_method | CommentManager | `delete_annotation` | yes | deleteAnnotation |
| 157 | rust_method | CommentManager | `update_comment` | yes | updateComment |
| 169 | rust_method | Default for CommentManager | `default` |  |  |

### `crates/pdf-viewer-ui/src/document/comment.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 38 | rust_fn |  | `list_page_comments` | yes |  |
| 45 | rust_fn |  | `list_page_annotation_targets` | yes |  |
| 52 | rust_fn |  | `review_document_comments` | yes |  |
| 59 | rust_fn |  | `load_comment_review` | yes |  |
| 67 | rust_fn |  | `load_comment_overlay` | yes |  |
| 76 | rust_fn |  | `load_comment_target_overlay` | yes |  |
| 84 | rust_fn |  | `set_comment_review_panel_open_and_load` | yes |  |
| 93 | rust_fn |  | `toggle_comment_review_panel_and_load` | yes |  |
| 101 | rust_fn |  | `set_comment_review_scope_and_load` | yes |  |
| 110 | rust_fn |  | `set_comment_review_query_and_load` | yes |  |
| 119 | rust_fn |  | `select_comment_review_and_load` | yes |  |
| 128 | rust_fn |  | `load_comment_review_from_session` |  |  |
| 156 | rust_fn |  | `build_comment_review_panel` |  |  |
| 213 | rust_fn |  | `build_comment_review_card_actions` |  |  |
| 233 | rust_fn |  | `build_comment_overlay_display` |  |  |
| 255 | rust_fn |  | `build_comment_target_overlay_display` |  |  |
| 278 | rust_fn |  | `build_percent_frame` |  |  |
| 294 | rust_fn |  | `add_region_comment` | yes |  |
| 301 | rust_fn |  | `delete_page_annotation` | yes |  |
| 308 | rust_fn |  | `update_page_comment` | yes |  |

### `crates/pdf-viewer-ui/src/document/document_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 32 | rust_method | DocumentSession | `new` | yes | new |
| 40 | rust_method | DocumentSession | `open` | yes | open |
| 48 | rust_method | DocumentSession | `close` | yes | close |
| 60 | rust_method | DocumentSession | `undo` | yes | undo |
| 66 | rust_method | DocumentSession | `redo` | yes | redo |
| 74 | rust_method | DocumentSession | `rotate` | yes | rotate |
| 85 | rust_method | DocumentSession | `has_unsaved_changes` | yes | hasUnsavedChanges |
| 95 | rust_method | DocumentSession | `patch_count` | yes | patchCount |
| 102 | rust_method | DocumentSession | `can_undo` | yes | canUndo |
| 108 | rust_method | DocumentSession | `can_redo` | yes | canRedo |
| 119 | rust_method | DocumentSession | `request_refresh` | yes | requestRefresh |
| 127 | rust_method | Default for DocumentSession | `default` |  |  |

### `crates/pdf-viewer-ui/src/document/document_types.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | rust_fn |  | `response_to_js` |  |  |
| 15 | rust_fn |  | `ok_response` | yes |  |
| 26 | rust_fn |  | `ok_empty` | yes |  |
| 36 | rust_fn |  | `err_response` | yes |  |

### `crates/pdf-viewer-ui/src/document/free_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_fn |  | `undo_document_pipeline` | yes | undo_document_pipeline |
| 21 | rust_fn |  | `redo_document_pipeline` | yes | redo_document_pipeline |
| 26 | rust_fn |  | `open_document_pipeline` | yes | open_document_pipeline |
| 33 | rust_fn |  | `pick_document_pipeline` | yes | pick_document_pipeline |
| 39 | rust_fn |  | `rotate_document_pipeline` | yes | rotate_document_pipeline |
| 45 | rust_fn |  | `close_document_pipeline` | yes | close_document_pipeline |
| 54 | rust_fn |  | `read_viewer_session` | yes | read_viewer_session |
| 60 | rust_fn |  | `get_viewer_session` | yes | get_viewer_session |
| 65 | rust_fn |  | `set_viewer_document` | yes | set_viewer_document |

### `crates/pdf-viewer-ui/src/document/history.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 3 | rust_fn |  | `undo_document_edit` | yes |  |
| 7 | rust_fn |  | `redo_document_edit` | yes |  |

### `crates/pdf-viewer-ui/src/document/host_pipeline.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 60 | rust_fn |  | `open_session_from_file_result` |  |  |
| 94 | rust_fn |  | `open_document_pipeline` | yes |  |
| 107 | rust_fn |  | `pick_document_pipeline` | yes |  |
| 125 | rust_fn |  | `close_document_pipeline` | yes |  |
| 138 | rust_fn |  | `rotate_document_pipeline` | yes |  |
| 145 | rust_fn |  | `undo_document_pipeline` | yes |  |
| 151 | rust_fn |  | `redo_document_pipeline` | yes |  |

### `crates/pdf-viewer-ui/src/document/io.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 20 | rust_fn |  | `open_pdf_file` | yes |  |
| 34 | rust_fn |  | `pick_pdf_file` | yes |  |
| 44 | rust_fn |  | `rotate_current_page` | yes |  |

### `crates/pdf-viewer-ui/src/document/mutation_pipeline.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 15 | rust_fn |  | `request_document_refresh` | yes |  |

### `crates/pdf-viewer-ui/src/document/patch_persistence.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 15 | rust_fn |  | `apply_document_patch_direct` | yes |  |
| 26 | rust_fn |  | `apply_document_patch` | yes |  |
| 47 | rust_fn |  | `has_persistable_patches` | yes |  |
| 51 | rust_fn |  | `collect_persistable_patches_js` | yes |  |
| 55 | rust_fn |  | `clear_persistable_patches` | yes |  |
| 60 | rust_fn |  | `save_persistable_patches` | yes |  |

### `crates/pdf-viewer-ui/src/document/review.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | rust_fn |  | `read_review_feed` | yes |  |

### `crates/pdf-viewer-ui/src/editor/activation.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 79 | rust_fn |  | `resolve_page_point_from_client` |  |  |
| 112 | rust_fn |  | `resolve_shell_center_page_point` |  |  |
| 119 | rust_fn |  | `point_in_bbox` |  |  |
| 126 | rust_fn |  | `resolve_target_at_page_point` |  |  |
| 163 | rust_fn |  | `activate_editor_from_client_point` | yes |  |
| 295 | rust_fn |  | `activate_region_editor` | yes |  |
| 304 | rust_fn |  | `move_caret_to_client_point` | yes |  |
| 382 | rust_fn |  | `save_editor_session` | yes |  |

### `crates/pdf-viewer-ui/src/editor/command.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 21 | rust_fn |  | `command_name` |  |  |
| 30 | rust_fn |  | `effective_editor_state` |  |  |
| 64 | rust_fn |  | `apply_editor_input_command` | yes |  |
| 68 | rust_fn |  | `apply_host_input` | yes |  |

### `crates/pdf-viewer-ui/src/editor/editor_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 71 | rust_method | EditorSession | `new` | yes | new |
| 80 | rust_method | EditorSession | `begin` | yes | begin |
| 97 | rust_method | EditorSession | `hit_test` | yes | hitTest |
| 149 | rust_method | EditorSession | `open_block` | yes | openBlock |
| 231 | rust_method | EditorSession | `move_caret` | yes | moveCaret |
| 276 | rust_method | EditorSession | `close_block` | yes | closeBlock |
| 300 | rust_method | EditorSession | `commit` | yes | commit |
| 352 | rust_method | EditorSession | `end` | yes | end |
| 368 | rust_method | EditorSession | `discard` | yes | discard |
| 399 | rust_method | EditorSession | `read_snapshot` | yes | readSnapshot |
| 419 | rust_method | EditorSession | `get_snapshot` | yes | getSnapshot |
| 425 | rust_method | EditorSession | `is_active` | yes | isActive |
| 434 | rust_method | EditorSession | `has_unsaved_changes` | yes | hasUnsavedChanges |
| 442 | rust_method | EditorSession | `sync_input` | yes | syncInput |
| 477 | rust_method | EditorSession | `apply_command` | yes | applyCommand |
| 532 | rust_method | EditorSession | `set_edit_mode` | yes | setEditMode |
| 558 | rust_method | EditorSession | `read_legacy_snapshot` | yes | readLegacySnapshot |
| 567 | rust_method | EditorSession | `paint_canvas` | yes | paintCanvas |
| 580 | rust_method | EditorSession | `utf16_to_char_index` | yes | utf16ToCharIndex |
| 587 | rust_method | EditorSession | `char_to_utf16_offset` | yes | charToUtf16Offset |
| 594 | rust_method | EditorSession | `has_session_changes` | yes | hasSessionChanges |
| 601 | rust_method | EditorSession | `open_region` | yes | openRegion |
| 681 | rust_method | EditorSession | `set_display_zoom` | yes | setDisplayZoom |
| 688 | rust_method | EditorSession | `read_diagnostics` | yes | readDiagnostics |
| 695 | rust_method | EditorSession | `save_session` | yes | saveSession |
| 704 | rust_method | EditorSession | `insert_text` | yes | insertText |
| 727 | rust_method | EditorSession | `delete_text` | yes | deleteText |
| 760 | rust_method | EditorSession | `apply_format` | yes | applyFormat |
| 795 | rust_method | EditorSession | `read_text_blocks` | yes | readTextBlocks |
| 818 | rust_method | EditorSession | `get_text_blocks` | yes | getTextBlocks |
| 824 | rust_method | EditorSession | `read_format_state` | yes | readFormatState |
| 834 | rust_method | EditorSession | `get_format_state` | yes | getFormatState |
| 844 | rust_method | EditorSession | `on_state_change` | yes | onStateChange |
| 865 | rust_method | EditorSession | `on_change` | yes | onChange |
| 886 | rust_method | EditorSession | `commit_draft_internal` |  |  |
| 892 | rust_fn |  | `build_frame_request` |  |  |
| 912 | rust_fn |  | `resolve_target_at_page_point` |  |  |
| 944 | rust_fn |  | `collect_text_blocks` |  |  |

### `crates/pdf-viewer-ui/src/editor/editor_controller.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 26 | rust_fn |  | `summarize_object_ids` |  |  |
| 47 | rust_fn |  | `collect_paragraph_targets` | yes |  |
| 52 | rust_fn |  | `open_editor_at_page_point` | yes |  |
| 211 | rust_fn |  | `build_region_text_patch` | yes |  |
| 230 | rust_fn |  | `open_region_editor` | yes |  |
| 281 | rust_fn |  | `build_active_editor_patch` | yes |  |
| 361 | rust_fn |  | `patch_is_noop` |  |  |
| 398 | rust_fn |  | `find_paragraph_shell_bbox` | yes |  |
| 402 | rust_fn |  | `set_editor_caret` | yes |  |
| 406 | rust_fn |  | `sync_editor_input` | yes |  |

### `crates/pdf-viewer-ui/src/editor/editor_format.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 40 | rust_fn |  | `build_active_editor_format_state` |  |  |
| 60 | rust_fn |  | `resolve_font_size_step` |  |  |
| 79 | rust_fn |  | `parse_alignment` |  |  |
| 89 | rust_fn |  | `parse_list_kind` |  |  |
| 100 | rust_fn |  | `toggle_active_editor_bold` | yes |  |
| 111 | rust_fn |  | `toggle_active_editor_italic` | yes |  |
| 122 | rust_fn |  | `toggle_active_editor_underline` | yes |  |
| 133 | rust_fn |  | `set_active_editor_color` | yes |  |
| 144 | rust_fn |  | `set_active_editor_font_family` | yes |  |
| 155 | rust_fn |  | `set_active_editor_font_size` | yes |  |
| 166 | rust_fn |  | `step_active_editor_font_size` | yes |  |
| 178 | rust_fn |  | `set_active_editor_char_spacing` | yes |  |
| 189 | rust_fn |  | `set_active_editor_line_height` | yes |  |
| 200 | rust_fn |  | `set_active_editor_paragraph_mode` | yes |  |
| 211 | rust_fn |  | `set_active_editor_alignment` | yes |  |
| 225 | rust_fn |  | `set_active_editor_list_kind` | yes |  |
| 239 | rust_fn |  | `active_editor_format_state` | yes |  |
| 249 | rust_fn |  | `apply_active_editor_format_action` | yes |  |

### `crates/pdf-viewer-ui/src/editor/editor_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 23 | rust_fn |  | `read_state` | yes |  |
| 27 | rust_fn |  | `set_state` | yes |  |
| 39 | rust_fn |  | `read_active_block_id` | yes |  |
| 43 | rust_fn |  | `set_active_block_id` | yes |  |
| 61 | rust_fn |  | `set_state_change_callback` | yes |  |
| 68 | rust_fn |  | `set_change_callback` | yes |  |
| 73 | rust_fn |  | `notify_state_change` |  |  |
| 85 | rust_fn |  | `notify_change` |  |  |
| 99 | rust_fn |  | `notify_state_change` |  |  |
| 102 | rust_fn |  | `notify_change` |  |  |
| 105 | rust_fn |  | `state_camel_case` |  |  |
| 119 | rust_fn |  | `transition_to_editing` | yes |  |
| 124 | rust_fn |  | `transition_to_editing_block` | yes |  |
| 130 | rust_fn |  | `transition_to_viewing` | yes |  |
| 136 | rust_fn |  | `transition_switch_block` | yes |  |
| 142 | rust_fn |  | `transition_to_saving` | yes |  |
| 147 | rust_fn |  | `transition_save_complete` | yes |  |

### `crates/pdf-viewer-ui/src/editor/editor_types.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | rust_fn |  | `response_to_js` |  |  |
| 12 | rust_fn |  | `ok_response` | yes |  |
| 23 | rust_fn |  | `ok_empty` | yes |  |
| 34 | rust_fn |  | `err_response` | yes |  |

### `crates/pdf-viewer-ui/src/editor/format/list_format.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 28 | rust_fn |  | `resolve_active_marker_text` | yes |  |
| 39 | rust_fn |  | `collect_marker_overrides` | yes |  |
| 57 | rust_fn |  | `reconcile_numbering_patches` | yes |  |
| 128 | rust_fn |  | `build_numbering_override_map` |  |  |
| 180 | rust_fn |  | `build_paragraph_list_context` |  |  |
| 237 | rust_fn |  | `resolve_symbolic_marker_text` |  |  |
| 254 | rust_fn |  | `collect_ordered_page_paragraphs` |  |  |
| 270 | rust_fn |  | `resolve_patch_for_base_paragraph` |  |  |

### `crates/pdf-viewer-ui/src/editor/format/text_geometry.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 21 | rust_fn |  | `measure_editor_layout_text_width` | yes |  |
| 103 | rust_fn |  | `create_measure_context` |  |  |
| 118 | rust_fn |  | `dedupe_caret_stops_local` |  |  |
| 123 | rust_fn |  | `convert_render_plan_caret_lines` |  |  |
| 145 | rust_fn |  | `build_unified_draft_caret_lines` |  |  |
| 157 | rust_fn |  | `resolve_caret_index_for_draft_point` |  |  |
| 238 | rust_fn |  | `build_draft_caret_lines` |  |  |
| 245 | rust_fn |  | `resolve_caret_visual_from_draft` |  |  |
| 282 | rust_fn |  | `active_caret_visual` | yes |  |
| 290 | rust_fn |  | `move_caret_by_key` | yes |  |
| 308 | rust_fn |  | `active_caret_index_at_page_point` | yes |  |
| 333 | rust_fn |  | `active_caret_index_at_shell_point` | yes |  |

### `crates/pdf-viewer-ui/src/editor/host_mode.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_fn |  | `toggle_text_edit_mode` | yes |  |
| 27 | rust_fn |  | `set_text_edit_mode` | yes |  |

### `crates/pdf-viewer-ui/src/editor/host_runtime.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_method | Default for EditorHostRuntimeState | `default` |  |  |
| 26 | rust_fn |  | `read_state` | yes |  |
| 30 | rust_fn |  | `reset_state` | yes |  |
| 36 | rust_fn |  | `set_display_zoom` | yes |  |
| 47 | rust_fn |  | `begin_commit` | yes |  |
| 59 | rust_fn |  | `finish_commit` | yes |  |

### `crates/pdf-viewer-ui/src/editor/host_snapshot.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 66 | rust_fn |  | `sanitize_projection_zoom` |  |  |
| 76 | rust_fn |  | `resolve_editor_projection_zoom` |  |  |
| 88 | rust_fn |  | `resolve_editor_host_snapshot` | yes |  |
| 125 | rust_fn |  | `resolve_active_editor_diagnostics` | yes |  |

### `crates/pdf-viewer-ui/src/editor/host_workflow.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | rust_fn |  | `move_caret_to_client_point` | yes |  |
| 15 | rust_fn |  | `save_editor_session` | yes |  |
| 19 | rust_fn |  | `read_paragraph_shell_bbox` | yes |  |

### `crates/pdf-viewer-ui/src/editor/mode.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | rust_fn |  | `read_active_edit_paragraph` | yes |  |
| 13 | rust_fn |  | `read_active_editor_target` | yes |  |
| 17 | rust_fn |  | `read_active_editor_state` | yes |  |
| 21 | rust_fn |  | `is_text_edit_mode_enabled` | yes |  |
| 25 | rust_fn |  | `set_text_edit_mode_enabled` | yes |  |
| 29 | rust_fn |  | `set_active_edit_paragraph` | yes |  |
| 33 | rust_fn |  | `close_active_editor` | yes |  |
| 39 | rust_fn |  | `reset_editor_mode` | yes |  |

### `crates/pdf-viewer-ui/src/editor/orchestrator/commit.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | rust_fn |  | `commit_pending_edit_if_any` | yes |  |
| 25 | rust_fn |  | `commit_active_editor_text` | yes |  |

### `crates/pdf-viewer-ui/src/editor/orchestrator/render_transaction.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 39 | rust_fn |  | `schedule_editor_render` |  |  |
| 46 | rust_fn |  | `schedule_editor_render_with_reason` |  |  |
| 60 | rust_fn |  | `open_editor_tx` | yes |  |
| 77 | rust_fn |  | `open_region_editor_tx` | yes |  |
| 91 | rust_fn |  | `sync_input_tx` | yes |  |
| 114 | rust_fn |  | `apply_input_tx` | yes |  |
| 128 | rust_fn |  | `apply_host_input_tx` | yes |  |
| 144 | rust_fn |  | `commit_editor_tx` | yes |  |
| 176 | rust_fn |  | `commit_editor_silent_tx` | yes |  |
| 202 | rust_fn |  | `close_editor_tx` | yes |  |
| 230 | rust_fn |  | `format_render_tx` |  |  |
| 240 | rust_fn |  | `apply_format_action_tx` | yes |  |

### `crates/pdf-viewer-ui/src/editor/orchestrator/replace_pipeline.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 31 | rust_fn |  | `apply_region_text_replacements_tx` | yes |  |

### `crates/pdf-viewer-ui/src/editor/overlay/navigation.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 5 | rust_fn |  | `execute_editor_navigation_key` | yes |  |

### `crates/pdf-viewer-ui/src/editor/overlay/paragraph_overlay.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 23 | rust_fn |  | `target_source_object_indices` |  |  |
| 27 | rust_fn |  | `persisted_patch_source_indices` |  |  |
| 41 | rust_fn |  | `collect_paragraph_render_overlays` | yes |  |
| 257 | rust_fn |  | `make_active_editor_target` |  |  |
| 278 | rust_fn |  | `make_glyph_plan` |  |  |
| 300 | rust_fn |  | `clear_state` |  |  |
| 318 | rust_fn |  | `patch_yields_overlay` |  |  |
| 390 | rust_fn |  | `commit_preserves_edit` |  |  |
| 434 | rust_fn |  | `skips_mismatched_page` |  |  |

### `crates/pdf-viewer-ui/src/editor/overlay/projection.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 53 | rust_fn |  | `project_paragraph_interaction_targets` | yes |  |
| 90 | rust_fn |  | `project_active_editor_shell` | yes |  |
| 127 | rust_fn |  | `sanitize_display_zoom` |  |  |

### `crates/pdf-viewer-ui/src/editor/overlay/visual.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 17 | rust_fn |  | `sanitize_projection_zoom` |  |  |
| 27 | rust_fn |  | `resolve_editor_projection_zoom` |  |  |
| 39 | rust_fn |  | `scene_shell_width` |  |  |
| 43 | rust_fn |  | `scene_shell_height` |  |  |
| 47 | rust_fn |  | `body_left_offset` |  |  |
| 52 | rust_fn |  | `body_top_offset` |  |  |
| 56 | rust_fn |  | `source_line_bbox_for_caret` |  |  |
| 63 | rust_fn |  | `resolve_caret_rect` |  |  |
| 80 | rust_fn |  | `render_active_editor_canvas` | yes |  |

### `crates/pdf-viewer-ui/src/editor/search_facade.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 85 | rust_fn |  | `build_frame_request` |  |  |
| 90 | rust_fn |  | `facade_search_page` | yes | searchFacadePage |
| 108 | rust_fn |  | `facade_search_document` | yes | searchFacadeDocument |
| 125 | rust_fn |  | `facade_replace` | yes | searchFacadeReplace |
| 150 | rust_fn |  | `facade_batch_replace` | yes | searchFacadeBatchReplace |
| 167 | rust_fn |  | `facade_set_session` | yes | searchFacadeSetSession |
| 194 | rust_fn |  | `facade_clear_session` | yes | searchFacadeClearSession |
| 205 | rust_fn |  | `facade_move_match` | yes | searchFacadeMoveMatch |
| 219 | rust_fn |  | `facade_get_session` | yes | searchFacadeGetSession |

### `crates/pdf-viewer-ui/src/editor/session/session.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 37 | rust_method | Default for EditorModeState | `default` |  |  |
| 51 | rust_fn |  | `reset_editor_mode` | yes |  |
| 57 | rust_fn |  | `is_text_edit_enabled` | yes |  |
| 61 | rust_fn |  | `set_text_edit_enabled` | yes |  |
| 81 | rust_fn |  | `active_edit_paragraph_id` | yes |  |
| 85 | rust_fn |  | `set_active_edit_paragraph` | yes |  |
| 95 | rust_fn |  | `active_editor_state` | yes |  |
| 104 | rust_fn |  | `starts_disabled` |  |  |
| 113 | rust_fn |  | `active_editor_target` | yes |  |
| 117 | rust_fn |  | `open_paragraph_editor` | yes |  |
| 201 | rust_fn |  | `close_active_editor` | yes |  |
| 209 | rust_fn |  | `active_editor_draft_text` | yes |  |
| 213 | rust_fn |  | `active_editor_has_session_changes` | yes |  |
| 223 | rust_fn |  | `active_editor_caret_index` | yes |  |
| 229 | rust_fn |  | `set_active_editor_caret_index` | yes |  |
| 258 | rust_fn |  | `sync_active_editor_input` | yes |  |
| 310 | rust_fn |  | `render_scene_key` | yes |  |

### `crates/pdf-viewer-ui/src/editor/workflow.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 14 | rust_fn |  | `build_paragraph_interaction_targets` | yes |  |
| 33 | rust_fn |  | `open_paragraph_editor` | yes |  |
| 54 | rust_fn |  | `resolve_paragraph_shell_bbox` | yes |  |
| 64 | rust_fn |  | `build_paragraph_patch` | yes |  |
| 80 | rust_fn |  | `build_region_text_patch` | yes |  |
| 105 | rust_fn |  | `build_active_editor_patch` | yes |  |

### `crates/pdf-viewer-ui/src/events.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 65 | rust_fn |  | `add_listener` | yes |  |
| 77 | rust_fn |  | `remove_listener` | yes |  |
| 96 | rust_fn |  | `emit` | yes |  |
| 114 | rust_fn |  | `clear_all` | yes |  |
| 122 | rust_fn |  | `listener_count` | yes |  |
| 135 | rust_fn |  | `add_listener` | yes |  |
| 138 | rust_fn |  | `remove_listener` | yes |  |
| 143 | rust_fn |  | `emit` | yes |  |
| 146 | rust_fn |  | `clear_all` | yes |  |
| 149 | rust_fn |  | `listener_count` | yes |  |

### `crates/pdf-viewer-ui/src/find/controller_facade.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | rust_fn |  | `facade_open` | yes | findControllerOpen |
| 17 | rust_fn |  | `facade_close` | yes | findControllerClose |
| 22 | rust_fn |  | `facade_toggle` | yes | findControllerToggle |
| 27 | rust_fn |  | `facade_set_result` | yes | findControllerSetResult |
| 37 | rust_fn |  | `facade_clear` | yes | findControllerClear |
| 42 | rust_fn |  | `facade_move_active` | yes | findControllerMoveActive |
| 47 | rust_fn |  | `facade_set_current_page` | yes | findControllerSetCurrentPage |
| 52 | rust_fn |  | `facade_get_toolbar_state` | yes | findControllerGetToolbarState |
| 57 | rust_fn |  | `facade_get_replace_requests` | yes | findControllerGetReplaceRequests |

### `crates/pdf-viewer-ui/src/find/find_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 24 | rust_method | FindSession | `new` | yes | new |
| 32 | rust_method | FindSession | `open` | yes | open |
| 38 | rust_method | FindSession | `close` | yes | close |
| 44 | rust_method | FindSession | `toggle` | yes | toggle |
| 50 | rust_method | FindSession | `clear` | yes | clear |
| 56 | rust_method | FindSession | `set_current_page` | yes | setCurrentPage |
| 66 | rust_method | FindSession | `read_state` | yes | readState |
| 72 | rust_method | FindSession | `get_state` | yes | getState |
| 78 | rust_method | Default for FindSession | `default` |  |  |

### `crates/pdf-viewer-ui/src/find/find_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 81 | rust_method | FindSessionState | `as_str` | yes |  |
| 90 | rust_method | FindSessionState | `derive` |  |  |
| 105 | rust_fn |  | `read_find_state` | yes |  |
| 198 | rust_fn |  | `open_find` | yes |  |
| 209 | rust_fn |  | `close_find` | yes |  |
| 219 | rust_fn |  | `toggle_find` | yes |  |
| 228 | rust_fn |  | `set_search_result` | yes |  |
| 260 | rust_fn |  | `clear_search` | yes |  |
| 269 | rust_fn |  | `move_active` | yes |  |
| 294 | rust_fn |  | `set_current_page` | yes |  |
| 302 | rust_fn |  | `read_toolbar_state` | yes |  |
| 309 | rust_fn |  | `build_replace_requests` | yes |  |
| 368 | rust_fn |  | `is_editable_kind` |  |  |
| 372 | rust_fn |  | `build_state_update` |  |  |
| 397 | rust_fn |  | `build_current_page_matches` |  |  |
| 421 | rust_fn |  | `build_toolbar_state` |  |  |

### `crates/pdf-viewer-ui/src/find/host_find_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 22 | rust_fn |  | `clear_find_session` | yes |  |
| 28 | rust_fn |  | `read_find_session` | yes |  |
| 32 | rust_fn |  | `set_find_session` | yes |  |
| 60 | rust_fn |  | `move_find_match` | yes |  |
| 81 | rust_fn |  | `resolve_initial_active_index` |  |  |
| 91 | rust_fn |  | `wrapped_between` |  |  |

### `crates/pdf-viewer-ui/src/geometry_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 64 | rust_method | GeometryApi | `new` | yes | new |
| 75 | rust_method | GeometryApi | `client_to_page` | yes | clientToPage |
| 94 | rust_method | GeometryApi | `page_to_client` | yes | pageToClient |
| 110 | rust_method | GeometryApi | `page_to_raw` | yes | pageToRaw |
| 121 | rust_method | GeometryApi | `raw_to_page` | yes | rawToPage |
| 136 | rust_method | GeometryApi | `client_to_raw` | yes | clientToRaw |
| 157 | rust_method | GeometryApi | `measure_scale` | yes | measureScale |
| 171 | rust_method | GeometryApi | `project_rect` | yes | projectRect |
| 189 | rust_method | Default for GeometryApi | `default` |  |  |
| 196 | rust_fn |  | `build_transform` |  |  |

### `crates/pdf-viewer-ui/src/host/command.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 26 | rust_fn |  | `open_document_session` | yes |  |
| 43 | rust_fn |  | `reset_host_document_session` | yes |  |
| 58 | rust_fn |  | `navigate_prev_page` | yes |  |
| 76 | rust_fn |  | `navigate_next_page` | yes |  |
| 94 | rust_fn |  | `apply_zoom_selection` | yes |  |

### `crates/pdf-viewer-ui/src/host/layout.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 39 | rust_fn |  | `sync_host_layout` | yes |  |

### `crates/pdf-viewer-ui/src/host/scroll.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | rust_fn |  | `resolve_host_scroll_refresh` | yes |  |

### `crates/pdf-viewer-ui/src/lib.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 37 | rust_fn |  | `start` | yes | start |

### `crates/pdf-viewer-ui/src/page/context.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | rust_fn |  | `init_page_context_from_models` | yes |  |
| 33 | rust_fn |  | `update_page_viewport_workflow` | yes |  |

### `crates/pdf-viewer-ui/src/page/page_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_fn |  | `with_page_state` | yes |  |
| 20 | rust_fn |  | `with_page_and_scene` | yes |  |
| 24 | rust_fn |  | `with_progressive_task_mut` | yes |  |
| 30 | rust_fn |  | `set_progressive_task` | yes |  |
| 36 | rust_fn |  | `init_page_context` | yes |  |
| 71 | rust_fn |  | `update_page_viewport` | yes |  |
| 103 | rust_fn |  | `reset_progressive_render_task` | yes |  |

### `crates/pdf-viewer-ui/src/present/facade.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | rust_fn |  | `resolve_render_zoom` | yes |  |
| 14 | rust_fn |  | `resolve_frame_plan` | yes |  |
| 19 | rust_fn |  | `resolve_viewport_layout` | yes |  |
| 29 | rust_fn |  | `resolve_viewport_tile` | yes |  |

### `crates/pdf-viewer-ui/src/present/plan_builder.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_fn |  | `resolve_anchor_layout_from_zoom_state` |  |  |
| 54 | rust_fn |  | `build_frame_plan_result` | yes |  |

### `crates/pdf-viewer-ui/src/present/present_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 34 | rust_fn |  | `with_present_state` | yes |  |
| 38 | rust_fn |  | `build_frame_plan_result` | yes |  |
| 57 | rust_fn |  | `resolve_viewport_refresh` | yes |  |
| 64 | rust_fn |  | `touch_frame_cache_entry` | yes |  |
| 69 | rust_fn |  | `store_frame_cache_entry` | yes |  |
| 74 | rust_fn |  | `reset_frame_cache` | yes |  |
| 80 | rust_fn |  | `reset_present_runtime` | yes |  |
| 96 | rust_fn |  | `schedule_render_frame_request` | yes |  |
| 131 | rust_fn |  | `commit_render_frame` | yes |  |
| 148 | rust_fn |  | `settle_render_frame` | yes |  |

### `crates/pdf-viewer-ui/src/presentation/page_turn.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 45 | rust_method | Default for PageTurnSnapshot | `default` |  |  |
| 122 | rust_fn |  | `read_page_turn_snapshot` | yes |  |
| 126 | rust_fn |  | `reset_page_turn_state` | yes |  |
| 132 | rust_fn |  | `request_page_turn` | yes |  |
| 202 | rust_fn |  | `is_latest_page_turn` | yes |  |
| 209 | rust_fn |  | `mark_page_visible` | yes |  |
| 241 | rust_fn |  | `can_prefetch` | yes |  |
| 249 | rust_fn |  | `admit_page_asset` | yes |  |
| 302 | rust_fn |  | `decide_adjacent_prefetch` | yes |  |
| 349 | rust_fn |  | `reject` |  |  |
| 370 | rust_fn |  | `normalize_reason` |  |  |
| 377 | rust_fn |  | `normalize_surface` |  |  |
| 384 | rust_fn |  | `normalize_role` |  |  |
| 391 | rust_fn |  | `normalize_asset_kind` |  |  |
| 399 | rust_fn |  | `visible_phase` |  |  |
| 409 | rust_fn |  | `phase_allows_prefetch` |  |  |
| 420 | rust_fn |  | `current_asset_priority` |  |  |
| 431 | rust_fn |  | `prefetch_priority` |  |  |
| 440 | rust_fn |  | `prefetch_window_for_asset` |  |  |
| 459 | rust_fn |  | `push_prefetch_runway` |  |  |
| 514 | rust_fn |  | `push_prefetch_candidate` |  |  |
| 535 | rust_fn |  | `reject_prefetch` |  |  |
| 550 | rust_fn |  | `resolve_direction` |  |  |
| 560 | rust_fn |  | `emit_decision` |  |  |
| 567 | rust_fn |  | `emit_visible` |  |  |
| 581 | rust_fn |  | `reset_with_document` |  |  |
| 589 | rust_fn |  | `rejects_stale_page` |  |  |
| 616 | rust_fn |  | `prefers_turn_direction` |  |  |
| 642 | rust_fn |  | `rejects_stale_assets` |  |  |
| 678 | rust_fn |  | `rejects_invalid_turn` |  |  |
| 695 | rust_fn |  | `activates_fast_flip` |  |  |
| 716 | rust_fn |  | `throttles_fast_flip` |  |  |
| 749 | rust_fn |  | `normal_mode_includes_vector_prefetch` |  |  |

### `crates/pdf-viewer-ui/src/presentation/presentation_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_method | PagePresentationRuntime | `new` | yes | new |
| 21 | rust_method | PagePresentationRuntime | `request_page_turn` | yes | requestPageTurn |
| 26 | rust_method | PagePresentationRuntime | `read_page_turn` | yes | readPageTurn |
| 31 | rust_method | PagePresentationRuntime | `is_latest_page_turn` | yes | isLatestPageTurn |
| 36 | rust_method | PagePresentationRuntime | `mark_page_visible` | yes | markPageVisible |
| 41 | rust_method | PagePresentationRuntime | `can_prefetch` | yes | canPrefetch |
| 46 | rust_method | PagePresentationRuntime | `admit_page_asset` | yes | admitPageAsset |
| 51 | rust_method | PagePresentationRuntime | `decide_adjacent_prefetch` | yes | decideAdjacentPrefetch |
| 56 | rust_method | PagePresentationRuntime | `resolve_render_queue_action` | yes | resolveRenderQueueAction |
| 73 | rust_method | PagePresentationRuntime | `reset` | yes | reset |
| 79 | rust_method | Default for PagePresentationRuntime | `default` |  |  |

### `crates/pdf-viewer-ui/src/presentation/render_queue.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 17 | rust_fn |  | `resolve_render_queue_action` | yes |  |
| 76 | rust_fn |  | `normalize_render_source` |  |  |
| 88 | rust_fn |  | `suppresses_scroll_immediately_after_commit` |  |  |
| 97 | rust_fn |  | `dispatches_when_idle` |  |  |
| 104 | rust_fn |  | `replaces_navigation_while_executing` |  |  |
| 111 | rust_fn |  | `replaces_non_navigation_while_executing` |  |  |

### `crates/pdf-viewer-ui/src/projection_workflow.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 14 | rust_fn |  | `resolve_font_face` | yes |  |
| 23 | rust_fn |  | `build_editable_segments` | yes |  |
| 28 | rust_fn |  | `resolve_editor_projection` | yes |  |
| 44 | rust_fn |  | `build_pagination_commands` | yes |  |
| 61 | rust_fn |  | `build_page_region_context` | yes |  |
| 66 | rust_fn |  | `project_page_rect` | yes |  |
| 82 | rust_fn |  | `measure_dom_to_page_scale` | yes |  |
| 92 | rust_fn |  | `resolve_page_point` | yes |  |

### `crates/pdf-viewer-ui/src/render/canvas_overlay.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | rust_fn |  | `path_bbox_summary` | yes |  |
| 36 | rust_fn |  | `summarize_overlay_render_plan` |  |  |
| 76 | rust_fn |  | `count_overlay_underline_runs` |  |  |
| 87 | rust_fn |  | `draw_editor_marker_page` | yes |  |
| 187 | rust_fn |  | `draw_active_editor_shell_overlay_page` | yes |  |
| 255 | rust_fn |  | `draw_persisted_paragraph_overlay_page` | yes |  |

### `crates/pdf-viewer-ui/src/render/canvas.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 46 | rust_fn |  | `render_run_standalone` | yes | render_run_standalone |
| 87 | rust_fn |  | `current_time_ms` |  |  |
| 91 | rust_fn |  | `active_shell_bbox_for_debug` |  |  |
| 96 | rust_fn |  | `debug_bbox_intersects_active_shell` |  |  |
| 102 | rust_fn |  | `debug_log_canvas_method` |  |  |
| 143 | rust_method | CanvasRenderer | `new_overlay` | yes |  |
| 166 | rust_method | CanvasRenderer | `new_hijacked` | yes |  |
| 192 | rust_method | CanvasRenderer | `new_offscreen` | yes |  |
| 211 | rust_method | CanvasRenderer | `sync_size` | yes |  |
| 235 | rust_method | CanvasRenderer | `measure_text_metrics` | yes |  |
| 268 | rust_method | CanvasRenderer | `draw_text_run` | yes |  |
| 302 | rust_method | CanvasRenderer | `clear_dirty_rect` | yes |  |
| 328 | rust_method | CanvasRenderer | `prepare_page_surface` |  |  |
| 351 | rust_method | CanvasRenderer | `apply_page_transform` |  |  |
| 365 | rust_method | CanvasRenderer | `draw_vector_object` |  |  |
| 552 | rust_method | CanvasRenderer | `render_vector_slice` | yes |  |
| 651 | rust_method | CanvasRenderer | `render_page` | yes |  |
| 863 | rust_method | PdfRenderer for CanvasRenderer | `render` |  |  |
| 950 | rust_method | PdfRenderer for CanvasRenderer | `clear` |  |  |
| 967 | rust_method | PdfRenderer for CanvasRenderer | `name` |  |  |
| 972 | rust_fn |  | `draw_text_run_core` | yes |  |

### `crates/pdf-viewer-ui/src/render/commit.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_fn |  | `commit_render_result` | yes |  |

### `crates/pdf-viewer-ui/src/render/free_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 64 | rust_fn |  | `resolve_frame_plan` | yes | resolve_frame_plan |
| 70 | rust_fn |  | `take_frame_plan` | yes | take_frame_plan |
| 78 | rust_fn |  | `schedule_render_frame` | yes | schedule_render_frame |
| 87 | rust_fn |  | `commit_render_result` | yes | commit_render_result |
| 103 | rust_fn |  | `settle_render_frame` | yes | settle_render_frame |
| 108 | rust_fn |  | `abort_render_frame` | yes | abort_render_frame |
| 113 | rust_fn |  | `is_render_frame_current` | yes | is_render_frame_current |
| 118 | rust_fn |  | `schedule_render_follow_up` | yes | schedule_render_follow_up |
| 129 | rust_fn |  | `queue_render_loop_frame` | yes | queue_render_loop_frame |
| 138 | rust_fn |  | `advance_render_loop_frame` | yes | advance_render_loop_frame |
| 149 | rust_fn |  | `step_zoom_frame_plan` | yes | step_zoom_frame_plan |
| 155 | rust_fn |  | `resolve_viewport_refresh` | yes | resolve_viewport_refresh |
| 161 | rust_fn |  | `resolve_host_scroll_refresh` | yes | resolve_host_scroll_refresh |
| 167 | rust_fn |  | `clear_zoom_preview_host_state` | yes | clear_zoom_preview_host_state |
| 172 | rust_fn |  | `resolve_wheel_render_decision` | yes | resolve_wheel_render_decision |
| 178 | rust_fn |  | `resolve_preview_tick_decision` | yes | resolve_preview_tick_decision |
| 184 | rust_fn |  | `handle_wheel_zoom_host` | yes | handle_wheel_zoom_host |
| 190 | rust_fn |  | `step_preview_host` | yes | step_preview_host |
| 198 | rust_fn |  | `resolve_render_execution_plan` | yes | resolve_render_execution_plan |
| 208 | rust_fn |  | `resolve_layer_execution_plan` | yes | resolve_layer_execution_plan |
| 218 | rust_fn |  | `resolve_layer_present_decision` | yes | resolve_layer_present_decision |
| 230 | rust_fn |  | `update_page_viewport` | yes | update_page_viewport |
| 251 | rust_fn |  | `render_page` | yes | render_page |
| 256 | rust_fn |  | `render_page_offscreen` | yes | render_page_offscreen |
| 261 | rust_fn |  | `start_progressive_render` | yes | start_progressive_render |
| 266 | rust_fn |  | `step_progressive_render` | yes | step_progressive_render |
| 282 | rust_fn |  | `step_progressive_render_offscreen` | yes | step_progressive_render_offscreen |
| 300 | rust_fn |  | `cancel_progressive_render` | yes | cancel_progressive_render |
| 305 | rust_fn |  | `resolve_progressive_render_policy` | yes | resolve_progressive_render_policy |
| 313 | rust_fn |  | `touch_frame_cache_entry` | yes | touch_frame_cache_entry |
| 319 | rust_fn |  | `store_frame_cache_entry` | yes | store_frame_cache_entry |
| 324 | rust_fn |  | `reset_frame_cache` | yes | reset_frame_cache |
| 331 | rust_fn |  | `set_wheel_render_pending` | yes | set_wheel_render_pending |
| 336 | rust_fn |  | `get_wheel_render_pending` | yes | get_wheel_render_pending |
| 341 | rust_fn |  | `queue_committed_frame` | yes | queue_committed_frame |
| 348 | rust_fn |  | `take_ready_committed_frame` | yes | take_ready_committed_frame |

### `crates/pdf-viewer-ui/src/render/host_runtime.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_fn |  | `queue_render_loop_frame` | yes |  |
| 33 | rust_fn |  | `advance_render_loop_frame` | yes |  |
| 49 | rust_fn |  | `reset_render_loop_runtime` | yes |  |

### `crates/pdf-viewer-ui/src/render/loop_workflow.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 8 | rust_fn |  | `resolve_render_follow_up_runtime` | yes |  |
| 15 | rust_fn |  | `schedule_render_follow_up_runtime` | yes |  |

### `crates/pdf-viewer-ui/src/render/progressive_workflow.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 18 | rust_fn |  | `start_progressive_render` | yes |  |
| 66 | rust_fn |  | `step_progressive_render` | yes |  |
| 119 | rust_fn |  | `cancel_progressive_render` | yes |  |
| 123 | rust_fn |  | `render_page` | yes |  |
| 142 | rust_fn |  | `render_page_offscreen` | yes |  |
| 161 | rust_fn |  | `step_progressive_render_offscreen` | yes |  |

### `crates/pdf-viewer-ui/src/render/render_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | rust_fn |  | `reset_render_state` | yes |  |
| 17 | rust_fn |  | `is_render_frame_current` | yes |  |
| 24 | rust_fn |  | `schedule_render_frame` | yes |  |
| 82 | rust_fn |  | `settle_render_frame` | yes |  |

### `crates/pdf-viewer-ui/src/render/wasm_facade.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 32 | rust_fn |  | `stub` |  |  |
| 43 | rust_fn |  | `facade_start_progressive` | yes | renderFacadeStartProgressive |
| 48 | rust_fn |  | `facade_step_progressive` | yes | renderFacadeStepProgressive |
| 64 | rust_fn |  | `facade_cancel_progressive` | yes | renderFacadeCancelProgressive |
| 69 | rust_fn |  | `facade_render_page` | yes | renderFacadeRenderPage |
| 76 | rust_fn |  | `facade_commit_result` | yes | renderFacadeCommitResult |
| 92 | rust_fn |  | `facade_abort_frame` | yes | renderFacadeAbortFrame |
| 98 | rust_fn |  | `facade_is_frame_current` | yes | renderFacadeIsFrameCurrent |
| 105 | rust_fn |  | `facade_touch_cache` | yes | renderFacadeTouchCache |
| 110 | rust_fn |  | `facade_store_cache` | yes | renderFacadeStoreCache |
| 115 | rust_fn |  | `facade_reset_cache` | yes | renderFacadeResetCache |
| 123 | rust_fn |  | `facade_snapshot_png` | yes | renderFacadeSnapshotPng |
| 129 | rust_fn |  | `facade_prewarm_cache` | yes | renderFacadePrewarmCache |
| 135 | rust_fn |  | `facade_set_quality` | yes | renderFacadeSetQuality |
| 141 | rust_fn |  | `facade_set_debug_overlay` | yes | renderFacadeSetDebugOverlay |

### `crates/pdf-viewer-ui/src/render/workflow.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 17 | rust_fn |  | `settle_render_frame_inner` | yes |  |

### `crates/pdf-viewer-ui/src/review/review_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 36 | rust_method | ReviewSessionState | `as_str` | yes |  |
| 43 | rust_method | ReviewSessionState | `derive` |  |  |
| 53 | rust_fn |  | `read_review_state` | yes |  |
| 65 | rust_method | ReviewSession | `new` | yes | new |
| 71 | rust_method | ReviewSession | `read_feed` | yes | readFeed |
| 77 | rust_method | ReviewSession | `accept` | yes | accept |
| 83 | rust_method | ReviewSession | `reject` | yes | reject |
| 89 | rust_method | ReviewSession | `accept_all` | yes | acceptAll |
| 95 | rust_method | ReviewSession | `reject_all` | yes | rejectAll |
| 104 | rust_method | ReviewSession | `read_state` | yes | readState |
| 110 | rust_method | ReviewSession | `get_state` | yes | getState |
| 116 | rust_method | Default for ReviewSession | `default` |  |  |

### `crates/pdf-viewer-ui/src/review/review_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 20 | rust_fn |  | `clear_comment_review_session` | yes |  |
| 24 | rust_fn |  | `read_comment_review_session` | yes |  |
| 28 | rust_fn |  | `replace_comment_review_session` |  |  |
| 35 | rust_fn |  | `update_comment_review_session` |  |  |
| 43 | rust_fn |  | `set_comment_review_panel_open` | yes |  |
| 49 | rust_fn |  | `toggle_comment_review_panel` | yes |  |
| 55 | rust_fn |  | `set_comment_review_scope` | yes |  |
| 61 | rust_fn |  | `set_comment_review_query` | yes |  |
| 67 | rust_fn |  | `select_comment_review_comment` | yes |  |

### `crates/pdf-viewer-ui/src/ui_state_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | rust_fn |  | `read_patch_state` | yes |  |
| 15 | rust_fn |  | `current_patch_revision` | yes |  |
| 22 | rust_fn |  | `current_paragraph_patch_text` | yes |  |
| 29 | rust_fn |  | `current_paragraph_patch` | yes |  |
| 36 | rust_fn |  | `remember_paragraph_replacement_target` | yes |  |
| 43 | rust_fn |  | `record_patch` | yes |  |
| 70 | rust_fn |  | `collect_persistable_patches` | yes |  |
| 81 | rust_fn |  | `clear_persistable_patches` | yes |  |
| 100 | rust_fn |  | `can_undo` | yes |  |
| 107 | rust_fn |  | `can_redo` | yes |  |
| 114 | rust_fn |  | `undo_depth` | yes |  |
| 121 | rust_fn |  | `redo_depth` | yes |  |
| 133 | rust_fn |  | `clear_history_stacks` | yes |  |
| 139 | rust_fn |  | `undo` | yes |  |
| 154 | rust_fn |  | `redo` | yes |  |
| 165 | rust_fn |  | `collect_review_changes` | yes |  |
| 192 | rust_fn |  | `reject_review_change` | yes |  |
| 238 | rust_fn |  | `accept_review_change` | yes |  |
| 255 | rust_fn |  | `accept_all_review_changes` | yes |  |
| 279 | rust_fn |  | `reject_all_review_changes` | yes |  |

### `crates/pdf-viewer-ui/src/utils/chain_trace.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 34 | rust_fn |  | `set_chain_trace_enabled` | yes |  |
| 38 | rust_fn |  | `is_chain_trace_enabled` | yes |  |
| 43 | rust_fn |  | `trace_step` | yes |  |

### `crates/pdf-viewer-ui/src/viewer/free_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_fn |  | `init_page_context` | yes | init_page_context |
| 70 | rust_fn |  | `set_current_page` | yes | set_current_page |
| 78 | rust_fn |  | `dump_editor_debug_trace` | yes | dump_editor_debug_trace |

### `crates/pdf-viewer-ui/src/viewer/viewer_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 26 | rust_method | ViewerSession | `new` | yes | new |
| 32 | rust_method | ViewerSession | `read` | yes | read |
| 38 | rust_method | ViewerSession | `set_document` | yes | setDocument |
| 44 | rust_method | ViewerSession | `reset` | yes | reset |
| 50 | rust_method | ViewerSession | `set_current_page` | yes | setCurrentPage |
| 56 | rust_method | ViewerSession | `set_current_zoom` | yes | setCurrentZoom |
| 62 | rust_method | ViewerSession | `set_page_dimensions` | yes | setPageDimensions |
| 71 | rust_method | ViewerSession | `read_state` | yes | readState |
| 77 | rust_method | ViewerSession | `get_state` | yes | getState |
| 97 | rust_method | ViewerSession | `set_state` | yes | setState |
| 144 | rust_method | Default for ViewerSession | `default` |  |  |

### `crates/pdf-viewer-ui/src/viewer/viewer_controller.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 28 | rust_fn |  | `reset_viewer_runtime` | yes |  |
| 47 | rust_fn |  | `reset_session` | yes |  |
| 57 | rust_fn |  | `reset_zoom_view` | yes |  |
| 67 | rust_fn |  | `note_document_mutation` | yes |  |
| 77 | rust_fn |  | `read_session` | yes |  |
| 81 | rust_fn |  | `set_document` | yes |  |
| 91 | rust_fn |  | `set_page` | yes |  |
| 107 | rust_fn |  | `set_zoom` | yes |  |
| 117 | rust_fn |  | `set_page_size` | yes |  |

### `crates/pdf-viewer-ui/src/viewer/viewer_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 26 | rust_method | ViewerSessionState | `as_str` | yes |  |
| 35 | rust_fn |  | `read_viewer_state` | yes |  |
| 50 | rust_fn |  | `reset_viewer_session` | yes |  |
| 56 | rust_fn |  | `set_viewer_document` | yes |  |
| 67 | rust_fn |  | `set_current_page` | yes |  |
| 73 | rust_fn |  | `set_current_zoom` | yes |  |
| 79 | rust_fn |  | `set_zoom_and_page_dimensions` | yes |  |
| 92 | rust_fn |  | `read_viewer_session` | yes |  |
| 96 | rust_fn |  | `set_page_dimensions` | yes |  |
| 104 | rust_fn |  | `bump_document_revision` | yes |  |
| 112 | rust_fn |  | `current_document_revision` | yes |  |
| 116 | rust_fn |  | `sanitize_zoom` |  |  |

### `crates/pdf-viewer-ui/src/zoom/event.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 42 | rust_fn |  | `execute_wheel_zoom` | yes |  |
| 65 | rust_fn |  | `step_preview_host` | yes |  |

### `crates/pdf-viewer-ui/src/zoom/free_api.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 14 | rust_fn |  | `resolve_wheel_zoom` | yes | resolve_wheel_zoom |
| 21 | rust_fn |  | `reset_zoom_state` | yes | reset_zoom_state |
| 26 | rust_fn |  | `read_zoom_state` | yes | read_zoom_state |
| 32 | rust_fn |  | `get_zoom_state` | yes | get_zoom_state |
| 37 | rust_fn |  | `set_target_zoom` | yes | set_target_zoom |
| 42 | rust_fn |  | `mark_rendered_zoom` | yes | mark_rendered_zoom |
| 47 | rust_fn |  | `clear_pending_anchor` | yes | clear_pending_anchor |
| 52 | rust_fn |  | `apply_zoom_selection` | yes | apply_zoom_selection |

### `crates/pdf-viewer-ui/src/zoom/preview_host.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | rust_fn |  | `reset_zoom_preview_host` | yes |  |
| 11 | rust_fn |  | `clear_zoom_preview_host_state` | yes |  |
| 21 | rust_fn |  | `settle_zoom_preview_at_target` | yes |  |
| 39 | rust_fn |  | `set_wheel_render_pending` | yes |  |
| 45 | rust_fn |  | `set_preview_active` | yes |  |
| 51 | rust_fn |  | `is_preview_active` | yes |  |
| 55 | rust_fn |  | `is_wheel_render_pending` | yes |  |
| 59 | rust_fn |  | `queue_committed_frame` | yes |  |
| 65 | rust_fn |  | `take_ready_committed_frame` | yes |  |

### `crates/pdf-viewer-ui/src/zoom/request.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 8 | rust_fn |  | `resolve_wheel_zoom` | yes |  |
| 28 | rust_fn |  | `resolve_anchor_scroll` | yes |  |

### `crates/pdf-viewer-ui/src/zoom/zoom_controller.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_fn |  | `reset_zoom_runtime` | yes |  |
| 22 | rust_fn |  | `read_zoom_state` | yes |  |
| 26 | rust_fn |  | `set_target_zoom` | yes |  |
| 39 | rust_fn |  | `mark_rendered_zoom` | yes |  |
| 45 | rust_fn |  | `step_zoom_animation` | yes |  |
| 52 | rust_fn |  | `step_zoom_frame_plan` | yes |  |
| 71 | rust_fn |  | `take_pending_anchor_scroll` | yes |  |
| 95 | rust_fn |  | `peek_pending_anchor_scroll` | yes |  |
| 119 | rust_fn |  | `peek_pending_anchor_layout` | yes |  |
| 143 | rust_fn |  | `take_pending_anchor_layout` | yes |  |
| 167 | rust_fn |  | `clear_pending_anchor` | yes |  |
| 173 | rust_fn |  | `set_visual_layout` | yes |  |
| 196 | rust_fn |  | `clear_preview_present` | yes |  |

### `crates/pdf-viewer-ui/src/zoom/zoom_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 29 | rust_method | ZoomSessionState | `as_str` | yes |  |
| 39 | rust_fn |  | `read_zoom_session_state` | yes |  |
| 57 | rust_fn |  | `read_zoom_state` | yes |  |
| 61 | rust_fn |  | `with_zoom_state` | yes |  |
| 65 | rust_fn |  | `with_zoom_state_mut` | yes |  |
| 69 | rust_fn |  | `reset_zoom_state` | yes |  |
| 86 | rust_fn |  | `sanitize_zoom` |  |  |

### `scratch/src/main.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 4 | rust_fn |  | `main` |  |  |

### `scripts/dev.mjs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | function |  | `checkPort` |  |  |
| 22 | function |  | `findFreePort` |  |  |
| 36 | function |  | `run` |  |  |
| 43 | arrow_fn |  | `cleanup` |  |  |
| 114 | arrow_fn |  | `handleExit` |  |  |

### `scripts/generate-method-inventory.mjs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | function |  | `toPosix` |  |  |
| 13 | function |  | `rel` |  |  |
| 17 | function |  | `shouldSkipDir` |  |  |
| 24 | function |  | `collectFiles` |  |  |
| 39 | function |  | `countChar` |  |  |
| 45 | function |  | `isSnake` |  |  |
| 49 | function |  | `isCamel` |  |  |
| 53 | function |  | `isPascal` |  |  |
| 57 | function |  | `isCamelOrPascal` |  |  |
| 61 | function |  | `snakeParts` |  |  |
| 65 | function |  | `nameParts` |  |  |
| 75 | function |  | `isTestPath` |  |  |
| 79 | function |  | `nameComplexity` |  |  |
| 88 | function |  | `isLongOrSentenceLike` |  |  |
| 93 | function |  | `readAttributeBlockEndingAt` |  |  |
| 102 | function |  | `leadingRustAttributes` |  |  |
| 131 | function |  | `extractRust` |  |  |
| 203 | function |  | `extractTs` |  |  |
| 303 | function |  | `extractItems` |  |  |
| 309 | function |  | `extractRustTypes` |  |  |
| 333 | function |  | `extractTsTypes` |  |  |
| 357 | function |  | `extractTypeItems` |  |  |
| 363 | function |  | `mdEscape` |  |  |
| 367 | function |  | `groupBy` |  |  |
| 578 | function |  | `emitFindingTable` |  |  |

### `src-tauri/build.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 1 | rust_fn |  | `main` |  |  |

### `src-tauri/src/app_state.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 36 | rust_method | DocumentStore | `new` |  |  |
| 58 | rust_method | CacheStore | `new` |  |  |
| 78 | rust_method | HistoryStore | `new` |  |  |
| 92 | rust_method | RendererState | `new` |  |  |
| 109 | rust_method | AppState | `new` | yes |  |
| 121 | rust_method | Default for AppState | `default` |  |  |

### `src-tauri/src/application/pdf/comment_review.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | rust_fn |  | `review_document_comments` | yes |  |
| 81 | rust_fn |  | `review_request_defaults_to_document_scope` |  |  |

### `src-tauri/src/application/pdf/page_annotation.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 55 | rust_fn |  | `default_highlight_color` |  |  |
| 58 | rust_fn |  | `collect_page_annotation_targets` | yes |  |
| 124 | rust_fn |  | `list_page_annotation_targets` | yes |  |
| 145 | rust_fn |  | `list_page_highlights` | yes |  |
| 192 | rust_fn |  | `list_page_comments` | yes |  |
| 240 | rust_fn |  | `add_region_highlight` | yes |  |
| 293 | rust_fn |  | `add_region_comment` | yes |  |
| 352 | rust_fn |  | `delete_page_annotation` | yes |  |
| 373 | rust_fn |  | `update_page_comment` | yes |  |
| 399 | rust_fn |  | `resolve_region_box` |  |  |
| 424 | rust_fn |  | `from_region_box` |  |  |
| 432 | rust_fn |  | `summarize_label` |  |  |
| 447 | rust_fn |  | `parse_annotation_object_id` |  |  |

### `src-tauri/src/application/pdf/page_asset.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | rust_method | PageAssetRole | `from_request` | yes |  |
| 19 | rust_method | PageAssetRole | `as_str` | yes |  |
| 34 | rust_fn |  | `waits_for_inflight_key` |  |  |
| 98 | rust_fn |  | `separates_revision_locks` |  |  |
| 130 | rust_fn |  | `clears_asset_locks` |  |  |
| 199 | rust_fn |  | `widens_preview_runway` |  |  |
| 247 | rust_method | PageAssetKind | `as_str` |  |  |
| 260 | rust_method | PageAssetAdmissionService | `set_test_delay_ms` | yes |  |
| 267 | rust_method | PageAssetAdmissionService | `apply_test_delay` | yes |  |
| 277 | rust_method | PageAssetAdmissionService | `emit_event` |  |  |
| 281 | rust_method | PageAssetAdmissionService | `lock_for` |  |  |
| 302 | rust_method | PageAssetAdmissionService | `acquire_inflight_lock` | yes |  |
| 342 | rust_method | PageAssetAdmissionService | `admit_before_work` | yes |  |
| 360 | rust_method | PageAssetAdmissionService | `mark_current_page` | yes |  |
| 390 | rust_method | PageAssetAdmissionService | `admit_after_wait` | yes |  |
| 400 | rust_method | PageAssetAdmissionService | `admit_after_work` | yes |  |
| 422 | rust_method | PageAssetAdmissionService | `admit_prefetch` |  |  |
| 471 | rust_method | PageAssetAdmissionService | `reject` |  |  |

### `src-tauri/src/application/pdf/page_context.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | rust_fn |  | `build_page_region_context_from_vector_model` | yes |  |
| 16 | rust_fn |  | `native_page_from_vector_model` | yes |  |
| 33 | rust_fn |  | `native_text_from_vector_text` |  |  |

### `src-tauri/src/application/pdf/page_search.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 57 | rust_fn |  | `search_page_regions` | yes |  |
| 102 | rust_fn |  | `search_document_regions` | yes |  |
| 144 | rust_fn |  | `search_page_matches` |  |  |
| 182 | rust_fn |  | `collect_paragraph_matches` |  |  |
| 209 | rust_fn |  | `collect_list_item_matches` |  |  |
| 237 | rust_fn |  | `collect_field_row_matches` |  |  |
| 275 | rust_fn |  | `contains_query` |  |  |
| 282 | rust_fn |  | `summarize_preview` |  |  |
| 291 | rust_fn |  | `from_region_box` |  |  |

### `src-tauri/src/error.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 28 | rust_fn |  | `read_metadata` | yes |  |
| 37 | rust_fn |  | `read_metadata` | yes |  |
| 86 | rust_method | PdfError | `other` | yes |  |
| 97 | rust_method | From<PdfError> for String | `from` |  |  |
| 111 | rust_fn |  | `document_not_found_renders_path` |  |  |
| 121 | rust_fn |  | `renders_page_indices` |  |  |
| 129 | rust_fn |  | `other_passes_through` |  |  |

### `src-tauri/src/infrastructure/pdf_read/backend.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 3 | rust_fn |  | `open` |  |  |
| 4 | rust_fn |  | `read_page_preview` |  |  |

### `src-tauri/src/infrastructure/pdf_read/facade.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | rust_method | PdfReadFacade | `new` | yes |  |
| 16 | rust_method | PdfReadFacade | `open` | yes |  |
| 22 | rust_method | PdfReadFacade | `probe_kind_fast` | yes |  |
| 25 | rust_method | PdfReadFacade | `read_page_preview` | yes |  |

### `src-tauri/src/infrastructure/pdf_read/scanned_backend.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 33 | rust_method | ScannedReadBackend | `new` | yes |  |
| 36 | rust_method | ScannedReadBackend | `load_document` |  |  |
| 94 | rust_method | ScannedReadBackend | `resolve_page_tree` |  |  |
| 197 | rust_method | ScannedReadBackend | `read_page_context` |  |  |
| 214 | rust_method | ScannedReadBackend | `cache_jpeg` |  |  |
| 228 | rust_method | ScannedReadBackend | `qualifies_as_scanned_page` |  |  |
| 248 | rust_method | ScannedReadBackend | `likely_ocr_scanned_document` |  |  |
| 259 | rust_method | ScannedReadBackend | `scanned_confidence` |  |  |
| 281 | rust_method | ScannedReadBackend | `classify_open_decision` |  |  |
| 340 | rust_method | ScannedReadBackend | `_classify_page_kind` |  |  |
| 381 | rust_method | ScannedReadBackend | `page_has_text_content` |  |  |
| 405 | rust_method | ScannedReadBackend | `primitive_may_contain_text_operators` |  |  |
| 427 | rust_method | ScannedReadBackend | `bytes_may_contain_text_operators` |  |  |
| 433 | rust_method | ScannedReadBackend | `is_text_op` |  |  |
| 454 | rust_method | PdfReadBackend for ScannedReadBackend | `open` |  |  |
| 527 | rust_method | PdfReadBackend for ScannedReadBackend | `read_page_preview` |  |  |

### `src-tauri/src/infrastructure/pdf_read/vector_backend.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | rust_method | VectorReadBackend | `new` | yes |  |
| 12 | rust_method | PdfReadBackend for VectorReadBackend | `open` |  |  |
| 23 | rust_method | PdfReadBackend for VectorReadBackend | `read_page_preview` |  |  |

### `src-tauri/src/infrastructure/pdf/annotation_store.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 17 | rust_fn |  | `read_page_highlights` | yes |  |
| 68 | rust_fn |  | `read_page_comments` | yes |  |
| 123 | rust_fn |  | `read_page_annotation_refs` |  |  |
| 150 | rust_fn |  | `read_page_height` |  |  |
| 164 | rust_fn |  | `parse_rect_array` |  |  |
| 175 | rust_fn |  | `parse_color_array` |  |  |
| 185 | rust_fn |  | `object_to_f32` |  |  |
| 191 | rust_fn |  | `pdf_rect_to_top_down_box` |  |  |

### `src-tauri/src/infrastructure/pdf/cache.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 18 | rust_fn |  | `page_cache_key` | yes |  |
| 22 | rust_fn |  | `page_revision_cache_key` | yes |  |
| 33 | rust_fn |  | `light_page_cache_key` | yes |  |
| 37 | rust_fn |  | `invalidate_pdf_light_page_cache` | yes |  |
| 47 | rust_fn |  | `invalidate_pdf_page_cache` | yes |  |
| 78 | rust_fn |  | `invalidate_pdf_layout_cache` | yes |  |

### `src-tauri/src/infrastructure/pdf/commands.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | rust_fn |  | `execute` |  |  |
| 21 | rust_method | PdfEditCommand for ReplaceTextCommand | `execute` |  |  |
| 39 | rust_method | PdfEditCommand for PersistableRegionPatchCommand | `execute` |  |  |
| 70 | rust_method | PdfEditCommand for TextReflowCommand | `execute` |  |  |
| 99 | rust_method | PdfEditCommand for BatchTextReflowCommand | `execute` |  |  |
| 110 | rust_fn |  | `truncate_for_log` |  |  |
| 132 | rust_method | PdfEditCommand for ReplaceImageCommand | `execute` |  |  |
| 139 | rust_method | PdfEditCommand for DeletePageCommand | `execute` |  |  |
| 146 | rust_method | PdfEditCommand for RotatePageCommand | `execute` |  |  |
| 153 | rust_method | PdfEditCommand for InsertPageCommand | `execute` |  |  |
| 160 | rust_method | PdfEditCommand for AddHighlightCommand | `execute` |  |  |
| 174 | rust_method | PdfEditCommand for AddCommentCommand | `execute` |  |  |
| 187 | rust_method | PdfEditCommand for UpdateCommentCommand | `execute` |  |  |
| 199 | rust_method | PdfEditCommand for DeleteAnnotationCommand | `execute` |  |  |
| 206 | rust_method | PdfEditCommand for UpdateMetadataCommand | `execute` |  |  |

### `src-tauri/src/infrastructure/pdf/document_service.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 24 | rust_fn |  | `release_working_copy` |  |  |
| 44 | rust_method | PdfDocumentService | `resolve_working_path` | yes |  |
| 48 | rust_method | PdfDocumentService | `release_pdf_resources` | yes |  |
| 87 | rust_method | PdfDocumentService | `release_all_pdf_resources` | yes |  |
| 151 | rust_method | PdfDocumentService | `open_pdf` | yes |  |
| 233 | rust_method | PdfDocumentService | `save_pdf` | yes |  |
| 366 | rust_method | PdfDocumentService | `read_last_pdf_materialization_report` | yes |  |
| 374 | rust_method | PdfDocumentService | `rollback_pdf` | yes |  |
| 410 | rust_method | PdfDocumentService | `redo_pdf` | yes |  |
| 446 | rust_method | PdfDocumentService | `generate_demo_pdf` | yes |  |
| 454 | rust_fn |  | `load_pdf_public` | yes |  |
| 462 | rust_fn |  | `load_pdf_lenient` |  |  |
| 527 | rust_fn |  | `repair_and_load` |  |  |

### `src-tauri/src/infrastructure/pdf/font/catalog.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_fn |  | `load_system_font_candidates` | yes |  |
| 17 | rust_fn |  | `enum_font_proc` |  |  |
| 88 | rust_fn |  | `load_system_font_candidates` | yes |  |
| 93 | rust_fn |  | `enrich_and_dedupe_candidates` |  |  |
| 133 | rust_fn |  | `enrich_and_dedupe_candidates` |  |  |
| 138 | rust_fn |  | `wide_to_string` |  |  |
| 147 | rust_fn |  | `expand_candidate_aliases` |  |  |
| 180 | rust_fn |  | `expand_candidate_aliases` |  |  |
| 183 | rust_fn |  | `infer_style_name` |  |  |
| 191 | rust_fn |  | `build_full_name` |  |  |
| 198 | rust_fn |  | `build_postscript_name` |  |  |
| 206 | rust_fn |  | `sanitize_postscript_token` |  |  |
| 212 | rust_fn |  | `alias_families` |  |  |
| 243 | rust_fn |  | `estimate_windows_coverage_score` |  |  |

### `src-tauri/src/infrastructure/pdf/font/embedded_program.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 1 | rust_fn |  | `normalize_embedded_font_program` | yes |  |
| 29 | rust_fn |  | `sfnt_directory_bounds` |  |  |
| 55 | rust_fn |  | `is_sorted_records` |  |  |
| 64 | rust_fn |  | `sorts_sfnt_records` |  |  |

### `src-tauri/src/infrastructure/pdf/font/matching.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 19 | rust_method | PdfSystemFontMatcher | `new` | yes |  |
| 27 | rust_method | PdfSystemFontMatcher | `resolve` | yes |  |
| 39 | rust_method | PdfSystemFontMatcher | `candidate_count` | yes |  |
| 42 | rust_method | PdfSystemFontMatcher | `resolve_native_text` | yes |  |
| 97 | rust_method | PdfSystemFontMatcher | `maybe_log_resolution` |  |  |
| 160 | rust_fn |  | `build_cache_key` |  |  |
| 174 | rust_fn |  | `build_native_text_cache_key` |  |  |
| 186 | rust_fn |  | `map_embedded_font_kind` |  |  |
| 205 | rust_fn |  | `matcher_caches_resolved_font` |  |  |
| 222 | rust_fn |  | `uses_descriptor_cache` |  |  |

### `src-tauri/src/infrastructure/pdf/font/metrics.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | rust_fn |  | `get_character_width_pdf_units` | yes |  |

### `src-tauri/src/infrastructure/pdf/font/ttc.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 1 | rust_fn |  | `extract_ttc_face_as_ttf` | yes |  |
| 112 | rust_fn |  | `sfnt_search_params` |  |  |
| 120 | rust_fn |  | `checksum` |  |  |
| 130 | rust_fn |  | `align4` |  |  |

### `src-tauri/src/infrastructure/pdf/geometry_service.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | rust_method | PdfEditorGeometryService | `resolve_layout_inference` | yes |  |
| 17 | rust_method | PdfEditorGeometryService | `resolve_layout_inference_revisioned` | yes |  |
| 32 | rust_method | PdfEditorGeometryService | `resolve_glyph_paint_plan` | yes |  |
| 40 | rust_method | PdfEditorGeometryService | `resolve_plan` | yes |  |
| 55 | rust_method | PdfEditorGeometryService | `read_image_cache` | yes |  |
| 77 | rust_method | PdfEditorGeometryService | `resolve_editor_caret_index` | yes |  |
| 89 | rust_method | PdfEditorGeometryService | `resolve_field_hit` | yes |  |
| 95 | rust_method | PdfEditorGeometryService | `resolve_field_hit_target` | yes |  |
| 101 | rust_method | PdfEditorGeometryService | `resolve_field_projection` | yes |  |
| 107 | rust_method | PdfEditorGeometryService | `resolve_field_editor_params` | yes |  |

### `src-tauri/src/infrastructure/pdf/layout_analyzer.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | rust_method | LayoutGraphAnalyzer | `new` | yes |  |
| 16 | rust_method | LayoutGraphAnalyzer | `analyze` | yes |  |
| 26 | rust_method | LayoutGraphAnalyzer | `detect_column_bands` | yes |  |

### `src-tauri/src/infrastructure/pdf/layout_engine.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 43 | rust_method | LayoutGraphAnalyzer | `new` | yes |  |
| 58 | rust_method | LayoutGraphAnalyzer | `resolve_regions` | yes |  |
| 102 | rust_method | LayoutGraphAnalyzer | `detect_layout_pattern` |  |  |
| 135 | rust_method | LayoutGraphAnalyzer | `create_semantic_region` |  |  |

### `src-tauri/src/infrastructure/pdf/log_service.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 14 | rust_fn |  | `set_pdf_log_level` | yes |  |
| 17 | rust_fn |  | `get_pdf_log_level` | yes |  |
| 21 | rust_fn |  | `clear_pdf_event_log` | yes |  |
| 29 | rust_fn |  | `read_pdf_event_log` | yes |  |
| 46 | rust_fn |  | `timestamp` |  |  |
| 50 | rust_fn |  | `level_label` |  |  |
| 60 | rust_fn |  | `level_color` |  |  |
| 71 | rust_fn |  | `layer_for_event` |  |  |
| 86 | rust_fn |  | `format_fields` |  |  |
| 94 | rust_fn |  | `format_layered_line` |  |  |
| 117 | rust_fn |  | `format_plain_event_line` |  |  |
| 132 | rust_fn |  | `record_pdf_event` |  |  |
| 143 | rust_fn |  | `log_pdf_event` | yes |  |
| 154 | rust_fn |  | `log_terminal_message` | yes |  |
| 167 | rust_method | PdfEventSpan | `begin` | yes |  |
| 180 | rust_method | PdfEventSpan | `finish` | yes |  |
| 192 | rust_method | Drop for PdfEventSpan | `drop` |  |  |
| 231 | rust_method | ProfileSpan | `new` | yes |  |
| 239 | rust_method | Drop for ProfileSpan | `drop` |  |  |

### `src-tauri/src/infrastructure/pdf/models.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 22 | rust_fn |  | `is_false` |  |  |
| 25 | rust_fn |  | `default_alpha` |  |  |
| 28 | rust_fn |  | `is_default_alpha` |  |  |
| 93 | rust_fn |  | `default_scale_x` |  |  |
| 176 | rust_method | NativePathModel | `flip_y` | yes |  |
| 186 | rust_method | Default for NativePathModel | `default` |  |  |
| 205 | rust_fn |  | `is_zero_u8` |  |  |
| 275 | rust_method | NativeVectorPageModel | `flip_y` | yes |  |

### `src-tauri/src/infrastructure/pdf/page_classifier.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 2 | rust_fn |  | `classify_page` | yes |  |

### `src-tauri/src/infrastructure/pdf/page_intermediate_service.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_method | PdfPageIntermediateService | `resolve_page_display_list_from_app_state` | yes |  |
| 115 | rust_method | PdfPageIntermediateService | `resolve_vector_page_model` | yes |  |
| 132 | rust_method | PdfPageIntermediateService | `resolve_vector_page_model_from_app_state` | yes |  |
| 171 | rust_method | PdfPageIntermediateService | `resolve_layout_inference_from_app_state` | yes |  |
| 209 | rust_method | PdfPageIntermediateService | `resolve_glyph_paint_plan` | yes |  |
| 219 | rust_method | PdfPageIntermediateService | `resolve_glyph_paint_plan_from_app_state` | yes |  |
| 247 | rust_method | PdfPageIntermediateService | `resolve_page_asset_bundle` | yes |  |
| 275 | rust_fn |  | `styled_run` |  |  |
| 292 | rust_fn |  | `display_list` |  |  |
| 307 | rust_fn |  | `uses_seeded_display_list` |  |  |
| 384 | rust_fn |  | `shares_derived_page_model` |  |  |

### `src-tauri/src/infrastructure/pdf/page_model_service.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | rust_method | PdfPageModelService | `read_pdf_metadata_from_app_state` | yes |  |
| 64 | rust_method | PdfPageModelService | `read_pdf_metadata` | yes |  |
| 71 | rust_method | PdfPageModelService | `resolve_vector_page_model_from_app_state` | yes |  |
| 87 | rust_method | PdfPageModelService | `resolve_model_from_state` | yes |  |
| 104 | rust_method | PdfPageModelService | `resolve_vector_page_model` | yes |  |
| 113 | rust_method | PdfPageModelService | `resolve_model` | yes |  |
| 130 | rust_method | PdfPageModelService | `resolve_light_page_model` | yes |  |

### `src-tauri/src/infrastructure/pdf/pdf_font.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 14 | rust_method | CMap | `new` | yes |  |
| 18 | rust_method | CMap | `from_codepoint_pairs` | yes |  |
| 49 | rust_method | ParsedFont | `is_multibyte` | yes |  |
| 53 | rust_method | ParsedFont | `can_encode` | yes |  |
| 61 | rust_method | ParsedFont | `encode_text` | yes |  |
| 97 | rust_method | ParsedFont | `resolve_text_width` | yes |  |
| 128 | rust_fn |  | `resolve_glyph_geom` | yes |  |
| 289 | rust_fn |  | `read_cmap` | yes |  |
| 363 | rust_fn |  | `hex_to_string` |  |  |
| 398 | rust_method | ResourceCache | `new` | yes |  |
| 406 | rust_fn |  | `break_text_into_lines` | yes |  |
| 456 | rust_fn |  | `simplify_path_segments` | yes |  |
| 498 | rust_fn |  | `simplify_points` |  |  |
| 523 | rust_fn |  | `perpendicular_distance` |  |  |
| 535 | rust_fn |  | `parse_font_from_dict` | yes |  |

### `src-tauri/src/infrastructure/pdf/pdf_read_service.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 20 | rust_method | PdfReadService | `resolve_working_path` | yes |  |
| 62 | rust_method | PdfReadService | `open_pdf` | yes |  |
| 110 | rust_method | PdfReadService | `read_pdf_metadata_from_app_state` | yes |  |
| 126 | rust_method | PdfReadService | `read_pdf_metadata` | yes |  |
| 134 | rust_method | PdfReadService | `resolve_vector_page_model_from_app_state` | yes |  |
| 150 | rust_method | PdfReadService | `resolve_vector_page_model` | yes |  |
| 159 | rust_method | PdfReadService | `resolve_layout_inference` | yes |  |
| 171 | rust_method | PdfReadService | `resolve_glyph_paint_plan` | yes |  |
| 183 | rust_method | PdfReadService | `read_image_cache` | yes |  |

### `src-tauri/src/infrastructure/pdf/pdf_read.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 33 | rust_method | GraphicsState | `new` | yes |  |
| 57 | rust_method | GraphicsState | `transform_point` | yes |  |
| 72 | rust_fn |  | `read_resources` | yes |  |
| 111 | rust_fn |  | `operands_to_f32` | yes |  |
| 123 | rust_fn |  | `multiply_matrices` | yes |  |
| 143 | rust_fn |  | `resolve_paths` | yes |  |
| 336 | rust_fn |  | `parse_content_stream` | yes |  |
| 663 | rust_fn |  | `with_alpha` |  |  |
| 919 | rust_fn |  | `extract_metadata` | yes |  |
| 946 | rust_fn |  | `read_page_count` | yes |  |
| 950 | rust_fn |  | `extract_page_bbox` | yes |  |
| 971 | rust_fn |  | `extract_vector_page_model` | yes |  |
| 978 | rust_fn |  | `extract_layout_inference` | yes |  |
| 985 | rust_fn |  | `extract_glyph_paint_plan` | yes |  |
| 1055 | rust_fn |  | `apply_png_predictor` |  |  |
| 1114 | rust_fn |  | `read_decode_params` |  |  |
| 1152 | rust_fn |  | `manual_flate_decompress` |  |  |
| 1174 | rust_fn |  | `build_image_as_jpeg` |  |  |

### `src-tauri/src/infrastructure/pdf/pdf_write_font_resolver.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 28 | rust_method | PdfTextWriteFont | `encode_text` | yes |  |
| 37 | rust_method | PdfTextWriteFont | `source_label` | yes |  |
| 55 | rust_fn |  | `resolve_text_write_font` | yes |  |
| 103 | rust_fn |  | `can_pdf_font_encode_text` |  |  |
| 116 | rust_fn |  | `build_preferred_font_names` |  |  |
| 145 | rust_fn |  | `push_font_name_variants` |  |  |
| 170 | rust_fn |  | `push_unique` |  |  |
| 181 | rust_fn |  | `strip_subset_prefix` |  |  |
| 191 | rust_fn |  | `resolve_full_font_program` |  |  |
| 214 | rust_fn |  | `load_managed_font_dirs` |  |  |
| 227 | rust_fn |  | `resolve_family_from_db` |  |  |
| 245 | rust_fn |  | `try_known_font_files` |  |  |
| 284 | rust_fn |  | `build_resolved_program_from_face_data` |  |  |
| 335 | rust_fn |  | `normalize_font_program_for_pdf` |  |  |
| 346 | rust_fn |  | `font_covers_text` |  |  |
| 350 | rust_fn |  | `missing_chars_for_face` |  |  |
| 362 | rust_fn |  | `describe_missing_from_candidate_pool` |  |  |
| 372 | rust_fn |  | `collect_glyphs` |  |  |
| 390 | rust_fn |  | `encode_text_as_glyph_ids` |  |  |
| 416 | rust_fn |  | `parsed_font_from_resolved_program` |  |  |
| 439 | rust_fn |  | `ensure_resolved_font_in_page` |  |  |
| 493 | rust_fn |  | `build_type0_font_object` |  |  |
| 573 | rust_fn |  | `build_width_array` |  |  |
| 598 | rust_fn |  | `build_to_unicode_cmap` |  |  |
| 623 | rust_fn |  | `utf16be_hex` |  |  |
| 631 | rust_fn |  | `page_resources` |  |  |
| 645 | rust_fn |  | `page_resource_dictionary` |  |  |
| 659 | rust_fn |  | `build_font_alias` |  |  |
| 672 | rust_fn |  | `post_script_name` |  |  |
| 678 | rust_fn |  | `sanitize_pdf_name` |  |  |
| 691 | rust_fn |  | `source_label` |  |  |
| 698 | rust_fn |  | `truncate_log` |  |  |

### `src-tauri/src/infrastructure/pdf/pdf_write_service.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | rust_method | PdfWriteService | `save_pdf` | yes |  |
| 70 | rust_method | PdfWriteService | `rollback_pdf` | yes |  |
| 129 | rust_method | PdfWriteService | `redo_pdf` | yes |  |
| 194 | rust_method | PdfWriteService | `generate_demo_pdf` | yes |  |
| 237 | rust_method | PdfWriteService | `invalidate_caches` |  |  |

### `src-tauri/src/infrastructure/pdf/pdf_write.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 16 | rust_fn |  | `apply_text_patch` |  |  |
| 24 | rust_fn |  | `apply_atomic_reflow_to_doc` |  |  |
| 37 | rust_fn |  | `apply_batch_reflow_to_doc` |  |  |
| 42 | rust_fn |  | `replace_image_xobject` |  |  |
| 47 | rust_fn |  | `delete_page` |  |  |
| 48 | rust_fn |  | `rotate_page` |  |  |
| 49 | rust_fn |  | `insert_blank_page` |  |  |
| 50 | rust_fn |  | `add_highlight` |  |  |
| 56 | rust_fn |  | `add_text_comment` |  |  |
| 63 | rust_fn |  | `update_text_comment` |  |  |
| 69 | rust_fn |  | `delete_annotation` |  |  |
| 70 | rust_fn |  | `update_metadata` |  |  |
| 80 | rust_method | PdfDocExt for Document | `apply_text_patch` |  |  |
| 126 | rust_method | PdfDocExt for Document | `apply_atomic_reflow_to_doc` |  |  |
| 154 | rust_method | PdfDocExt for Document | `apply_batch_reflow_to_doc` |  |  |
| 375 | rust_method | PdfDocExt for Document | `replace_image_xobject` |  |  |
| 383 | rust_method | PdfDocExt for Document | `delete_page` |  |  |
| 388 | rust_method | PdfDocExt for Document | `rotate_page` |  |  |
| 401 | rust_method | PdfDocExt for Document | `insert_blank_page` |  |  |
| 405 | rust_method | PdfDocExt for Document | `add_highlight` |  |  |
| 464 | rust_method | PdfDocExt for Document | `add_text_comment` |  |  |
| 520 | rust_method | PdfDocExt for Document | `update_text_comment` |  |  |
| 553 | rust_method | PdfDocExt for Document | `delete_annotation` |  |  |
| 563 | rust_method | PdfDocExt for Document | `update_metadata` |  |  |
| 588 | rust_fn |  | `patch_content_recursive` |  |  |
| 785 | rust_method | ReflowCluster<'a> | `build` | yes |  |
| 806 | rust_fn |  | `patch_atomic_reflow_recursive` |  |  |
| 1058 | rust_fn |  | `break_text_into_lines` |  |  |
| 1104 | rust_fn |  | `read_page_height` |  |  |
| 1122 | rust_fn |  | `append_page_annotation` |  |  |
| 1159 | rust_fn |  | `remove_page_annotation` |  |  |
| 1200 | rust_fn |  | `read_page_annotation_refs` |  |  |
| 1221 | rust_fn |  | `resolve_line_color` |  |  |
| 1229 | rust_fn |  | `resolve_line_underline` |  |  |
| 1232 | rust_fn |  | `parse_pdf_hex_color` |  |  |

### `src-tauri/src/infrastructure/pdf/preview_engine.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 5 | rust_fn |  | `page_dimensions` |  |  |
| 70 | rust_fn |  | `collect_page_xobjects` |  |  |
| 111 | rust_fn |  | `page_has_font_resources` |  |  |
| 141 | rust_fn |  | `page_has_text_operators` |  |  |
| 156 | rust_fn |  | `cache_image_asset` |  |  |
| 246 | rust_fn |  | `build_light_page_model` | yes |  |

### `src-tauri/src/infrastructure/pdf/region_materializer.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 22 | rust_method | RegionMaterializationPlan | `to_report` | yes |  |
| 81 | rust_fn |  | `snapshot_text` |  |  |
| 88 | rust_fn |  | `snapshot_lines_len` |  |  |
| 96 | rust_fn |  | `snapshot_style_runs_len` |  |  |
| 110 | rust_fn |  | `snapshot_line_reflows` |  |  |
| 152 | rust_fn |  | `snapshot_field_texts` |  |  |
| 158 | rust_fn |  | `rebuild_field_row_text_from_value_patch` |  |  |
| 173 | rust_fn |  | `normalize_region_text` |  |  |
| 176 | rust_fn |  | `snapshot_list_item_texts` |  |  |
| 191 | rust_fn |  | `combine_list_item_text` |  |  |
| 202 | rust_fn |  | `is_valid_patch_target` |  |  |
| 205 | rust_fn |  | `has_non_empty_snapshot_text` |  |  |
| 208 | rust_fn |  | `has_structured_paragraph_snapshot` |  |  |
| 211 | rust_fn |  | `is_valid_field_row_patch` |  |  |
| 216 | rust_fn |  | `is_valid_paragraph_patch` |  |  |
| 222 | rust_fn |  | `merge_text_reflows` |  |  |
| 258 | rust_fn |  | `materialize_field_row_patch_to_text_reflow` |  |  |
| 315 | rust_fn |  | `materialize_snapshot_lines_to_text_reflows` |  |  |
| 337 | rust_fn |  | `materialize_paragraph_patch_to_text_reflow` |  |  |
| 411 | rust_fn |  | `materialize_list_item_patch_to_text_reflow` |  |  |
| 495 | rust_fn |  | `materialize_region_patch_to_text_reflow` |  |  |
| 571 | rust_fn |  | `build_region_materialization_plan` | yes |  |

### `src-tauri/src/infrastructure/pdf/save_engine.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | rust_fn |  | `apply_pdf_commands` | yes |  |

### `src-tauri/src/infrastructure/pdf/save_text_write_plan.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 15 | rust_fn |  | `truncate_for_log` | yes |  |

### `src-tauri/src/infrastructure/pdf/spatial_graph.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 20 | rust_method | SpatialGraph | `new` | yes |  |
| 28 | rust_method | SpatialGraph | `build_adjacency` | yes |  |
| 40 | rust_method | SpatialGraph | `are_neighbors` |  |  |
| 75 | rust_method | SpatialGraph | `add_edge` |  |  |
| 81 | rust_method | SpatialGraph | `find_components` | yes |  |

### `src-tauri/src/infrastructure/pdf/tests_reflow.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 4 | rust_fn |  | `test_reflow_displacement_math` |  |  |
| 33 | rust_fn |  | `test_encoding_reversal_detection` |  |  |
| 49 | rust_fn |  | `test_chinese_reflow_overlap_guard` |  |  |

### `src-tauri/src/infrastructure/pdf/vector_engine.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 8 | rust_fn |  | `resolve_display_list` | yes |  |
| 35 | rust_fn |  | `resolve_model` | yes |  |
| 55 | rust_fn |  | `build_vector_page_model_from_display_list` | yes |  |
| 423 | rust_fn |  | `resolve_layout_inference` | yes |  |
| 432 | rust_fn |  | `resolve_layout_inference_from_display_list` | yes |  |

### `src-tauri/src/infrastructure/pdf/vello_renderer.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 27 | rust_method | VelloRenderer | `new` | yes |  |
| 81 | rust_method | VelloRenderer | `render_objects_to_png` | yes |  |
| 201 | rust_method | VelloRenderer | `draw_image_cpu` |  |  |
| 291 | rust_method | VelloRenderer | `draw_text_bitmap_deprecated` |  |  |
| 394 | rust_method | VelloRenderer | `composite_glyph` |  |  |
| 493 | rust_method | VelloRenderer | `text_fill_color` |  |  |
| 503 | rust_method | VelloRenderer | `text_stroke_color` |  |  |
| 511 | rust_method | VelloRenderer | `text_stroke_width` |  |  |
| 519 | rust_method | VelloRenderer | `paint_text_outline` |  |  |
| 556 | rust_method | VelloRenderer | `raw_outline_transform` |  |  |
| 575 | rust_method | VelloRenderer | `perform_vello_render_raw` |  |  |
| 692 | rust_fn |  | `blend` |  |  |
| 695 | rust_fn |  | `parse_hex_color_rgb` |  |  |
| 704 | rust_fn |  | `parse_hex_vello_color` |  |  |
| 708 | rust_fn |  | `text_fill_enabled` |  |  |
| 711 | rust_fn |  | `text_stroke_enabled` |  |  |
| 714 | rust_fn |  | `text_is_non_painting` |  |  |
| 720 | rust_method | VelloRenderer | `draw_text_vector` |  |  |
| 927 | rust_method | VelloRenderer | `resolve_pdf_font` |  |  |
| 930 | rust_method | VelloRenderer | `draw_embedded_text_vector` |  |  |
| 1102 | rust_method | VelloRenderer | `build_embedded_glyph_positions` |  |  |
| 1132 | rust_method | VelloRenderer | `embedded_glyph_count` |  |  |
| 1138 | rust_method | VelloRenderer | `resolve_embedded_glyph_id` |  |  |
| 1217 | rust_method | VelloRenderer | `resolve_cached_cid_glyph_id` |  |  |
| 1233 | rust_method | VelloRenderer | `prefers_pdf_code_glyph_mapping` |  |  |
| 1240 | rust_method | VelloRenderer | `resolve_cosmic_family` |  |  |
| 1263 | rust_fn |  | `preview_text` |  |  |

### `src-tauri/src/interfaces/pdf/annotation.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | rust_fn |  | `read_annotation_targets` | yes | read_annotation_targets |
| 22 | rust_fn |  | `read_highlights` | yes | read_highlights |
| 31 | rust_fn |  | `apply_highlight` | yes | apply_highlight |
| 40 | rust_fn |  | `delete_annotation` | yes | delete_annotation |

### `src-tauri/src/interfaces/pdf/comment.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | rust_fn |  | `read_comments` | yes | read_comments |
| 20 | rust_fn |  | `read_comment_review` | yes | read_comment_review |
| 29 | rust_fn |  | `apply_comment` | yes | apply_comment |
| 38 | rust_fn |  | `apply_comment_update` | yes | apply_comment_update |

### `src-tauri/src/interfaces/pdf/document.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 8 | rust_fn |  | `open_pdf` | yes | open_pdf |
| 24 | rust_fn |  | `clear_cache` | yes | clear_cache |
| 44 | rust_fn |  | `save_pdf` | yes | save_pdf |
| 66 | rust_fn |  | `undo` | yes | undo |
| 78 | rust_fn |  | `redo` | yes | redo |

### `src-tauri/src/interfaces/pdf/ipc_converters.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 15 | rust_fn |  | `ensure_document_loaded` | yes |  |
| 42 | rust_fn |  | `execute_region_patches` | yes |  |
| 75 | rust_fn |  | `apply_highlight_annotation` | yes |  |
| 93 | rust_fn |  | `apply_text_comment` | yes |  |
| 113 | rust_fn |  | `delete_annotation_internal` | yes |  |
| 129 | rust_fn |  | `update_text_comment` | yes |  |
| 147 | rust_fn |  | `truncate_for_log` | yes |  |
| 164 | rust_fn |  | `execute_commands` | yes |  |

### `src-tauri/src/interfaces/pdf/page.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 15 | rust_fn |  | `read_preview` | yes | read_preview |

### `src-tauri/src/interfaces/pdf/render.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 18 | rust_fn |  | `read_page_asset_bundle` | yes | read_page_asset_bundle |
| 73 | rust_fn |  | `read_vector` | yes | read_vector |
| 110 | rust_fn |  | `read_glyph_plan` | yes | read_glyph_plan |
| 144 | rust_fn |  | `read_images` | yes | read_images |
| 149 | rust_fn |  | `diagnose_page` | yes | diagnose_page |

### `src-tauri/src/interfaces/pdf/replace.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | rust_fn |  | `apply_region_patches` | yes | apply_region_patches |

### `src-tauri/src/interfaces/pdf/search.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | rust_fn |  | `find_in_page` | yes | find_in_page |
| 35 | rust_fn |  | `find_in_document` | yes | find_in_document |

### `src-tauri/src/interfaces/pdf/system.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | rust_fn |  | `create_demo_pdf` | yes | create_demo_pdf |
| 12 | rust_fn |  | `set_log_level` | yes | set_log_level |
| 17 | rust_fn |  | `clear_pdf_event_log` | yes | clear_pdf_event_log |
| 22 | rust_fn |  | `read_pdf_event_log` | yes | read_pdf_event_log |
| 27 | rust_fn |  | `set_page_asset_test_delay_ms` | yes | set_page_asset_test_delay_ms |
| 32 | rust_fn |  | `terminal_log` | yes | terminal_log |
| 37 | rust_fn |  | `resolve_asset_url` | yes | resolve_asset_url |
| 44 | rust_fn |  | `pick_file` | yes | pick_file |

### `src-tauri/src/lib.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | rust_fn |  | `run` | yes |  |

### `src-tauri/src/main.rs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 2 | rust_fn |  | `main` |  |  |

### `src/bridge/ai/resume_ai_apply.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | function |  | `describeError` |  |  |
| 17 | function |  | `normalizePath` |  |  |
| 22 | object_arrow_method |  | `getViewerSession` |  |  |
| 24 | object_arrow_method |  | `setCurrentPage` |  |  |
| 25 | object_arrow_method |  | `logAiChain` |  |  |
| 28 | function |  | `toTextEdit` |  |  |
| 39 | function |  | `applySingleSuggestion` | yes |  |
| 71 | function |  | `applyAllPendingSuggestions` | yes |  |
| 141 | function |  | `saveAsSeparatePdf` | yes |  |

### `src/bridge/ai/resume_ai_client.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 70 | function |  | `sessionKey` |  |  |
| 74 | function |  | `getOrCreateSession` |  |  |
| 84 | function |  | `makeView` |  |  |
| 113 | function |  | `toGeminiContents` |  |  |
| 122 | function |  | `callGemini` |  |  |
| 154 | function |  | `planResumeAiEdits` | yes |  |
| 161 | function |  | `applyResumeAiEdits` | yes |  |
| 167 | function |  | `syncResumeAiSession` | yes |  |
| 180 | function |  | `submitResumeAiPrompt` | yes |  |
| 201 | function |  | `applyResumeAiSuggestion` | yes |  |
| 210 | function |  | `markResumeAiSuggestionApplied` | yes |  |
| 216 | function |  | `markResumeAiSuggestionFailed` | yes |  |
| 228 | function |  | `applyAllResumeAiSuggestions` | yes |  |
| 238 | function |  | `markAllResumeAiSuggestionsApplied` | yes |  |
| 244 | function |  | `clearResumeAiSuggestions` | yes |  |

### `src/bridge/ai/resume_ai_controller.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 31 | object_arrow_method |  | `getViewerSession` |  |  |
| 33 | object_arrow_method |  | `openPdfPath` |  |  |
| 34 | object_arrow_method |  | `renderCurrentPage` |  |  |
| 35 | object_arrow_method |  | `setCurrentPage` |  |  |
| 39 | object_arrow_method |  | `applyAllSuggestions` |  |  |
| 40 | object_arrow_method |  | `initialize` |  |  |
| 41 | object_arrow_method |  | `saveAsAiVersion` |  |  |
| 42 | object_arrow_method |  | `syncViewerState` |  |  |
| 43 | object_arrow_method |  | `togglePanel` |  |  |
| 46 | function |  | `getElement` |  |  |
| 50 | function |  | `describeError` |  |  |
| 57 | function |  | `normalizePath` |  |  |
| 61 | function |  | `buildSuggestedPdfCopyPath` |  |  |
| 68 | function |  | `buildApplyContext` |  |  |
| 77 | function |  | `formatScopeLabel` |  |  |
| 110 | class_method | PdfResumeAiController | `constructor` |  |  |
| 114 | class_method | PdfResumeAiController | `initialize` |  |  |
| 169 | arrow_fn |  | `routeActionEvent` |  |  |
| 195 | class_method | PdfResumeAiController | `syncViewerState` |  |  |
| 237 | class_method | PdfResumeAiController | `togglePanel` |  |  |
| 246 | class_method | PdfResumeAiController | `applyAllSuggestions` |  |  |
| 280 | class_method | PdfResumeAiController | `saveAsAiVersion` |  |  |
| 330 | class_method | PdfResumeAiController | `clearSuggestions` |  |  |
| 343 | class_method | PdfResumeAiController | `applySuggestion` |  |  |
| 383 | class_method | PdfResumeAiController | `handleApplySuggestionClick` |  |  |
| 388 | class_method | PdfResumeAiController | `triggerApplySuggestion` |  |  |
| 398 | class_method | PdfResumeAiController | `handleAiPanelActionEvent` |  |  |
| 428 | class_method | PdfResumeAiController | `handleSendMessage` |  |  |
| 489 | class_method | PdfResumeAiController | `pushTurn` |  |  |
| 494 | class_method | PdfResumeAiController | `applyThreadView` |  |  |
| 502 | class_method | PdfResumeAiController | `syncRustSession` |  |  |
| 521 | class_method | PdfResumeAiController | `renderMessages` |  |  |
| 531 | object_arrow_method |  | `onApplyPointerDownLog` |  |  |
| 534 | object_arrow_method |  | `onApplySuggestion` |  |  |
| 540 | class_method | PdfResumeAiController | `renderSuggestions` |  |  |
| 560 | class_method | PdfResumeAiController | `saveApiKey` |  |  |
| 585 | class_method | PdfResumeAiController | `restoreApiKey` |  |  |
| 603 | class_method | PdfResumeAiController | `setBusy` |  |  |
| 614 | class_method | PdfResumeAiController | `toggleWideMode` |  |  |
| 620 | class_method | PdfResumeAiController | `setPanelOpen` |  |  |
| 640 | class_method | PdfResumeAiController | `refreshViewer` |  |  |
| 651 | class_method | PdfResumeAiController | `setStatus` |  |  |
| 656 | class_method | PdfResumeAiController | `syncIdleStatus` |  |  |
| 665 | class_method | PdfResumeAiController | `scheduleIdleStatusSync` |  |  |
| 673 | class_method | PdfResumeAiController | `clearStatusResetTimer` |  |  |
| 680 | class_method | PdfResumeAiController | `expandApiKeyEditor` |  |  |
| 685 | class_method | PdfResumeAiController | `cancelApiKeyEditing` |  |  |
| 693 | class_method | PdfResumeAiController | `syncApiKeySection` |  |  |
| 697 | class_method | PdfResumeAiController | `logAiChain` |  |  |
| 702 | function |  | `createResumeAiController` | yes |  |

### `src/bridge/ai/resume_ai_diff_preview.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | function |  | `tokenizeForDiff` |  |  |
| 10 | function |  | `buildDiffTokens` |  |  |
| 55 | function |  | `countDiffStats` | yes |  |
| 67 | function |  | `createDiffPreview` | yes |  |

### `src/bridge/ai/resume_ai_panel_state_view.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | function |  | `applyResumeAiBusyState` | yes |  |
| 32 | function |  | `applyResumeAiWideMode` | yes |  |
| 50 | function |  | `applyResumeAiPanelOpen` | yes |  |
| 63 | function |  | `setResumeAiStatus` | yes |  |
| 94 | function |  | `syncResumeAiApiKeySection` | yes |  |
| 114 | function |  | `getElement` |  |  |

### `src/bridge/ai/resume_ai_panel_view.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | object_arrow_method |  | `onApplySuggestion` |  |  |
| 12 | object_arrow_method |  | `onApplyPointerDownLog` |  |  |
| 24 | function |  | `renderResumeAiConversation` | yes |  |
| 58 | function |  | `syncResumeAiSuggestionSummary` | yes |  |
| 80 | function |  | `appendSuggestionGroups` |  |  |
| 111 | function |  | `createSuggestionCard` |  |  |
| 212 | function |  | `countSuggestionStates` |  |  |
| 229 | function |  | `createMessageBubble` |  |  |

### `src/bridge/annotation/pdf_annotation_controller.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 48 | object_arrow_method |  | `getViewerSession` |  |  |
| 53 | object_arrow_method |  | `toggle` |  |  |
| 54 | object_arrow_method |  | `refresh` |  |  |
| 55 | object_arrow_method |  | `clear` |  |  |
| 60 | function |  | `getNodes` |  |  |
| 68 | function |  | `colorToCss` |  |  |
| 69 | arrow_fn |  | `channel` |  |  |
| 73 | function |  | `createPdfAnnotationController` | yes |  |
| 81 | function |  | `syncButtonState` |  |  |
| 87 | function |  | `clear` |  |  |
| 103 | function |  | `renderPersistedHighlights` |  |  |
| 138 | function |  | `deleteHighlight` |  |  |
| 159 | function |  | `addRegionHighlight` |  |  |
| 182 | function |  | `renderAnnotationTargets` |  |  |
| 214 | function |  | `refresh` |  |  |
| 251 | function |  | `toggle` |  |  |

### `src/bridge/comment/pdf_comment_contracts.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 144 | function |  | `normalizeReviewSession` | yes |  |
| 153 | function |  | `normalizeOverlayDisplay` | yes |  |
| 157 | function |  | `normalizeTargetOverlayDisplay` | yes |  |
| 163 | function |  | `normalizeReviewDisplay` | yes |  |

### `src/bridge/comment/pdf_comment_controller.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 21 | object_arrow_method |  | `getViewerSession` |  |  |
| 22 | object_arrow_method |  | `getWasmApi` |  |  |
| 24 | object_arrow_method |  | `goToPage` |  |  |
| 28 | object_arrow_method |  | `initialize` |  |  |
| 29 | object_arrow_method |  | `toggle` |  |  |
| 30 | object_arrow_method |  | `refresh` |  |  |
| 31 | object_arrow_method |  | `clear` |  |  |
| 32 | object_arrow_method |  | `togglePanel` |  |  |
| 35 | function |  | `readReviewScope` |  |  |
| 39 | function |  | `escapeCssSelector` |  |  |
| 49 | function |  | `waitForAnimationFrame` |  |  |
| 55 | function |  | `scrollToCommentMarker` |  |  |
| 65 | function |  | `createPdfCommentController` | yes |  |
| 82 | object_arrow_method |  | `readBusy` |  |  |
| 83 | object_arrow_method |  | `setBusy` |  |  |
| 86 | object_arrow_method |  | `markNeedsReload` |  |  |
| 89 | object_arrow_method |  | `refreshController` |  |  |
| 92 | object_arrow_method |  | `renderOverlayFromDisplay` |  |  |
| 98 | function |  | `syncButtonState` |  |  |
| 102 | function |  | `clearReviewView` |  |  |
| 115 | function |  | `clear` |  |  |
| 126 | function |  | `renderPersistedComments` |  |  |
| 142 | function |  | `renderCommentTargets` |  |  |
| 159 | function |  | `fetchReview` |  |  |
| 167 | function |  | `renderReviewList` |  |  |
| 178 | object_arrow_method |  | `onSummaryChipClick` |  |  |
| 185 | object_arrow_method |  | `onCardClick` |  |  |
| 198 | object_arrow_method |  | `onActionClick` |  |  |
| 209 | function |  | `refreshReviewPanel` |  |  |
| 222 | function |  | `refresh` |  |  |
| 257 | function |  | `toggle` |  |  |
| 263 | function |  | `togglePanel` |  |  |
| 276 | function |  | `initialize` |  |  |

### `src/bridge/comment/pdf_comment_dom.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 19 | function |  | `getPdfCommentDomNodes` | yes |  |
| 36 | function |  | `syncPdfCommentDomState` | yes |  |
| 58 | function |  | `clearPdfCommentLayerContainers` | yes |  |

### `src/bridge/comment/pdf_comment_host_actions.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | object_arrow_method |  | `getViewerSession` |  |  |
| 14 | object_arrow_method |  | `goToPage` |  |  |
| 15 | object_arrow_method |  | `readBusy` |  |  |
| 16 | object_arrow_method |  | `setBusy` |  |  |
| 17 | object_arrow_method |  | `markNeedsReload` |  |  |
| 18 | object_arrow_method |  | `refreshController` |  |  |
| 19 | object_arrow_method |  | `renderOverlayFromDisplay` |  |  |
| 20 | object_arrow_method |  | `scrollToCommentMarker` |  |  |
| 24 | object_arrow_method |  | `deleteComment` |  |  |
| 25 | object_arrow_method |  | `editComment` |  |  |
| 26 | object_arrow_method |  | `addRegionComment` |  |  |
| 27 | object_arrow_method |  | `focusComment` |  |  |
| 28 | object_arrow_method |  | `handleReviewCardAction` |  |  |
| 33 | function |  | `createPdfCommentHostActions` | yes |  |
| 36 | function |  | `deleteComment` |  |  |
| 57 | function |  | `editComment` |  |  |
| 82 | function |  | `addRegionComment` |  |  |
| 105 | function |  | `focusComment` |  |  |
| 115 | function |  | `handleReviewCardAction` |  |  |

### `src/bridge/comment/pdf_comment_overlay_view.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | function |  | `renderCommentOverlay` | yes |  |
| 9 | object_arrow_method |  | `onCommentClick` |  |  |
| 49 | function |  | `renderCommentTargetOverlay` | yes |  |
| 53 | object_arrow_method |  | `onTargetClick` |  |  |

### `src/bridge/comment/pdf_comment_review_view.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 14 | object_arrow_method |  | `onSummaryChipClick` |  |  |
| 15 | object_arrow_method |  | `onCardClick` |  |  |
| 16 | object_arrow_method |  | `onActionClick` |  |  |
| 19 | function |  | `clearCommentReviewView` | yes |  |
| 34 | function |  | `renderCommentReviewView` | yes |  |
| 140 | function |  | `renderSummary` |  |  |
| 143 | object_arrow_method |  | `onSummaryChipClick` |  |  |
| 167 | function |  | `applyReviewCardActionStyle` |  |  |

### `src/bridge/comment/pdf_comment_wasm_bridge.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 17 | object_arrow_method |  | `getViewerSession` |  |  |
| 18 | object_arrow_method |  | `getWasmApi` |  |  |
| 26 | function |  | `getCommentManager` |  |  |
| 36 | function |  | `getReviewSession` |  |  |
| 47 | object_arrow_method |  | `readReviewSession` |  |  |
| 48 | object_arrow_method |  | `clearReviewSession` |  |  |
| 49 | object_arrow_method |  | `setReviewPanelOpenAndLoad` |  |  |
| 50 | object_arrow_method |  | `toggleReviewPanelAndLoad` |  |  |
| 51 | object_arrow_method |  | `setReviewScopeAndLoad` |  |  |
| 52 | object_arrow_method |  | `setReviewQueryAndLoad` |  |  |
| 53 | object_arrow_method |  | `selectReviewCommentAndLoad` |  |  |
| 54 | object_arrow_method |  | `loadCommentOverlay` |  |  |
| 55 | object_arrow_method |  | `loadCommentTargetOverlay` |  |  |
| 56 | object_arrow_method |  | `loadCommentReview` |  |  |
| 57 | object_arrow_method |  | `addRegionCommentRequest` |  |  |
| 58 | object_arrow_method |  | `deletePageAnnotationRequest` |  |  |
| 59 | object_arrow_method |  | `updatePageCommentRequest` |  |  |
| 62 | function |  | `createPdfCommentWasmBridge` | yes |  |
| 65 | function |  | `readReviewSession` |  |  |
| 73 | function |  | `withCurrentDocument` |  |  |
| 74 | object_arrow_method |  | `loader` |  |  |
| 83 | arrow_fn |  | `cm` |  |  |
| 87 | object_arrow_method |  | `clearReviewSession` |  |  |
| 90 | object_arrow_method |  | `setReviewPanelOpenAndLoad` |  |  |
| 96 | object_arrow_method |  | `toggleReviewPanelAndLoad` |  |  |
| 102 | object_arrow_method |  | `setReviewScopeAndLoad` |  |  |
| 108 | object_arrow_method |  | `setReviewQueryAndLoad` |  |  |
| 114 | object_arrow_method |  | `selectReviewCommentAndLoad` |  |  |
| 120 | object_arrow_method |  | `loadCommentOverlay` |  |  |
| 127 | object_arrow_method |  | `loadCommentTargetOverlay` |  |  |
| 134 | object_arrow_method |  | `loadCommentReview` |  |  |
| 136 | object_arrow_method |  | `addRegionCommentRequest` |  |  |
| 139 | object_arrow_method |  | `deletePageAnnotationRequest` |  |  |
| 142 | object_arrow_method |  | `updatePageCommentRequest` |  |  |

### `src/bridge/document/document_edit_api.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 46 | object_arrow_method |  | `getWasmApi` |  |  |
| 47 | object_arrow_method |  | `getCurrentPath` |  |  |
| 48 | object_arrow_method |  | `getCurrentPage` |  |  |
| 49 | object_arrow_method |  | `getCurrentZoom` |  |  |
| 50 | object_arrow_method |  | `buildRenderRequest` |  |  |
| 51 | object_arrow_method |  | `renderScheduledFrame` |  |  |
| 52 | object_arrow_method |  | `invalidateRenderCache` |  |  |
| 57 | object_arrow_method |  | `applyPatch` |  |  |
| 58 | object_arrow_method |  | `editRegionText` |  |  |
| 63 | object_arrow_method |  | `getReviewFeed` |  |  |
| 64 | object_arrow_method |  | `acceptReviewChange` |  |  |
| 65 | object_arrow_method |  | `rejectReviewChange` |  |  |
| 66 | object_arrow_method |  | `acceptAllReviewChanges` |  |  |
| 67 | object_arrow_method |  | `rejectAllReviewChanges` |  |  |
| 68 | object_arrow_method |  | `saveEdits` |  |  |
| 69 | object_arrow_method |  | `refreshDocument` |  |  |
| 72 | function |  | `createDocumentEditApi` | yes |  |
| 75 | function |  | `logEditApi` |  |  |
| 79 | function |  | `refreshDocument` |  |  |
| 134 | function |  | `applyPatch` |  |  |
| 143 | function |  | `buildTextPatch` |  |  |
| 169 | function |  | `editRegionText` |  |  |
| 174 | function |  | `replaceRegionTexts` |  |  |
| 202 | function |  | `saveEdits` |  |  |
| 237 | function |  | `getReviewFeed` |  |  |
| 241 | function |  | `acceptReviewChange` |  |  |
| 251 | function |  | `rejectReviewChange` |  |  |
| 261 | function |  | `acceptAllReviewChanges` |  |  |
| 269 | function |  | `rejectAllReviewChanges` |  |  |

### `src/bridge/document/pdf_document_runtime.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | function |  | `getDocumentSession` |  |  |
| 24 | object_arrow_method |  | `ensureWasmInitialized` |  |  |
| 25 | object_arrow_method |  | `getWasmApi` |  |  |
| 26 | object_arrow_method |  | `getTargetZoom` |  |  |
| 27 | object_arrow_method |  | `resolveHostScrollRefresh` |  |  |
| 28 | object_arrow_method |  | `getScrollContainer` |  |  |
| 30 | object_arrow_method |  | `renderCurrentFrame` |  |  |
| 31 | object_arrow_method |  | `refreshMutatedDocument` |  |  |
| 32 | object_arrow_method |  | `clearVectorHost` |  |  |
| 33 | object_arrow_method |  | `clearEditorHost` |  |  |
| 34 | object_arrow_method |  | `syncZoomSelect` |  |  |
| 35 | object_arrow_method |  | `syncTextEditButton` |  |  |
| 36 | object_arrow_method |  | `syncViewerState` |  |  |
| 37 | object_arrow_method |  | `resetZoomPreview` |  |  |
| 38 | object_arrow_method |  | `clearPendingAnchor` |  |  |
| 39 | object_arrow_method |  | `showEmptyDocumentState` |  |  |
| 45 | object_arrow_method |  | `renderCurrentPage` |  |  |
| 46 | object_arrow_method |  | `bindTileRefreshOnScroll` |  |  |
| 47 | object_arrow_method |  | `openTextPdfFlow` |  |  |
| 48 | object_arrow_method |  | `resetPdfViewerState` |  |  |
| 51 | function |  | `createPdfDocumentRuntime` | yes |  |
| 54 | function |  | `renderCurrentPage` |  |  |
| 65 | function |  | `bindTileRefreshOnScroll` |  |  |
| 82 | function |  | `openTextPdfFlow` |  |  |
| 115 | function |  | `resetPdfViewerState` |  |  |

### `src/bridge/editor/api.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 27 | function |  | `getSession` |  |  |
| 39 | function |  | `begin` | yes |  |
| 43 | function |  | `hitTest` | yes |  |
| 47 | function |  | `openBlock` | yes |  |
| 51 | function |  | `moveCaret` | yes |  |
| 55 | function |  | `closeBlock` | yes |  |
| 59 | function |  | `commit` | yes |  |
| 63 | function |  | `discard` | yes |  |
| 67 | function |  | `getSnapshot` | yes |  |
| 71 | function |  | `isActive` | yes |  |
| 75 | function |  | `hasUnsavedChanges` | yes |  |
| 81 | function |  | `syncInput` | yes |  |
| 85 | function |  | `applyCommand` | yes |  |
| 89 | function |  | `setEditMode` | yes |  |
| 93 | function |  | `readLegacySnapshot` | yes |  |
| 97 | function |  | `paintCanvas` | yes |  |
| 106 | function |  | `utf16ToCharIndex` | yes |  |
| 110 | function |  | `charToUtf16Offset` | yes |  |
| 114 | function |  | `hasSessionChanges` | yes |  |
| 120 | function |  | `insertText` | yes |  |
| 124 | function |  | `deleteText` | yes |  |
| 128 | function |  | `applyFormat` | yes |  |
| 132 | function |  | `getTextBlocks` | yes |  |
| 136 | function |  | `getFormatState` | yes |  |
| 142 | function |  | `openRegion` | yes |  |
| 153 | function |  | `setDisplayZoom` | yes |  |
| 157 | function |  | `readDiagnostics` | yes |  |
| 161 | function |  | `saveSession` | yes |  |

### `src/bridge/editor/editor_host_view.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 86 | object_arrow_method |  | `readCaretIndex` |  |  |
| 87 | object_arrow_method |  | `writeCaretIndex` |  |  |
| 88 | object_arrow_method |  | `onCommitRequested` |  |  |
| 98 | object_arrow_method |  | `onCompositionSyncRequested` |  |  |
| 99 | object_arrow_method |  | `shouldSuppressNativeInput` |  |  |
| 100 | object_arrow_method |  | `shouldSuppressBlurCommit` |  |  |
| 101 | object_arrow_method |  | `onBlurCommitSuppressed` |  |  |
| 102 | object_arrow_method |  | `onBlurCommitRequested` |  |  |
| 112 | object_arrow_method |  | `onRootPointerDown` |  |  |
| 113 | object_arrow_method |  | `logNode` |  |  |
| 116 | function |  | `markPrimaryPointerDown` |  |  |
| 120 | function |  | `shouldIgnoreCompatibilityMouseDown` |  |  |
| 127 | function |  | `bindPrimaryPress` |  |  |
| 129 | object_arrow_method |  | `handler` |  |  |
| 143 | function |  | `ensureInteractionRoot` |  |  |
| 169 | function |  | `ensureEditorHostView` | yes |  |
| 286 | function |  | `hideEditorShell` | yes |  |
| 294 | function |  | `hideInteractionTargets` | yes |  |
| 299 | function |  | `showInteractionTargets` |  |  |
| 303 | function |  | `snapshotHostOverlays` | yes |  |
| 312 | function |  | `suspendHostOverlays` | yes |  |
| 323 | function |  | `restoreHostOverlays` | yes |  |
| 334 | function |  | `positionEditorShell` | yes |  |
| 358 | function |  | `renderInteractionTargets` | yes |  |
| 361 | object_arrow_method |  | `onTargetPointerDown` |  |  |
| 391 | function |  | `readHostReferenceBox` | yes |  |
| 401 | function |  | `bindTextareaEvents` |  |  |

### `src/bridge/editor/editor_wasm_api.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 18 | function |  | `getDocumentSession` |  |  |
| 28 | function |  | `getReviewSession` |  |  |
| 95 | object_arrow_method |  | `applyDocumentPatch` |  |  |
| 115 | object_arrow_method |  | `getReviewFeed` |  |  |
| 117 | object_arrow_method |  | `acceptReviewChange` |  |  |
| 119 | object_arrow_method |  | `rejectReviewChange` |  |  |
| 121 | object_arrow_method |  | `acceptAllReviewChanges` |  |  |
| 123 | object_arrow_method |  | `rejectAllReviewChanges` |  |  |
| 125 | object_arrow_method |  | `saveSession` |  |  |
| 128 | function |  | `callMethod` |  |  |
| 138 | function |  | `createEditorWasmApi` | yes |  |

### `src/bridge/editor/index.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 32 | object_arrow_method |  | `getWasmApi` |  |  |
| 33 | object_arrow_method |  | `getCurrentPath` |  |  |
| 34 | object_arrow_method |  | `getCurrentPage` |  |  |
| 35 | object_arrow_method |  | `getCurrentZoom` |  |  |
| 36 | object_arrow_method |  | `getPageWidth` |  |  |
| 37 | object_arrow_method |  | `getPageHeight` |  |  |
| 38 | object_arrow_method |  | `getVectorContainer` |  |  |
| 39 | object_arrow_method |  | `buildRenderRequest` |  |  |
| 40 | object_arrow_method |  | `renderScheduledFrame` |  |  |
| 41 | object_arrow_method |  | `renderCurrentPage` |  |  |
| 42 | object_arrow_method |  | `saveEditorSession` |  |  |
| 49 | object_arrow_method |  | `syncTargets` |  |  |
| 50 | object_arrow_method |  | `clear` |  |  |
| 51 | object_arrow_method |  | `commitActiveEditor` |  |  |
| 52 | object_arrow_method |  | `saveEdits` |  |  |
| 53 | object_arrow_method |  | `applyFormatAction` |  |  |
| 60 | object_arrow_method |  | `hasPendingEdits` |  |  |
| 61 | object_arrow_method |  | `setTextEditEnabled` |  |  |
| 62 | object_arrow_method |  | `isTextEditEnabled` |  |  |
| 67 | function |  | `createEditorHost` | yes |  |
| 81 | function |  | `withSuppressedNativeInput` |  |  |
| 90 | function |  | `readTextareaCaret` |  |  |
| 96 | function |  | `writeTextareaCaret` |  |  |
| 109 | function |  | `rememberRustCaret` |  |  |
| 115 | function |  | `clearDomSelection` |  |  |
| 129 | function |  | `getLastDisplayZoom` |  |  |
| 135 | function |  | `ensureNodes` |  |  |
| 139 | object_arrow_method |  | `onCommitRequested` |  |  |
| 145 | object_arrow_method |  | `onNavigationRequested` |  |  |
| 160 | object_arrow_method |  | `onBeforeInputRequested` |  |  |
| 200 | object_arrow_method |  | `onCompositionSyncRequested` |  |  |
| 205 | object_arrow_method |  | `shouldSuppressNativeInput` |  |  |
| 206 | object_arrow_method |  | `shouldSuppressBlurCommit` |  |  |
| 210 | object_arrow_method |  | `onBlurCommitSuppressed` |  |  |
| 213 | object_arrow_method |  | `onBlurCommitRequested` |  |  |
| 217 | object_arrow_method |  | `onShellPointerDown` |  |  |
| 255 | object_arrow_method |  | `onTargetPointerDown` |  |  |
| 258 | object_arrow_method |  | `onRootPointerDown` |  |  |
| 324 | object_arrow_method |  | `logNode` |  |  |
| 332 | function |  | `setupActiveEditor` |  |  |
| 361 | function |  | `scheduleOpenFocusStabilization` |  |  |
| 382 | function |  | `hideEditorShell` |  |  |
| 390 | function |  | `readLegacySnapshot` |  |  |
| 394 | function |  | `renderActiveEditor` |  |  |
| 426 | function |  | `commitEditor` |  |  |
| 453 | function |  | `commitForSave` |  |  |
| 474 | function |  | `closeEditor` |  |  |
| 484 | function |  | `syncFormatButtons` |  |  |
| 501 | function |  | `openEditor` |  |  |
| 554 | function |  | `resolveTargetReferenceBox` |  |  |
| 570 | function |  | `syncTargets` |  |  |
| 623 | function |  | `saveEdits` |  |  |
| 649 | object_arrow_method |  | `clear` |  |  |
| 657 | object_arrow_method |  | `applyFormatAction` |  |  |
| 662 | object_arrow_method |  | `openRegionEditor` |  |  |
| 685 | object_arrow_method |  | `hasPendingEdits` |  |  |
| 686 | object_arrow_method |  | `setTextEditEnabled` |  |  |
| 696 | object_arrow_method |  | `isTextEditEnabled` |  |  |

### `src/bridge/find/find_facade.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 76 | function |  | `callWasm` |  |  |
| 89 | function |  | `findInPageAsync` | yes |  |
| 102 | function |  | `findInDocumentAsync` | yes |  |
| 115 | function |  | `replaceOne` | yes |  |
| 119 | function |  | `replaceAll` | yes |  |

### `src/bridge/find/pdf_find_controller.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 27 | object_arrow_method |  | `getViewerSession` |  |  |
| 28 | object_arrow_method |  | `getWasmApi` |  |  |
| 29 | object_arrow_method |  | `getScrollContainer` |  |  |
| 30 | object_arrow_method |  | `goToPage` |  |  |
| 41 | object_arrow_method |  | `initialize` |  |  |
| 42 | object_arrow_method |  | `toggle` |  |  |
| 43 | object_arrow_method |  | `open` |  |  |
| 44 | object_arrow_method |  | `close` |  |  |
| 45 | object_arrow_method |  | `refresh` |  |  |
| 46 | object_arrow_method |  | `clear` |  |  |
| 47 | object_arrow_method |  | `focusInput` |  |  |
| 48 | object_arrow_method |  | `next` |  |  |
| 49 | object_arrow_method |  | `prev` |  |  |
| 50 | object_arrow_method |  | `replaceCurrent` |  |  |
| 51 | object_arrow_method |  | `replaceAll` |  |  |
| 98 | function |  | `getFindSession` |  |  |
| 108 | function |  | `callSession` |  |  |
| 135 | function |  | `getNodes` |  |  |
| 154 | function |  | `createPdfFindController` | yes |  |
| 158 | function |  | `findSession` |  |  |
| 159 | function |  | `readScope` |  |  |
| 165 | function |  | `renderToolbarFromWasm` |  |  |
| 184 | function |  | `renderOverlayFromUpdate` |  |  |
| 225 | function |  | `scrollActiveIntoView` |  |  |
| 238 | function |  | `renderFindUi` |  |  |
| 250 | function |  | `executeSearch` |  |  |
| 273 | function |  | `scheduleSearch` |  |  |
| 280 | function |  | `focusInput` |  |  |
| 286 | function |  | `open` |  |  |
| 293 | function |  | `close` |  |  |
| 297 | function |  | `toggle` |  |  |
| 307 | function |  | `next` |  |  |
| 312 | function |  | `prev` |  |  |
| 317 | function |  | `replaceCurrent` |  |  |
| 349 | function |  | `replaceAll` |  |  |
| 376 | function |  | `refresh` |  |  |
| 396 | function |  | `clear` |  |  |
| 403 | function |  | `initialize` |  |  |

### `src/bridge/index.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 12 | object_arrow_method |  | `initialize` |  |  |
| 38 | object_arrow_method |  | `destroy` |  |  |
| 46 | object_arrow_method |  | `readPath` |  |  |
| 47 | object_arrow_method |  | `readCurrentPage` |  |  |
| 48 | object_arrow_method |  | `readPageCount` |  |  |
| 49 | object_arrow_method |  | `requestPageTurn` |  |  |
| 51 | object_arrow_method |  | `setCurrentPage` |  |  |
| 52 | object_arrow_method |  | `refreshDocument` |  |  |

### `src/bridge/presentation/page_presenter.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 25 | object_arrow_method |  | `getWrapper` |  |  |
| 26 | object_arrow_method |  | `getRasterTarget` |  |  |
| 27 | object_arrow_method |  | `getEmptyState` |  |  |
| 28 | object_arrow_method |  | `clearEditorOverlay` |  |  |
| 31 | function |  | `logPresent` |  |  |
| 35 | function |  | `createPagePresenter` | yes |  |
| 36 | function |  | `prepareRasterSurface` |  |  |
| 92 | function |  | `commitRasterSurface` |  |  |
| 154 | function |  | `presentRaster` |  |  |
| 172 | function |  | `commitReadySurfaceOrFallback` |  |  |

### `src/bridge/render/frame_plan.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 123 | object_arrow_method |  | `getWasmApi` |  |  |
| 124 | object_arrow_method |  | `getScrollContainer` |  |  |
| 125 | object_arrow_method |  | `getPageWidth` |  |  |
| 126 | object_arrow_method |  | `getPageHeight` |  |  |
| 127 | object_arrow_method |  | `getMaxZoom` |  |  |
| 128 | object_arrow_method |  | `getMaxCanvasDim` |  |  |
| 134 | object_arrow_method |  | `buildRenderRequest` |  |  |
| 135 | object_arrow_method |  | `peek` |  |  |
| 136 | object_arrow_method |  | `take` |  |  |
| 137 | object_arrow_method |  | `stepPreview` |  |  |
| 138 | object_arrow_method |  | `resolveViewportRefresh` |  |  |
| 139 | object_arrow_method |  | `resolveHostScrollRefresh` |  |  |
| 140 | object_arrow_method |  | `scheduleRender` |  |  |
| 141 | object_arrow_method |  | `settleRender` |  |  |
| 142 | object_arrow_method |  | `abortRender` |  |  |
| 143 | object_arrow_method |  | `commitRenderResult` |  |  |
| 144 | object_arrow_method |  | `resolveWheelRenderDecision` |  |  |
| 145 | object_arrow_method |  | `resolvePreviewTickDecision` |  |  |
| 146 | object_arrow_method |  | `scheduleRenderFollowUp` |  |  |
| 147 | object_arrow_method |  | `handleWheelZoomHost` |  |  |
| 148 | object_arrow_method |  | `stepPreviewHost` |  |  |
| 149 | object_arrow_method |  | `resolveLayerExecutionPlan` |  |  |
| 150 | object_arrow_method |  | `resolveLayerPresentDecision` |  |  |
| 151 | object_arrow_method |  | `setWheelRenderPending` |  |  |
| 152 | object_arrow_method |  | `getWheelRenderPending` |  |  |
| 153 | object_arrow_method |  | `queueCommittedFrame` |  |  |
| 154 | object_arrow_method |  | `takeReadyCommittedFrame` |  |  |
| 155 | object_arrow_method |  | `isRenderFrameCurrent` |  |  |
| 156 | object_arrow_method |  | `queueRenderLoopFrame` |  |  |
| 157 | object_arrow_method |  | `advanceRenderLoopFrame` |  |  |
| 160 | function |  | `createFramePlanAdapter` | yes |  |
| 163 | function |  | `buildRequest` |  |  |
| 203 | function |  | `buildRenderRequest` |  |  |
| 207 | function |  | `peek` |  |  |
| 215 | function |  | `take` |  |  |
| 223 | function |  | `stepPreview` |  |  |
| 235 | function |  | `resolveViewportRefresh` |  |  |
| 247 | function |  | `resolveHostScrollRefresh` |  |  |
| 259 | function |  | `scheduleRender` |  |  |
| 267 | function |  | `settleRender` |  |  |
| 283 | function |  | `abortRender` |  |  |
| 294 | function |  | `commitRenderResult` |  |  |
| 312 | function |  | `resolveWheelRenderDecision` |  |  |
| 320 | function |  | `resolvePreviewTickDecision` |  |  |
| 328 | function |  | `scheduleRenderFollowUp` |  |  |
| 339 | function |  | `handleWheelZoomHost` |  |  |
| 358 | function |  | `stepPreviewHost` |  |  |
| 377 | function |  | `setWheelRenderPending` |  |  |
| 384 | function |  | `getWheelRenderPending` |  |  |
| 392 | function |  | `queueCommittedFrame` |  |  |
| 399 | function |  | `takeReadyCommittedFrame` |  |  |
| 407 | function |  | `isRenderFrameCurrent` |  |  |
| 416 | function |  | `queueRenderLoopFrame` |  |  |
| 424 | function |  | `advanceRenderLoopFrame` |  |  |
| 432 | function |  | `resolveLayerExecutionPlan` |  |  |
| 443 | function |  | `resolveLayerPresentDecision` |  |  |

### `src/bridge/render/layout_trace.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 37 | function |  | `round` |  |  |
| 41 | function |  | `snapshotElement` |  |  |
| 69 | function |  | `readPdfLayoutSnapshot` | yes |  |
| 109 | function |  | `compactValue` |  |  |
| 161 | function |  | `compactDetails` |  |  |
| 167 | function |  | `formatDetails` |  |  |
| 185 | function |  | `logPdfLayoutTrace` | yes |  |

### `src/bridge/render/raster_image_cache.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 15 | function |  | `summarizeRasterSrc` |  |  |
| 27 | function |  | `logRasterCache` |  |  |
| 37 | function |  | `getBitmapByteSize` |  |  |
| 42 | function |  | `rememberRasterImage` |  |  |
| 81 | function |  | `readDecodedRasterImage` | yes |  |
| 89 | function |  | `warmRasterImage` | yes |  |
| 111 | arrow_fn |  | `task` |  |  |
| 149 | function |  | `clearRasterImageCache` | yes |  |

### `src/bridge/render/render_flow.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 10 | object_arrow_method |  | `targetInvokeV3` |  |  |
| 14 | object_arrow_method |  | `clearPendingAnchor` |  |  |
| 15 | object_arrow_method |  | `commitRenderedFrame` |  |  |
| 16 | object_arrow_method |  | `getWrapper` |  |  |
| 17 | object_arrow_method |  | `getRasterTarget` |  |  |
| 18 | object_arrow_method |  | `getEmptyState` |  |  |
| 19 | object_arrow_method |  | `getPageIndicator` |  |  |
| 20 | object_arrow_method |  | `showWrapper` |  |  |
| 21 | object_arrow_method |  | `onPageDimensionsResolved` |  |  |
| 22 | object_arrow_method |  | `syncEditorOverlay` |  |  |
| 23 | object_arrow_method |  | `clearEditorOverlay` |  |  |
| 24 | object_arrow_method |  | `prepareRenderFrame` |  |  |
| 25 | object_arrow_method |  | `scheduleRenderFollowUp` |  |  |
| 26 | object_arrow_method |  | `commitRenderResult` |  |  |
| 27 | object_arrow_method |  | `onRenderCommitted` |  |  |
| 32 | function |  | `createRenderFlow` | yes |  |
| 42 | function |  | `logRenderFlow` |  |  |
| 46 | function |  | `shouldPresentPreviewFirst` |  |  |
| 50 | function |  | `runRenderLoop` |  |  |
| 276 | object_arrow_method |  | `beforePresent` |  |  |
| 300 | object_arrow_method |  | `beforePresent` |  |  |
| 386 | function |  | `updateRasterFallback` |  |  |
| 412 | function |  | `renderCurrentPage` |  |  |
| 416 | function |  | `executeActualRender` |  |  |
| 437 | object_arrow_method |  | `getLastVisibleSurface` |  |  |
| 438 | object_arrow_method |  | `getLastRenderedPageIndex` |  |  |

### `src/bridge/render/render_scheduler.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 19 | object_arrow_method |  | `executeRender` |  |  |
| 24 | object_arrow_method |  | `requestRender` |  |  |
| 25 | object_arrow_method |  | `notifyCommit` |  |  |
| 26 | object_arrow_method |  | `reset` |  |  |
| 31 | object_arrow_method |  | `resolve` |  |  |
| 34 | function |  | `createRenderScheduler` | yes |  |
| 47 | function |  | `resolveQueueAction` |  |  |
| 56 | function |  | `isScrollSuppressed` |  |  |
| 60 | function |  | `dispatch` |  |  |
| 125 | function |  | `makeRequest` |  |  |
| 139 | function |  | `requestScroll` |  |  |
| 170 | function |  | `requestRender` |  |  |
| 188 | function |  | `notifyCommit` |  |  |
| 192 | function |  | `reset` |  |  |

### `src/bridge/render/render_wasm_api.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 53 | object_arrow_method |  | `resolveFramePlan` |  |  |
| 54 | object_arrow_method |  | `takeFramePlan` |  |  |
| 55 | object_arrow_method |  | `stepZoomFramePlan` |  |  |
| 56 | object_arrow_method |  | `resolveViewportRefresh` |  |  |
| 57 | object_arrow_method |  | `resolveHostScrollRefresh` |  |  |
| 58 | object_arrow_method |  | `scheduleRenderFrame` |  |  |
| 59 | object_arrow_method |  | `markRenderedZoom` |  |  |
| 60 | object_arrow_method |  | `settleRenderFrame` |  |  |
| 61 | object_arrow_method |  | `abortRenderFrame` |  |  |
| 68 | object_arrow_method |  | `resolveWheelRenderDecision` |  |  |
| 69 | object_arrow_method |  | `resolvePreviewTickDecision` |  |  |
| 70 | object_arrow_method |  | `scheduleRenderFollowUp` |  |  |
| 71 | object_arrow_method |  | `handleWheelZoomHost` |  |  |
| 72 | object_arrow_method |  | `stepPreviewHost` |  |  |
| 73 | object_arrow_method |  | `setWheelRenderPending` |  |  |
| 74 | object_arrow_method |  | `getWheelRenderPending` |  |  |
| 75 | object_arrow_method |  | `queueCommittedFrame` |  |  |
| 76 | object_arrow_method |  | `takeReadyCommittedFrame` |  |  |
| 77 | object_arrow_method |  | `isRenderFrameCurrent` |  |  |
| 78 | object_arrow_method |  | `queueRenderLoopFrame` |  |  |
| 79 | object_arrow_method |  | `advanceRenderLoopFrame` |  |  |
| 80 | object_arrow_method |  | `resolveLayerExecutionPlan` |  |  |
| 81 | object_arrow_method |  | `resolveRenderExecutionPlan` |  |  |
| 82 | object_arrow_method |  | `resolveLayerPresentDecision` |  |  |
| 83 | object_arrow_method |  | `cancelProgressiveRender` |  |  |
| 84 | object_arrow_method |  | `resetFrameCache` |  |  |
| 103 | object_arrow_method |  | `touchFrameCacheEntry` |  |  |
| 104 | object_arrow_method |  | `storeFrameCacheEntry` |  |  |
| 105 | object_arrow_method |  | `startProgressiveRender` |  |  |
| 106 | object_arrow_method |  | `renderPage` |  |  |
| 107 | object_arrow_method |  | `renderPageOffscreen` |  |  |
| 108 | object_arrow_method |  | `resolveProgressiveRenderPolicy` |  |  |
| 124 | function |  | `createRenderWasmApi` | yes |  |

### `src/bridge/render/vector_canvas_host.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 37 | function |  | `hideLegacyRasterHost` |  |  |
| 45 | function |  | `configureCanvas` |  |  |
| 56 | function |  | `configureStageCanvas` |  |  |
| 69 | function |  | `ensureCanvas` |  |  |
| 80 | function |  | `ensureStageCanvas` |  |  |
| 91 | function |  | `ensureCanvasBitmap` |  |  |
| 102 | function |  | `applyCanvasCssBox` |  |  |
| 115 | function |  | `hideDetailCanvas` |  |  |
| 125 | function |  | `getPresentCanvas` |  |  |
| 129 | function |  | `clearVectorCanvasHost` | yes |  |
| 147 | function |  | `ensureVectorCanvasHost` | yes |  |
| 202 | function |  | `getExistingVectorCanvasHost` | yes |  |
| 214 | function |  | `hideVectorCanvasHostForPreview` | yes |  |
| 221 | function |  | `getRenderBufferCanvas` | yes |  |
| 225 | function |  | `applyViewportCanvasFrame` | yes |  |
| 277 | function |  | `presentViewportCanvas` | yes |  |
| 316 | function |  | `presentViewportCanvasFromSource` | yes |  |
| 355 | function |  | `stageViewportCanvasFromSource` | yes |  |

### `src/bridge/render/vector_canvas_pool.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 14 | class_method | CanvasPool | `rent` |  |  |
| 48 | class_method | CanvasPool | `recycle` |  |  |
| 74 | class_method | CanvasPool | `clear` |  |  |

### `src/bridge/render/vector_frame_cache.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 6 | function |  | `cloneCanvas` |  |  |
| 15 | function |  | `clearVectorFrameCache` | yes |  |
| 25 | function |  | `readViewportFrameCache` | yes |  |
| 36 | function |  | `writeViewportFrameCache` | yes |  |
| 50 | function |  | `deleteViewportFrameCacheKeys` | yes |  |

### `src/bridge/render/vector_host.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 31 | function |  | `ensureVectorWorker` |  |  |
| 106 | function |  | `logRenderChain` |  |  |
| 112 | function |  | `isFrameCurrent` |  |  |
| 121 | function |  | `abortStaleFrameIfNeeded` |  |  |
| 135 | function |  | `clearVectorHost` | yes |  |
| 148 | function |  | `invalidateVectorRenderCache` | yes |  |
| 160 | function |  | `ensureVectorHost` | yes |  |
| 164 | function |  | `commitVectorRenderResult` | yes |  |
| 205 | function |  | `renderVectorPage` |  |  |
| 220 | function |  | `renderVectorPageWithPlan` | yes |  |
| 696 | function |  | `renderViewportProgressiveIfNeeded` |  |  |

### `src/bridge/render/vector_page_bundle.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 26 | object_arrow_method |  | `getWasmApi` |  |  |
| 29 | object_arrow_method |  | `getWasmApi` |  |  |
| 30 | object_arrow_method |  | `getFallbackPageWidth` |  |  |
| 31 | object_arrow_method |  | `getFallbackPageHeight` |  |  |
| 34 | function |  | `configureVectorPageBundleRuntime` | yes |  |
| 42 | function |  | `isFrameCurrent` |  |  |
| 52 | function |  | `getCurrentPageIndex` |  |  |
| 62 | function |  | `resolveAssetRole` |  |  |
| 68 | function |  | `admitPageAsset` |  |  |
| 82 | function |  | `findCachedBundle` |  |  |
| 105 | function |  | `insertCachedBundle` |  |  |
| 115 | function |  | `invalidateVectorPageCache` | yes |  |
| 122 | function |  | `summarizeText` |  |  |
| 127 | function |  | `summarizeVectorModel` |  |  |
| 155 | function |  | `summarizePaintPlan` |  |  |
| 185 | function |  | `loadImageCacheMapForPage` |  |  |
| 230 | function |  | `resolveVectorPageBundle` | yes |  |
| 394 | function |  | `prefetchAdjacentPages` | yes |  |
| 417 | function |  | `hasVectorPageBundle` | yes |  |
| 422 | function |  | `prefetchVectorPage` | yes |  |
| 428 | function |  | `isPageBundleCached` | yes |  |

### `src/bridge/review/pdf_review_controller.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 21 | object_arrow_method |  | `getViewerSession` |  |  |
| 23 | object_arrow_method |  | `goToPage` |  |  |
| 56 | object_arrow_method |  | `initialize` |  |  |
| 57 | object_arrow_method |  | `togglePanel` |  |  |
| 58 | object_arrow_method |  | `refresh` |  |  |
| 59 | object_arrow_method |  | `clear` |  |  |
| 69 | function |  | `getNodes` |  |  |
| 85 | function |  | `normalizeText` |  |  |
| 89 | function |  | `matchesReviewQuery` |  |  |
| 103 | function |  | `summarizeText` |  |  |
| 109 | function |  | `createBadge` |  |  |
| 121 | function |  | `computeVisibleChanges` |  |  |
| 135 | function |  | `createPdfReviewController` | yes |  |
| 142 | function |  | `syncPanelVisibility` |  |  |
| 158 | function |  | `clearView` |  |  |
| 172 | function |  | `locateChange` |  |  |
| 189 | function |  | `rejectChange` |  |  |
| 208 | function |  | `acceptChange` |  |  |
| 227 | function |  | `acceptAllChanges` |  |  |
| 244 | function |  | `rejectAllChanges` |  |  |
| 261 | function |  | `renderFeed` |  |  |
| 468 | function |  | `refresh` |  |  |
| 482 | function |  | `togglePanel` |  |  |
| 491 | function |  | `clear` |  |  |
| 497 | function |  | `initialize` |  |  |

### `src/bridge/review/review_wasm_facade.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | function |  | `getReviewSession` |  |  |
| 53 | function |  | `callMethod` |  |  |
| 65 | function |  | `callRawWasm` |  |  |
| 78 | function |  | `getReviewFeed` | yes |  |
| 82 | function |  | `acceptChange` | yes |  |
| 87 | function |  | `rejectChange` | yes |  |
| 92 | function |  | `acceptAllChanges` | yes |  |
| 97 | function |  | `rejectAllChanges` | yes |  |
| 102 | function |  | `locateChange` | yes |  |

### `src/bridge/shared/diagnostics.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 33 | function |  | `diagnosticsEnabled` |  |  |
| 37 | function |  | `verbosePdfDiagnosticsEnabled` | yes |  |
| 41 | function |  | `compactString` |  |  |
| 50 | function |  | `compactValue` |  |  |
| 93 | function |  | `nowStamp` |  |  |
| 102 | function |  | `normalizeLayer` |  |  |
| 117 | function |  | `inferLevel` |  |  |
| 127 | function |  | `formatFields` |  |  |
| 135 | function |  | `formatLayeredDiagnostic` |  |  |
| 151 | function |  | `formatPdfDiagnostic` | yes |  |
| 155 | function |  | `emitPdfDiagnostic` | yes |  |

### `src/bridge/shared/wasm_loader.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 14 | function |  | `installTargetInvokeBridge` |  |  |
| 49 | function |  | `ensureWasmInitialized` | yes |  |
| 81 | function |  | `getWasmApi` | yes |  |
| 87 | function |  | `targetInvokeV3` | yes |  |

### `src/bridge/viewer/page_presentation_runtime.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 66 | object_arrow_method |  | `requestPageTurn` |  |  |
| 67 | object_arrow_method |  | `readPageTurn` |  |  |
| 68 | object_arrow_method |  | `isLatestPageTurn` |  |  |
| 69 | object_arrow_method |  | `markPageVisible` |  |  |
| 70 | object_arrow_method |  | `canPrefetch` |  |  |
| 71 | object_arrow_method |  | `admitPageAsset` |  |  |
| 72 | object_arrow_method |  | `decideAdjacentPrefetch` |  |  |
| 79 | object_arrow_method |  | `reset` |  |  |
| 83 | object_arrow_method |  | `getWasmApi` |  |  |
| 90 | function |  | `getRuntimeHandle` |  |  |
| 100 | function |  | `normalizeDecision` |  |  |
| 113 | function |  | `normalizeVisibleDecision` |  |  |
| 125 | function |  | `normalizeAssetAdmission` |  |  |
| 137 | function |  | `normalizePrefetchDecision` |  |  |
| 156 | function |  | `fallbackRenderQueueAction` |  |  |
| 202 | function |  | `normalizePendingQueueEffect` |  |  |
| 219 | function |  | `normalizeRenderQueueAction` |  |  |
| 249 | function |  | `createPagePresentationRuntimeAdapter` | yes |  |
| 252 | function |  | `runtime` |  |  |
| 256 | function |  | `requestPageTurn` |  |  |
| 274 | function |  | `readPageTurn` |  |  |
| 282 | function |  | `isLatestPageTurn` |  |  |
| 290 | function |  | `markPageVisible` |  |  |
| 306 | function |  | `canPrefetch` |  |  |
| 314 | function |  | `admitPageAsset` |  |  |
| 330 | function |  | `decideAdjacentPrefetch` |  |  |
| 345 | function |  | `resolveRenderQueueAction` |  |  |
| 359 | function |  | `reset` |  |  |

### `src/bridge/viewer/pdf_keyboard.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 4 | object_arrow_method |  | `isTextEditEnabled` |  |  |
| 5 | object_arrow_method |  | `getScrollContainer` |  |  |
| 6 | object_arrow_method |  | `openFind` |  |  |
| 7 | object_arrow_method |  | `undo` |  |  |
| 8 | object_arrow_method |  | `redo` |  |  |
| 9 | object_arrow_method |  | `toggleBold` |  |  |
| 10 | object_arrow_method |  | `toggleItalic` |  |  |
| 11 | object_arrow_method |  | `toggleUnderline` |  |  |
| 12 | object_arrow_method |  | `prevPage` |  |  |
| 13 | object_arrow_method |  | `nextPage` |  |  |
| 17 | function |  | `isPdfViewerKeyboardScope` |  |  |
| 31 | function |  | `isPlainEditableTarget` |  |  |
| 38 | function |  | `createPdfKeyboardShortcutHandler` | yes |  |

### `src/bridge/viewer/pdf_layout_sync.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 19 | object_arrow_method |  | `getWasmApi` |  |  |
| 20 | object_arrow_method |  | `getPageWidth` |  |  |
| 21 | object_arrow_method |  | `getPageHeight` |  |  |
| 22 | object_arrow_method |  | `readZoomState` |  |  |
| 25 | function |  | `createLayoutSync` | yes |  |
| 26 | function |  | `syncLayoutBox` |  |  |

### `src/bridge/viewer/pdf_runtime.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 52 | object_arrow_method |  | `getWasmApi` |  |  |
| 64 | object_arrow_method |  | `renderCurrentPage` |  |  |
| 65 | object_arrow_method |  | `openTextPdfFlow` |  |  |
| 66 | object_arrow_method |  | `resetPdfViewerState` |  |  |
| 67 | object_arrow_method |  | `readTargetZoom` |  |  |
| 69 | object_arrow_method |  | `syncZoomSelect` |  |  |
| 70 | object_arrow_method |  | `syncTextEditButton` |  |  |
| 71 | object_arrow_method |  | `bindTileRefreshOnScroll` |  |  |
| 72 | object_arrow_method |  | `bindWheelZoom` |  |  |
| 73 | object_arrow_method |  | `handlePdfViewerKeydown` |  |  |
| 78 | function |  | `createPdfViewerRuntime` | yes |  |
| 82 | object_arrow_method |  | `getWasmApi` |  |  |
| 83 | object_arrow_method |  | `getFallbackPageWidth` |  |  |
| 84 | object_arrow_method |  | `getFallbackPageHeight` |  |  |
| 87 | object_arrow_method |  | `getWasmApi` |  |  |
| 94 | function |  | `getCurrentPageWidthValue` |  |  |
| 98 | function |  | `getCurrentPageHeightValue` |  |  |
| 102 | function |  | `readZoomState` |  |  |
| 125 | object_arrow_method |  | `getWasmApi` |  |  |
| 127 | object_arrow_method |  | `getPageWidth` |  |  |
| 128 | object_arrow_method |  | `getPageHeight` |  |  |
| 130 | object_arrow_method |  | `getMaxCanvasDim` |  |  |
| 134 | object_arrow_method |  | `getWasmApi` |  |  |
| 135 | object_arrow_method |  | `getPageWidth` |  |  |
| 136 | object_arrow_method |  | `getPageHeight` |  |  |
| 140 | function |  | `syncZoomSelect` |  |  |
| 151 | object_arrow_method |  | `getWasmApi` |  |  |
| 152 | object_arrow_method |  | `getCurrentPath` |  |  |
| 153 | object_arrow_method |  | `getCurrentPage` |  |  |
| 154 | object_arrow_method |  | `getCurrentZoom` |  |  |
| 155 | object_arrow_method |  | `buildRenderRequest` |  |  |
| 157 | object_arrow_method |  | `renderScheduledFrame` |  |  |
| 158 | object_arrow_method |  | `invalidateRenderCache` |  |  |
| 159 | object_arrow_method |  | `syncViewerState` |  |  |
| 167 | object_arrow_method |  | `getViewerSession` |  |  |
| 168 | object_arrow_method |  | `getWasmApi` |  |  |
| 171 | object_arrow_method |  | `goToPage` |  |  |
| 175 | object_arrow_method |  | `openRegionEditor` |  |  |
| 184 | object_arrow_method |  | `getViewerSession` |  |  |
| 188 | object_arrow_method |  | `getViewerSession` |  |  |
| 189 | object_arrow_method |  | `getWasmApi` |  |  |
| 191 | object_arrow_method |  | `goToPage` |  |  |
| 197 | object_arrow_method |  | `getViewerSession` |  |  |
| 199 | object_arrow_method |  | `goToPage` |  |  |
| 203 | object_arrow_method |  | `openRegionEditor` |  |  |
| 213 | function |  | `syncTextEditButton` |  |  |
| 218 | object_arrow_method |  | `getWasmApi` |  |  |
| 219 | object_arrow_method |  | `getCurrentPath` |  |  |
| 220 | object_arrow_method |  | `getCurrentPage` |  |  |
| 221 | object_arrow_method |  | `getCurrentZoom` |  |  |
| 222 | object_arrow_method |  | `getPageWidth` |  |  |
| 223 | object_arrow_method |  | `getPageHeight` |  |  |
| 225 | object_arrow_method |  | `buildRenderRequest` |  |  |
| 227 | object_arrow_method |  | `renderScheduledFrame` |  |  |
| 228 | object_arrow_method |  | `renderCurrentPage` |  |  |
| 229 | object_arrow_method |  | `saveEditorSession` |  |  |
| 230 | object_arrow_method |  | `syncViewerState` |  |  |
| 234 | object_arrow_method |  | `getCurrentPath` |  |  |
| 236 | object_arrow_method |  | `resetZoomPreviewState` |  |  |
| 243 | object_arrow_method |  | `getCurrentPageWidth` |  |  |
| 244 | object_arrow_method |  | `getCurrentPageHeight` |  |  |
| 250 | object_arrow_method |  | `requestRender` |  |  |
| 253 | object_arrow_method |  | `peekFramePlan` |  |  |
| 254 | object_arrow_method |  | `takeFramePlan` |  |  |
| 256 | object_arrow_method |  | `clearPendingAnchor` |  |  |
| 260 | object_arrow_method |  | `clearPreviewPresent` |  |  |
| 264 | object_arrow_method |  | `resolveWheelRenderDecision` |  |  |
| 265 | object_arrow_method |  | `handleWheelZoomHost` |  |  |
| 267 | object_arrow_method |  | `stepPreviewHost` |  |  |
| 269 | object_arrow_method |  | `setWheelRenderPending` |  |  |
| 270 | object_arrow_method |  | `getWheelRenderPending` |  |  |
| 271 | object_arrow_method |  | `queueCommittedFrame` |  |  |
| 272 | object_arrow_method |  | `takeReadyCommittedFrame` |  |  |
| 280 | object_arrow_method |  | `clearPendingAnchor` |  |  |
| 281 | object_arrow_method |  | `commitRenderedFrame` |  |  |
| 293 | object_arrow_method |  | `onPageDimensionsResolved` |  |  |
| 334 | object_arrow_method |  | `syncEditorOverlay` |  |  |
| 337 | object_arrow_method |  | `clearEditorOverlay` |  |  |
| 340 | object_arrow_method |  | `prepareRenderFrame` |  |  |
| 343 | object_arrow_method |  | `scheduleRenderFollowUp` |  |  |
| 345 | object_arrow_method |  | `commitRenderResult` |  |  |
| 364 | object_arrow_method |  | `onRenderCommitted` |  |  |
| 368 | object_arrow_method |  | `getViewerSession` |  |  |
| 370 | object_arrow_method |  | `openPdfPath` |  |  |
| 371 | object_arrow_method |  | `renderCurrentPage` |  |  |
| 372 | object_arrow_method |  | `setCurrentPage` |  |  |
| 377 | object_arrow_method |  | `getWasmApi` |  |  |
| 386 | object_arrow_method |  | `setPageDimensions` |  |  |
| 387 | object_arrow_method |  | `getPageWidth` |  |  |
| 388 | object_arrow_method |  | `getPageHeight` |  |  |
| 393 | function |  | `prefetchAdjacentPreviews` |  |  |
| 429 | function |  | `prefetchAdjacentAssets` |  |  |
| 443 | object_arrow_method |  | `isTextEditEnabled` |  |  |
| 445 | object_arrow_method |  | `openFind` |  |  |
| 446 | object_arrow_method |  | `undo` |  |  |
| 447 | object_arrow_method |  | `redo` |  |  |
| 448 | object_arrow_method |  | `toggleBold` |  |  |
| 449 | object_arrow_method |  | `toggleItalic` |  |  |
| 450 | object_arrow_method |  | `toggleUnderline` |  |  |
| 451 | object_arrow_method |  | `prevPage` |  |  |
| 452 | object_arrow_method |  | `nextPage` |  |  |
| 457 | object_arrow_method |  | `getWasmApi` |  |  |
| 458 | object_arrow_method |  | `getTargetZoom` |  |  |
| 459 | object_arrow_method |  | `resolveHostScrollRefresh` |  |  |
| 463 | object_arrow_method |  | `renderCurrentFrame` |  |  |
| 464 | object_arrow_method |  | `refreshMutatedDocument` |  |  |
| 466 | object_arrow_method |  | `clearEditorHost` |  |  |
| 469 | object_arrow_method |  | `syncViewerState` |  |  |
| 470 | object_arrow_method |  | `resetZoomPreview` |  |  |
| 471 | object_arrow_method |  | `clearPendingAnchor` |  |  |
| 479 | object_arrow_method |  | `executeRender` |  |  |
| 569 | function |  | `renderCurrentPage` |  |  |
| 573 | function |  | `openTextPdfFlow` |  |  |
| 583 | function |  | `resetPdfViewerState` |  |  |
| 596 | object_arrow_method |  | `getWasmApi` |  |  |
| 611 | object_arrow_method |  | `readTargetZoom` |  |  |
| 616 | object_arrow_method |  | `bindWheelZoom` |  |  |

### `src/bridge/viewer/pdf_viewer_api.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 11 | object_arrow_method |  | `ensureWasmInitialized` |  |  |
| 12 | object_arrow_method |  | `getWasmApi` |  |  |
| 13 | object_arrow_method |  | `readPath` |  |  |
| 14 | object_arrow_method |  | `readCurrentPage` |  |  |
| 15 | object_arrow_method |  | `readPageCount` |  |  |
| 16 | object_arrow_method |  | `requestPageTurn` |  |  |
| 17 | object_arrow_method |  | `setCurrentPage` |  |  |
| 18 | object_arrow_method |  | `refreshDocument` |  |  |
| 19 | object_arrow_method |  | `resetPdfViewerState` |  |  |
| 21 | object_arrow_method |  | `renderCurrentPage` |  |  |
| 22 | object_arrow_method |  | `clampZoom` |  |  |
| 23 | object_arrow_method |  | `syncZoomSelect` |  |  |
| 24 | object_arrow_method |  | `syncTextEditButton` |  |  |
| 25 | object_arrow_method |  | `readTargetZoom` |  |  |
| 27 | object_arrow_method |  | `clear` |  |  |
| 28 | object_arrow_method |  | `isTextEditEnabled` |  |  |
| 29 | object_arrow_method |  | `commitActiveEditor` |  |  |
| 30 | object_arrow_method |  | `saveEdits` |  |  |
| 35 | object_arrow_method |  | `applyFormatAction` |  |  |
| 36 | object_arrow_method |  | `setTextEditEnabled` |  |  |
| 37 | object_arrow_method |  | `syncTargets` |  |  |
| 43 | object_arrow_method |  | `toggle` |  |  |
| 44 | object_arrow_method |  | `open` |  |  |
| 45 | object_arrow_method |  | `close` |  |  |
| 46 | object_arrow_method |  | `next` |  |  |
| 47 | object_arrow_method |  | `prev` |  |  |
| 50 | object_arrow_method |  | `togglePanel` |  |  |
| 51 | object_arrow_method |  | `applyAllSuggestions` |  |  |
| 55 | object_arrow_method |  | `openTextPdfFlow` |  |  |
| 56 | object_arrow_method |  | `clearVectorHost` |  |  |
| 63 | class_method | PdfViewerAPI | `constructor` |  |  |
| 69 | class_method | PdfViewerAPI | `openPdfFile` |  |  |
| 90 | class_method | PdfViewerAPI | `closePdf` |  |  |
| 96 | class_method | PdfViewerAPI | `prevPage` |  |  |
| 108 | class_method | PdfViewerAPI | `nextPage` |  |  |
| 122 | class_method | PdfViewerAPI | `setZoom` |  |  |
| 135 | class_method | PdfViewerAPI | `undo` |  |  |
| 148 | class_method | PdfViewerAPI | `redo` |  |  |
| 161 | class_method | PdfViewerAPI | `rotate` |  |  |
| 167 | class_method | PdfViewerAPI | `save` |  |  |
| 180 | class_method | PdfViewerAPI | `toggleTextEditMode` |  |  |
| 194 | class_method | PdfViewerAPI | `toggleAnnotation` |  |  |
| 200 | class_method | PdfViewerAPI | `toggleComment` |  |  |
| 204 | class_method | PdfViewerAPI | `toggleCommentPanel` |  |  |
| 210 | class_method | PdfViewerAPI | `toggleReviewPanel` |  |  |
| 216 | class_method | PdfViewerAPI | `toggleBold` |  |  |
| 220 | class_method | PdfViewerAPI | `toggleItalic` |  |  |
| 224 | class_method | PdfViewerAPI | `toggleUnderline` |  |  |
| 228 | class_method | PdfViewerAPI | `setColor` |  |  |
| 232 | class_method | PdfViewerAPI | `setFontFamily` |  |  |
| 236 | class_method | PdfViewerAPI | `setFontSize` |  |  |
| 240 | class_method | PdfViewerAPI | `increaseFontSize` |  |  |
| 244 | class_method | PdfViewerAPI | `decreaseFontSize` |  |  |
| 248 | class_method | PdfViewerAPI | `setCharSpacing` |  |  |
| 252 | class_method | PdfViewerAPI | `setLineHeight` |  |  |
| 256 | class_method | PdfViewerAPI | `setParagraphMode` |  |  |
| 260 | class_method | PdfViewerAPI | `setAlignment` |  |  |
| 264 | class_method | PdfViewerAPI | `setListKind` |  |  |
| 270 | class_method | PdfViewerAPI | `toggleAiPanel` |  |  |
| 276 | class_method | PdfViewerAPI | `toggleFind` |  |  |
| 280 | class_method | PdfViewerAPI | `openFind` |  |  |
| 284 | class_method | PdfViewerAPI | `closeFind` |  |  |
| 288 | class_method | PdfViewerAPI | `findNext` |  |  |
| 292 | class_method | PdfViewerAPI | `findPrev` |  |  |
| 298 | class_method | PdfViewerAPI | `undoSavedEdit` |  |  |
| 311 | class_method | PdfViewerAPI | `redoSavedEdit` |  |  |
| 336 | function |  | `wait` |  |  |
| 340 | function |  | `runPageTurnBench` |  |  |
| 420 | function |  | `registerPdfViewerAPI` | yes |  |
| 485 | function |  | `getPdfViewerAPI` | yes |  |

### `src/bridge/viewer/pdf_viewer_dom.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | function |  | `getWrapper` | yes |  |
| 17 | function |  | `getScrollContainer` | yes |  |
| 21 | function |  | `getVectorContainer` | yes |  |
| 25 | function |  | `getRasterTarget` | yes |  |
| 29 | function |  | `getEmptyState` | yes |  |
| 33 | function |  | `getPageIndicator` | yes |  |
| 37 | function |  | `getDynamicMaxZoom` | yes |  |
| 41 | function |  | `clampZoom` | yes |  |
| 46 | function |  | `showDocumentWrapper` | yes |  |
| 53 | function |  | `showEmptyDocumentState` | yes |  |
| 66 | function |  | `syncZoomSelect` | yes |  |
| 93 | function |  | `syncTextEditButton` | yes |  |
| 99 | function |  | `syncEditorFormatButtons` | yes |  |
| 127 | arrow_fn |  | `applyState` |  |  |
| 162 | function |  | `bindSaveFocusGuard` | yes |  |
| 166 | arrow_fn |  | `keepEditorFocus` |  |  |
| 173 | function |  | `setToolbarButtonActive` | yes |  |

### `src/bridge/viewer/viewer_geometry_probe.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 7 | object_arrow_method |  | `ensureWasmInitialized` |  |  |
| 8 | object_arrow_method |  | `getWasmApi` |  |  |
| 11 | object_arrow_method |  | `getZoomState` |  |  |
| 12 | object_arrow_method |  | `getScrollContainer` |  |  |
| 13 | object_arrow_method |  | `getVectorContainer` |  |  |
| 26 | object_arrow_method |  | `syncZoomSelect` |  |  |
| 27 | object_arrow_method |  | `showWrapper` |  |  |
| 28 | object_arrow_method |  | `setPageDimensions` |  |  |
| 29 | object_arrow_method |  | `getPageWidth` |  |  |
| 30 | object_arrow_method |  | `getPageHeight` |  |  |
| 31 | object_arrow_method |  | `clampZoom` |  |  |
| 32 | object_arrow_method |  | `getMaxZoom` |  |  |
| 46 | object_arrow_method |  | `init` |  |  |
| 47 | object_arrow_method |  | `snapshot` |  |  |
| 48 | object_arrow_method |  | `wheelAtClient` |  |  |
| 51 | function |  | `projectSnapshot` |  |  |
| 91 | function |  | `applyRenderPlan` |  |  |
| 119 | function |  | `createViewerGeometryProbe` | yes |  |
| 120 | function |  | `init` |  |  |
| 142 | function |  | `snapshot` |  |  |
| 146 | function |  | `wheelAtClient` |  |  |

### `src/bridge/viewer/viewer_session.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 25 | object_arrow_method |  | `read` |  |  |
| 26 | object_arrow_method |  | `setDocument` |  |  |
| 27 | object_arrow_method |  | `reset` |  |  |
| 28 | object_arrow_method |  | `setCurrentPage` |  |  |
| 29 | object_arrow_method |  | `setCurrentZoom` |  |  |
| 30 | object_arrow_method |  | `setPageDimensions` |  |  |
| 34 | object_arrow_method |  | `getWasmApi` |  |  |
| 35 | object_arrow_method |  | `getFallbackPageWidth` |  |  |
| 36 | object_arrow_method |  | `getFallbackPageHeight` |  |  |
| 41 | function |  | `getViewerSession` |  |  |
| 51 | function |  | `createViewerSessionAdapter` | yes |  |
| 52 | function |  | `session` |  |  |
| 54 | function |  | `read` |  |  |
| 79 | function |  | `setDocument` |  |  |
| 83 | function |  | `reset` |  |  |
| 87 | function |  | `setCurrentPage` |  |  |
| 102 | function |  | `setCurrentZoom` |  |  |
| 106 | function |  | `setPageDimensions` |  |  |

### `src/bridge/zoom/zoom_controller.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 54 | object_arrow_method |  | `getCurrentPath` |  |  |
| 55 | object_arrow_method |  | `getZoomState` |  |  |
| 56 | object_arrow_method |  | `resetZoomPreviewState` |  |  |
| 57 | object_arrow_method |  | `getCurrentPageWidth` |  |  |
| 58 | object_arrow_method |  | `getCurrentPageHeight` |  |  |
| 59 | object_arrow_method |  | `getWrapper` |  |  |
| 60 | object_arrow_method |  | `getScrollContainer` |  |  |
| 61 | object_arrow_method |  | `getVectorContainer` |  |  |
| 62 | object_arrow_method |  | `syncLayoutBox` |  |  |
| 63 | object_arrow_method |  | `syncZoomSelect` |  |  |
| 64 | object_arrow_method |  | `requestRender` |  |  |
| 65 | object_arrow_method |  | `peekFramePlan` |  |  |
| 66 | object_arrow_method |  | `takeFramePlan` |  |  |
| 67 | object_arrow_method |  | `getMaxZoom` |  |  |
| 68 | object_arrow_method |  | `clearPendingAnchor` |  |  |
| 69 | object_arrow_method |  | `clearPreviewPresent` |  |  |
| 70 | object_arrow_method |  | `resolveWheelRenderDecision` |  |  |
| 71 | object_arrow_method |  | `handleWheelZoomHost` |  |  |
| 72 | object_arrow_method |  | `stepPreviewHost` |  |  |
| 73 | object_arrow_method |  | `setWheelRenderPending` |  |  |
| 74 | object_arrow_method |  | `getWheelRenderPending` |  |  |
| 75 | object_arrow_method |  | `queueCommittedFrame` |  |  |
| 76 | object_arrow_method |  | `takeReadyCommittedFrame` |  |  |
| 80 | object_arrow_method |  | `bindWheelZoom` |  |  |
| 81 | object_arrow_method |  | `resetVisualZoomPreview` |  |  |
| 82 | object_arrow_method |  | `applyVisualZoomPreview` |  |  |
| 83 | object_arrow_method |  | `prepareImmediateRenderFrame` |  |  |
| 84 | object_arrow_method |  | `commitRenderedFrame` |  |  |
| 85 | object_arrow_method |  | `restorePendingAnchor` |  |  |
| 86 | object_arrow_method |  | `clearPendingAnchor` |  |  |
| 89 | function |  | `createZoomController` | yes |  |
| 94 | function |  | `isImmediateMutationFrame` |  |  |
| 98 | function |  | `stopSmoothZoomPreview` |  |  |
| 105 | function |  | `applyCommittedFrame` |  |  |
| 128 | function |  | `flushCommittedFrameIfSettled` |  |  |
| 135 | function |  | `applyVisualZoomPreview` |  |  |
| 169 | function |  | `applyPreviewFrame` |  |  |
| 203 | function |  | `resetVisualZoomPreview` |  |  |
| 224 | function |  | `startSmoothZoomPreview` |  |  |
| 227 | arrow_fn |  | `tick` |  |  |
| 278 | function |  | `restorePendingAnchor` |  |  |
| 292 | function |  | `commitRenderedFrame` |  |  |
| 322 | function |  | `prepareImmediateRenderFrame` |  |  |
| 348 | function |  | `scheduleWheelZoomRender` |  |  |
| 400 | function |  | `bindWheelZoom` |  |  |
| 472 | function |  | `clearPendingAnchor` |  |  |

### `src/dev/verify_editor_bugs.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 19 | function |  | `sleep` |  |  |
| 25 | function |  | `dispatchClickAtViewportPoint` |  |  |
| 45 | function |  | `getActiveTextarea` |  |  |
| 49 | function |  | `isEditorOpen` |  |  |
| 65 | function |  | `listParagraphBoxes` |  |  |
| 74 | function |  | `findBlankPoint` |  |  |
| 93 | function |  | `verifyBug1BlankClickExitsAndPersists` |  |  |
| 147 | function |  | `verifyBug2CaretLandsAtClickPosition` |  |  |
| 187 | function |  | `diagDom` |  |  |
| 205 | function |  | `ensureCleanEditorMode` |  |  |
| 222 | function |  | `isEditModeOn` |  |  |
| 242 | function |  | `verifyEditorBugs` | yes |  |

### `src/main.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | function |  | `api` |  |  |
| 13 | function |  | `init` |  |  |
| 38 | arrow_fn |  | `handleFileOpen` |  |  |

### `tests/e2e/helpers/app.js`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 9 | function |  | `waitForApp` |  |  |
| 38 | function |  | `loadFixturePdf` |  |  |

### `tests/e2e/specs/editor_bugs.spec.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 49 | function |  | `clickAtViewportPoint` |  |  |
| 65 | function |  | `readActiveTextarea` |  |  |

### `tests/e2e/specs/hello.spec.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | arrow_fn |  | `info` |  |  |
| 26 | arrow_fn |  | `found` |  |  |

### `tests/e2e/specs/page_presentation_runtime.spec.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 51 | function |  | `normalizedFixturePath` |  |  |
| 55 | function |  | `invokeTauriCommand` |  |  |
| 82 | function |  | `installDiagnosticCapture` |  |  |
| 92 | arrow_fn |  | `capture` |  |  |
| 126 | function |  | `readViewerState` |  |  |

### `tools/cdp-tail.mjs`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 19 | arrow_fn |  | `send` |  |  |

### `utils/ai-settings.ts`

| 行 | 类型 | 上下文 | 名称 | 是否导出 | Command / js_name |
|---:|---|---|---|---|---|
| 13 | function |  | `loadAiSettings` | yes |  |
| 22 | function |  | `saveAiSettings` | yes |  |

## 类型和类

### `crates/pdf-viewer-core/src/annotation/annotation_types.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 23 | rust_enum | `AnnotationKind` | yes |
| 53 | rust_struct | `Annotation` | yes |
| 79 | rust_struct | `AnnotationBBox` | yes |
| 90 | rust_enum | `AnnotationError` | yes |
| 107 | rust_struct | `AnnotationResponse` | yes |
| 117 | rust_struct | `CommentBoxRect` | yes |
| 124 | rust_type | `PdfPageAnnotationBox` | yes |
| 128 | rust_struct | `CommentPercentFrame` | yes |
| 137 | rust_struct | `PdfPageCommentItem` | yes |
| 149 | rust_struct | `PdfPageCommentList` | yes |
| 158 | rust_struct | `PdfPageAnnotationTarget` | yes |
| 170 | rust_struct | `PdfPageAnnotationTargetResult` | yes |
| 179 | rust_struct | `PdfCommentTargetOverlayMarker` | yes |
| 190 | rust_struct | `PdfCommentTargetOverlayDisplay` | yes |
| 196 | rust_struct | `PdfCommentReviewPageSummary` | yes |
| 204 | rust_struct | `PdfCommentReviewRequest` | yes |
| 212 | rust_struct | `PdfCommentReviewResult` | yes |
| 222 | rust_struct | `PdfCommentReviewSummaryChip` | yes |
| 229 | rust_struct | `PdfCommentReviewCardAction` | yes |
| 237 | rust_struct | `PdfCommentReviewCard` | yes |
| 250 | rust_struct | `PdfCommentReviewPanel` | yes |
| 259 | rust_struct | `PdfCommentOverlayMarker` | yes |
| 268 | rust_struct | `PdfCommentOverlayDisplay` | yes |
| 274 | rust_struct | `PdfCommentReviewDisplay` | yes |
| 283 | rust_struct | `PdfRegionCommentRequest` | yes |
| 293 | rust_struct | `PdfRegionCommentResult` | yes |
| 301 | rust_struct | `PdfDeleteAnnotationRequest` | yes |
| 308 | rust_struct | `PdfDeleteAnnotationResult` | yes |
| 316 | rust_struct | `PdfUpdateCommentRequest` | yes |
| 324 | rust_struct | `PdfUpdateCommentResult` | yes |

### `crates/pdf-viewer-core/src/document/document_types.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_enum | `DocumentError` | yes |
| 30 | rust_struct | `DocumentResponse` | yes |

### `crates/pdf-viewer-core/src/document/page_region_models.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `ParagraphRegionSnapshotLine` | yes |
| 40 | rust_struct | `ParagraphRegionSnapshot` | yes |
| 52 | rust_struct | `FieldGroupSnapshot` | yes |
| 67 | rust_struct | `BoundingBoxOutput` | yes |
| 76 | rust_struct | `ParagraphProjectionOutput` | yes |
| 86 | rust_struct | `ParagraphLineProjectionOutput` | yes |
| 97 | rust_struct | `FieldGroupProjectionOutput` | yes |
| 107 | rust_struct | `StyleSource` | yes |
| 129 | rust_struct | `StyleRunSnapshot` | yes |
| 146 | rust_struct | `ParagraphLineOutput` | yes |
| 175 | rust_struct | `ParagraphRegionOutput` | yes |
| 200 | rust_struct | `ListItemRegionOutput` | yes |
| 239 | rust_struct | `KeyBox` | yes |
| 248 | rust_struct | `KeyValuePairOutput` | yes |
| 268 | rust_struct | `FieldRowRegionGroupOutput` | yes |
| 295 | rust_struct | `FieldRowRegionOutput` | yes |
| 311 | rust_struct | `LineProjectionOutput` | yes |
| 321 | rust_struct | `LineRegionModelOutput` | yes |
| 335 | rust_struct | `PageRegionContextOutput` | yes |

### `crates/pdf-viewer-core/src/edit/active_target.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | rust_struct | `ActiveEditorTarget` | yes |

### `crates/pdf-viewer-core/src/edit/bridge.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `ParagraphInteractionTarget` | yes |

### `crates/pdf-viewer-core/src/edit/debug_trace.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 11 | rust_struct | `EditorDebugField` | yes |
| 18 | rust_struct | `EditorDebugTraceEvent` | yes |
| 26 | rust_struct | `EditorDebugTraceState` |  |

### `crates/pdf-viewer-core/src/edit/document_edit_ops.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 8 | rust_struct | `EditorTextMutation` | yes |

### `crates/pdf-viewer-core/src/edit/document_plan.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 25 | rust_struct | `ParagraphEditorMarker` | yes |
| 35 | rust_struct | `EditorDocumentPlan` | yes |
| 79 | rust_struct | `EditorDocumentLinePlan` | yes |
| 137 | rust_struct | `SessionSplit` |  |

### `crates/pdf-viewer-core/src/edit/document_runtime.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_struct | `EditorResolvedDocumentState` | yes |

### `crates/pdf-viewer-core/src/edit/draft_layout.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `DraftCaretStop` | yes |
| 22 | rust_struct | `DraftCaretLine` | yes |
| 30 | rust_struct | `EditorDraftRenderPlan` | yes |

### `crates/pdf-viewer-core/src/edit/edit_target.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | rust_struct | `EditorEditTarget` | yes |
| 117 | rust_struct | `VisualSegment` |  |
| 122 | rust_type | `IndexedRunRef` |  |

### `crates/pdf-viewer-core/src/edit/editor_types.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_enum | `SessionState` | yes |
| 33 | rust_enum | `EditorError` | yes |
| 50 | rust_struct | `EditorResponse` | yes |
| 64 | rust_struct | `HitTestResult` | yes |
| 72 | rust_struct | `OpenBlockResult` | yes |
| 80 | rust_struct | `MoveCaretResult` | yes |
| 86 | rust_struct | `CommitResult` | yes |
| 92 | rust_struct | `SnapshotResult` | yes |
| 102 | rust_struct | `TextBlockInfo` | yes |
| 112 | rust_struct | `FormatState` | yes |
| 121 | rust_struct | `SyncInputResult` | yes |
| 128 | rust_struct | `ApplyCommandResult` | yes |
| 136 | rust_struct | `SetEditModeResult` | yes |

### `crates/pdf-viewer-core/src/edit/engine_state.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 14 | rust_struct | `LiveEditorParagraphState` | yes |

### `crates/pdf-viewer-core/src/edit/paragraph_overlay.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_enum | `ParagraphRenderOverlayOwner` | yes |
| 13 | rust_struct | `ParagraphRenderOverlay` | yes |

### `crates/pdf-viewer-core/src/edit/paragraph_scene.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `ParagraphEditorScene` | yes |

### `crates/pdf-viewer-core/src/edit/replacement_region.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | rust_struct | `ParagraphReplacementRegion` | yes |

### `crates/pdf-viewer-core/src/edit/replacement_snapshot.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 11 | rust_struct | `EditReplacementSnapshot` | yes |

### `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 21 | rust_struct | `PageViewPoint` | yes |
| 31 | rust_struct | `EditorLocalPoint` | yes |
| 38 | rust_struct | `HostReferenceRect` | yes |
| 47 | rust_struct | `ClientPoint` | yes |
| 54 | rust_struct | `PageSize` | yes |
| 61 | rust_struct | `PageScale` | yes |
| 68 | rust_struct | `HostPageTransform` | yes |
| 145 | rust_struct | `PdfToPageViewTransform` | yes |
| 175 | rust_struct | `PdfCoordinateSpace` | yes |
| 209 | rust_struct | `EditorViewportTransform` | yes |

### `crates/pdf-viewer-core/src/geometry/dom_projection.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `DomRectLike` | yes |
| 15 | rust_struct | `DomPointLike` | yes |
| 24 | rust_struct | `ScalePair` | yes |

### `crates/pdf-viewer-core/src/geometry/layout_engine.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 26 | rust_struct | `VisualLine` | yes |
| 38 | rust_struct | `ParagraphLayout` | yes |

### `crates/pdf-viewer-core/src/geometry/reflow_engine.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 13 | rust_struct | `ReflowUnit` |  |

### `crates/pdf-viewer-core/src/history/history_types.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_enum | `HistoryError` | yes |
| 29 | rust_struct | `HistoryState` | yes |
| 47 | rust_struct | `HistoryStepResult` | yes |
| 58 | rust_struct | `HistoryResponse` | yes |

### `crates/pdf-viewer-core/src/models/document_runtime.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `PageState` | yes |
| 28 | rust_struct | `BaseEditIntent` | yes |
| 36 | rust_enum | `EditIntent` | yes |
| 49 | rust_enum | `LightPageKind` | yes |
| 59 | rust_struct | `LightPageModel` | yes |
| 69 | rust_enum | `PdfDocumentKind` | yes |
| 79 | rust_enum | `ClassificationReason` | yes |
| 91 | rust_struct | `ReadDocumentMeta` | yes |
| 103 | rust_enum | `PaginationAction` | yes |
| 111 | rust_struct | `PaginationCommand` | yes |
| 120 | rust_struct | `DeletePageCommand` | yes |
| 126 | rust_struct | `RotatePageCommand` | yes |
| 133 | rust_struct | `InsertPageCommand` | yes |
| 139 | rust_struct | `AddHighlightCommand` | yes |
| 147 | rust_struct | `UpdateMetadataCommand` | yes |

### `crates/pdf-viewer-core/src/models/font.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `FontHints` | yes |
| 26 | rust_enum | `FontSourceKind` | yes |
| 36 | rust_enum | `SymbolClass` | yes |
| 45 | rust_struct | `ResolvedFontIdentity` | yes |
| 57 | rust_struct | `ResolvedFontFace` | yes |

### `crates/pdf-viewer-core/src/models/geometry.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | rust_struct | `BoundingBox` | yes |

### `crates/pdf-viewer-core/src/models/glyph.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `GlyphPaintRun` | yes |
| 44 | rust_struct | `EditorControlStyle` | yes |
| 56 | rust_struct | `GlyphPaintParagraph` | yes |
| 71 | rust_enum | `ExternalObject` | yes |
| 92 | rust_struct | `GlyphPaintRegion` | yes |
| 105 | rust_struct | `GlyphPaintPlan` | yes |

### `crates/pdf-viewer-core/src/models/interaction.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `RectBox` | yes |
| 15 | rust_struct | `FieldProjection` | yes |
| 25 | rust_struct | `FieldProjectionRequest` | yes |
| 43 | rust_enum | `FieldPartKind` | yes |
| 51 | rust_struct | `FieldHitRequest` | yes |
| 64 | rust_struct | `FieldHitResolution` | yes |
| 73 | rust_struct | `FieldHitTarget` | yes |
| 85 | rust_struct | `FieldHitBatchRequest` | yes |
| 93 | rust_struct | `FieldHitMatch` | yes |
| 100 | rust_struct | `FieldEditorParamsRequest` | yes |
| 110 | rust_struct | `FieldEditorParams` | yes |
| 119 | rust_struct | `InteractionProjection` | yes |
| 129 | rust_struct | `InteractionTarget` | yes |
| 140 | rust_struct | `FieldEditorProjection` | yes |

### `crates/pdf-viewer-core/src/models/layout.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | rust_enum | `FieldKind` | yes |
| 17 | rust_enum | `SemanticRole` | yes |
| 34 | rust_struct | `EditableFieldGroup` | yes |
| 50 | rust_struct | `EditableSegment` | yes |
| 84 | rust_enum | `LayoutRole` | yes |
| 98 | rust_enum | `LayoutAlignment` | yes |
| 108 | rust_enum | `LayoutMode` | yes |
| 121 | rust_struct | `RunStyle` | yes |
| 137 | rust_struct | `LayoutRun` | yes |
| 190 | rust_struct | `ParagraphStyle` | yes |
| 201 | rust_struct | `LayoutParagraph` | yes |
| 229 | rust_struct | `ParagraphEditContext` | yes |
| 236 | rust_struct | `SemanticRegion` | yes |
| 259 | rust_struct | `LayoutInferenceResult` | yes |
| 278 | rust_enum | `PaintMode` | yes |

### `crates/pdf-viewer-core/src/models/styled_run.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 27 | rust_struct | `StyledRun` | yes |
| 137 | rust_struct | `NativeTextModel` | yes |
| 307 | rust_struct | `NativePathObject` | yes |
| 311 | rust_struct | `NativeImageObject` | yes |
| 315 | rust_enum | `NativePageObject` | yes |
| 323 | rust_struct | `NativePageModel` | yes |

### `crates/pdf-viewer-core/src/models/vector.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_struct | `VectorPathSegment` | yes |
| 15 | rust_struct | `VectorPathObject` | yes |
| 33 | rust_struct | `VectorImageObject` | yes |
| 45 | rust_struct | `VectorTextObject` | yes |
| 55 | rust_enum | `VectorRenderObject` | yes |
| 68 | rust_struct | `VectorPageModel` | yes |

### `crates/pdf-viewer-core/src/persistence/history_store.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 26 | rust_struct | `HistoryStore` | yes |

### `crates/pdf-viewer-core/src/persistence/models.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `PersistableRegionPatch` | yes |
| 51 | rust_struct | `RegionTextReflow` | yes |
| 60 | rust_struct | `PersistableSavePlan` | yes |

### `crates/pdf-viewer-core/src/persistence/patch_store.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 13 | rust_struct | `GlobalPatchState` | yes |
| 47 | rust_struct | `PatchCommand` | yes |
| 55 | rust_struct | `ReviewChangeEntry` | yes |
| 67 | rust_struct | `ReviewBulkChangeResult` | yes |

### `crates/pdf-viewer-core/src/persistence/review_types.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_struct | `ReviewFeedResult` | yes |
| 15 | rust_struct | `RejectReviewChangeResult` | yes |
| 23 | rust_struct | `AcceptReviewChangeResult` | yes |

### `crates/pdf-viewer-core/src/render/comment_review_state.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_enum | `HostCommentReviewScope` | yes |
| 13 | rust_struct | `HostCommentReviewSession` | yes |

### `crates/pdf-viewer-core/src/render/effective_page_plan.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 30 | rust_enum | `EffectiveVectorRenderEntry` | yes |
| 39 | rust_struct | `GlyphParagraphRef` | yes |
| 47 | rust_enum | `EffectiveGlyphRenderEntry` | yes |
| 52 | rust_struct | `PreparedOverlay` |  |

### `crates/pdf-viewer-core/src/render/facade_types.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `ViewportLayoutRequest` | yes |
| 14 | rust_struct | `ViewportTileRequest` | yes |

### `crates/pdf-viewer-core/src/render/find_state.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_enum | `HostFindScope` | yes |
| 13 | rust_struct | `HostFindSession` | yes |
| 23 | rust_struct | `HostFindNavigationResult` | yes |

### `crates/pdf-viewer-core/src/render/layer.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `LayerExecutionPlan` | yes |
| 25 | rust_struct | `LayerPresentDecision` | yes |
| 32 | rust_struct | `RenderLayerRuntimePlan` | yes |
| 43 | rust_struct | `RenderExecutionPlan` | yes |

### `crates/pdf-viewer-core/src/render/plan_builder.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | rust_struct | `RenderZoomRequest` | yes |
| 20 | rust_struct | `RenderZoomResult` | yes |
| 30 | rust_struct | `FramePlanRequest` | yes |
| 48 | rust_struct | `FramePlanResult` | yes |
| 84 | rust_struct | `ViewportLayoutResult` | yes |
| 93 | rust_struct | `ViewportTileResult` | yes |
| 102 | rust_struct | `AnchorViewportLayoutResult` | yes |

### `crates/pdf-viewer-core/src/render/prepared_scene.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 13 | rust_struct | `PreparedPageScene` | yes |

### `crates/pdf-viewer-core/src/render/present_plan.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | rust_struct | `PresentPolicy` | yes |

### `crates/pdf-viewer-core/src/render/preview.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `PreviewPresentPlan` | yes |

### `crates/pdf-viewer-core/src/render/progressive.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 21 | rust_struct | `ProgressiveRenderStart` | yes |
| 27 | rust_struct | `ProgressiveRenderStep` | yes |
| 36 | rust_struct | `ProgressiveRenderPolicy` | yes |
| 43 | rust_struct | `ProgressiveVectorRenderTask` | yes |
| 115 | rust_struct | `ProgressiveRenderPolicyRequest` | yes |

### `crates/pdf-viewer-core/src/render/renderer.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_enum | `DrawCommand` | yes |
| 33 | rust_trait | `PdfRenderer` | yes |

### `crates/pdf-viewer-core/src/render/scheduler.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `HostRenderState` | yes |
| 33 | rust_struct | `RenderFrameEnvelope` | yes |
| 40 | rust_struct | `RenderFrameTransition` | yes |

### `crates/pdf-viewer-core/src/render/snapshot_paint_plan.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 123 | rust_struct | `RunLayout` | yes |

### `crates/pdf-viewer-core/src/render/source_suppression.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 16 | rust_struct | `SuppressedVectorTextRuns` | yes |

### `crates/pdf-viewer-core/src/render/tile_cache.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `BaseLayerCacheEntry` | yes |
| 24 | rust_struct | `DetailTileCacheEntry` | yes |
| 36 | rust_struct | `HostPresentState` | yes |
| 44 | rust_struct | `HostFrameCacheState` | yes |
| 51 | rust_struct | `FrameCacheStoreResult` | yes |

### `crates/pdf-viewer-core/src/render/viewer_session.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `HostViewerSession` | yes |

### `crates/pdf-viewer-core/src/render/viewport_refresh.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_struct | `HostViewportRefreshState` | yes |
| 13 | rust_struct | `ViewportRefreshDecision` | yes |

### `crates/pdf-viewer-core/src/render/workflow.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_type | `RenderFrameEnvelope` | yes |
| 7 | rust_type | `RenderFrameTransition` | yes |
| 11 | rust_struct | `ProgressiveRenderStartResult` | yes |
| 18 | rust_struct | `ProgressiveRenderStepResult` | yes |

### `crates/pdf-viewer-core/src/render/zoom_host.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `WheelRenderDecisionRequest` | yes |
| 15 | rust_struct | `WheelRenderDecision` | yes |
| 24 | rust_struct | `PreviewTickDecisionRequest` | yes |
| 34 | rust_struct | `PreviewTickDecision` | yes |
| 43 | rust_struct | `RenderFollowUpDecision` | yes |

### `crates/pdf-viewer-core/src/render/zoom_interaction.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | rust_struct | `WheelZoomRequest` | yes |
| 35 | rust_struct | `WheelZoomResult` | yes |
| 47 | rust_struct | `AnchorScrollRequest` | yes |
| 60 | rust_struct | `AnchorScrollResult` | yes |
| 67 | rust_struct | `ZoomLimitsRequest` | yes |
| 77 | rust_struct | `ZoomLimitsResult` | yes |
| 83 | rust_struct | `ZoomPreviewFrame` | yes |

### `crates/pdf-viewer-core/src/render/zoom_state.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `ZoomAnchorState` | yes |
| 16 | rust_struct | `VisualLayoutState` | yes |
| 24 | rust_struct | `PreviewTransformState` | yes |
| 32 | rust_struct | `PendingCommittedFrame` | yes |
| 45 | rust_struct | `PreviewHostState` | yes |
| 53 | rust_struct | `HostZoomState` | yes |
| 83 | rust_struct | `ZoomAnimationStep` | yes |

### `crates/pdf-viewer-core/src/text/caret_geometry.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | rust_struct | `EditorCaretVisualPosition` | yes |
| 16 | rust_struct | `CaretStop` | yes |
| 22 | rust_struct | `CaretLine` | yes |

### `crates/pdf-viewer-core/src/text/editable_segments.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `FieldLabelAnchor` |  |
| 13 | rust_struct | `FieldGroup` |  |

### `crates/pdf-viewer-core/src/text/glyph_layout.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 22 | rust_struct | `DecorativePrefixLayout` | yes |
| 37 | rust_struct | `EditorSessionTextPlan` | yes |
| 47 | rust_enum | `EditorGlyphSlotKind` | yes |
| 57 | rust_struct | `EditorGlyphSlot` | yes |

### `crates/pdf-viewer-core/src/text/list_semantics.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_enum | `ListMarkerKind` | yes |
| 17 | rust_struct | `ListTextSemantic` | yes |

### `crates/pdf-viewer-core/src/text/search_replace.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 2 | rust_struct | `SearchReplaceOptions` | yes |

### `crates/pdf-viewer-core/src/text/semantic_axiom.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 3 | rust_struct | `AxiomEngine` | yes |

### `crates/pdf-viewer-core/src/text/style_mapper.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_struct | `StyleSpan` | yes |
| 15 | rust_struct | `StyleMapper` | yes |

### `crates/pdf-viewer-core/src/text/text_model.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `EditorTextModel` | yes |

### `crates/pdf-viewer-core/src/typography/engine.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `TypographyEngine` | yes |

### `crates/pdf-viewer-core/src/typography/models.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_enum | `PdfFontSourceKind` | yes |
| 16 | rust_enum | `RenderFontKind` | yes |
| 24 | rust_enum | `PdfEmbeddedFontKind` | yes |
| 35 | rust_struct | `NormalizedPdfFontIdentity` | yes |
| 46 | rust_struct | `PdfFontDescriptor` | yes |
| 62 | rust_struct | `PdfFontMatchRequest` | yes |
| 70 | rust_struct | `SystemFontCandidate` | yes |
| 85 | rust_struct | `MatchReason` | yes |
| 93 | rust_struct | `SystemFontMatchResult` | yes |
| 101 | rust_struct | `ResolvedPdfFont` | yes |

### `crates/pdf-viewer-ui/src/annotation/annotation_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 65 | rust_struct | `AnnotationManager` | yes |

### `crates/pdf-viewer-ui/src/app_controller.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 57 | rust_struct | `PdfLogger` | yes |

### `crates/pdf-viewer-ui/src/application.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 47 | rust_struct | `ApplicationState` | yes |
| 90 | rust_struct | `Application` | yes |

### `crates/pdf-viewer-ui/src/commands.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_enum | `PdfEditCommand` | yes |

### `crates/pdf-viewer-ui/src/comment/comment_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 31 | rust_struct | `CommentManager` | yes |

### `crates/pdf-viewer-ui/src/document/comment.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 21 | rust_type | `PdfCommentReviewDisplay` | yes |
| 26 | rust_struct | `PathPageArgs` |  |
| 33 | rust_struct | `PathRequestArgs` |  |

### `crates/pdf-viewer-ui/src/document/document_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 27 | rust_struct | `DocumentSession` | yes |

### `crates/pdf-viewer-ui/src/document/host_pipeline.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `OpenDocumentPipelineRequest` | yes |
| 24 | rust_struct | `OpenDocumentPipelineResult` | yes |
| 34 | rust_struct | `CloseDocumentPipelineResult` | yes |
| 42 | rust_struct | `PickDocumentPipelineRequest` | yes |
| 50 | rust_struct | `RotateDocumentPipelineResult` | yes |
| 56 | rust_struct | `DocumentMutationPipelineResult` | yes |

### `crates/pdf-viewer-ui/src/document/io.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | rust_struct | `OpenPdfFileResult` | yes |
| 16 | rust_struct | `RotateCurrentPageResult` | yes |

### `crates/pdf-viewer-ui/src/document/mutation_pipeline.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | rust_struct | `DocumentRefreshPipelineResult` | yes |

### `crates/pdf-viewer-ui/src/editor/activation.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 24 | rust_struct | `OpenEditorAtClientPointRequest` | yes |
| 51 | rust_struct | `MoveCaretToClientPointRequest` | yes |
| 72 | rust_struct | `SaveEditorSessionResult` | yes |

### `crates/pdf-viewer-ui/src/editor/command.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 14 | rust_enum | `EditorInputCommand` | yes |

### `crates/pdf-viewer-ui/src/editor/editor_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 14 | rust_struct | `HitTestRequest` |  |
| 27 | rust_struct | `OpenBlockRequest` |  |
| 45 | rust_struct | `MoveCaretRequest` |  |
| 58 | rust_struct | `CommitRequest` |  |
| 66 | rust_struct | `EditorSession` | yes |
| 447 | rust_struct | `SyncInputRequest` |  |
| 482 | rust_struct | `CommandRequest` |  |
| 619 | rust_struct | `OpenRegionRequest` |  |

### `crates/pdf-viewer-ui/src/editor/editor_controller.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 37 | rust_struct | `EditorVisibilityAction` | yes |

### `crates/pdf-viewer-ui/src/editor/editor_format.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_struct | `ActiveEditorFormatState` | yes |
| 24 | rust_enum | `EditorFormatAction` | yes |

### `crates/pdf-viewer-ui/src/editor/format/list_format.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `EffectiveListState` |  |
| 21 | rust_struct | `ParagraphListContext` |  |

### `crates/pdf-viewer-ui/src/editor/host_mode.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 8 | rust_struct | `ToggleEditorModeResult` | yes |

### `crates/pdf-viewer-ui/src/editor/host_runtime.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_struct | `EditorHostRuntimeState` | yes |

### `crates/pdf-viewer-ui/src/editor/host_snapshot.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 17 | rust_struct | `ActiveEditorRunDiagnostic` | yes |
| 28 | rust_struct | `ActiveEditorDiagnostics` | yes |
| 45 | rust_struct | `ActiveEditorSlotDiagnostic` | yes |
| 55 | rust_struct | `EditorHostSnapshot` | yes |

### `crates/pdf-viewer-ui/src/editor/orchestrator/render_transaction.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 24 | rust_struct | `EditorRenderTransactionResult` | yes |
| 31 | rust_struct | `EditorInputRenderTransactionResult` | yes |

### `crates/pdf-viewer-ui/src/editor/orchestrator/replace_pipeline.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | rust_struct | `RegionTextReplaceRequest` | yes |
| 25 | rust_struct | `RegionTextReplaceResult` | yes |

### `crates/pdf-viewer-ui/src/editor/overlay/projection.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 11 | rust_struct | `ProjectedParagraphInteractionTarget` | yes |
| 31 | rust_struct | `ProjectedEditorShell` | yes |

### `crates/pdf-viewer-ui/src/editor/search_facade.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `SearchPageRequest` | yes |
| 24 | rust_struct | `SearchDocumentRequest` | yes |
| 37 | rust_struct | `FindSessionData` | yes |
| 47 | rust_struct | `ReplaceResult` | yes |
| 54 | rust_struct | `BatchReplaceRequest` | yes |
| 64 | rust_struct | `BatchReplaceResult` | yes |
| 72 | rust_struct | `SearchFacadeResult` | yes |
| 80 | rust_struct | `FindNavigation` | yes |

### `crates/pdf-viewer-ui/src/editor/session/session.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 19 | rust_struct | `EditorModeState` | yes |
| 27 | rust_struct | `ActiveEditorInputSyncResult` | yes |

### `crates/pdf-viewer-ui/src/events.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 56 | rust_struct | `EventBusInner` |  |

### `crates/pdf-viewer-ui/src/find/find_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 19 | rust_struct | `FindSession` | yes |

### `crates/pdf-viewer-ui/src/find/find_store.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | rust_struct | `SearchMatch` | yes |
| 28 | rust_struct | `SearchBox` | yes |
| 37 | rust_struct | `SearchResult` | yes |
| 45 | rust_enum | `FindScope` | yes |
| 73 | rust_enum | `FindSessionState` | yes |
| 118 | rust_struct | `FindControllerState` | yes |
| 131 | rust_struct | `FindStateUpdate` | yes |
| 141 | rust_struct | `CurrentPageMatch` | yes |
| 156 | rust_struct | `ReplaceRequest` | yes |
| 173 | rust_struct | `FindToolbarState` | yes |
| 188 | rust_struct | `FindControllerInner` |  |

### `crates/pdf-viewer-ui/src/geometry_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 32 | rust_struct | `PointResult` | yes |
| 39 | rust_struct | `RectResult` | yes |
| 48 | rust_struct | `TransformContext` | yes |
| 59 | rust_struct | `GeometryApi` | yes |

### `crates/pdf-viewer-ui/src/host/command.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | rust_struct | `OpenDocumentSessionRequest` | yes |
| 20 | rust_struct | `HostActionResult` | yes |

### `crates/pdf-viewer-ui/src/host/layout.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | rust_struct | `HostLayoutOverride` | yes |
| 18 | rust_struct | `SyncHostLayoutRequest` | yes |
| 29 | rust_struct | `SyncHostLayoutResult` | yes |

### `crates/pdf-viewer-ui/src/page/page_store.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 14 | rust_type | `HostPageState` | yes |

### `crates/pdf-viewer-ui/src/presentation/page_turn.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 18 | rust_enum | `PageTurnPhase` | yes |
| 29 | rust_struct | `PageTurnSnapshot` | yes |
| 63 | rust_struct | `PageTurnDecision` | yes |
| 76 | rust_struct | `PageVisibleDecision` | yes |
| 88 | rust_struct | `PageAssetAdmission` | yes |
| 100 | rust_struct | `PagePrefetchTarget` | yes |
| 109 | rust_struct | `PagePrefetchDecision` | yes |

### `crates/pdf-viewer-ui/src/presentation/presentation_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 11 | rust_struct | `PagePresentationRuntime` | yes |

### `crates/pdf-viewer-ui/src/presentation/render_queue.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 8 | rust_struct | `RenderQueueAction` | yes |

### `crates/pdf-viewer-ui/src/render/canvas.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 31 | rust_enum | `CoordinateMode` | yes |
| 36 | rust_struct | `CanvasRenderer` | yes |
| 81 | rust_struct | `TextMetricsSnapshot` | yes |

### `crates/pdf-viewer-ui/src/render/commit.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | rust_struct | `RenderCommitResult` | yes |

### `crates/pdf-viewer-ui/src/render/host_runtime.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `HostRenderLoopState` | yes |

### `crates/pdf-viewer-ui/src/render/wasm_facade.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 27 | rust_struct | `StubResult` |  |

### `crates/pdf-viewer-ui/src/review/review_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 30 | rust_enum | `ReviewSessionState` | yes |
| 60 | rust_struct | `ReviewSession` | yes |

### `crates/pdf-viewer-ui/src/viewer/viewer_api.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 21 | rust_struct | `ViewerSession` | yes |

### `crates/pdf-viewer-ui/src/viewer/viewer_controller.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 21 | rust_struct | `ViewerRuntimeResetOptions` | yes |

### `crates/pdf-viewer-ui/src/viewer/viewer_store.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 20 | rust_enum | `ViewerSessionState` | yes |

### `crates/pdf-viewer-ui/src/zoom/event.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 16 | rust_struct | `WheelZoomHostRequest` | yes |
| 23 | rust_struct | `WheelZoomHostResult` | yes |
| 31 | rust_struct | `PreviewHostStepRequest` | yes |
| 37 | rust_struct | `PreviewHostStepResult` | yes |

### `crates/pdf-viewer-ui/src/zoom/zoom_store.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 22 | rust_enum | `ZoomSessionState` | yes |

### `src-tauri/src/app_state.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 29 | rust_struct | `DocumentStore` | yes |
| 46 | rust_struct | `CacheStore` | yes |
| 72 | rust_struct | `HistoryStore` | yes |
| 87 | rust_struct | `RendererState` | yes |
| 100 | rust_struct | `AppState` | yes |

### `src-tauri/src/application/pdf/page_annotation.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 20 | rust_struct | `PdfPageHighlightItem` | yes |
| 31 | rust_struct | `PdfPageHighlightList` | yes |
| 40 | rust_struct | `PdfRegionHighlightRequest` | yes |
| 50 | rust_struct | `PdfRegionHighlightResult` | yes |

### `src-tauri/src/application/pdf/page_asset.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_enum | `PageAssetRole` | yes |
| 239 | rust_enum | `PageAssetKind` | yes |
| 257 | rust_struct | `PageAssetAdmissionService` | yes |

### `src-tauri/src/application/pdf/page_search.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 8 | rust_struct | `PdfPageSearchRequest` | yes |
| 16 | rust_struct | `PdfPageSearchBox` | yes |
| 25 | rust_struct | `PdfPageSearchMatch` | yes |
| 41 | rust_struct | `PdfPageSearchResult` | yes |
| 52 | rust_struct | `PdfDocumentSearchResult` | yes |

### `src-tauri/src/error.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 50 | rust_enum | `PdfError` | yes |
| 104 | rust_type | `PdfResult` | yes |

### `src-tauri/src/infrastructure/pdf_read/backend.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 2 | rust_trait | `PdfReadBackend` | yes |

### `src-tauri/src/infrastructure/pdf_read/facade.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `PdfReadFacade` | yes |

### `src-tauri/src/infrastructure/pdf_read/scanned_backend.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 20 | rust_struct | `ScannedReadBackend` | yes |
| 26 | rust_struct | `ClassificationDecision` |  |

### `src-tauri/src/infrastructure/pdf_read/types.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `PagePreview` | yes |

### `src-tauri/src/infrastructure/pdf_read/vector_backend.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `VectorReadBackend` | yes |

### `src-tauri/src/infrastructure/pdf/annotation_store.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 4 | rust_struct | `StoredPdfHighlight` | yes |
| 11 | rust_struct | `StoredPdfComment` | yes |

### `src-tauri/src/infrastructure/pdf/commands.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | rust_trait | `PdfEditCommand` | yes |
| 17 | rust_struct | `ReplaceTextCommand` | yes |
| 35 | rust_struct | `PersistableRegionPatchCommand` | yes |
| 66 | rust_struct | `TextReflowCommand` | yes |
| 95 | rust_struct | `BatchTextReflowCommand` | yes |
| 127 | rust_struct | `ReplaceImageCommand` | yes |
| 167 | rust_struct | `AddCommentCommand` | yes |
| 181 | rust_struct | `UpdateCommentCommand` | yes |
| 194 | rust_struct | `DeleteAnnotationCommand` | yes |

### `src-tauri/src/infrastructure/pdf/document_service.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 41 | rust_struct | `PdfDocumentService` | yes |

### `src-tauri/src/infrastructure/pdf/font/matching.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | rust_struct | `PdfSystemFontMatcher` | yes |

### `src-tauri/src/infrastructure/pdf/font/ttc.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 105 | rust_struct | `SfntTableRecord` |  |

### `src-tauri/src/infrastructure/pdf/geometry_service.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | rust_struct | `PdfEditorGeometryService` | yes |

### `src-tauri/src/infrastructure/pdf/layout_analyzer.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | rust_struct | `LayoutGraphAnalyzer` | yes |

### `src-tauri/src/infrastructure/pdf/layout_engine.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 35 | rust_struct | `LayoutGraphAnalyzer` | yes |

### `src-tauri/src/infrastructure/pdf/log_service.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 158 | rust_struct | `PdfEventSpan` | yes |
| 226 | rust_struct | `ProfileSpan` | yes |

### `src-tauri/src/infrastructure/pdf/models.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_struct | `EmbeddedGlyphMap` | yes |
| 40 | rust_struct | `PageModel` | yes |
| 51 | rust_struct | `PageTextInfo` | yes |
| 66 | rust_struct | `TextObjectInfo` | yes |
| 74 | rust_struct | `TextReflowPatch` | yes |
| 100 | rust_struct | `PdfMaterializationDecisionReport` | yes |
| 109 | rust_struct | `PdfMaterializationSourceStats` | yes |
| 117 | rust_struct | `PdfMaterializationReport` | yes |
| 130 | rust_struct | `PdfModifications` | yes |
| 142 | rust_struct | `PathSegment` | yes |
| 149 | rust_struct | `NativePathModel` | yes |
| 211 | rust_struct | `NativeImageModel` | yes |
| 231 | rust_struct | `VectorPalette` | yes |
| 238 | rust_struct | `TextPatch` | yes |
| 248 | rust_enum | `RenderObject` | yes |
| 255 | rust_struct | `PageDisplayList` | yes |
| 265 | rust_struct | `NativeVectorPageModel` | yes |
| 292 | rust_enum | `LightPageKind` | yes |
| 301 | rust_struct | `LightPageModel` | yes |
| 311 | rust_struct | `PdfMetadata` | yes |

### `src-tauri/src/infrastructure/pdf/page_intermediate_service.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 8 | rust_struct | `PageIntermediateBundle` | yes |
| 13 | rust_struct | `PdfPageIntermediateService` | yes |

### `src-tauri/src/infrastructure/pdf/page_model_service.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | rust_struct | `PdfPageModelService` | yes |

### `src-tauri/src/infrastructure/pdf/pdf_font.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 8 | rust_struct | `CMap` | yes |
| 33 | rust_struct | `ParsedFont` | yes |
| 386 | rust_struct | `ParsedImage` | yes |
| 392 | rust_struct | `ResourceCache` | yes |

### `src-tauri/src/infrastructure/pdf/pdf_read_service.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 16 | rust_struct | `PdfReadService` | yes |

### `src-tauri/src/infrastructure/pdf/pdf_read.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | rust_struct | `GraphicsState` | yes |
| 70 | rust_type | `FlatResources` | yes |

### `src-tauri/src/infrastructure/pdf/pdf_write_font_resolver.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | rust_struct | `PdfTextWriteFont` | yes |
| 20 | rust_enum | `PdfTextWriteEncoding` |  |
| 41 | rust_struct | `ResolvedFontProgram` |  |

### `src-tauri/src/infrastructure/pdf/pdf_write_service.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 8 | rust_struct | `PdfWriteService` | yes |

### `src-tauri/src/infrastructure/pdf/pdf_write.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | rust_trait | `PdfDocExt` | yes |
| 762 | rust_struct | `PdfTextState` |  |
| 778 | rust_struct | `ReflowCluster` |  |

### `src-tauri/src/infrastructure/pdf/region_materializer.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | rust_struct | `RegionMaterializationDecision` | yes |
| 17 | rust_struct | `RegionMaterializationPlan` | yes |
| 106 | rust_struct | `SnapshotLineReflow` |  |

### `src-tauri/src/infrastructure/pdf/save_text_write_plan.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 2 | rust_struct | `PersistedTextLinePlan` | yes |

### `src-tauri/src/infrastructure/pdf/spatial_graph.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 14 | rust_struct | `SpatialGraph` | yes |

### `src-tauri/src/infrastructure/pdf/vello_renderer.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 17 | rust_struct | `VelloRenderer` | yes |

### `src-tauri/src/interfaces/pdf/render.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | rust_struct | `PageAssetBundle` | yes |

### `src-tauri/src/state.rs`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 4 | rust_enum | `LoadingStatus` | yes |

### `src/bridge/ai/resume_ai_apply.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 21 | ts_type | `ApplyContext` | yes |

### `src/bridge/ai/resume_ai_client.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 24 | ts_interface | `PlanResumeAiRequest` | yes |
| 33 | ts_interface | `ApplyResumeAiRequest` | yes |
| 39 | ts_interface | `SyncResumeAiSessionRequest` | yes |
| 45 | ts_interface | `SubmitResumeAiPromptRequest` | yes |
| 53 | ts_interface | `ApplyResumeAiSuggestionRequest` | yes |
| 60 | ts_interface | `SessionState` |  |
| 102 | ts_interface | `GeminiPart` |  |
| 103 | ts_interface | `GeminiContent` |  |
| 104 | ts_interface | `GeminiResponse` |  |

### `src/bridge/ai/resume_ai_controller.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 25 | ts_type | `ApplyContext` |  |
| 28 | ts_type | `StatusTone` |  |
| 30 | ts_type | `ResumeAiControllerDeps` |  |
| 38 | ts_type | `ResumeAiController` | yes |
| 81 | ts_class | `PdfResumeAiController` |  |

### `src/bridge/ai/resume_ai_diff_preview.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 1 | ts_type | `DiffToken` |  |

### `src/bridge/ai/resume_ai_panel_state_view.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 3 | ts_type | `StatusTone` |  |
| 5 | ts_type | `BusyStateArgs` |  |

### `src/bridge/ai/resume_ai_panel_view.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 4 | ts_type | `ApplySuggestionSource` |  |
| 6 | ts_type | `RenderResumeAiConversationArgs` |  |
| 15 | ts_type | `SyncResumeAiSummaryArgs` |  |

### `src/bridge/ai/resume_ai_types.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 1 | ts_type | `ResumeRegionKind` | yes |
| 3 | ts_type | `ResumeChatRole` | yes |
| 5 | ts_type | `ResumeAiScope` | yes |
| 7 | ts_interface | `ResumeChatTurn` | yes |
| 12 | ts_interface | `PdfPersistableRegionPatch` | yes |
| 28 | ts_interface | `ResumeAiEditDraft` | yes |
| 36 | ts_interface | `ResumeAiPlan` | yes |
| 41 | ts_interface | `ResumeAiPlanResult` | yes |
| 47 | ts_interface | `ResumeAiThreadView` | yes |
| 58 | ts_interface | `ResumeAiSuggestion` | yes |
| 73 | ts_interface | `RawParagraphRegionLine` | yes |
| 80 | ts_interface | `RawParagraphRegion` | yes |
| 89 | ts_interface | `RawListItemRegion` | yes |
| 99 | ts_interface | `RawPageRegionContext` | yes |
| 105 | ts_interface | `ResumeEditableRegion` | yes |
| 119 | ts_interface | `ResumePageContext` | yes |
| 126 | ts_interface | `ResumeDocumentContext` | yes |

### `src/bridge/annotation/pdf_annotation_controller.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | ts_type | `ViewerSessionSnapshot` |  |
| 10 | ts_type | `PdfPageAnnotationTarget` |  |
| 25 | ts_type | `PdfPageAnnotationTargetResult` |  |
| 29 | ts_type | `PdfPageHighlightItem` |  |
| 43 | ts_type | `PdfPageHighlightList` |  |
| 47 | ts_type | `CreatePdfAnnotationControllerDeps` |  |
| 52 | ts_type | `PdfAnnotationController` | yes |

### `src/bridge/comment/pdf_comment_contracts.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 1 | ts_type | `ViewerSessionSnapshot` | yes |
| 6 | ts_type | `PdfPageAnnotationTarget` | yes |
| 13 | ts_type | `PdfCommentTargetOverlayMarker` | yes |
| 27 | ts_type | `PdfCommentTargetOverlayDisplay` | yes |
| 31 | ts_type | `PdfPageCommentItem` | yes |
| 46 | ts_type | `PdfCommentOverlayMarker` | yes |
| 58 | ts_type | `PdfCommentOverlayDisplay` | yes |
| 62 | ts_type | `PdfCommentReviewPageSummary` | yes |
| 68 | ts_type | `PdfCommentReviewResult` | yes |
| 76 | ts_type | `PdfCommentReviewSummaryChip` | yes |
| 81 | ts_type | `PdfCommentReviewCardAction` | yes |
| 87 | ts_type | `PdfCommentReviewCard` | yes |
| 98 | ts_type | `PdfCommentReviewPanel` | yes |
| 105 | ts_type | `CommentReviewScope` | yes |
| 107 | ts_type | `CommentReviewSession` | yes |
| 114 | ts_type | `PdfCommentReviewDisplay` | yes |

### `src/bridge/comment/pdf_comment_controller.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 20 | ts_type | `CreatePdfCommentControllerDeps` |  |
| 27 | ts_type | `PdfCommentController` | yes |

### `src/bridge/comment/pdf_comment_dom.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 4 | ts_type | `PdfCommentDomNodes` | yes |

### `src/bridge/comment/pdf_comment_host_actions.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | ts_type | `CreatePdfCommentHostActionsDeps` |  |
| 23 | ts_type | `PdfCommentHostActions` | yes |

### `src/bridge/comment/pdf_comment_review_view.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | ts_type | `ReviewViewNodes` |  |
| 13 | ts_type | `ReviewViewHandlers` |  |

### `src/bridge/comment/pdf_comment_wasm_bridge.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 16 | ts_type | `CreatePdfCommentWasmBridgeDeps` |  |
| 46 | ts_type | `PdfCommentWasmBridge` | yes |

### `src/bridge/document/document_edit_api.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | ts_type | `RegionTextReplaceRequest` |  |
| 6 | ts_type | `RegionTextReplaceResult` |  |
| 7 | ts_type | `AcceptReviewChangeResult` |  |
| 8 | ts_type | `RejectReviewChangeResult` |  |
| 9 | ts_type | `ReviewBulkChangeResult` |  |
| 10 | ts_type | `ReviewFeedResult` |  |
| 14 | ts_type | `PdfEditSource` | yes |
| 28 | ts_type | `PdfSaveResult` | yes |
| 34 | ts_type | `PdfRegionTextEdit` | yes |
| 43 | ts_type | `PdfRegionTextReplace` | yes |
| 45 | ts_type | `DocumentEditApiDeps` |  |
| 56 | ts_type | `DocumentEditApi` | yes |

### `src/bridge/document/pdf_document_runtime.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 23 | ts_type | `CreatePdfDocumentRuntimeDeps` |  |
| 44 | ts_type | `PdfDocumentRuntime` | yes |

### `src/bridge/editor/editor_host_view.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 32 | ts_type | `ParagraphInteractionTarget` | yes |
| 49 | ts_type | `ActiveEditorTarget` | yes |
| 67 | ts_type | `HostReferenceBox` | yes |
| 74 | ts_type | `EditorHostNodes` | yes |
| 83 | ts_type | `BeforeInputCommand` |  |
| 85 | ts_type | `EditorHostViewDeps` |  |

### `src/bridge/editor/editor_wasm_api.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 11 | ts_type | `GetWasmApi` |  |
| 40 | ts_type | `RegionTextReplaceRequest` | yes |
| 50 | ts_type | `RegionTextReplaceResult` | yes |
| 56 | ts_type | `DocumentRefreshResult` | yes |
| 61 | ts_type | `ReviewChangeEntry` | yes |
| 71 | ts_type | `ReviewFeedResult` | yes |
| 77 | ts_type | `AcceptReviewChangeResult` | yes |
| 83 | ts_type | `RejectReviewChangeResult` | yes |
| 85 | ts_type | `ReviewBulkChangeResult` | yes |
| 93 | ts_type | `EditorWasmApi` | yes |

### `src/bridge/editor/index.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 20 | ts_type | `ActiveEditorTarget` |  |
| 21 | ts_type | `EditorHostNodes` |  |
| 22 | ts_type | `HostReferenceBox` |  |
| 23 | ts_type | `ParagraphInteractionTarget` |  |
| 31 | ts_type | `EditorHostDeps` |  |
| 48 | ts_type | `EditorHost` |  |

### `src/bridge/editor/types.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 3 | ts_type | `SessionState` | yes |
| 5 | ts_type | `EditorError` | yes |
| 15 | ts_type | `EditorResponse` | yes |
| 24 | ts_type | `HitTestResult` | yes |
| 30 | ts_type | `OpenBlockResult` | yes |
| 36 | ts_type | `MoveCaretResult` | yes |
| 40 | ts_type | `CommitResult` | yes |
| 44 | ts_type | `SnapshotResult` | yes |
| 52 | ts_type | `TextBlockInfo` | yes |
| 62 | ts_type | `HitTestRequest` | yes |
| 73 | ts_type | `OpenBlockRequest` | yes |
| 87 | ts_type | `MoveCaretRequest` | yes |
| 98 | ts_type | `CommitRequest` | yes |
| 103 | ts_type | `SyncInputRequest` | yes |
| 108 | ts_type | `ApplyCommandRequest` | yes |
| 115 | ts_type | `SyncInputResult` | yes |
| 120 | ts_type | `ApplyCommandResult` | yes |
| 126 | ts_type | `SetEditModeResult` | yes |
| 134 | ts_type | `LegacyActiveTarget` | yes |
| 152 | ts_type | `LegacyInteractionTarget` | yes |
| 169 | ts_type | `LegacySnapshot` | yes |
| 178 | ts_type | `EditorFormatAction` | yes |

### `src/bridge/find/find_facade.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | ts_type | `SearchMatch` | yes |
| 24 | ts_type | `SearchResult` | yes |
| 30 | ts_type | `SearchPageRequest` | yes |
| 37 | ts_type | `SearchDocumentRequest` | yes |
| 44 | ts_type | `ReplaceRequest` | yes |
| 55 | ts_type | `ReplaceResult` | yes |
| 60 | ts_type | `BatchReplaceRequest` | yes |
| 68 | ts_type | `BatchReplaceResult` | yes |

### `src/bridge/find/pdf_find_controller.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 13 | ts_type | `SearchResult` |  |
| 14 | ts_type | `SearchMatch` |  |
| 18 | ts_type | `ViewerSessionSnapshot` |  |
| 24 | ts_type | `FindScope` |  |
| 26 | ts_type | `CreatePdfFindControllerDeps` |  |
| 40 | ts_type | `PdfFindController` | yes |
| 61 | ts_type | `FindStateUpdate` |  |
| 75 | ts_type | `CurrentPageMatch` |  |
| 88 | ts_type | `FindToolbarState` |  |
| 120 | ts_type | `FindNodes` |  |

### `src/bridge/presentation/page_presenter.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | ts_type | `RasterSurfaceRole` | yes |
| 8 | ts_type | `RasterSurfaceOptions` | yes |
| 14 | ts_type | `PreparedRasterSurface` |  |
| 24 | ts_type | `PagePresenterDeps` |  |

### `src/bridge/render/frame_plan.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 4 | ts_type | `RustFramePlan` | yes |
| 37 | ts_type | `RustPreviewFrame` | yes |
| 50 | ts_type | `RustRenderFrame` | yes |
| 55 | ts_type | `RustRenderTransition` | yes |
| 60 | ts_type | `RustRenderCommitResult` | yes |
| 67 | ts_type | `RustViewportRefreshDecision` | yes |
| 72 | ts_type | `RustWheelRenderDecision` | yes |
| 78 | ts_type | `RustPreviewTickDecision` | yes |
| 85 | ts_type | `RustWheelZoomHostResult` | yes |
| 93 | ts_type | `RustPreviewHostStepResult` | yes |
| 98 | ts_type | `RustLayerExecutionPlan` | yes |
| 106 | ts_type | `RustLayerPresentDecision` | yes |
| 111 | ts_type | `RustCommittedFrame` |  |
| 122 | ts_type | `FramePlanAdapterDeps` |  |
| 131 | ts_type | `RenderReason` | yes |
| 133 | ts_type | `FramePlanAdapter` | yes |

### `src/bridge/render/layout_trace.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 3 | ts_type | `ElementSnapshot` |  |
| 24 | ts_type | `LayoutKeySnapshot` |  |

### `src/bridge/render/raster_image_cache.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | ts_type | `RasterWarmOptions` |  |

### `src/bridge/render/render_flow.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | ts_type | `RenderFlowDeps` |  |
| 30 | ts_type | `VisibleSurface` | yes |

### `src/bridge/render/render_scheduler.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 5 | ts_type | `RenderSource` | yes |
| 7 | ts_type | `RenderRequestContext` | yes |
| 12 | ts_type | `RenderRequest` | yes |
| 18 | ts_type | `RenderSchedulerDeps` | yes |
| 23 | ts_type | `RenderScheduler` | yes |
| 29 | ts_type | `QueuedRenderRequest` |  |

### `src/bridge/render/render_wasm_api.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 15 | ts_type | `GetWasmApi` |  |
| 17 | ts_type | `ProgressiveRenderStart` |  |
| 22 | ts_type | `ProgressiveRenderPolicy` |  |
| 28 | ts_type | `RenderLayerRuntimePlan` | yes |
| 37 | ts_type | `RenderExecutionPlan` | yes |
| 43 | ts_type | `ProgressiveRenderStep` |  |
| 48 | ts_type | `FrameCacheStoreResult` |  |
| 52 | ts_type | `RenderWasmApi` | yes |

### `src/bridge/render/vector_canvas_host.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | ts_type | `VectorHostRefs` | yes |
| 20 | ts_type | `PresentViewportCanvasOptions` |  |
| 25 | ts_type | `ViewportCanvasFrame` |  |

### `src/bridge/render/vector_canvas_pool.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | ts_class | `CanvasPool` | yes |

### `src/bridge/render/vector_host.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 12 | ts_type | `VectorHostRefs` |  |
| 58 | ts_type | `VectorRenderResult` | yes |
| 65 | ts_type | `VectorCommitOptions` | yes |
| 69 | ts_type | `VectorLayerPresent` | yes |
| 80 | ts_type | `RenderZoomPlan` | yes |

### `src/bridge/render/vector_page_bundle.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | ts_type | `VectorPageBundle` | yes |
| 18 | ts_type | `VectorPageBundleResolution` |  |

### `src/bridge/render/vector_worker.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 4 | ts_type | `VectorWorkerRequest` | yes |
| 25 | ts_type | `VectorWorkerResponse` | yes |

### `src/bridge/review/pdf_review_controller.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | ts_type | `ReviewChangeEntry` |  |
| 10 | ts_type | `ReviewFeedResult` |  |
| 11 | ts_type | `ReviewLocateResult` |  |
| 15 | ts_type | `ViewerSessionSnapshot` |  |
| 20 | ts_type | `CreatePdfReviewControllerDeps` |  |
| 32 | ts_type | `ReviewScope` |  |
| 34 | ts_type | `ReviewNodes` |  |
| 48 | ts_type | `ReviewUiState` |  |
| 55 | ts_type | `PdfReviewController` | yes |

### `src/bridge/review/review_wasm_facade.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 21 | ts_type | `ReviewChangeEntry` | yes |
| 31 | ts_type | `ReviewFeedResult` | yes |
| 37 | ts_type | `ReviewLocateResult` | yes |
| 44 | ts_type | `ReviewFacadeResult` | yes |

### `src/bridge/shared/diagnostics.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 3 | ts_type | `DiagnosticFields` |  |
| 5 | ts_type | `DiagnosticOptions` |  |
| 12 | ts_type | `DiagnosticLevel` |  |

### `src/bridge/viewer/page_presentation_runtime.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 1 | ts_type | `PageTurnDecision` | yes |
| 15 | ts_type | `PageVisibleDecision` | yes |
| 25 | ts_type | `PageAssetAdmission` | yes |
| 35 | ts_type | `PagePrefetchTarget` | yes |
| 42 | ts_type | `PagePrefetchDecision` | yes |
| 51 | ts_type | `RenderQueueAction` | yes |
| 65 | ts_type | `PagePresentationRuntimeAdapter` | yes |
| 82 | ts_type | `PagePresentationRuntimeDeps` |  |

### `src/bridge/viewer/pdf_keyboard.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 3 | ts_type | `PdfKeyboardShortcutDeps` | yes |

### `src/bridge/viewer/pdf_layout_sync.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | ts_type | `LayoutOverride` |  |
| 18 | ts_type | `LayoutSyncDeps` |  |

### `src/bridge/viewer/pdf_runtime.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 43 | ts_type | `ZoomStateSnapshot` |  |
| 50 | ts_type | `PdfViewerRuntime` | yes |

### `src/bridge/viewer/pdf_viewer_api.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 10 | ts_type | `PdfViewerApiDeps` | yes |
| 60 | ts_class | `PdfViewerAPI` | yes |
| 328 | ts_type | `PageTurnBenchOptions` |  |

### `src/bridge/viewer/pdf_viewer_dom.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 9 | ts_type | `PdfZoomSnapshot` | yes |

### `src/bridge/viewer/viewer_geometry_probe.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 6 | ts_type | `GeometryProbeDeps` |  |
| 35 | ts_type | `GeometryProbeSnapshot` |  |
| 45 | ts_type | `GeometryProbeApi` |  |

### `src/bridge/viewer/viewer_session.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 14 | ts_type | `ViewerSessionSnapshot` | yes |
| 24 | ts_type | `ViewerSessionAdapter` | yes |
| 33 | ts_type | `ViewerSessionDeps` |  |

### `src/bridge/zoom/zoom_controller.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 3 | ts_type | `AnchorViewportLayout` |  |
| 12 | ts_type | `RustAnchorFramePlan` |  |
| 20 | ts_type | `RustWheelRenderDecision` |  |
| 26 | ts_type | `RustPreviewTickDecision` |  |
| 33 | ts_type | `RustWheelZoomHostResult` |  |
| 37 | ts_type | `RustPreviewHostStepResult` |  |
| 53 | ts_type | `ZoomControllerDeps` | yes |
| 79 | ts_type | `ZoomController` | yes |

### `src/dev/verify_editor_bugs.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 59 | ts_interface | `ParagraphBox` |  |
| 87 | ts_interface | `VerifyResult` |  |

### `tests/e2e/specs/page_presentation_runtime.spec.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 21 | ts_type | `DiagnosticWindow` |  |
| 32 | ts_type | `PageSearchResult` |  |
| 42 | ts_type | `AnnotationTargetResult` |  |

### `tests/e2e/wdio.conf.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 44 | ts_interface | `CustomCaps` |  |
| 50 | ts_type | `CapItem` |  |
| 51 | ts_type | `CustomConfig` |  |

### `utils/ai-settings.ts`

| 行 | 类型 | 名称 | 是否导出 |
|---:|---|---|---|
| 7 | ts_interface | `AiSettings` | yes |
