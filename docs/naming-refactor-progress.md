# 命名重构进度记录

> 保存日期：2026-06-19
> 目的：记录已完成的重构变更，防止上下文丢失导致遗漏

---

## 一、已完成的重构

### 1. edit 模块 (pdf-viewer-core/src/edit)

| 原名 | 新名 | 文件 | 状态 |
|------|------|------|------|
| `make_edit_segment_target_id` | `build_segment_id` | edit_target.rs | ✅ 已改定义+调用 |
| `edit_target_base_paragraph_id` | `get_base_paragraph_id` | edit_target.rs | ✅ 已改定义+调用 |
| `edit_target_segment_key` | `get_segment_key` | edit_target.rs | ✅ 已改定义+调用 |
| `whole_session_target` | `create_from_session` | edit_target.rs | ✅ 已改定义+调用 |
| `replacement_target_from_patch_snapshot` | `find_target` | replacement_snapshot.rs | ✅ 已改定义+调用 |
| `replacement_object_indices` | `collect_object_indices` | replacement_snapshot.rs | ✅ 已改定义+调用 |
| `paragraph_replacement_region` | `build_region` | replacement_region.rs | ✅ 已改定义+调用 |
| `preferred_source_bbox` | `resolve_preferred_bbox` | replacement_region.rs | ✅ 已改定义+调用 |
| `build_source_to_runs_index_map` + `build_runs_to_source_index_map` | `build_index_map()` 返回 `(Vec<usize>, Vec<usize>)` | draft_text_diff.rs | ✅ 已合并定义+调用 |

**调用处更新：**
- bridge.rs: ✅ `get_base_paragraph_id`
- draft_style.rs: ✅ `build_index_map(source_text, &runs_text).0`
- draft_layout.rs: ✅ `build_index_map(source, runs)` 测试中取 `.1`
- draft_text_diff.rs: ✅ 内部测试中取 `(_, inverse) = build_index_map(...)`
- paragraph_overlay.rs (ui): ✅ `get_base_paragraph_id`, `find_target`
- list_format.rs (ui): ✅ `get_base_paragraph_id`
- canvas.rs (ui): ✅ `build_region`
- canvas_overlay.rs (ui): ✅ `build_region`
- glyph_plan.rs (core): ✅ `build_region`
- source_suppression.rs (core): ✅ `build_region`
- overlay_ops.rs (core): ✅ `build_region`
- path_suppression.rs (core): ✅ `build_region`

### 2. text 模块 (pdf-viewer-core/src/text)

| 原名 | 新名 | 文件 | 状态 |
|------|------|------|------|
| `utf16_offset_to_char_index` + `char_index_to_utf16_offset` | `CharIndex`/`Utf16Offset` newtype + `to_utf16()`/`to_char()` | index_convert.rs | ✅ 已改定义+调用 |
| `map_raw_to_reconstructed` | `to_reconstructed` | glyph_layout.rs | ✅ 已改定义 |
| `map_reconstructed_to_raw` | `to_raw` | glyph_layout.rs | ✅ 已改定义+调用(document_plan) |
| `reconstructed_char_count` | `char_count` | glyph_layout.rs | ✅ 已改定义 |

**调用处更新：**
- editor_api.rs (ui): ✅ `Utf16Offset(...).to_char(text).0`, `CharIndex(...).to_utf16(text).0`
- text_index.rs (ui): ✅ re-export `CharIndex`, `Utf16Offset`
- document_plan.rs (core): ✅ `full_text_plan.to_raw(body_char_start)`
- `reconstructed_char_count` 字段名在 document_plan.rs 的 struct 中保留（它是 struct field，不是函数）

### 3. geometry 模块 (pdf-viewer-core/src/geometry)

| 原名 | 新名 | 文件 | 状态 |
|------|------|------|------|
| `normalize_y` | `to_y_down` | coordinate_transform.rs | ✅ 已改定义+调用 |
| `denormalize_y` | `to_y_up` | coordinate_transform.rs | ✅ 已改定义+调用 |
| `client_to_page` + `client_to_page_in_box` | `to_page(clamp_box: Option<BoundingBox>)` | coordinate_transform.rs | ✅ 已改定义+调用 |
| `client_to_local_in_box` | `to_local` | coordinate_transform.rs | ✅ 已改定义+调用 |
| `point_from_pdf` | `project_point` | coordinate_transform.rs | ✅ 已改定义 |
| `x_from_pdf` | `project_x` | coordinate_transform.rs | ✅ 已改定义 |
| `baseline_y_from_pdf` | `project_baseline_y` | coordinate_transform.rs | ✅ 已改定义 |
| `baseline_y_from_anchor_relative` | `project_relative_y` | coordinate_transform.rs | ✅ 已改定义 |
| `source_session_visual_bbox` | `compute_session_bbox` | source_geometry.rs | ✅ 已改定义+调用 |
| `source_visual_bbox_from_runs` | `compute_bbox_from_runs` | source_geometry.rs | ✅ 已改定义+调用 |
| `caret_line_bbox` | `compute_caret_line_bbox` | source_geometry.rs | ✅ 已改定义+调用 |
| `source_run_visual_bbox` | `compute_run_bbox` | source_geometry.rs | ✅ 已改定义+调用 |

