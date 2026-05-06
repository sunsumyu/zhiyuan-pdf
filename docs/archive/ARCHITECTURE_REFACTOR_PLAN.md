# Sovereignty PDF Viewer - 完整架构整改方案

> 制定日期: 2026-05-05  
> 审计范围: 2,029 个方法，204 个源文件，4 大层级  
> 原则: 每个方法必须对应一个明确功能，无功能的方法必须删除或合并  
> 设计原则: SOLID + 关注点分离 + 不过度设计

---

## 一、当前真实状态（截至本次审视）

| 项目 | 已完成 | 未完成 |
|------|--------|--------|
| **V3 后缀清理** | ✅ 0个`*_v3` 残留 | — |
| **`wasm_` 前缀** | ⚠️ 部分清理（导出名`js_name`仍含） | ~30+处 |
| **`_and_schedule_render` 后缀** | ❌ 几乎未清理 | 10处仍存在 |
| **Facade 层（Rust）** | ✅ 7个 facade 模块创建 | 部分仍是 stub |
| **Facade 层（TS）** | ✅ 7个 bridge 创建 | search/review 已接真实API |
| **测试外迁** | ❌ 12个`test_*` 仍在生产代码 | — |
| **`bbox_*` 工具合并** | ❌ 仍散落 7+ 文件 | — |
| **`sanitize_*` 工具合并** | ❌ 仍散落 3 文件 | — |
| **`pdf_write_font_resolver.rs`** | ❌ 46方法未拆 | — |
| **engine.rs 拆分** | ❌ 263方法未拆 | — |

**结论：之前的方案文档制定得很全，但执行只完成了大约 30%（主要是 facade 搭建，没真正消除技术债）。

---

## 二、命名问题的根因诊断

之前方案的命名为何"不合理"：

### 根因 1：动词同义词泛滥
- `resolve_` / `build_` / `compute_` / `extract_` 混用，没有清晰界定
- 同一行为在不同模块用不同动词

### 根因 2：副作用泄漏到方法名
- `_and_schedule_render` 把"内部调度"暴露成API合约 — 违反封装

### 根因 3：技术细节出现在业务名上
- `wasm_*`、`_v3`、`_inner`、`_from_app_state` — 调用方不该关心实现层

### 根因 4：名词冗余链
- `editor_input_command_editor_state_runtime_workflow` 这种 N 段连接词
- 没有"对象上下文 = 模块名"的纪律

---

## 三、新命名规范（重制定）

### 3.1 三段式约束：`<动词>_<对象>[_<限定>]?`

最多三段，对象名优先借用模块路径而非堆叠在方法名上。

### 3.2 动词词典（精简到 10 个，覆盖所有场景）

| 动词 | 语义 | 副作用 | 返回 |
|------|------|--------|------|
| `read_` | 纯查询 | 无 | 数据快照 |
| `find_` | 搜索匹配 | 无 | 集合 |
| `resolve_` | 计算/推导 | 无 | 计算结果 |
| `is_/has_/can_` | 谓词 | 无 | bool |
| `apply_` | 修改状态 | 有 | 操作摘要 |
| `set_` | 简单赋值 | 有 | 旧值/void |
| `clear_` | 清空 | 有 | void |
| `open_/close_` | 生命周期 | 有 | 句柄/void |
| `commit_` | 提交事务 | 有 | 事务结果 |
| `step_` | 推进帧/迭代 | 有 | 推进结果 |

**禁用词**：`get_`、`do_`、`process_`、`handle_`、`calculate_`、`build_`（除构造模式）、`make_`、`update_`（用 `apply_`）、`extract_`（用 `read_` 或 `resolve_`）

### 3.3 命名长度硬约束

- **公共 API**：≤ 3 个词（如 `apply_format`，`resolve_zoom`）
- **内部方法**：≤ 4 个词
- **超过 4 个词必须拆分模块**

---

## 四、按功能落地的完整重构清单

### Phase A：消除技术债（5 天）

#### A1. 删除 `_and_schedule_render` 副作用泄漏（10 处）

| 当前 | 新名 | 文件 |
|------|------|------|
| `apply_active_editor_format_action_and_schedule_render` | `apply_format` | `wasm_api/editor.rs:267` |
| `apply_editor_input_command_and_schedule_render` | `apply_input` | `wasm_api/editor.rs:169` |
| `apply_region_text_replacements_and_schedule_render` | `apply_replacements` | `wasm_api/editor.rs:207` |
| `close_active_editor_and_schedule_render` | `close_editor` | `wasm_api/editor.rs:188` |
| `open_paragraph_editor_at_client_point_and_schedule_render` | `open_editor_at_point` | `wasm_api/editor.rs:122` |
| `open_region_editor_and_schedule_render` | `open_region_editor` | `wasm_api/editor.rs:132` |
| `sync_active_editor_input_and_schedule_render` | `sync_input` | `wasm_api/editor.rs:247` |
| `sync_and_commit_active_editor_text_and_schedule_render` | `commit_editor` | `wasm_api/editor.rs:222` |

**实现要求**：方法内部自动调度渲染，不在签名中暴露。

