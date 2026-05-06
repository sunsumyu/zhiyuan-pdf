# Sovereignty PDF Viewer - 全面重构方案

> 制定日期: 2026-05-03  
> 审计范围: 2,029 个方法，204 个源文件，4 大层级  
> 原则: 每个方法必须对应一个明确功能，无功能的方法必须删除或合并

---

## 第一部分：推荐方案

基于架构分析，针对 633 个孤儿方法，我的推荐如下：

| 问题 | 推荐方案 | 理由 |
|------|---------|------|
| 1. 测试方法归属 | **A** - 移至 `tests/` | 测试代码不应混入生产代码 |
| 2. 字符计数功能 | **C** - 保留两处但统一接口 | 布局分析和编辑统计用途不同 |
| 3. UI组件分类 | **A** - 新增 F29 UI组件系统 | UI构建是独立关注点 |
| 4. 诊断系统 | **B** - 归入 F25 调试诊断 | 诊断是调试子功能 |
| 5. 通知系统 | **A** - 新增 F31 通知系统 | 消息/建议/通知是独立能力 |
| 6. 通用工具 | **A** - 保留，新增 F33 通用工具库 | 避免外部依赖，保持自包含 |
| 7. PDF工具重复 | **A** - 删除Utils层重复功能 | 消除冗余，统一调用Kernel |
| 8. WASM基础设施 | **A** - 新增 F32 WASM基础设施 | WASM加载是独立生命周期 |

**推荐答案**: `1:A 2:C 3:A 4:B 5:A 6:A 7:A 8:A`

**说明**: 问题1的"A"指保留测试在原处（已有`#[test]`标记），重构期间不移动，重构完成后再评估是否需要移动到`tests/`目录。

---

## 第二部分：更新后的功能域架构

基于推荐方案，功能域从 28 个扩展到 **33 个**：

### 原有功能域（28个）
- F1-F28: 保持不变（详见 `function_method_mapping.md`）

### 新增功能域（5个）

#### F29 UI组件系统
**功能描述**: 管理所有UI组件的创建、渲染、更新、销毁
**包含方法**:
- `createDiagnosticPanel`, `createDiagnosticItem`, `createDiagnosticList`
- `buildCommentOverlay`, `buildCommentOverlayView`
- `buildCommentReviewPanel`, `buildCommentReviewPanelView`
- `createMessageBubble`, `createSuggestionCard`
- `buildHostAction`, `buildHostActionList`
- 所有 `create*`/`build*` UI构建方法

#### F30 诊断面板（归入F25）
**功能描述**: 诊断信息的UI展示和交互（合并到F25调试诊断）
**包含方法**:
- `showDiagnosticPanel`, `updateDiagnosticPanel`
- `formatDiagnosticMessage`, `formatDiagnosticTime`
- `getDiagnosticIcon`, `getDiagnosticLevel`
- 所有 `*diagnostics*` UI方法

#### F31 通知系统
**功能描述**: 消息气泡、建议卡片、通知提示的统一管理
**包含方法**:
- `createMessageBubble`, `createSuggestionCard`
- `updateSuggestion`, `describeError`
- 所有通知相关UI方法

#### F32 WASM基础设施
**功能描述**: WASM模块加载、初始化、进度管理
**包含方法**:
- `loadWasm`, `loadWasmWithProgress`
- `ensureWasmInitialized`, `getWasmApi`
- `initSync`, `__wbg*` 绑定方法

#### F33 通用工具库
**功能描述**: 字符串、数组、数学、日期等通用工具函数
**包含方法**:
- `capitalize`, `truncate`, `escapeHtml`, `unescapeHtml`, `formatBytes`
- `chunk`, `flatten`, `unique`, `sortBy`, `groupBy`
- `clamp`, `lerp`, `randomBetween`, `roundTo`, `toRadians`
- `formatDate`, `formatDateTime`, `getCurrentTime`, `parseDate`, `timeAgo`
- `colorToCss`, `hexToRgb`, `parseColor`, `rgbToHex`

---

## 第三部分：重构行动计划

### P0 - 立即执行（消除技术债务）

#### 1. 删除V3桥接重复（21个方法）
**文件**: `crates/pdf-viewer-ui/pkg/pdf_viewer_ui.d.ts`  
**删除方法**:
```
build_editable_segments_v3
build_field_group_snapshot_paint_runs_v3
build_page_region_context_v3
build_paragraph_snapshot_paint_runs_v3
build_persistable_save_plan_v3
clear_history_v3
collect_legacy_text_reflows_v3
collect_persistable_region_patches_v3
convert_client_point_to_page_point_v3
derive_list_text_semantics_v3
distribute_text_across_runs_v3
is_colon_token_v3
looks_like_short_field_token_v3
measure_dom_to_page_scale_v3
preserve_changed_line_styles_v3
redo_v3
resolve_field_hit_v3
split_runs_by_body_start_v3
undo_v3
wasm_apply_document_patch_v3
wasm_render_page_v3
```
**执行步骤**:
1. 删除所有 `*_v3` 方法声明
2. 更新所有调用点使用主方法名
3. 验证编译通过

#### 2. 删除 `wasm_` 前缀（40+对重复）
**文件**: `crates/pdf-viewer-ui/src/wasm_api/`  
**重命名方法**:
```
wasm_abort_render_frame → abort_render_frame
wasm_accept_all_review_changes → accept_all_review_changes
wasm_accept_review_change → accept_review_change
wasm_active_editor_has_session_changes → active_editor_has_session_changes
wasm_add_region_comment → add_region_comment
wasm_advance_render_loop_frame → advance_render_loop_frame
wasm_apply_active_editor_format_action_and_schedule_render → apply_format
wasm_apply_document_patch → apply_document_patch
wasm_apply_editor_input_command_and_schedule_render → apply_input
wasm_apply_region_text_replacements_and_schedule_render → apply_replacements
wasm_apply_zoom_selection → apply_zoom_selection
wasm_begin_render_frame → begin_render_frame
wasm_build_editable_segments → build_editable_segments
wasm_build_page_region_context → build_page_region_context
wasm_build_region_text_patch → build_region_text_patch
wasm_calculate_editor_projection → resolve_projection
wasm_cancel_progressive_render → cancel_render
wasm_clear_comment_review_session → clear_comment_review_session
wasm_clear_find_session → clear_find_session
wasm_clear_pending_anchor → clear_pending_anchor
wasm_clear_preview_present → clear_preview_present
wasm_clear_zoom_preview_host_state → clear_zoom_preview_host_state
wasm_close_active_editor_and_schedule_render → close_editor
wasm_close_document_pipeline → close_document_pipeline
wasm_commit_render_frame → commit_frame
wasm_navigate_next_page → next_page
wasm_navigate_prev_page → prev_page
wasm_resolve_render_zoom → resolve_zoom
wasm_start_progressive_render → start_render
wasm_step_preview_tick → step_preview
wasm_sync_active_editor_input_and_schedule_render → sync_input
wasm_sync_and_commit_active_editor_text_and_schedule_render → commit_editor
wasm_toggle_comment_review_panel_and_load → toggle_comment_review_panel
```
**执行步骤**:
1. 重命名WASM导出方法（去掉 `wasm_` 前缀）
2. 使用 `#[wasm_bindgen]` 宏自动添加 `wasm_` 前缀到JS绑定
3. 更新TypeScript调用点
4. 验证编译通过

