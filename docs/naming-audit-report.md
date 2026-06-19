# 命名审查报告

> 审查日期：2026-06-18
> 规则依据：docs/naming-conventions.md
> 审查范围：crates/pdf-viewer-core、crates/pdf-viewer-ui、src-tauri

---

## 一、审查规则摘要

| 能力类型 | 约定动词 | 示例 |
|---------|---------|------|
| Query | `read_`, `find_`, `list_`, `search_` | `read_metadata()` |
| Resolve | `resolve_`, `compute_` | `resolve_layout()` |
| Create | `new`, `create_`, `build_`, `init_` | `build_styles()` |
| Validate | `is_`, `has_`, `should_`, `can_` | `is_preview_active()` |
| Mutate | `set_`, `update_`, `apply_`, `toggle_` | `apply_highlight()` |
| Execute | `execute_`, `commit_`, `dispatch_` | `commit_edit()` |
| Lifecycle | `start_`, `stop_`, `schedule_`, `advance_`, `cancel_` | `start_render_frame()` |
| Sync | `sync_` | `sync_editor_input()` |

**禁止模式：**
- `get_*` → 改用 `read_`
- `process_*`, `handle_*`, `do_*` → 改用具体动词
- `_for_`, `_to_`, `_into_` 后缀 → 改用 `_from_`, `_with_` 前置
- 名词短语函数名 → 必须动词开头
- `make_` 非约定动词 → 改用 `build_`/`create_`

---

## 二、不合规命名汇总

### P0: 动词违规（共 21 处）

| 模块 | 文件 | 当前名 | 中文功能 | 问题 | 建议改名 |
|-----|------|--------|----------|------|----------|
| core | edit/edit_target.rs | `make_edit_segment_target_id` | 构造segment编辑目标ID | make非约定动词 | `build_edit_segment_target_id` |
| core | text/style_preservation.rs | `make_style_run` | 创建样式快照Run | make非约定动词 | `create_style_run` |
| core | text/style_mapper.rs | `dominant_style` | 读取主导样式 | 缺少动词 | `read_dominant_style` |
| core | text/glyph_layout.rs | `glyph_left` | 读取字形左侧X坐标 | 缺少动词 | `read_glyph_left` |
| core | text/glyph_layout.rs | `glyph_right` | 读取字形右侧X坐标 | 缺少动词 | `read_glyph_right` |
| core | text/glyph_layout.rs | `glyph_visual_width` | 读取字形可视宽度 | 缺少动词 | `read_glyph_visual_width` |
| core | text/caret_geometry.rs | `session_caret_visual` | 读取会话光标视觉位置 | 缺少动词 | `read_session_caret_visual` |
| core | geometry/source_geometry.rs | `source_session_visual_bbox` | 获取会话可视边界框 | 缺少动词 | `read_session_visual_bbox` |
| core | geometry/source_geometry.rs | `caret_line_bbox` | 获取光标所在行边界框 | 缺少动词 | `read_caret_line_bbox` |
| core | geometry/coordinate_transform.rs | `HostPageTransform::scale` | 读取缩放比例 | 缺少动词 | `read_scale` |
| core | geometry/coordinate_transform.rs | `PdfToPageViewTransform::point` | 投射点到视图坐标系 | 缺少动词 | `project_point` |
| core | geometry/bbox_ops.rs | `union_bbox` | 合并两边界框 | 名词短语非动词 | `merge_bbox` |
| core | render/viewport_culling.rs | `styled_run_bbox` | 计算样式run包围盒 | 缺少动词 | `compute_styled_run_bbox` |
| core | render/prepared_scene.rs | `visible_vector_indices` | 查询可见vector索引 | 缺少动词 | `find_visible_vector_indices` |
| core | render/prepared_scene.rs | `active_text_object_ids` | 查询活跃文本对象ID | 缺少动词 | `find_active_text_object_ids` |
| tauri | font/metrics.rs | `get_character_width_pdf_units` | 获取字符宽度 | get_前缀 | `read_character_width_pdf_units` |
| tauri | log_service.rs | `get_pdf_log_level` | 读取PDF日志级别 | get_前缀 | `read_pdf_log_level` |
| tauri | font/matching.rs | `candidate_count` | 返回候选字体数量 | 缺少动词 | `read_candidate_count` |
| tauri | pdf_utils.rs | `effective_height` | 有效高度 | 缺少动词 | `read_effective_height` |
| tauri | pdf_write_font_resolver.rs | `source_label` | 返回来源标签 | 缺少动词 | `read_source_label` |
| tauri | cache.rs | `page_cache_key` | 生成页面缓存键 | 缺少动词 | `build_page_cache_key` |

