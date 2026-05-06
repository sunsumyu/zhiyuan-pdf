# PDF Viewer 全面命名规范

## 1. 核心原则

1. **动词 + 名词** - 清楚表达"做什么" + "对什么做"
2. **语义明确** - 避免模糊词汇（get, do, process, handle）
3. **能力分层** - 按函数能力类型选择动词

## 2. 能力分类（Skills）

| 能力 | 动词 | 副作用 | 示例 |
|------|------|-------|------|
| **Query** | read, find, list, search | 无 | read_metadata() |
| **Resolve** | resolve, compute | 无 | resolve_layout() |
| **Transform** | convert, transform, project | 无 | convert_to_layout_runs() |
| **Validate** | is, has, should | 无 | is_preview_active() |
| **Mutate** | set, update, apply, toggle | 有 | apply_highlight() |
| **Create** | create, build, init | 有 | create_document() |
| **Destroy** | delete, remove, clear, close | 有 | close_pdf_resources() |
| **Execute** | execute, commit, dispatch | 有 | execute_save() |
| **Lifecycle** | start, stop, schedule | 有 | start_render_frame() |

## 3. 主要修改规则

### Query 查询类
- `get_*` → `read_*`（明确语义，避免与JS getter混淆）
- `get_preview_active()` → `is_preview_active()`（返回bool用is）

### Resolve 计算类
- `extract_*` → `resolve_*`（已完成）
- `build_*`（构建计划） → `resolve_*`
- `measure_*` → `resolve_*`
- `project_*`（数学投影） → 保留

### Transform 转换类
- `to_layout_runs()` → `convert_to_layout_runs()`

### Validate 验证类
- `requires_source_replacement()` → `should_replace_source()`
- `is_bold_any()` → `has_bold_span()`
- `is_bold_all()` → `is_all_bold()`

### Mutate 修改类
- `set_bold_all()` → `apply_bold_to_all()`（对集合应用操作）
- `set_italic_all()` → `apply_italic_to_all()`
- `toggle_bold_all()` → `toggle_bold()`（去掉冗余all）
- 简单赋值 `set_*` 保留（set_alignment, set_caret_index）

### Create 创建类
- `open_pdf()` → `create_document()`
- `open_document_readonly()` → `read_document()`
- `generate_demo_pdf()` → `create_demo_pdf()`
- `init_page_context()` → `create_page_context()`

### Destroy 销毁类
- `release_pdf_resources()` → `close_pdf_resources()`
- `release_document_cache()` → `clear_document_cache()`

### Execute 执行类
- `save_pdf()` → `execute_save()`
- `rollback_pdf()` → `execute_rollback()`
- `redo_pdf()` → `execute_redo()`

### Lifecycle 生命周期
- `begin_render_frame()` → `start_render_frame()`
- `step_zoom_animation()` → `advance_zoom_animation()`

## 4. 具体修改清单（关键函数）

### src-tauri/src/interfaces/multimedia/pdf.rs
| 当前名 | 建议名 | 能力类型 |
|-------|-------|---------|
| get_metadata | read_metadata | Query |
| open_document | create_document | Create |
| open_document_readonly | read_document | Query |
| get_page_preview | read_page_preview | Query |
| prefetch_page_preview | load_page_preview | Query |
| save_document | execute_save | Execute |
| commit_document_edits | execute_commit | Execute |
| undo_document | execute_undo | Execute |
| redo_document | execute_redo | Execute |
| release_document_cache | clear_document_cache | Destroy |
| get_last_materialization_report | read_last_materialization_report | Query |
| apply_region_patches | execute_region_patches | Execute |
| apply_highlight | apply_highlight | Mutate (保留) |
| apply_comment | apply_comment | Mutate (保留) |
| delete_page_annotation | remove_page_annotation | Destroy |
| apply_comment_update | apply_comment_update | Mutate (保留) |
| apply_batch_replace | execute_batch_replace | Execute |
| apply_replace | execute_replace | Execute |
| search_page_regions | find_page_regions | Query |
| search_document_regions | find_document_regions | Query |
| get_images | read_images | Query |
| resolve_caret | resolve_caret | Resolve (保留) |
| resolve_hit | resolve_hit | Resolve (保留) |
| resolve_hit_target | resolve_hit_target | Resolve (保留) |
| resolve_projection | resolve_projection | Resolve (保留) |
| resolve_params | resolve_params | Resolve (保留) |