#### 3. 移除 `_and_schedule_render` 后缀（10个方法）
**文件**: `crates/pdf-viewer-ui/src/editor/`, `src/bridge/`  
**重命名并重构**:
```
wasm_apply_active_editor_format_action_and_schedule_render → apply_format
wasm_apply_editor_input_command_and_schedule_render → apply_input
wasm_apply_region_text_replacements_and_schedule_render → apply_replacements
wasm_close_active_editor_and_schedule_render → close_editor
wasm_open_paragraph_editor_at_client_point_and_schedule_render → open_editor_at_point
wasm_open_region_editor_and_schedule_render → open_region_editor
wasm_sync_active_editor_input_and_schedule_render → sync_input
wasm_sync_and_commit_active_editor_text_and_schedule_render → commit_editor
sync_active_editor_input_and_schedule_render → sync_input
sync_and_commit_active_editor_text_and_schedule_render → commit_editor
```
**执行步骤**:
1. 重命名方法，去掉 `_and_schedule_render` 后缀
2. 在方法内部自动调用 `schedule_render()`
3. 更新所有调用点
4. 验证渲染调度正常

#### 4. 保留测试方法在原处（12个方法）
**说明**: 这些方法已有 `#[test]` 属性，重构期间保留在原处以验证重构正确性。

**涉及的测试方法**:
```
test_cjk_no_start_rule (layout_engine.rs)
test_justified_alignment (layout_engine.rs)
test_bbox (document_plan.rs)
test_layout_run (document_plan.rs)
test_layout_run_with_char_gaps (document_plan.rs)
test_paint_run (document_plan.rs)
test_resolved_font (document_plan.rs)
test_style (document_plan.rs)
test_styled_run (document_plan.rs)
test_run (draft_layout.rs)
test_run_with_origins (source_geometry.rs)
test_deletion_at_head (style_mapper.rs)
test_deletion_multi_byte_chinese (style_mapper.rs)
test_full_deletion_protection (style_mapper.rs)
```
**执行步骤**:
1. 确保所有测试方法都有 `#[cfg(test)]` 包裹（当前已满足）
2. 重构过程中运行 `cargo test` 验证功能正确性
3. 重构完成后评估是否需要移动到 `tests/` 目录

### P1 - 短期整改（1-2周）

#### 5. 合并渲染管线透传（12对方法）
**问题**: `present/facade.rs` ↔ `present/runtime.rs` 1:1 透传
**合并方案**:
```
schedule_render_frame (facade) + schedule_render_frame (runtime) → schedule_render_frame (runtime)
settle_render_frame (facade) + settle_render_frame (runtime) → settle_render_frame (runtime)
store_frame_cache_entry (facade) + store_frame_cache_entry (runtime) → store_frame_cache_entry (runtime)
touch_frame_cache_entry (facade) + touch_frame_cache_entry (runtime) → touch_frame_cache_entry (runtime)
reset_frame_cache (facade) + reset_frame_cache (runtime) → reset_frame_cache (runtime)
resolve_viewport_refresh (facade) + resolve_viewport_refresh (runtime) → resolve_viewport_refresh (runtime)
```
**执行步骤**:
1. 删除 `facade.rs` 中的透传方法
2. 更新调用点直接调用 `runtime.rs`
3. 删除 `facade.rs` 文件（如果无其他用途）
4. 验证渲染管线正常

#### 6. 提取公共工具函数（15+个方法）
**提取到 `geometry/bbox_utils.rs`**:
```
bbox_height, bbox_width, bbox_intersects, union_bbox
```
**提取到 `utils/sanitize.rs`**:
```
sanitize_non_negative, sanitize_positive, sanitize_zoom_state
```
**提取到 `utils/debug.rs`**:
```
truncate_debug_text, truncate_for_log, compactString, compactValue
```
**提取到 `utils/text-utils.rs`**:
```
chars_count, split_key_value_text, get_object_display_text
```
**执行步骤**:
1. 创建对应的工具模块
2. 移动方法到新模块
3. 更新所有引用
4. 验证编译通过

#### 7. 删除Utils层PDF工具重复（5个方法）
**文件**: `utils/pdf-utils.ts`
**删除方法**:
```
calculatePdfPageCount
extractPdfText
getPdfMetadata
isPdfFile
parsePdfPage
```
**执行步骤**:
1. 删除 `utils/pdf-utils.ts` 文件
2. 查找所有调用点
3. 改为调用 Tauri 命令（`get_metadata`, `extract_page_info` 等）
4. 验证功能正常

#### 8. 拆分 `pdf_write_font_resolver.rs`（46 methods → 3 files）
**拆分方案**:
```
pdf_write_font_resolver.rs (46 methods) →
  ├─ font_resolve.rs (字体匹配逻辑)
  ├─ font_encode.rs (字体编码逻辑)
  └─ font_ttf.rs (TTF解析逻辑)
```
**执行步骤**:
1. 分析方法归属
2. 创建3个新文件
3. 移动对应方法
4. 更新引用
5. 验证编译通过

#### 9. 合并 `execute_pdf_commands_v1` / `_inner`
**文件**: `src-tauri/src/interfaces/multimedia/pdf.rs`
**合并方案**:
```
execute_pdf_commands_v1 + execute_pdf_commands_v1_inner → execute_commands
```
**执行步骤**:
1. 合并两个方法为一个
2. 删除 `_v1` 后缀
3. 更新调用点
4. 验证功能正常

### P2 - 中期优化（2-4周）

#### 10. 拆分 `editor/` 模块（412 methods → 5个子模块）
**拆分方案**:
```
editor/ (38 files, 412 methods) →
  ├─ editor/source/ (source_identity, source_geometry, source_runs, source_text)
  ├─ editor/draft/ (draft_layout, edited_text_layout, source_layout)
  ├─ editor/session/ (session, engine_state, host_runtime)
  ├─ editor/overlay/ (paragraph_overlay, paragraph_scene, visual)
  └─ editor/format/ (list_format, style_preservation)
```
**执行步骤**:
1. 分析每个文件的职责
2. 规划5个子模块的边界
3. 逐个移动文件
4. 更新引用
5. 验证编译通过

#### 11. 拆分 `engine.rs`（263 methods → 3 services）
**文件**: `src-tauri/src/infrastructure/multimedia/pdf/engine.rs`
**拆分方案**:
```
engine.rs (263 methods) →
  ├─ pdf_read_service.rs (读取相关)
  ├─ pdf_write_service.rs (写入相关)
  └─ pdf_geometry_service.rs (几何相关)
```
**执行步骤**:
1. 分析方法职责
2. 创建3个服务文件
3. 移动对应方法
4. 更新引用
5. 验证编译通过

#### 12. 统一命名规范（全局）
**命名原则**: 准确、极简、无虚词
**重命名规则**:
```
calculate_* → resolve_*
get_model_* → read_*
get_document_* → 去掉 document
build_* → 保留（构造语义）
resolve_* → 保留（推导语义）
is_* → 保留（布尔判断）
should_* → 保留（布尔判断）
```
**执行步骤**:
1. 列出所有违反命名规范的方法
2. 逐个重命名
3. 更新所有引用
4. 验证编译通过