**调用处更新：**
- geometry_api.rs (ui): ✅ `to_y_up`, `to_y_down`, `to_page`
- activation.rs (ui): ✅ `to_page`
- editor_api.rs (ui): ✅ `to_page`
- projection_workflow.rs (ui): ✅ `to_page`
- merged_impl.rs (src-tauri): ✅ `to_y_down`
- visual.rs (ui): ✅ `compute_caret_line_bbox`
- edit_target.rs: ✅ `compute_run_bbox`, `compute_bbox_from_runs`
- source_runs.rs: ✅ `compute_run_bbox`, `compute_bbox_from_runs`
- replacement_region.rs: ✅ `compute_session_bbox`
- document_plan.rs: ✅ `compute_bbox_from_runs`

### 4. render 模块 (pdf-viewer-core/src/render)

| 原名 | 新名 | 文件 | 状态 |
|------|------|------|------|
| `styled_run_bbox` | `run_bbox` | viewport_culling.rs | ✅ 已改定义+调用 |
| `path_object_bbox` | `path_bbox` | viewport_culling.rs | ✅ 已改定义+调用 |

**调用处更新：**
- overlay_ops.rs: ✅
- prepared_scene.rs: ✅
- path_suppression.rs: ✅
- source_suppression.rs: ✅
- text_suppression.rs: ✅
- canvas.rs (ui): ✅

### 5. src-tauri 模块

| 原名 | 新名 | 文件 | 状态 |
|------|------|------|------|
| `set_pdf_log_level` | `set_log_level` | log_service.rs | ✅ 已改定义+调用 |
| `get_pdf_log_level` | `get_log_level` | log_service.rs | ✅ 已改定义 |
| `clear_pdf_event_log` | `clear_event_log` | log_service.rs | ✅ 已改定义+调用 |
| `read_pdf_event_log` | `read_event_log` | log_service.rs | ✅ 已改定义+调用 |
| `log_pdf_event` | `log_event` | log_service.rs | ✅ 已改定义 |
| `release_pdf_resources` | `release_resources` | document_service.rs | ✅ 已改定义+调用 |
| `release_all_pdf_resources` | `release_all_resources` | document_service.rs | ✅ 已改定义+调用 |
| `read_last_pdf_materialization_report` | `read_materialization_report` | document_service.rs | ✅ 已改定义 |
| `load_pdf_public` | `load_public` | document_service.rs | ✅ 已改定义+调用 |
| `generate_demo_pdf` | `generate_demo` | document_service.rs + pdf_write_service.rs | ✅ 已改定义+调用 |
| `parse_pdf_hex_color` | `parse_hex_color` | color_utils.rs | ✅ 已改定义 |
| `get_character_width_pdf_units` | `get_character_width` | font/metrics.rs | ✅ 已改定义 |
| `apply_pdf_commands` | `apply_commands` | save_engine.rs | ✅ 已改定义+调用 |

**interfaces 层更新：**
- system.rs: ✅ `create_demo`, `set_log_level`, `clear_event_log`, `read_event_log`
- document.rs: ✅ `release_all_resources`
- ipc_converters.rs: ✅ `load_public`, `apply_commands`
- lib.rs: ✅ `create_demo`, `clear_event_log`, `read_event_log`

---

## 二、未完成的重构项

### 报告中还有但尚未执行的

1. **EditorGlyphSlotKind → EditorGlyphSlotType** (glyph_layout.rs) — Kind→Type 后缀
2. **ListMarkerKind → ListMarkerType** (list_semantics.rs) — Kind→Type 后缀
3. **ListTextSemantic → ListSemanticResult** (list_semantics.rs) — Semantic→Result
4. **derive_list_text_semantics → resolve_list_text_semantics** (list_semantics.rs) — derive→resolve
5. **looks_like_short_field_token → is_short_field_token** (editable_segments.rs) — looks_like→is
6. **is_preview_active** — 合规，不改
7. **shell_width → compute_shell_width** (draft_style.rs)
8. **same_existing_layout_line → is_same_layout_line** (draft_style.rs)
9. **source_baseline_y → get_baseline_y** (draft_style.rs)
10. **body_runs_text → read_body_runs_text** (draft_text_diff.rs)
11. **body_runs_match_source_text → is_body_matching_source** (draft_text_diff.rs)
12. **line_selection_range → read_line_selection_range** (style_preservation.rs)
13. **has_style_changes_against_paragraph → has_style_changes** (style_mapper.rs)
14. **is_preview_active** — 合规
15. **should_suppress** — 合规
16. **glyph_left → get_glyph_left** (glyph_layout.rs)
17. **glyph_right → get_glyph_right** (glyph_layout.rs)
18. **glyph_visual_width → get_glyph_width** (glyph_layout.rs) — visual冗余
19. **estimated_gap_source_advance → compute_estimated_gap_advance** (glyph_layout.rs)
20. **needs_gap → should_insert_gap** (glyph_layout.rs)
21. **infer_run_advance → resolve_run_advance** (glyph_layout.rs)
22. **EditorSessionTextPlan → EditorSessionTextLayout** (glyph_layout.rs) — Plan→Layout
23. **EditorGlyphSlotKind → EditorGlyphSlotType** (glyph_layout.rs)
24. **has_suspicious_run_geometry → has_abnormal_run_geometry** (glyph_layout.rs)
25. **same_existing_session_line → is_same_line** (caret_geometry.rs)
26. **caret_index_at_page_point → resolve_caret_index_at_page_point** (caret_geometry.rs)
27. **resolve_index → resolve_caret_index** (caret_geometry.rs)
28. **session_caret_visual → compute_session_caret_visual** → 需确认改为 get 还是 compute
29. **session_plan_caret_visual → compute_caret_visual** (caret_geometry.rs)
30. **populate_line_stops_from_text_plan → populate_line_stops** (caret_geometry.rs)
31. **resolve_navigation_from_lines → resolve_navigation** (caret_geometry.rs)
32. **to_layout_runs → build_layout_runs** (style_mapper.rs)
33. **whole_session_target → From trait** (edit_target.rs) — 已改为 create_from_session，From trait 未实现
34. **build_editor_document_plan_from_session → From trait** (document_plan.rs) — 未实现
35. **paragraph_editor_scene_from_plan → From trait** (paragraph_scene.rs) — 未实现