### P1: 介词后缀（共 15 处）

| 模块 | 文件 | 当前名 | 问题 | 建议改名 |
|-----|------|--------|------|----------|
| core | text/index_convert.rs | `utf16_offset_to_char_index` | to后缀 | `read_char_index_from_utf16_offset` |
| core | text/index_convert.rs | `char_index_to_utf16_offset` | to后缀 | `read_utf16_offset_from_char_index` |
| core | text/editable_segments.rs | `build_contiguous_segments_in_range` | in_range后缀 | `build_contiguous_segments_from_range` |
| core | text/style_mapper.rs | `has_style_changes_against_paragraph` | against后缀 | `has_style_changes_from_paragraph` |
| core | text/style_mapper.rs | `to_layout_runs` | to后缀 | `build_layout_runs` |
| core | text/caret_geometry.rs | `populate_line_stops_from_text_plan` | from后缀冗余 | `populate_line_stops_with_plan` |
| core | render/zoom_host.rs | `resolve_anchor_from_visible_preview_state` | from后缀冗余 | `resolve_visible_preview_anchor` |
| tauri | font/ttc.rs | `extract_ttc_face_as_ttf` | as后缀 | `extract_ttf_from_ttc` |
| tauri | pdf_font.rs | `break_text_into_lines` | into后缀 | `compute_text_lines` |
| tauri | pdf_read/resource_reader.rs | `find_xobject_by_name` | by后缀 | `find_xobject_with_name` |
| tauri | save_text_write_plan.rs | `truncate_for_log` | for后缀 | `truncate_with_log_limit` |
| tauri | vello_renderer.rs | `render_objects_to_png` | to后缀 | `render_png_from_objects` |
| core | edit/document_plan.rs | `build_editor_document_plan_from_session` | from后缀冗余 | `build_from_session` |
| core | edit/edit_target.rs | `edit_target_base_paragraph_id` | 缺动词+隐含from | `read_base_paragraph_id_from_target` |
| core | edit/edit_target.rs | `edit_target_segment_key` | 缺动词+隐含from | `read_segment_key_from_target` |

### P2: 非约定动词（共 12 处）

| 模块 | 文件 | 当前名 | 问题 | 建议改名 |
|-----|------|--------|------|----------|
| core | text/editable_segments.rs | `detect_field_label_anchors` | detect非约定 | `find_field_label_anchors` |
| core | text/editable_segments.rs | `looks_like_short_field_token` | looks_like非标准 | `is_short_field_token` |
| core | text/glyph_layout.rs | `extract_decorative_prefix` | extract非约定 | `read_decorative_prefix` |
| core | text/glyph_layout.rs | `map_raw_to_reconstructed` | map非约定 | `read_reconstructed_index_from_raw` |
| core | text/glyph_layout.rs | `needs_gap` | needs非约定 | `should_insert_gap` |
| core | text/caret_geometry.rs | `dedupe_caret_stops` | dedupe非约定 | `apply_caret_stop_dedup` |
| core | text/caret_geometry.rs | `same_existing_session_line` | same非标准动词 | `is_same_session_line` |
| core | render/text_suppression.rs | `process_visible_objects` | process禁用 | `apply_suppression_to_visible_objects` |
| core | render/glyph_plan.rs | `process_glyph_paragraph` | process禁用 | `apply_overlay_suppression_to_paragraph` |
| core | render/overlay_ops.rs | `insert_overlay_if_needed` | if_needed后缀 | `try_insert_overlay` |
| core | render/prepared_scene.rs | `build` | 缺名词 | `build_prepared_scene` |
| tauri | interfaces/system.rs | `terminal_log` | 无动词 | `log_terminal_message` |