#### 13. 清理 `window.*` API（42个全局函数）
**文件**: `src/bridge/pdf_window_api.ts`
**重构方案**:
```
window.openPdfFile → PdfViewerAPI.openPdf()
window.pdfPrevPage → PdfViewerAPI.prevPage()
window.pdfNextPage → PdfViewerAPI.nextPage()
window.pdfZoomChange → PdfViewerAPI.setZoom()
window.pdfUndo → PdfViewerAPI.undo()
window.pdfRedo → PdfViewerAPI.redo()
window.pdfSave → PdfViewerAPI.save()
window.pdfRotate → PdfViewerAPI.rotate()
... (42个方法)
```
**执行步骤**:
1. 定义 `PdfViewerAPI` 接口
2. 实现所有方法
3. 删除全局 `window.*` 赋值
4. 更新 `src/main.ts` 使用新API
5. 验证UI功能正常

### P3 - 长期架构（1-2月）

#### 14. 新增功能域模块化
**创建新模块结构**:
```
src/ui-components/          # F29 UI组件系统
  ├── diagnostic/
  ├── comment/
  ├── review/
  └── notification/

src/notification/           # F31 通知系统
  ├── message-bubble/
  ├── suggestion-card/
  └── notification-manager/

src/wasm-infrastructure/    # F32 WASM基础设施
  ├── loader/
  ├── initializer/
  └── progress-tracker/

src/utils/                  # F33 通用工具库
  ├── string-utils/
  ├── array-utils/
  ├── math-utils/
  └── date-utils/
```

#### 15. 删除内部方法不当导出（22个方法）
**文件**: `crates/pdf-viewer-ui/src/wasm_api/`
**删除导出的内部方法**:
```
wasm_get_wheel_render_pending
wasm_set_wheel_render_pending
wasm_peek_pending_anchor_layout
wasm_peek_pending_anchor_scroll
wasm_take_pending_anchor_layout
wasm_take_pending_anchor_scroll
wasm_take_ready_committed_frame
wasm_queue_committed_frame
wasm_queue_render_loop_frame
wasm_advance_render_loop_frame
wasm_is_render_frame_current
wasm_mark_rendered_zoom
wasm_store_frame_cache_entry
wasm_touch_frame_cache_entry
wasm_reset_frame_cache
wasm_set_visual_layout
wasm_set_wheel_render_pending
wasm_editor_char_index_to_utf16_offset
wasm_editor_utf16_offset_to_char_index
wasm_peek_pending_anchor_layout
wasm_peek_pending_anchor_scroll
```
**执行步骤**:
1. 移除 `#[wasm_bindgen]` 属性
2. 改为内部方法（`pub(crate)`）
3. 验证编译通过

---

## 第四部分：方法删除清单

### 可直接删除的方法（无明确功能）

| 方法名 | 文件 | 删除理由 |
|--------|------|---------|
| `mock_run` | `layout_engine.rs` | 调试用，非生产代码 |
| `get_core_version` | `lib.rs` | 仅调试用，无业务价值 |
| `name` | `renderer.rs` | 无实际逻辑 |
| `clear` | `renderer.rs` | 重复，使用通用clear |
| `push` | `history_manager.rs` | 通用操作，应使用栈抽象 |
| `clear` | `history_manager.rs` | 通用操作，应使用栈抽象 |

### 合并后可删除的方法（重复）

| 原方法 | 合并为 | 删除理由 |
|--------|--------|---------|
| `execute_pdf_commands_v1` | `execute_commands` | 透传方法 |
| `execute_pdf_commands_v1_inner` | `execute_commands` | 内部实现 |
| `build_editable_segments_v3` | `build_editable_segments` | V3遗留 |
| `undo_v3` | `undo` | V3遗留 |
| `redo_v3` | `redo` | V3遗留 |
| `wasm_render_page_v3` | `wasm_render_page` | V3遗留 |
| `wasm_apply_document_patch_v3` | `wasm_apply_document_patch` | V3遗留 |

---

## 第五部分：功能→方法完整映射

### F1 文档生命周期 (60 methods)

**功能**: PDF文档的打开、关闭、保存、旋转、元数据读取  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `open_document` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `open_document_readonly` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `close_document` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `save_document` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `pick_file` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `get_metadata` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `probe_document_kind` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `release_document_cache` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `open_pdf` | Frontend | `pdf_window_api.ts` |
| `closePdf` | Frontend | `pdf_window_api.ts` |
| `pdfSave` | Frontend | `pdf_window_api.ts` |
| `pdfRotate` | Frontend | `pdf_window_api.ts` |
| `init_demo` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `create_demo_pdf` | Frontend | `main.ts` |
| `commit_document_edits` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `apply_region_patches` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `get_last_materialization_report` | Tauri Host | `interfaces/multimedia/pdf.rs` |

### F2 页面导航 (37 methods)

**功能**: 翻页、缩放、滚动、页面导航  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `next_page` | Frontend | `pdf_window_api.ts` |
| `prev_page` | Frontend | `pdf_window_api.ts` |
| `set_current_page` | Frontend | `viewer_session.ts` |
| `get_current_page` | Frontend | `viewer_session.ts` |
| `get_page_count` | Frontend | `pdf_runtime.ts` |
| `navigat` | Frontend | `router.ts` |
| `navigateTo` | Frontend | `router.ts` |
| `goBack` | Frontend | `router.ts` |
| `scroll` | Frontend | `pdf_window_api.ts` |
| `anchor_scroll` | Frontend | `pdf_runtime.ts` |

### F3 缩放控制 (85 methods)

**功能**: 缩放级别管理、平滑缩放、视口适配  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `apply_zoom` | WASM UI | `wasm_api/viewer.rs` |
| `resolve_zoom` | WASM UI | `wasm_api/viewer.rs` |
| `wheel_zoom` | WASM UI | `zoom/interaction.rs` |
| `clamp_zoom` | Frontend | `pdf_runtime.ts` |
| `dynamic_max_zoom` | Frontend | `frame_plan.ts` |
| `mark_rendered_zoom` | WASM UI | `zoom/state.rs` |
| `zoom_preview` | WASM UI | `zoom/preview_host.rs` |
| `step_zoom` | WASM UI | `zoom/interaction.rs` |
| `target_zoom` | Frontend | `zoom_controller.ts` |
| `set_target_zoom` | Frontend | `zoom_controller.ts` |
| `read_zoom_state` | Frontend | `zoom_controller.ts` |
| `sanitize_zoom_state` | Frontend | `zoom_controller.ts` |
| `sync_zoom_select` | Frontend | `pdf_runtime.ts` |
| `pdfZoomChange` | Frontend | `pdf_window_api.ts` |

### F4 渲染管线 (343 methods)