### crates/pdf-viewer-ui/src/wasm_api/viewer.rs
| 当前名 | 建议名 | 能力类型 |
|-------|-------|---------|
| start_progressive_render | start_progressive_render | Lifecycle (保留) |
| resolve_progressive_render_policy | resolve_progressive_render_policy | Resolve (保留) |
| step_progressive_render | advance_progressive_render | Lifecycle |
| cancel_progressive_render | cancel_progressive_render | Lifecycle (保留) |
| render_page | execute_render_page | Execute |
| commit_render_result | commit_render_result | Execute (保留) |
| resolve_font_face | resolve_font_face | Resolve (保留) |
| build_editable_segments | resolve_editable_segments | Resolve |
| resolve_editor_projection | resolve_editor_projection | Resolve (保留) |
| get_pagination_commands | read_pagination_commands | Query |
| build_page_region_context | resolve_page_region_context | Resolve |
| project_page_rect_to_layer_rect | resolve_page_rect_projection | Resolve |
| measure_dom_to_page_scale | resolve_dom_to_page_scale | Resolve |
| resolve_page_point | resolve_page_point | Resolve (保留) |
| init_page_context | create_page_context | Create |
| update_page_viewport | update_page_viewport | Mutate (保留) |
| resolve_wheel_zoom | resolve_wheel_zoom | Resolve (保留) |
| handle_wheel_zoom_host | execute_wheel_zoom | Execute |
| resolve_anchor_scroll | resolve_anchor_scroll | Resolve (保留) |
| resolve_wheel_render_decision | resolve_wheel_render_decision | Resolve (保留) |
| resolve_preview_tick_decision | resolve_preview_tick_decision | Resolve (保留) |
| step_preview_host | advance_preview_host | Lifecycle |
| resolve_render_follow_up | resolve_render_follow_up | Resolve (保留) |
| schedule_render_follow_up | schedule_render_follow_up | Lifecycle (保留) |
| resolve_layer_execution_plan | resolve_layer_execution_plan | Resolve (保留) |
| resolve_render_execution_plan | resolve_render_execution_plan | Resolve (保留) |
| resolve_layer_present_decision | resolve_layer_present_decision | Resolve (保留) |
| resolve_zoom_limits | resolve_zoom_limits | Resolve (保留) |
| resolve_render_zoom | resolve_render_zoom | Resolve (保留) |
| resolve_frame_plan | resolve_frame_plan | Resolve (保留) |
| resolve_viewport_refresh | resolve_viewport_refresh | Resolve (保留) |
| resolve_host_scroll_refresh | resolve_host_scroll_refresh | Resolve (保留) |
| touch_frame_cache_entry | mark_frame_cache_touched | Mutate |
| store_frame_cache_entry | write_frame_cache_entry | Mutate |
| reset_frame_cache | clear_frame_cache | Destroy |
| take_frame_plan | read_frame_plan | Query |
| begin_render_frame | start_render_frame | Lifecycle |
| schedule_render_frame | schedule_render_frame | Lifecycle (保留) |
| is_render_frame_current | is_render_frame_current | Validate (保留) |
| commit_render_frame | commit_render_frame | Execute (保留) |

### 内部函数
| 当前名 | 建议名 | 能力类型 |
|-------|-------|---------|
| get_working_path | resolve_working_path | Resolve |
| get_last_pdf_materialization_report | read_last_pdf_materialization_report | Query |
| get_image_cache | read_image_cache | Query |

## 5. 例外规则

以下情况保留原命名：
1. **标准trait实现**：`Default::default()`, `Clone::clone()`
2. **构造函数**：`Document::new()`, `Page::new()`
3. **已实现规范的函数**：所有 `resolve_*` 函数
4. **WASM绑定入口**：`#[wasm_bindgen]` 函数在JS端可见，修改需同步前端
