# Sovereignty PDF Viewer - 重构执行计划

> 执行日期: 2026-05-03  
> 基于方案: `1:A 2:C 3:A 4:B 5:A 6:A 7:A 8:A`  
> 说明: 问题1的"A"指保留测试在原处（已有`#[test]`标记），重构期间不移动

---

## 执行策略

### 阶段划分
- **P0**: 消除技术债务（3-5天）
- **P1**: 短期整改（1-2周）
- **P2**: 中期优化（2-4周）
- **P3**: 长期架构（1-2月）

### 验证流程
每个P阶段完成后：
1. 运行 `cargo test` 确保所有测试通过
2. 运行 `cargo build` 确保编译通过
3. 手动验证核心功能（文档打开、编辑、保存）

---

## P0 - 立即执行（3-5天）

### Day 1: 删除V3桥接重复（21个方法）

**目标文件**: `crates/pdf-viewer-ui/pkg/pdf_viewer_ui.d.ts`

**删除清单**:
```typescript
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
1. 备份原文件
2. 删除所有 `*_v3` 方法声明
3. 搜索所有调用点，改为调用主方法名
4. 运行 `cargo build` 验证编译

### Day 2: 删除 `wasm_` 前缀（40+对重复）

**目标文件**: `crates/pdf-viewer-ui/src/wasm_api/`

**重命名清单**:
```rust
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
1. 使用 `#[wasm_bindgen]` 宏自动添加 `wasm_` 前缀到JS绑定
2. 重命名Rust方法名（去掉 `wasm_` 前缀）
3. 更新TypeScript调用点
4. 运行 `cargo build` 验证

### Day 3: 移除 `_and_schedule_render` 后缀（10个方法）

**目标文件**: `crates/pdf-viewer-ui/src/editor/`, `src/bridge/`

**重命名清单**:
```rust
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
2. 在每个方法内部自动调用 `schedule_render()`
3. 更新所有调用点
4. 运行 `cargo test` 验证渲染调度

### Day 4: 保留测试方法（12个方法）

**说明**: 保留在原处，确保有 `#[cfg(test)]` 包裹

**涉及的测试方法**:
```rust
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
1. 验证所有测试方法都有 `#[cfg(test)]` 包裹
2. 运行 `cargo test` 确保测试通过
3. 记录测试覆盖率

### Day 5: P0验证

**验证清单**:
- [ ] 所有V3方法已删除
- [ ] 所有 `wasm_` 前缀已移除
- [ ] 所有 `_and_schedule_render` 后缀已移除
- [ ] 所有测试通过
- [ ] 编译无警告
- [ ] 手动测试核心功能

---

## P1 - 短期整改（1-2周）

### Week 1: 合并渲染管线透传

**目标**: 消除 `present/facade.rs` ↔ `present/runtime.rs` 1:1 透传

**合并清单**:
```rust
schedule_render_frame → 保留runtime版本
settle_render_frame → 保留runtime版本
store_frame_cache_entry → 保留runtime版本
touch_frame_cache_entry → 保留runtime版本
reset_frame_cache → 保留runtime版本
resolve_viewport_refresh → 保留runtime版本
```

**执行步骤**:
1. 分析 facade.rs 中的所有透传方法
2. 更新调用点直接调用 runtime.rs
3. 删除 facade.rs 中的透传方法
4. 如果 facade.rs 无其他用途，删除文件
5. 运行测试验证渲染管线

### Week 2: 提取公共工具函数

**目标**: 创建工具模块，消除重复

**创建文件**:
- `geometry/bbox_utils.rs` - BBox相关工具
- `utils/sanitize.rs` - 数值校验工具
- `utils/debug.rs` - 调试工具
- `utils/text-utils.rs` - 文本工具

**移动方法**:
```rust
// bbox_utils.rs
bbox_height, bbox_width, bbox_intersects, union_bbox

// sanitize.rs
sanitize_non_negative, sanitize_positive, sanitize_zoom_state

// debug.rs
truncate_debug_text, truncate_for_log, compactString, compactValue

// text-utils.rs
chars_count, split_key_value_text, get_object_display_text
```

**执行步骤**:
1. 创建对应的工具模块
2. 移动方法到新模块
3. 更新所有引用
4. 运行测试验证

### Week 2: 删除Utils层PDF工具重复

**目标文件**: `utils/pdf-utils.ts`

**删除方法**:
```typescript
calculatePdfPageCount
extractPdfText
getPdfMetadata
isPdfFile
parsePdfPage
```

**执行步骤**:
1. 删除 `utils/pdf-utils.ts` 文件
2. 查找所有调用点
3. 改为调用 Tauri 命令
4. 验证功能正常

### Week 2: 拆分字体解析器

**目标文件**: `src-tauri/src/infrastructure/multimedia/pdf/pdf_write_font_resolver.rs`

**拆分方案**:
```
pdf_write_font_resolver.rs →
  ├─ font_resolve.rs (字体匹配逻辑)
  ├─ font_encode.rs (字体编码逻辑)
  └─ font_ttf.rs (TTF解析逻辑)
```

**执行步骤**:
1. 分析46个方法的职责归属
2. 创建3个新文件
3. 移动对应方法
4. 更新引用
5. 验证编译

### Week 2: 合并命令透传

**目标**: 合并 `execute_pdf_commands_v1` / `_inner`

**执行步骤**:
1. 合并两个方法为一个
2. 删除 `_v1` 后缀
3. 更新调用点
4. 验证功能

---

## P2 - 中期优化（2-4周）

### Week 3-4: 拆分editor模块

**目标**: 412 methods → 5个子模块

**拆分方案**:
```
editor/ →
  ├─ editor/source/ (source_identity, source_geometry, source_runs, source_text)
  ├─ editor/draft/ (draft_layout, edited_text_layout, source_layout)
  ├─ editor/session/ (session, engine_state, host_runtime)
  ├─ editor/overlay/ (paragraph_overlay, paragraph_scene, visual)
  └─ editor/format/ (list_format, style_preservation)
```

### Week 5-6: 拆分engine.rs

**目标**: 263 methods → 3 services

**拆分方案**:
```
engine.rs →
  ├─ pdf_read_service.rs (读取相关)
  ├─ pdf_write_service.rs (写入相关)
  └─ pdf_geometry_service.rs (几何相关)
```

### Week 7: 统一命名规范

**重命名规则**:
```rust
calculate_* → resolve_*
get_model_* → read_*
get_document_* → 去掉 document
```

### Week 8: 清理window.* API

**目标**: 42个全局函数 → `PdfViewerAPI` 接口

---

## P3 - 长期架构（1-2月）

### Month 1: 新增功能域模块化

**创建新模块结构**:
```
src/ui-components/
src/notification/
src/wasm-infrastructure/
src/utils/
```

### Month 2: 删除内部方法不当导出

**目标**: 22个内部方法改为 `pub(crate)`

---

## 验证标准

### 每个P阶段验证
- [ ] `cargo test` 通过
- [ ] `cargo build` 无警告
- [ ] 核心功能手动测试

### 最终验证
- [ ] 所有P0-P3任务完成
- [ ] 代码覆盖率不下降
- [ ] 性能无回归
- [ ] 文档更新

---

**执行开始**: 2026-05-03  
**预计完成**: 2026-08-03