**功能**: Canvas绘制、帧计划、渐进渲染、缓存管理、瓦片渲染  
**方法清单** (部分示例):

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `render_page` | WASM UI | `render/canvas.rs` |
| `schedule_render_frame` | WASM UI | `render/scheduler.rs` |
| `commit_render_frame` | WASM UI | `render/scheduler.rs` |
| `settle_render_frame` | WASM UI | `render/scheduler.rs` |
| `begin_render_frame` | WASM UI | `render/scheduler.rs` |
| `abort_render_frame` | WASM UI | `render/scheduler.rs` |
| `progressive_render` | WASM UI | `render/progressive.rs` |
| `start_progressive_render` | WASM UI | `render/progressive_workflow.rs` |
| `step_progressive_render` | WASM UI | `render/progressive_workflow.rs` |
| `cancel_progressive_render` | WASM UI | `render/progressive_workflow.rs` |
| `advance_render_loop_frame` | WASM UI | `render/loop_workflow.rs` |
| `queue_render_loop_frame` | WASM UI | `render/loop_workflow.rs` |
| `viewport_refresh` | WASM UI | `viewport_refresh.rs` |
| `tile_cache` | WASM UI | `render/tile_cache.rs` |
| `frame_cache` | WASM UI | `render/frame_cache.rs` |

### F5 矢量提取 (25 methods)

**功能**: 从PDF提取矢量路径、文本对象、图片、布局推断  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `extract_vector` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `extract_layout` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `get_page_model` | Kernel | `document/page_region_context.rs` |
| `get_light_page` | Kernel | `document/page_region_context.rs` |
| `get_page_preview` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `prefetch_page_preview` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `extract_page_info` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `get_glyph_plan` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `get_images` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `build_page_region_context` | Kernel | `document/page_region_context.rs` |
| `page_classifier` | Kernel | `document/page_region_context.rs` |

### F6 文本编辑-核心编辑 (75 methods)

**功能**: 文本编辑器激活、输入处理、光标移动、段落打开/关闭  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `open_editor` | WASM UI | `editor/activation.rs` |
| `close_editor` | WASM UI | `editor/activation.rs` |
| `activate_editor_from_point` | Frontend | `editor_host.ts` |
| `move_caret` | WASM UI | `editor/navigation.rs` |
| `resolve_caret` | WASM UI | `editor/text_geometry.rs` |
| `caret_index` | WASM UI | `editor/text_index.rs` |
| `build_caret` | Frontend | `editor_host.ts` |
| `delete_forward` | Frontend | `editor_host.ts` |
| `delete_backward` | Frontend | `editor_host.ts` |
| `insert_text` | WASM UI | `editor/command.rs` |
| `apply_input` | WASM UI | `editor/command.rs` |
| `handle_active_editor_input` | Frontend | `editor_host.ts` |
| `open_paragraph_editor` | WASM UI | `editor/activation.rs` |
| `open_region_editor` | Frontend | `editor_host.ts` |
| `sync_editor_input` | WASM UI | `editor/runtime.rs` |
| `commit_editor` | WASM UI | `editor/commit.rs` |
| `finish_commit` | WASM UI | `editor/commit.rs` |
| `begin_commit` | WASM UI | `editor/commit.rs` |

### F7 文本编辑-格式化 (58 methods)

**功能**: 粗体、斜体、下划线、颜色、字体、字号、对齐、列表  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `toggle_bold` | Frontend | `pdf_window_api.ts` |
| `toggle_italic` | Frontend | `pdf_window_api.ts` |
| `toggle_underline` | Frontend | `pdf_window_api.ts` |
| `set_bold` | WASM UI | `editor/command.rs` |
| `set_italic` | WASM UI | `editor/command.rs` |
| `set_underline` | WASM UI | `editor/command.rs` |
| `is_bold` | WASM UI | `editor/document_plan.rs` |
| `is_italic` | WASM UI | `editor/document_plan.rs` |
| `is_underline` | WASM UI | `editor/document_plan.rs` |
| `set_color` | Frontend | `pdf_window_api.ts` |
| `set_font_size` | Frontend | `pdf_window_api.ts` |
| `set_font_family` | Frontend | `pdf_window_api.ts` |
| `set_alignment` | Frontend | `pdf_window_api.ts` |
| `set_list` | Frontend | `pdf_window_api.ts` |
| `set_char_spacing` | Frontend | `pdf_window_api.ts` |
| `set_line_height` | Frontend | `pdf_window_api.ts` |
| `apply_format` | Frontend | `editor_host.ts` |
| `format_action` | Frontend | `editor_wasm_api.ts` |
| `alignment_label` | Frontend | `pdf_window_api.ts` |
| `list_kind` | Frontend | `pdf_window_api.ts` |

### F8 文本编辑-草稿布局 (44 methods)

**功能**: 编辑草稿文本、重排布局、样式保持、段落到行拆分  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `build_draft` | Kernel | `text/editable_segments.rs` |
| `draft_layout` | WASM UI | `editor/draft_layout.rs` |
| `draft_text` | WASM UI | `editor/draft_layout.rs` |
| `build_source_layout` | WASM UI | `editor/source_geometry.rs` |
| `source_geometry` | WASM UI | `editor/source_geometry.rs` |
| `build_style_run` | Kernel | `document/page_region_context.rs` |
| `reindex_style` | Kernel | `document/page_region_context.rs` |
| `preserve_style` | Kernel | `text/style_preservation.rs` |
| `distribute_text` | Kernel | `text/style_preservation.rs` |
| `build_paragraph` | Kernel | `geometry/layout_engine.rs` |
| `layout_paragraph` | Kernel | `geometry/layout_engine.rs` |
| `finish_line` | Kernel | `geometry/layout_engine.rs` |
| `layout_anchored` | Kernel | `geometry/layout_engine.rs` |
| `build_editable_segment` | Kernel | `text/editable_segments.rs` |
| `split_run` | Kernel | `text/editable_segments.rs` |
| `build_text_plan` | WASM UI | `editor/document_plan.rs` |
| `glyph_layout` | Kernel | `text/glyph_layout.rs` |
| `should_insert_gap` | Kernel | `text/glyph_layout.rs` |
| `typical_advance` | Kernel | `text/glyph_layout.rs` |

### F9 文本编辑-替换补丁 (48 methods)

**功能**: 文本替换、区域补丁、持久化保存计划  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `apply_patch` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `build_patch` | WASM UI | `editor/replace_pipeline.rs` |
| `collect_patch` | WASM UI | `editor/replace_pipeline.rs` |
| `has_persistable` | WASM UI | `editor/replace_pipeline.rs` |
| `clear_persistable` | WASM UI | `editor/replace_pipeline.rs` |
| `save_persistable` | WASM UI | `editor/replace_pipeline.rs` |
| `build_save_plan` | WASM UI | `editor/replace_pipeline.rs` |
| `apply_replace` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `apply_batch_replace` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `replace_region` | WASM UI | `editor/replacement_region.rs` |
| `replace_match` | WASM UI | `editor/replacement_region.rs` |
| `build_replacement` | WASM UI | `editor/replacement_region.rs` |
| `replacement_region` | WASM UI | `editor/replacement_region.rs` |
| `replacement_snapshot` | WASM UI | `editor/replacement_snapshot.rs` |
| `patch_is_noop` | WASM UI | `editor/replace_pipeline.rs` |
| `current_paragraph_patch` | WASM UI | `editor/replace_pipeline.rs` |
| `collect_reflow` | Kernel | `persistence/models.rs` |
| `build_persistable` | WASM UI | `editor/replace_pipeline.rs` |
| `collect_legacy` | WASM UI | `editor/replace_pipeline.rs` |

### F10 搜索替换 (28 methods)