#### A2. 消除 `wasm_` 前缀冗余（~30 处）

通过 `js_name` 属性自动加前缀，方法名本身去掉 `wasm_`：

```rust
// 之前：双倍命名 wasm_X / X
#[wasm_bindgen(js_name = "wasm_apply_format")]
pub fn wasm_apply_format(...) { ... }

// 之后：单一命名
#[wasm_bindgen(js_name = "applyFormat")]  // JS 端用 camelCase
pub fn apply_format(...) { ... }           // Rust 端用 snake_case
```

#### A3. 删除内部状态导出（22 处）

以下不该作为公开 WASM API 暴露，改为 `pub(crate)`：

```
wasm_get_wheel_render_pending      → 内部
wasm_set_wheel_render_pending      → 内部
wasm_peek_pending_anchor_layout    → 内部
wasm_peek_pending_anchor_scroll    → 内部
wasm_take_pending_anchor_layout    → 内部
wasm_take_pending_anchor_scroll    → 内部
wasm_take_ready_committed_frame    → 内部
wasm_queue_committed_frame         → 内部
wasm_queue_render_loop_frame       → 内部
wasm_advance_render_loop_frame     → 内部
wasm_is_render_frame_current       → 内部
wasm_mark_rendered_zoom            → 内部
wasm_store_frame_cache_entry       → 内部
wasm_touch_frame_cache_entry       → 内部
wasm_reset_frame_cache             → 内部
wasm_set_visual_layout             → 内部
wasm_editor_char_index_to_utf16_offset  → 移至 text_index.rs
wasm_editor_utf16_offset_to_char_index  → 移至 text_index.rs
wasm_convert_client_point_to_page_point → resolve_page_point
wasm_measure_dom_to_page_scale          → resolve_display_scale
wasm_step_preview_tick                  → step_preview
wasm_calculate_editor_projection        → resolve_projection
```

#### A4. 测试外迁（12 处）

将散落在 `src/` 的 `test_*` 移到 `tests/` 或保留 `#[cfg(test)]` 块——**不允许**裸露在生产模块中。

---

### Phase B：方法名重塑（1 周）

#### B1. Tauri Commands 全量重命名（按规范 3.2 / 3.3）

| 当前 | 新名 | 类别 |
|------|------|------|
| `open_document` | `open_pdf` | open |
| `open_document_readonly` | `read_pdf` | read |
| `probe_document_kind` | `probe_pdf` | read |
| `release_document_cache` | `clear_cache` | clear |
| `get_metadata` | `read_metadata` | read |
| `get_page_preview` | `read_preview` | read |
| `prefetch_page_preview` | `prefetch_preview` | read |
| `commit_document_edits` | `commit_edits` | commit |
| `apply_region_patches` | `apply_patches` | apply |
| `undo_document` | `undo` | apply |
| `redo_document` | `redo` | apply |
| `resolve_page_info` | `resolve_page_info` | ✅ 保留 |
| `resolve_vector_model` | `resolve_vector` | resolve |
| `search_page_regions` | `find_in_page` | find |
| `search_document_regions` | `find_in_document` | find |
| `resolve_annotation_targets` | `resolve_targets` | resolve |
| `resolve_highlights` | `read_highlights` | read |
| `resolve_comments` | `read_comments` | read |
| `resolve_comment_review` | `read_review` | read |
| `apply_batch_replace` | `apply_replace_all` | apply |
| `delete_page_annotation` | `delete_annotation` | clear |
| `get_last_materialization_report` | `read_last_report` | read |

**40+ 个 commands → 24 个本质命令**（合并冗余）。

#### B2. WASM Exports 全量重命名（核心精选）

| 当前 | 新名 |
|------|------|
| `wasm_init_page_context` | `init_page` |
| `wasm_navigate_next_page` | `next_page` |
| `wasm_navigate_prev_page` | `prev_page` |
| `wasm_resolve_render_zoom` | `resolve_zoom` |
| `wasm_start_progressive_render` | `start_render` |
| `wasm_cancel_progressive_render` | `cancel_render` |
| `wasm_step_progressive_render` | `step_render` |
| `wasm_step_preview_tick` | `step_preview` |
| `wasm_commit_render_frame` | `commit_frame` |
| `wasm_begin_render_frame` | `start_frame` |
| `wasm_abort_render_frame` | `abort_frame` |
| `wasm_apply_zoom_selection` | `apply_zoom` |
| `wasm_handle_wheel_zoom_host` | `apply_wheel` |
| `wasm_resolve_anchor_scroll` | `resolve_anchor` |
| `wasm_clear_pending_anchor` | `clear_anchor` |
| `wasm_clear_preview_present` | `clear_preview` |
| `wasm_close_document_pipeline` | `close_pdf` |
| `build_editable_segments` | `resolve_segments` |
| `build_page_region_context` | `resolve_regions` |
| `build_region_text_patch` | `build_patch` |
| `wasm_apply_document_patch` | `apply_patch` |
| `wasm_active_editor_has_session_changes` | `is_editor_dirty` |

#### B3. Facade 层精简

