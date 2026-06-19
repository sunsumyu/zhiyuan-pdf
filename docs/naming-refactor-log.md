# 命名重构变更日志

> 重构日期：2026-06-19
> 基于审查报告：docs/naming-audit-full-report.md

---

## 一、edit 模块

### edit_target.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `make_edit_segment_target_id` | `build_segment_id` | Create类动词 |
| `edit_target_base_paragraph_id` | `get_base_paragraph_id` | Query类动词 + 去掉模块路径冗余 |
| `edit_target_segment_key` | `get_segment_key` | 同上 |
| `whole_session_target` | `create_from_session` | Create类动词 |

### replacement_snapshot.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `replacement_target_from_patch_snapshot` | `find_target` | Query类动词 + 去掉模块路径冗余 |
| `replacement_object_indices` | `collect_object_indices` | Query类动词 + 去掉模块路径冗余 |

### replacement_region.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `paragraph_replacement_region` | `build_region` | Create类动词 + 去掉模块路径冗余 |
| `preferred_source_bbox` | `resolve_preferred_bbox` | Resolve类动词 |

### draft_text_diff.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `build_source_to_runs_index_map` + `build_runs_to_source_index_map` | `build_index_map()` | 合并成对转换函数 |

---

## 二、text 模块

### index_convert.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `utf16_offset_to_char_index` | `CharIndex::to_char(text)` | newtype承载编码 |
| `char_index_to_utf16_offset` | `Utf16Offset::to_utf16(text)` | newtype承载编码 |

新增类型：
```rust
pub struct CharIndex(pub usize);
pub struct Utf16Offset(pub usize);
```

### glyph_layout.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `map_raw_to_reconstructed` | `to_reconstructed` | 去掉`map_`前缀 |
| `map_reconstructed_to_raw` | `to_raw` | 同上 |
| `reconstructed_char_count` | `char_count` | 去掉`reconstructed_`前缀 |

---

## 三、geometry 模块

### coordinate_transform.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `normalize_y` | `to_y_down` | 方向由方法名表达 |
| `denormalize_y` | `to_y_up` | 同上 |
| `client_to_page` | `to_page(clamp_box: Option)` | 合并功能变体 |
| `client_to_page_in_box` | 合入`to_page` | 合并功能变体 |
| `client_to_local_in_box` | `to_local` | 去掉`_in_box`后缀 |
| `point_from_pdf` | `project_point` | Transform类动词 |
| `x_from_pdf` | `project_x` | 同上 |
| `baseline_y_from_pdf` | `project_baseline_y` | 同上 |
| `baseline_y_from_anchor_relative` | `project_relative_y` | 同上 |

### source_geometry.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `source_session_visual_bbox` | `compute_session_bbox` | Resolve类动词 + 去掉冗余修饰词 |
| `source_visual_bbox_from_runs` | `compute_bbox_from_runs` | 同上 |
| `caret_line_bbox` | `compute_caret_line_bbox` | Resolve类动词 |
| `source_run_visual_bbox` | `compute_run_bbox` | 去掉冗余修饰词 |

---

## 四、render 模块

### viewport_culling.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `styled_run_bbox` | `run_bbox` | 去掉`styled_`冗余 |
| `path_object_bbox` | `path_bbox` | 去掉`object_`冗余 |

---

## 五、src-tauri 模块

### log_service.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `get_pdf_log_level` | `get_log_level` | 去掉`pdf_`模块路径冗余 |
| `set_pdf_log_level` | `set_log_level` | 同上 |
| `clear_pdf_event_log` | `clear_event_log` | 同上 |
| `read_pdf_event_log` | `read_event_log` | 同上 |
| `log_pdf_event` | `log_event` | 同上 |

### font/metrics.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `get_character_width_pdf_units` | `get_character_width` | 去掉`pdf_units`冗余（返回类型体现） |

### document_service.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `release_pdf_resources` | `release_resources` | 去掉`pdf_`模块路径冗余 |
| `release_all_pdf_resources` | `release_all_resources` | 同上 |
| `read_last_pdf_materialization_report` | `read_materialization_report` | 去掉冗余修饰词 |
| `load_pdf_public` | `load_public` | 去掉`pdf_`冗余 |
| `generate_demo_pdf` | `generate_demo` | 同上 |

### 其他文件

| 文件 | 原名 | 新名 |
|------|------|------|
| color_utils.rs | `parse_pdf_hex_color` | `parse_hex_color` |
| save_engine.rs | `apply_pdf_commands` | `apply_commands` |
| interfaces/pdf/system.rs | `create_demo_pdf` | `create_demo` |

---

## 六、pdf-viewer-ui 模块

### editor_api.rs

| 原名 | 新名 | 变更类型 |
|------|------|----------|
| `utf16_to_char_index`调用 | `Utf16Offset::to_char()` | 使用newtype |
| `char_to_utf16_offset`调用 | `CharIndex::to_utf16()` | 使用newtype |

---

## 统计

| 模块 | 变更函数数 | 主要模式 |
|------|------------|----------|
| edit | 11 | 去掉模块路径冗余 + 合并成对函数 |
| text | 6 | newtype + 去掉冗余前缀 |
| geometry | 12 | Transform动词 + 去掉冗余修饰词 |
| render | 2 | 去掉冗余修饰词 |
| src-tauri | ~20 | 去掉`pdf_`模块路径冗余 |
| pdf-viewer-ui | 2 | 使用newtype |

**总计：约 50+ 函数重命名**

---

## 验证步骤

```bash
# 1. 检查编译
cargo check --workspace

# 2. 运行测试
cargo test --workspace

# 3. 检查 WASM 构建
cargo build --package pdf-viewer-ui --target wasm32-unknown-unknown
```

---

## 设计原则总结

1. **模块路径已承载的词不加** — `infrastructure::pdf` 下不加 `pdf_`
2. **编码/空间域用 newtype 承载** — `CharIndex(5).to_utf16(text)`
3. **成对转换函数合并** — `build_index_map()` 返回双向映射
4. **功能变体用参数控制** — `to_page(clamp_box: Option)`
5. **`get_` 用于纯属性获取，`read_` 用于有计算成本的查询**