**功能**: 文档内搜索、替换、批量替换、结果导航  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `search_page_regions` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `search_document_regions` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `find_session` | Frontend | `pdf_find_controller.ts` |
| `clear_find_session` | Frontend | `pdf_find_controller.ts` |
| `set_find_session` | Frontend | `pdf_find_controller.ts` |
| `find_next` | Frontend | `pdf_find_controller.ts` |
| `find_previous` | Frontend | `pdf_find_controller.ts` |
| `move_match` | Frontend | `pdf_find_controller.ts` |
| `update_find_scope` | Frontend | `pdf_find_controller.ts` |
| `slice_chars` | Frontend | `pdf_find_controller.ts` |
| `collect_match` | Frontend | `pdf_find_controller.ts` |
| `matches_query` | Frontend | `pdf_find_controller.ts` |
| `get_find_scope` | Frontend | `pdf_find_controller.ts` |
| `empty_result` | Frontend | `pdf_find_controller.ts` |
| `build_replace` | Frontend | `pdf_find_controller.ts` |

### F11 批注-高亮 (16 methods)

**功能**: 文本高亮、高亮管理、颜色设置  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `extract_highlights` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `apply_highlight` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `add_highlight` | WASM UI | `document/comment.rs` |
| `list_highlight` | WASM UI | `document/comment.rs` |
| `delete_highlight` | WASM UI | `document/comment.rs` |
| `remove_highlight` | WASM UI | `document/comment.rs` |
| `region_highlight` | WASM UI | `document/comment.rs` |
| `persist_highlight` | WASM UI | `document/comment.rs` |

### F12 批注-评论 (60 methods)

**功能**: 文本评论、评论列表、评论更新、评论审阅  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `extract_comments` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `apply_comment` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `apply_comment_update` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `extract_comment_review` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `add_comment` | Frontend | `pdf_comment_controller.ts` |
| `update_comment` | Frontend | `pdf_comment_controller.ts` |
| `delete_comment` | Frontend | `pdf_comment_controller.ts` |
| `load_comments` | Frontend | `pdf_comment_controller.ts` |
| `get_comment_review` | Frontend | `pdf_comment_controller.ts` |
| `set_comment_review` | Frontend | `pdf_comment_controller.ts` |
| `replace_comment` | Frontend | `pdf_comment_controller.ts` |
| `clear_comment_session` | Frontend | `pdf_comment_controller.ts` |
| `build_comment` | Frontend | `pdf_comment_wasm_bridge.ts` |
| `toggle_comment_review_panel` | Frontend | `pdf_window_api.ts` |
| `accept_review_change` | WASM UI | `document/review.rs` |
| `reject_review_change` | WASM UI | `document/review.rs` |
| `accept_all_review_changes` | WASM UI | `document/review.rs` |

### F13 批注-标注目标 (28 methods)

**功能**: 批注目标检测、标注区域、交互目标渲染  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `extract_annotation_targets` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `delete_page_annotation` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `annotation_target` | WASM UI | `document/comment.rs` |
| `collect_target` | WASM UI | `document/comment.rs` |
| `list_annotation` | WASM UI | `document/comment.rs` |
| `extract_annotation` | WASM UI | `document/comment.rs` |
| `apply_annotation` | WASM UI | `document/comment.rs` |
| `page_annotation` | Tauri Host | `application/pdf/page_annotation.rs` |
| `build_target` | Frontend | `pdf_comment_host_actions.ts` |
| `collect_interaction` | Frontend | `editor_host_view.ts` |
| `render_target` | Frontend | `editor_host_view.ts` |
| `add_region_comment` | Tauri Host | `interfaces/multimedia/pdf.rs` |

### F14 AI辅助 (28 methods)

**功能**: AI建议生成、差异预览、一键应用、对话管理  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `apply_suggestion` | Frontend | `resume_ai_controller.ts` |
| `build_diff` | Frontend | `resume_ai_controller.ts` |
| `clear_ai_session` | Frontend | `resume_ai_controller.ts` |
| `describe_error` | Frontend | `resume_ai_controller.ts` |
| `mark_ai_changes` | Frontend | `resume_ai_controller.ts` |
| `plan_ai_edit` | Frontend | `resume_ai_controller.ts` |
| `submit_prompt` | Frontend | `resume_ai_controller.ts` |
| `sync_viewer_state` | Frontend | `resume_ai_controller.ts` |
| `tokenize_diff` | Frontend | `resume_ai_controller.ts` |
| `update_suggestion` | Frontend | `resume_ai_controller.ts` |
| `create_message_bubble` | Frontend | `pdf_window_api.ts` |
| `create_suggestion_card` | Frontend | `pdf_window_api.ts` |
| `build_suggested` | Frontend | `pdf_window_api.ts` |
| `pdf_summarize` | Frontend | `main.ts` |
| `toggle_ai_assistant` | Frontend | `main.ts` |

### F15 字体排版 (199 methods)

**功能**: 字体解析、字体匹配、字形布局、字体嵌入、编码  
**方法清单** (部分):

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `resolve_font` | Kernel | `typography/font_resolver.rs` |
| `match_font` | Kernel | `typography/matcher.rs` |
| `font_resolver` | Kernel | `typography/font_resolver.rs` |
| `font_matching` | Kernel | `typography/matcher.rs` |
| `font_metrics` | Kernel | `typography/matcher.rs` |
| `glyph_layout` | Kernel | `text/glyph_layout.rs` |
| `glyph_count` | Kernel | `text/glyph_layout.rs` |
| `build_glyph` | Kernel | `text/glyph_layout.rs` |
| `resolve_glyph` | Kernel | `text/glyph_layout.rs` |
| `classify_font` | Kernel | `typography/font_resolver.rs` |
| `looks_like_font` | Kernel | `typography/font_resolver.rs` |
| `encode_text` | Kernel | `typography/font_resolver.rs` |
| `can_encode` | Kernel | `typography/font_resolver.rs` |
| `build_cmap` | Kernel | `typography/font_resolver.rs` |
| `build_ttf` | Kernel | `typography/font_resolver.rs` |
| `font_family` | Kernel | `typography/font_resolver.rs` |
| `font_program` | Kernel | `typography/font_resolver.rs` |
| `embedded_font` | Kernel | `typography/font_resolver.rs` |
| `sfnt` | Kernel | `typography/font_resolver.rs` |
| `typography` | Kernel | `typography/font_resolver.rs` |
| `font_catalog` | Kernel | `typography/font_resolver.rs` |
| `font_face` | Kernel | `typography/font_resolver.rs` |
| `render_family` | Kernel | `typography/font_resolver.rs` |
| `split_family` | Kernel | `typography/font_resolver.rs` |
| `strip_subset` | Kernel | `typography/font_resolver.rs` |
| `get_text_width` | Kernel | `typography/font_resolver.rs` |
| `break_text` | Kernel | `typography/font_resolver.rs` |
| `parse_font` | Kernel | `typography/font_resolver.rs` |
| `read_cmap` | Kernel | `typography/font_resolver.rs` |

### F16 PDF读写内核 (21 methods)