当前 7 个 facade：`facade.rs` / `search_facade.rs` / `review_facade.rs` / `ai_facade.rs` / `render_facade.rs` / `annotation_facade.rs`

**方案**：合并到 4 个清晰职责：

| 新模块 | 职责 | 接管 |
|--------|------|------|
| `editor_facade.rs` | 编辑器编排 | facade.rs（保留） |
| `find_facade.rs` | 搜索/替换/审阅（这三件本质都是"查找+操作"） | search/review 合并 |
| `annotation_facade.rs` | 高亮/评论/AI建议（都是页面标注） | annotation/ai 合并 |
| `render_facade.rs` | 渲染调度 | 保留 |

**合并理由**：search 和 review 在前端流程上高度耦合（都是 `查找 → 选中 → 应用变更`）。AI 建议本质是带建议的标注。

---

### Phase C：架构层模块拆分（2-4 周）

#### C1. 拆分 `editor/` (412 methods → 5 子模块)

```
editor/
├── source/        # 原文几何与文本 (source_identity/geometry/runs/text)
├── draft/         # 草稿布局 (draft_layout, edited_text_layout, source_layout)
├── session/       # 编辑会话 (session, engine_state)
├── overlay/       # 渲染叠加 (paragraph_overlay, paragraph_scene, visual)
└── format/        # 格式化 (list_format, style_preservation)
```

设计模式：**Strategy** (每个 sub-module 是一种编辑策略) + **State** (`session` 状态机)

#### C2. 拆分 `infrastructure/pdf/engine.rs` (263 methods → 3 services)

```
infrastructure/pdf/
├── read_service.rs      # 读取
├── write_service.rs     # 写入
└── geometry_service.rs  # 几何
```

设计模式：**Repository** + **Service Layer**

#### C3. 拆分 `pdf_write_font_resolver.rs` (46 methods → 3 文件)

```
infrastructure/pdf/font/
├── resolve.rs   # 字体匹配
├── encode.rs    # 字体编码
└── ttf.rs       # TTF 解析
```

#### C4. 提取公共工具

```
crates/pdf-viewer-core/src/
├── geometry/bbox_utils.rs   # bbox_height/width/intersects/union
├── utils/sanitize.rs        # sanitize_non_negative/positive
├── utils/debug.rs           # truncate_debug_text
└── text/index_convert.rs    # char_index ↔ utf16_offset
```

---

### Phase D：消除冗余前端 (1 周)

#### D1. 删除 Utils 层 PDF 工具重复

`utils/pdf-utils.ts` 中的 `calculatePdfPageCount`/`extractPdfText`/`getPdfMetadata`/`isPdfFile`/`parsePdfPage` 全部删除，统一调用 Tauri 命令。

#### D2. 收敛 `window.*` 全局函数

42 个 `window.pdf*` 改造为类型安全的 `PdfViewerAPI` 接口（已部分存在 `pdf_viewer_api.ts`）。

---

## 五、最终架构形态

```
┌────────────────────────────────────────────────────┐
│ TS Presentation (~80KB, 10 files)                  │
│ - 仅DOM事件、UI渲染、API调用                         │
│ - 无业务状态                                        │
└─────────────┬──────────────────────────────────────┘
              │ Tauri IPC + WASM Bridge
┌─────────────┴──────────────────────────────────────┐
│ WASM Application Facades (~50KB, 4 files)          │
│ editor_facade / find_facade / annotation_facade    │
│ render_facade                                      │
└─────────────┬──────────────────────────────────────┘
┌─────────────┴──────────────────────────────────────┐
│ Domain Layer (Rust, 5 模块)                        │
│ editor/{source,draft,session,overlay,format}       │
└─────────────┬──────────────────────────────────────┘
┌─────────────┴──────────────────────────────────────┐
│ Infrastructure (Rust, 3 services)                  │
│ pdf/{read,write,geometry}_service                  │
│ pdf/font/{resolve,encode,ttf}                      │
└────────────────────────────────────────────────────┘
```

设计模式应用：
- **Facade**: WASM facades 隔离前端
- **Repository + Service**: PDF infrastructure
- **State**: Editor session
- **Strategy**: Editor sub-modules
- **Command**: Tauri commands 即是 Command 模式

---

## 六、执行顺序与验收

| Phase | 工时 | 验收 |
|-------|------|------|
| **A** 消除技术债 | 5天 | `cargo build` 0 warning，10 个 `_and_schedule_render` 全清，22 个内部 API 不再导出 |
| **B** 命名重塑 | 1周 | 公共 API 名 ≤ 3 词，禁用词 0 出现 |
| **C** 模块拆分 | 2-4周 | 单文件 ≤ 80 methods，单模块 ≤ 200 methods |
| **D** 前端收敛 | 1周 | TS 体积下降 ≥ 60%，`window.*` 0 业务函数 |

---

## 七、关键决策建议

**建议从 Phase A1（消除 `_and_schedule_render`）开始** — 收益最大、风险最小，1天可完成，立刻让 10 个核心 API 命名清爽。

执行顺序：Phase A → B → C → D 逐步推进。