### P3: 后缀不统一（共 7 处）

| 模块 | 文件 | 当前名 | 问题 | 建议改名 |
|-----|------|--------|------|----------|
| core | text/style_mapper.rs | `set_bold_all` | all后缀不精确 | `apply_bold_all` |
| core | text/style_mapper.rs | `set_italic_all` | all后缀不精确 | `apply_italic_all` |
| core | text/style_mapper.rs | `set_underline_all` | all后缀不精确 | `apply_underline_all` |
| core | text/style_mapper.rs | `set_color_all` | all后缀不精确 | `apply_color_all` |
| core | text/style_mapper.rs | `set_font_name_all` | all后缀不精确 | `apply_font_name_all` |
| core | text/style_mapper.rs | `set_font_size_all` | all后缀不精确 | `apply_font_size_all` |
| core | text/style_mapper.rs | `set_char_spacing_all` | all后缀不精确 | `apply_char_spacing_all` |

---

## 三、统计汇总

| 模块 | 合规项 | 不合规项 | 合规率 |
|-----|-------|---------|--------|
| pdf-viewer-core/edit | 140 | 11 | 93% |
| pdf-viewer-core/text | 48 | 34 | 59% |
| pdf-viewer-core/geometry | 45 | 9 | 83% |
| pdf-viewer-core/render | 25 | 8 | 76% |
| pdf-viewer-core/typography | 24 | 0 | 100% |
| pdf-viewer-core/document | 18 | 0 | 100% |
| pdf-viewer-core/history | 4 | 0 | 100% |
| pdf-viewer-core/common | 18 | 0 | 100% |
| pdf-viewer-core/annotation | 30 | 0 | 100% |
| pdf-viewer-ui | 180+ | 15+ | ~92% |
| src-tauri | 60+ | 12 | ~83% |
| **总计** | **~590** | **~55** | **~91%** |

---

## 四、命名约定一致性检查

### 前缀使用统计

| 前缀 | 使用场景 | 一致性 |
|-----|---------|--------|
| `read_` | 查询状态/数据 | ✅ 一致，无get_混用 |
| `find_` | 返回Option | ✅ 一致 |
| `resolve_` | 带规则推导 | ✅ 一致，但个别用infer/derive |
| `build_` | 无副作用构造 | ✅ 一致 |
| `is_`/`has_`/`should_`/`can_` | 布尔判断 | ✅ 一致，个别用needs/same |
| `apply_` | 对集合操作 | ⚠️ 部分用set_xxx_all |
| `sync_` | 跨边界同步 | ✅ 一致 |

### 后缀使用统计

| 后缀 | 使用场景 | 一致性 |
|-----|---------|--------|
| `_all` | 全局/全部操作 | ⚠️ 不够精确，建议改apply_前缀 |
| `_revisioned` | 带版本 | ⚠️ 仅geometry_service用，建议统一with_revision |
| `_tx` | 事务操作 | ✅ 仅在需要区分时使用 |

---

## 五、建议修改优先级

### P0 必须修改（影响可读性）

1. 所有 `get_*` → `read_*`
2. 所有 `process_*` → `apply_*`
3. 所有缺少动词的函数名（名词短语）
4. 所有 `make_*` → `build_*`/`create_*`

### P1 应该修改（违反约定）

1. 介词后缀 `_for_`/`_to_`/`_into_`/`_by_` → 前置 `_from_`/`_with_`
2. 非约定动词 `detect`/`extract`/`map`/`needs`/`dedupe`

### P2 建议修改（提升一致性）

1. `set_xxx_all` → `apply_xxx_all`
2. 统一后缀语义（`_revisioned` → `_with_revision`）

---

## 六、详细模块报告

详细审查表格见各模块报告：

- `naming-audit-edit.md` — edit 模块
- `naming-audit-text.md` — text 模块
- `naming-audit-geometry.md` — geometry 模块
- `naming-audit-render.md` — render 模块
- `naming-audit-tauri.md` — src-tauri 模块