**功能**: PDF结构解析、内容流解析、文本提取、路径提取  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `pdf_read` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `pdf_write` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `extract_text` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `parse_content` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `extract_path` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `extract_stream` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `load_document` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `open_pdf` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `get_page` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `read_page` | Kernel | `infrastructure/multimedia/pdf/pdf_read.rs` |
| `apply_reflow` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `patch_text` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `materialize` | Kernel | `infrastructure/multimedia/pdf/region_materializer.rs` |
| `build_materialization` | Kernel | `infrastructure/multimedia/pdf/region_materializer.rs` |
| `apply_atomic` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `apply_batch` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `write_font` | Kernel | `infrastructure/multimedia/pdf/pdf_write_font_resolver.rs` |
| `build_width` | Kernel | `infrastructure/multimedia/pdf/pdf_write_font_resolver.rs` |
| `update_metadata` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `insert_page` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `delete_page` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `rotate_page` | Kernel | `infrastructure/multimedia/pdf/pdf_write.rs` |
| `save_engine` | Kernel | `infrastructure/multimedia/pdf/save_engine.rs` |
| `apply_pdf_command` | Kernel | `infrastructure/multimedia/pdf/engine.rs` |
| `build_reflow` | Kernel | `infrastructure/multimedia/pdf/reflow_engine.rs` |
| `collect_reflow` | Kernel | `persistence/models.rs` |
| `rebuild_text` | Kernel | `infrastructure/multimedia/pdf/reflow_engine.rs` |
| `snapshot_line` | Kernel | `infrastructure/multimedia/pdf/reflow_engine.rs` |
| `combine_text` | Kernel | `infrastructure/multimedia/pdf/reflow_engine.rs` |
| `merge_reflow` | Kernel | `infrastructure/multimedia/pdf/reflow_engine.rs` |
| `qualifies_as_scanned` | Kernel | `infrastructure/multimedia/pdf/page_classifier.rs` |

### F17 布局分析 (17 methods)

**功能**: 段落检测、栏检测、语义区域、列表识别、字段识别  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `detect_layout` | Kernel | `analysis/analyzer.rs` |
| `resolve_region` | Kernel | `analysis/analyzer.rs` |
| `create_region` | Kernel | `analysis/analyzer.rs` |
| `build_region` | Kernel | `analysis/analyzer.rs` |
| `analyze` | Kernel | `analysis/analyzer.rs` |
| `detect_column` | Kernel | `analysis/analyzer.rs` |
| `semantic` | Kernel | `analysis/analyzer.rs` |
| `infer_scene` | Kernel | `document/page_region_context.rs` |
| `build_paragraph` | Kernel | `document/page_region_context.rs` |
| `build_line` | Kernel | `document/page_region_context.rs` |
| `list_item` | Kernel | `document/list_item_region_builder.rs` |
| `field_row` | Kernel | `document/page_region_context.rs` |
| `split_run` | Kernel | `document/page_region_context.rs` |
| `merge_paragraph` | Kernel | `document/page_region_context.rs` |
| `should_merge` | Kernel | `document/page_region_context.rs` |
| `is_standalone` | Kernel | `document/page_region_context.rs` |
| `build_style_source` | Kernel | `document/page_region_context.rs` |
| `list_like` | Kernel | `text/list_semantics.rs` |
| `group_line` | Kernel | `document/page_region_context.rs` |
| `derive_list` | Kernel | `text/list_semantics.rs` |

### F18 坐标几何 (65 methods)

**功能**: PDF坐标↔屏幕坐标、缩放、视口、包围盒计算  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `coordinate_transform` | Kernel | `geometry/coordinate_transform.rs` |
| `transform` | Kernel | `geometry/coordinate_transform.rs` |
| `project` | Kernel | `geometry/field_projection.rs` |
| `client_to` | Frontend | `geometry_probe.ts` |
| `denormalize` | Kernel | `geometry/coordinate_transform.rs` |
| `normalize` | Kernel | `geometry/coordinate_transform.rs` |
| `point_from` | Kernel | `geometry/coordinate_transform.rs` |
| `baseline_y` | Kernel | `geometry/coordinate_transform.rs` |
| `scale` | Kernel | `geometry/coordinate_transform.rs` |
| `flip_y` | Kernel | `geometry/coordinate_transform.rs` |
| `positive_ratio` | Kernel | `geometry/coordinate_transform.rs` |
| `layout_engine` | Kernel | `geometry/layout_engine.rs` |
| `reflow_engine` | Kernel | `geometry/reflow_engine.rs` |
| `calculate_displacement` | Kernel | `geometry/reflow_engine.rs` |
| `reflow_displacement` | Kernel | `geometry/reflow_engine.rs` |
| `build_projection` | Kernel | `geometry/field_projection.rs` |
| `resolve_projection` | Frontend | `geometry_probe.ts` |
| `field_projection` | Kernel | `geometry/field_projection.rs` |
| `convert_point` | Frontend | `geometry_probe.ts` |
| `measure_dom` | Frontend | `geometry_probe.ts` |
| `client_point` | Frontend | `geometry_probe.ts` |
| `page_point` | Frontend | `geometry_probe.ts` |
| `local_box` | Frontend | `editor_host_view.ts` |

### F19 BBox视口裁剪 (60 methods)

**功能**: BBox计算、视口相交检测、裁剪决策  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `bbox_height` | Kernel | `geometry/coordinate_transform.rs` |
| `bbox_width` | Kernel | `geometry/coordinate_transform.rs` |
| `bbox_intersects` | Kernel | `geometry/coordinate_transform.rs` |
| `viewport_cull` | WASM UI | `viewport_culling.rs` |
| `region_intersect` | Kernel | `render/paint_plan.rs` |
| `glyph_intersect` | Kernel | `render/paint_plan.rs` |
| `paragraph_intersect` | Kernel | `render/paint_plan.rs` |
| `path_bbox` | Kernel | `render/paint_plan.rs` |
| `object_bbox` | Kernel | `render/paint_plan.rs` |
| `resolve_viewport` | Kernel | `render/paint_plan.rs` |
| `viewport_bbox` | Kernel | `render/paint_plan.rs` |
| `detect_intersection` | Kernel | `render/paint_plan.rs` |
| `visible_content` | Kernel | `render/paint_plan.rs` |
| `compute_visible` | Kernel | `render/paint_plan.rs` |
| `cull` | WASM UI | `viewport_culling.rs` |
| `overscan` | WASM UI | `viewport_culling.rs` |

### F20 历史撤销重做 (36 methods)

**功能**: 操作历史栈、撤销、重做、命令模式  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `history` | Kernel | `persistence/history_manager.rs` |
| `undo` | Frontend | `pdf_window_api.ts` |
| `redo` | Frontend | `pdf_window_api.ts` |
| `undo_document` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `redo_document` | Tauri Host | `interfaces/multimedia/pdf.rs` |
| `clear_history` | Kernel | `persistence/history_manager.rs` |
| `push_command` | Kernel | `persistence/history_manager.rs` |
| `execute` | Kernel | `persistence/history_manager.rs` |
| `command` | Kernel | `persistence/history_manager.rs` |
| `remember` | Kernel | `persistence/history_manager.rs` |
| `bump_revision` | Kernel | `persistence/history_manager.rs` |
| `current_revision` | Kernel | `persistence/history_manager.rs` |
| `rollback` | Kernel | `persistence/history_manager.rs` |

### F21 状态会话管理 (58 methods)