### geometry 模块剩余

1. **PdfToPageViewTransform::point → project_point** (coordinate_transform.rs)
2. **measure_dom_to_page_scale** (dom_projection.rs)

### render 模块剩余

1. **bbox_width → compute_bbox_width** (bbox_ops.rs)
2. **bbox_height → compute_bbox_height** (bbox_ops.rs)
3. **union_bbox → merge_bbox** (bbox_ops.rs)
4. **HostPageTransform::scale → get_scale** (coordinate_transform.rs)
5. **total_items → get_total_items** (progressive.rs)
6. **name → get_name** (renderer.rs)

### src-tauri 剩余

1. **effective_height → compute_effective_height** (pdf_utils.rs)
2. **candidate_count → get_candidate_count** (font/matching.rs)
3. **source_label → get_source_label** (pdf_write_font_resolver.rs)
4. **page_cache_key → build_page_cache_key** (cache.rs)
5. **page_revision_cache_key → build_page_revision_cache_key** (cache.rs)
6. **light_page_cache_key → build_light_page_cache_key** (cache.rs)
7. **extract_ttc_face_as_ttf → read_ttf** (font/ttc.rs)
8. **break_text_into_lines → compute_text_lines** (pdf_font.rs)
9. **find_xobject_by_name → find_xobject** (resource_reader.rs)
10. **truncate_for_log → truncate_for_logging** (save_text_write_plan.rs)
11. **render_objects_to_png → render_png** (vello_renderer.rs)
12. **resolve_layout_inference_revisioned → resolve_layout_inference** (geometry_service.rs)
13. **terminal_log → log_terminal_message** (interfaces/system.rs)

### pdf-viewer-ui 剩余

1. **snapshot_state → read_aggregated_state** (application.rs)
2. **read_zoom_state → get_state** (free_api/zoom)

---

## 三、注意事项

1. **`client_to_page` 合并为 `to_page(clamp_box: Option)`** — 之前 `client_to_page` 不带 clamp，现在改为 `to_page` 带 `Option<BoundingBox>` 参数。调用处原来不带 clamp 的需要传 `None`。
   - 已确认：所有原有 `client_to_page` 调用改成了 `to_page(ClientPoint {...})` — **这里有问题！没有传 `None` 参数！**
   - 需要修复：所有 `transform.to_page(ClientPoint { ... })` 应改为 `transform.to_page(ClientPoint { ... }, None)`

2. **`build_index_map` 合并** — 返回 `(source_to_runs, runs_to_source)`。调用处需要用 `.0` 或 `.1` 取对应映射。

3. **`CharIndex`/`Utf16Offset` newtype** — wasm_bindgen 接口中的 `utf16_to_char_index` 和 `char_to_utf16_offset` 方法名保留不变（这是 JS API 名），内部实现改为使用 newtype。

4. **`reconstructed_char_count` 字段名** — document_plan.rs 中有 `pub reconstructed_char_count: usize` 字段，这是 struct field 不是函数，暂未改名。

5. **`generate_demo` 重名冲突** — document_service.rs 和 pdf_write_service.rs 都有 `generate_demo` 方法，但一个是 `PdfDocumentService::generate_demo`，另一个是 `PdfWriteService::generate_demo`，通过不同 struct 区分，不会冲突。

6. **src-tauri 的 `parse_hex_color`** — 需要检查调用处是否也更新了。

7. **src-tauri 的 `get_character_width`** — 需要检查调用处是否也更新了。

8. **src-tauri 的 `log_event`** — 需要检查调用处是否也更新了。