**功能**: 应用状态、会话状态、查看器状态、缩放状态  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `state_manager` | Kernel | `persistence/state_manager.rs` |
| `session` | Frontend | `viewer_session.ts` |
| `get_session` | Frontend | `viewer_session.ts` |
| `set_document` | Frontend | `viewer_session.ts` |
| `reset_session` | Frontend | `viewer_session.ts` |
| `set_page` | Frontend | `viewer_session.ts` |
| `set_zoom` | Frontend | `viewer_session.ts` |
| `note_mutation` | WASM UI | `state_manager.rs` |
| `get_viewer` | Frontend | `viewer_session.ts` |
| `set_viewer` | Frontend | `viewer_session.ts` |
| `current_revision` | Frontend | `viewer_session.ts` |
| `sanitize_zoom` | Frontend | `zoom_controller.ts` |
| `reset_viewer` | Frontend | `viewer_session.ts` |
| `set_page_dimensions` | Frontend | `viewer_session.ts` |
| `get_viewer_session` | Frontend | `pdf_runtime.ts` |
| `reset_zoom` | Frontend | `pdf_runtime.ts` |
| `reset_state` | Frontend | `viewer_session.ts` |
| `get_state` | Frontend | `viewer_session.ts` |
| `set_state` | Frontend | `viewer_session.ts` |

### F22 WASM导出 (29 methods)

**功能**: WASM模块导出、JS绑定、初始化  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `initSync` | WASM UI | `wasm_api/document.rs` |
| `__wbg_` | WASM UI | `wasm_api/` (多个) |
| `render_run_standalone` | WASM UI | `wasm_api/` |
| `loadWasm` | Frontend | `wasm_loader.ts` |
| `loadWasmWithProgress` | Frontend | `wasm_loader.ts` |
| `ensureWasmInitialized` | Frontend | `pdf_runtime.ts` |
| `getWasmApi` | Frontend | `pdf_runtime.ts` |

### F23 DOM与UI (28 methods)

**功能**: DOM操作、事件绑定、UI状态同步、CSS操作  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `dom_projection` | WASM UI | `dom_projection.rs` |
| `bind_event` | Frontend | `pdf_window_api.ts` |
| `ensure_node` | Frontend | `editor_host_view.ts` |
| `position` | Frontend | `editor_host_view.ts` |
| `show_wrapper` | Frontend | `pdf_window_api.ts` |
| `hide_` | Frontend | `pdf_window_api.ts` |
| `sync_button` | Frontend | `pdf_window_api.ts` |
| `sync_select` | Frontend | `pdf_window_api.ts` |
| `getElement` | Frontend | `pdf_window_api.ts` |
| `append` | Frontend | `pdf_window_api.ts` |
| `render_overlay` | Frontend | `editor_host_view.ts` |
| `scroll_into` | Frontend | `pdf_window_api.ts` |
| `focus` | Frontend | `editor_host.ts` |
| `blur` | Frontend | `editor_host.ts` |
| `getNodes` | Frontend | `pdf_window_api.ts` |
| `colorToCss` | Frontend | `utils.ts` |
| `escape` | Frontend | `utils.ts` |
| `waitForAnimation` | Frontend | `pdf_window_api.ts` |

### F24 事件输入 (8 methods)

**功能**: 键盘快捷键、鼠标事件、滚轮事件、文本输入  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `keyboard` | Frontend | `main.ts` |
| `handle_key` | Frontend | `pdf_runtime.ts` |
| `handle_wheel` | Frontend | `pdf_runtime.ts` |
| `handle_mouse` | Frontend | `pdf_runtime.ts` |
| `bind_wheel` | Frontend | `pdf_runtime.ts` |
| `bind_zoom` | Frontend | `pdf_runtime.ts` |
| `on_cancel` | Frontend | `pdf_window_api.ts` |
| `on_commit` | Frontend | `pdf_window_api.ts` |
| `on_input` | Frontend | `pdf_window_api.ts` |
| `on_open` | Frontend | `pdf_window_api.ts` |
| `on_debug` | Frontend | `pdf_window_api.ts` |
| `handle_zoom` | Frontend | `pdf_runtime.ts` |
| `handle_render` | Frontend | `pdf_runtime.ts` |

### F25 调试诊断 (33 methods)

**功能**: 日志、追踪、诊断信息、性能监控  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `diagnostic` | Kernel | `render/paint_plan.rs` |
| `trace` | Frontend | `layout_trace.ts` |
| `log` | Frontend | `layout_trace.ts` |
| `emit_diagnostic` | Frontend | `diagnostics.ts` |
| `format_trace` | Frontend | `layout_trace.ts` |
| `format_diagnostic` | Frontend | `diagnostics.ts` |
| `compactString` | Frontend | `utils.ts` |
| `compactValue` | Frontend | `utils.ts` |
| `verbose` | Frontend | `diagnostics.ts` |
| `flush_trace` | Frontend | `layout_trace.ts` |
| `logNode` | Frontend | `diagnostics.ts` |
| `debug_trace` | Frontend | `diagnostics.ts` |
| `record_debug` | Frontend | `diagnostics.ts` |
| `resolve_diagnostic` | Frontend | `diagnostics.ts` |
| `enqueue_log` | Frontend | `diagnostics.ts` |
| `format_structured` | Frontend | `diagnostics.ts` |
| `format_terminal` | Frontend | `utils.ts` |
| `stringify_terminal` | Frontend | `utils.ts` |
| `formatDetails` | Frontend | `diagnostics.ts` |
| `logPdf` | Frontend | `layout_trace.ts` |
| `logEdit` | Frontend | `layout_trace.ts` |
| `logRender` | Frontend | `layout_trace.ts` |

### F26 插件基础设施 (9 methods)

**功能**: 插件加载、事件总线、路由、窗口API、模板  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `plugin` | Frontend | `plugin-loader.ts` |
| `eventBus` | Frontend | `event-bus.ts` |
| `router` | Frontend | `router.ts` |
| `registerRoute` | Frontend | `router.ts` |
| `navigateTo` | Frontend | `router.ts` |
| `getPlatform` | Frontend | `platform.ts` |
| `isTauri` | Frontend | `platform.ts` |
| `getTauri` | Frontend | `platform.ts` |
| `windowAction` | Frontend | `window-manager.ts` |
| `registerWindow` | Frontend | `window-manager.ts` |
| `unregister` | Frontend | `window-manager.ts` |
| `template` | Frontend | `template-loader.ts` |
| `loadComponent` | Frontend | `template-loader.ts` |
| `inject_component` | Frontend | `interfaces.ts` |
| `replace_component` | Frontend | `interfaces.ts` |
| `algorithm` | Frontend | `algorithm-manager.ts` |
| `getNamespaced` | Frontend | `algorithm-manager.ts` |
| `getRegistry` | Frontend | `algorithm-manager.ts` |
| `resolveOptions` | Frontend | `algorithm-manager.ts` |

### F27 算法基础 (26 methods)

**功能**: 图算法、LCA、连通分量、通用数据结构  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `graph` | Kernel | `algorithms/graph.rs` |
| `add_edge` | Kernel | `algorithms/graph.rs` |
| `are_neighbors` | Kernel | `algorithms/graph.rs` |
| `build_adjacency` | Kernel | `algorithms/graph.rs` |
| `find_connected_component` | Kernel | `algorithms/graph.rs` |
| `find_path` | Kernel | `algorithms/graph.rs` |
| `lca` | Kernel | `algorithms/lca.rs` |
| `add_child` | Kernel | `algorithms/lca.rs` |
| `find_lca` | Kernel | `algorithms/lca.rs` |

### F28 工具辅助 (26 methods)

**功能**: 字符串处理、数值校验、通用工具  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `truncate` | Frontend | `utils/string-utils.ts` |
| `sanitize` | Frontend | `utils/` |
| `clamp` | Frontend | `utils/math-utils.ts` |
| `normalize` | Frontend | `utils/` |
| `current_time` | Frontend | `utils/date-utils.ts` |
| `measure_metrics` | Frontend | `utils/` |
| `apply_transform` | Frontend | `utils/` |
| `sync_size` | Frontend | `utils/` |
| `prepare_surface` | Frontend | `utils/` |
| `hex_to` | Frontend | `utils.ts` |
| `parse_color` | Frontend | `utils.ts` |
| `object_to` | Frontend | `utils.ts` |
| `operands_to` | Frontend | `utils.ts` |
| `multiply_matrices` | Frontend | `utils.ts` |
| `read_u16` | Frontend | `utils.ts` |
| `read_u32` | Frontend | `utils.ts` |
| `write_u16` | Frontend | `utils.ts` |
| `checksum` | Frontend | `utils.ts` |
| `align4` | Frontend | `utils.ts` |

### F29 UI组件系统 (新增)

**功能**: 管理所有UI组件的创建、渲染、更新、销毁  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `createDiagnosticPanel` | Frontend | `diagnostics.ts` |
| `createDiagnosticItem` | Frontend | `diagnostics.ts` |
| `createDiagnosticList` | Frontend | `diagnostics.ts` |
| `buildCommentOverlay` | Frontend | `pdf_comment_dom.ts` |
| `buildCommentOverlayView` | Frontend | `pdf_comment_overlay_view.ts` |
| `buildCommentReviewPanel` | Frontend | `pdf_comment_dom.ts` |
| `buildCommentReviewPanelView` | Frontend | `pdf_comment_review_view.ts` |
| `createMessageBubble` | Frontend | `pdf_window_api.ts` |
| `createSuggestionCard` | Frontend | `pdf_window_api.ts` |
| `buildHostAction` | Frontend | `pdf_comment_host_actions.ts` |
| `buildHostActionList` | Frontend | `pdf_comment_host_actions.ts` |

### F30 诊断面板 (归入F25)

**功能**: 诊断信息的UI展示和交互  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `showDiagnosticPanel` | Frontend | `diagnostics.ts` |
| `updateDiagnosticPanel` | Frontend | `diagnostics.ts` |
| `formatDiagnosticMessage` | Frontend | `diagnostics.ts` |
| `formatDiagnosticTime` | Frontend | `diagnostics.ts` |
| `getDiagnosticIcon` | Frontend | `diagnostics.ts` |
| `getDiagnosticLevel` | Frontend | `diagnostics.ts` |

### F31 通知系统 (新增)

**功能**: 消息气泡、建议卡片、通知提示的统一管理  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `createMessageBubble` | Frontend | `pdf_window_api.ts` |
| `createSuggestionCard` | Frontend | `pdf_window_api.ts` |
| `updateSuggestion` | Frontend | `resume_ai_controller.ts` |
| `describeError` | Frontend | `resume_ai_controller.ts` |

### F32 WASM基础设施 (新增)

**功能**: WASM模块加载、初始化、进度管理  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `loadWasm` | Frontend | `wasm_loader.ts` |
| `loadWasmWithProgress` | Frontend | `wasm_loader.ts` |
| `ensureWasmInitialized` | Frontend | `pdf_runtime.ts` |
| `getWasmApi` | Frontend | `pdf_runtime.ts` |
| `initSync` | WASM UI | `wasm_api/document.rs` |
| `__wbg_` | WASM UI | `wasm_api/` (多个) |

### F33 通用工具库 (新增)

**功能**: 字符串、数组、数学、日期等通用工具函数  
**方法清单**:

| 方法名 | 层级 | 文件 |
|--------|------|------|
| `capitalize` | Frontend | `utils/string-utils.ts` |
| `truncate` | Frontend | `utils/string-utils.ts` |
| `escapeHtml` | Frontend | `utils/string-utils.ts` |
| `unescapeHtml` | Frontend | `utils/string-utils.ts` |
| `formatBytes` | Frontend | `utils/string-utils.ts` |
| `chunk` | Frontend | `utils/array-utils.ts` |
| `flatten` | Frontend | `utils/array-utils.ts` |
| `unique` | Frontend | `utils/array-utils.ts` |
| `sortBy` | Frontend | `utils/array-utils.ts` |
| `groupBy` | Frontend | `utils/array-utils.ts` |
| `clamp` | Frontend | `utils/math-utils.ts` |
| `lerp` | Frontend | `utils/math-utils.ts` |
| `randomBetween` | Frontend | `utils/math-utils.ts` |
| `roundTo` | Frontend | `utils/math-utils.ts` |
| `toRadians` | Frontend | `utils/math-utils.ts` |
| `formatDate` | Frontend | `utils/date-utils.ts` |
| `formatDateTime` | Frontend | `utils/date-utils.ts` |
| `getCurrentTime` | Frontend | `utils/date-utils.ts` |
| `parseDate` | Frontend | `utils/date-utils.ts` |
| `timeAgo` | Frontend | `utils/date-utils.ts` |
| `colorToCss` | Frontend | `utils.ts` |
| `hexToRgb` | Frontend | `utils.ts` |
| `parseColor` | Frontend | `utils.ts` |
| `rgbToHex` | Frontend | `utils.ts` |

---

## 第六部分：重构优先级总结

| 优先级 | 任务数 | 预计工作量 | 关键影响 |
|--------|--------|-----------|---------|
| P0 | 4项 | 3-5天 | 消除技术债务，命名规范 |
| P1 | 5项 | 1-2周 | 消除重复，提升可维护性 |
| P2 | 4项 | 2-4周 | 模块拆分，架构优化 |
| P3 | 2项 | 1-2月 | 新功能域，长期架构 |

**总计**: 15项任务，预计 **2-3个月** 完成全部重构

---

## 第七部分：验证标准

### 编译验证
- [ ] 所有P0任务完成后，项目编译通过
- [ ] 所有P1任务完成后，项目编译通过
- [ ] 所有P2任务完成后，项目编译通过

### 功能验证
- [ ] 文档打开/关闭/保存功能正常
- [ ] 页面导航/缩放功能正常
- [ ] 文本编辑/格式化功能正常
- [ ] 搜索/替换功能正常
- [ ] 批注/评论功能正常
- [ ] AI辅助功能正常

### 性能验证
- [ ] 渲染性能无明显下降
- [ ] 内存使用无明显增长
- [ ] WASM加载时间无明显增加

### 代码质量验证
- [ ] 无编译警告
- [ ] 无重复代码（SonarQube检测）
- [ ] 方法命名符合规范
- [ ] 每个方法都有明确功能归属

---

**重构方案完成**  
请确认是否按此方案执行，或提出调整建议。
