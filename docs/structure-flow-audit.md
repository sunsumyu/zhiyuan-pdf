# 结构 · 流程 · 状态机 深度审计（Batch 2 / 3）

> **批次说明**：Batch 1（API 表面 + 死码）已交付 → 本文 Batch 2 → Batch 3（Nutrient 对标）。本文基于静态扫描 + 依赖图分析，不改代码。

## 总览

| 维度 | 数值 | 结论 |
|------|-----:|------|
| 顶层单向依赖（core→ui/tauri） | **0 违规** | ✅ 干净 |
| UI 层直接 use tauri | **0 处** | ✅ 完全通过 `target_invoke` 隧道 |
| 显式状态机（enum 驱动） | **1 个**（EditorSession） | ⚠️ 其他 Session 全是隐式状态 |
| thread_local 全局存储 | **13 个** | ⚠️ 存在跨域重名/错位 |
| Session 句柄尺寸 | 全部 **0-sized unit struct** | ✅ 统一模式 |
| 跨域引用总次数 | 152 处 | 热点：render↔editor |

---

## 1. Crate 级单向依赖审计

扫描结论：

| 检查项 | 结果 |
|-------|------|
| core 里是否 `use pdf_viewer_ui / tauri / wasm_bindgen / web_sys / js_sys` | **0 处** ✅ |
| ui 里是否 `use tauri` | **0 处** ✅ |
| Tauri 后端是否直接依赖 ui crate | **0 处** ✅ |

三层架构**编译期边界完整**。任何对 Tauri 命令的调用都走 `smart_invoke("cmd_name", args)` 字符串隧道，native/WASM 无类型共享。代价是**命令名没有 IDE 跳转**。

---

## 2. UI 层模块依赖图

耦合矩阵（>3 次引用的边）：

| 被引用 ← 引用方 | 次数 |
|--|--:|
| editor ← **render** | **22** 🔥 |
| present ← wasm_api | 9 |
| editor ← wasm_api | 8 |
| page ← editor | 7 |
| document ← editor | 6 |
| present ← editor | 6 |
| render ← present | 6 |
| host ← wasm_api | 6 |
| editor ← document | 4 |
| viewer ← editor | 4 |
| zoom ← editor | 4 |

**叶子模块（0 入度）**：`find` / `review` / `annotation` / `comment` / `history`。这 5 个新 Session 域没被其他域侵入，边界清晰；但也意味着**它们当前仍是独立王国，与核心编辑/渲染流未打通**（印证 Batch 1 中这 5 个域的大量 NotImplemented stub）。

### 2.1 三大热点

**热点 1 · `render → editor` 22 次（最高耦合边）**

拆解这 22 次的具体符号：

| 符号 | 次数 | 性质 |
|------|---:|------|
| `editor::debug_trace::editor_debug_field` | 5 | 调试基础设施（横切） |
| `editor::debug_trace::record_editor_debug_event` | 3 | 调试基础设施（横切） |
| `editor::debug_trace`（模块级） | 2 | 调试基础设施（横切） |
| `editor::draft_layout::*` | 3 | overlay 几何（合理） |
| `editor::replacement_region::*` | 2 | overlay 绘制（合理） |
| `editor::paragraph_overlay::*` | 3 | overlay 绘制（合理） |
| `editor::session::ActiveEditorTarget` | 1 | 绘制 caret 需要（合理） |
| `editor::mode::get_active_editor_state` | 1 | 模式查询（合理） |
| `editor::text_geometry::measure_*` | 2 | 文本测量（可考虑下沉 core） |

**判定**：22 次中 **11 次是 debug_trace 横切依赖**，属于假性耦合。真实业务耦合只有 11 次，是"渲染 overlay 必须知道编辑态"的合理需求。

**行动项 §1**：把 `editor/edit_chain_trace.rs` → `utils/chain_trace.rs`，一次性消除 11/22 的假耦合。

**热点 2 · `editor` 最"贪心"（出度 31，涉及 9 模块）**

editor 引用了 document / page / present / render / utils / viewer / zoom 7 个域。这说明 editor **不只是文字编辑模块，它在做隐式的应用层编排**——翻页、缩放、渲染都要由它触发。

**行动项 §2**：把 editor 里的跨域调度函数（如 `renderCurrentPage` 的 Rust 侧对应者）提取为 `editor/orchestrator.rs`，诚实声明 orchestrator 身份。

**热点 3 · `wasm_api → 9 模块`（legacy facade 尚在自引）**

虽然 TS 不再调 `wasm_api/`，但 wasm_api 自己还在 present(9) / editor(8) / host(6) / zoom / viewer 各处抓东西。这些**内部自引用**必须先切断，才能推进 Batch 1 · A 的目录删除。

**行动项 §3**：在 Batch 1 · A 推进前，先做 wasm_api 内部引用拆除预处理。

---

## 3. Session 状态机盘点

### 3.1 全项目唯一的显式状态机：EditorSession

源文件 `crates/pdf-viewer-core/src/edit/editor_types.rs` 的 `SessionState` 枚举：

| 状态 | 语义 | 允许转入 |
|------|------|---------|
| `Viewing` | 只读浏览 | Editing（via begin） |
| `Editing` | 已进入编辑模式，未选中块 | EditingBlock / Saving / Viewing |
| `EditingBlock` | 选中具体块正在编辑 | Editing / Saving / Viewing |
| `Saving` | 提交补丁落盘中 | Viewing（成功）/ Editing（失败回退） |

- 状态驻留：`crates/pdf-viewer-ui/src/editor/editor_store.rs::SESSION_STATE: Cell<SessionState>`
- 转移封装：`guard_state!` 宏 + 显式 `transition_to(state)` 函数
- 观察者：`on_state_change` 回调（wasm32 cfg 守护）

**这是本项目最成熟的状态设计**。所有其他 Session 应以此为模板。

### 3.2 其他 8 个 Session 全是隐式状态

| Session | 零尺寸 struct | 状态驻留 thread_local | 字段承载的隐式状态 |
|---------|:-:|------|------|
| DocumentSession | ✓ | — | 无状态（纯命令代理） |
| ViewerSession | ✓ | `viewer::viewer_store::HOST_VIEWER_SESSION` | path / page / zoom / page_dims |
| FindSession | ✓ | `find::find_store::CONTROLLER` + `viewer::find_store::HOST_FIND_SESSION` ⚠️ | query / matches / active_index |
| ReviewSession | ✓ | `viewer::review_store::HOST_COMMENT_REVIEW_SESSION` ⚠️ | panel_open / scope / query |
| AnnotationManager | ✓ | — | 无状态 |
| CommentManager | ✓ | 共用 `viewer::review_store` ⚠️ | 共用 review session |
| HistoryController | ✓ | `pdf_viewer_core::history`（core 内部） | undo/redo depths |
| ZoomController | ✓ | `zoom::zoom_store::HOST_ZOOM_STATE` | current / target / pending_anchor |
| RenderPipeline | ✓ | `render::render_store::HOST_RENDER_STATE` + `render::host_runtime::HOST_RENDER_LOOP_STATE` | 渐进帧状态 |

**统一模式**：所有 Session 都是**零尺寸 unit struct** + thread_local 存真正的状态。WASM handle 本身不持状态，好处是 JS 侧可随意 new 多份都指向同一份逻辑状态；坏处是**不支持多文档并存**（除非重构 thread_local 为 `HashMap<DocumentId, State>`）。

### 3.3 隐式状态机的可推断状态转移

每个"零状态字段"实际上都有隐含语义转移。例如 ZoomController：

```
Idle (current = target) --setTarget--> Animating (current != target)
Animating --markRendered--> Idle
Animating --resolveWheelZoom(bigDelta)--> Animating (new target)
```

**问题**：这种"通过字段变化推断状态"的做法：
- 对单步调试友好（看字段值就知道）
- 对多人维护不友好（没有合法性约束，非法组合可能悄悄发生）
- 对未来 UI 不友好（无法订阅"进入/离开某状态"事件）

**行动项 §4**：给 Viewer / Zoom / Find / Review 各加显式 `enum State`，参照 EditorSession 模式。优先级：Zoom（动画状态清晰）> Find（有 session / no session 二态）> Viewer / Review。

---

## 4. thread_local 全局存储审计（13 个）

### 4.1 清单与归属

| # | 文件 | 名称 | 归属域 | 是否合理 |
|---|------|------|------|:---:|
| 1 | `editor/edit_chain_trace.rs` | `CHAIN_ENABLED` | editor ❌ 横切 | 应迁到 utils |
| 2 | `editor/editor_store.rs` | `SESSION_STATE` | editor | ✅ |
| 3 | `editor/editor_store.rs` | `ACTIVE_BLOCK_ID` | editor | ✅ |
| 4 | `editor/host_runtime.rs` | `HOST_EDITOR_HOST_RUNTIME_STATE` | editor | ✅ |
| 5 | `editor/session/session.rs` | `HOST_EDITOR_MODE` | editor | ✅ |
| 6 | `find/find_store.rs` | `CONTROLLER` | find | ✅ |
| 7 | `page/page_store.rs` | `HOST_PAGE_STATE` / `HOST_PREPARED_SCENE` | page | ✅ |
| 8 | `present/present_store.rs` | `HOST_PRESENT_STATE` | present | ✅ |
| 9 | `render/host_runtime.rs` | `HOST_RENDER_LOOP_STATE` | render | ✅ |
| 10 | `render/render_store.rs` | `HOST_RENDER_STATE` | render | ✅ |
| 11 | **`viewer/find_store.rs`** | `HOST_FIND_SESSION` | **viewer ❌ 错位** | 应在 find 域 |
| 12 | **`viewer/review_store.rs`** | `HOST_COMMENT_REVIEW_SESSION` | **viewer ❌ 错位** | 应在 review 域 |
| 13 | `viewer/viewer_store.rs` | `HOST_VIEWER_SESSION` | viewer | ✅ |
| 14 | `zoom/zoom_store.rs` | `HOST_ZOOM_STATE` | zoom | ✅ |

### 4.2 三个跨域错位问题

**问题 A · 两个 `find_store.rs`**：
- `find/find_store.rs`（find 控制器内部状态）
- `viewer/find_store.rs`（viewer 读的 find 快照）
- 同名但语义不同，维护者容易困惑

**问题 B · Review/Comment 状态住在 viewer 域**：
- `viewer/review_store.rs::HOST_COMMENT_REVIEW_SESSION` 被 ReviewSession 和 CommentManager 共用
- 导致 `comment/*` 和 `review/*` 必须 `use crate::viewer::review_store`，**违反域自包含**

**问题 C · `HOST_` 前缀滥用**：
- 8 个 thread_local 带 `HOST_` 前缀，但只有 3 个真是 host 语义
- 其他（VIEWER / ZOOM / FIND / REVIEW）用 HOST_ 是历史遗迹

### 4.3 行动项

| # | 动作 | 收益 |
|---|------|------|
| §5 | 合并 `find/find_store.rs` + `viewer/find_store.rs` → 统一到 find 域 | 消除同名文件 |
| §6 | `viewer/review_store.rs` → `review/review_store.rs`；CommentManager 反向依赖 review 域 | 恢复域自包含 |
| §7 | 批量去掉无语义的 `HOST_` 前缀 | 命名诚实 |

---

## 5. Composition Root · `pdf_runtime.ts` 流程审计

### 5.1 量化特征

- **421 LOC**（作为唯一的 composition root，规模合理）
- **20 imports**（域 controller + WASM handle）
- **16 次 createXxx()** 调用 → 对应 16 个注入点
- **9 个 top-level 函数**，其中 3 个是跨 controller 编排

### 5.2 三个关键编排函数

**① `renderCurrentPage(reason?)`** — 渲染调度核心
- 读 viewer/zoom state → 调 RenderPipeline.startProgressive → 通知 viewer session refresh
- 是唯一能"触发一次完整渲染"的入口，被 editor/zoom/document 等多处调用

**② `openTextPdfFlow(path)`** — 打开文档主流程

```
pick_file (opt) → open_pdf (Tauri) → DocumentSession.setDocument
→ ViewerSession.setDocument → renderCurrentPage → 重置 find/zoom/history
```

**③ `resetPdfViewerState()`** — 关闭/切换文档清理
- 清除所有 thread_local Session 状态
- 当前实现分散调用各 Session 的 reset/clear 方法

### 5.3 问题：编排散在 TS 侧

这三个函数承载了**跨 Session 的业务编排**——openTextPdfFlow 一次触发 5 个 Session 的状态变更。在当前架构里：
- 每个 Session 的 reset 语义由 TS 组合 → 易漏调用 / 顺序错乱
- 没有"打开新文档"这个事件的单点入口，WASM 各 Session 各自知道一部分

**行动项 §8**：在 WASM 侧增加顶层 `Application` handle（或让 DocumentSession 承担），把 `open/close/switch document` 的跨 Session 编排下沉到 Rust，TS 只负责 DOM 交互和命令触发。

---

## 6. 关键流程数据流

### 6.1 打开文档（openTextPdfFlow）

```
[TS] 用户选文件
  → invoke("pick_file")
    → Tauri system.rs::pick_file（native dialog）
  ← path: String
  → smart_invoke("open_pdf", { path })
    → Tauri document.rs::open_pdf（native fs + lopdf::Document::load_from）
    → AppState.docs.pdf_documents.insert(path, Arc<Document>)
  ← DocumentMeta
[TS] → DocumentSession.setDocument(path, meta)
  → Rust WASM: document_store 记录 path/meta
[TS] → ViewerSession.setDocument(path)
  → Rust WASM: viewer_store 记录 path / reset page=0 / zoom=1.0
[TS] → renderCurrentPage("initial")
  → RenderPipeline.startProgressive(page=0)
    → smart_invoke("read_vector", ...) → Tauri render.rs
    → smart_invoke("read_glyph_plan", ...) → Tauri render.rs
    ← vector + glyph → wasm 侧 canvas 绘制
```

**6 次 WASM ↔ Tauri 往返**——open_pdf 后紧接 5 次 read_* 读取单页渲染数据。

### 6.2 编辑提交（commit）

```
[TS] 失焦/Esc
  → EditorSession.commit()
    → Rust WASM: guard_state!(Editing|EditingBlock → Saving)
    → core::edit::build_region_patch() 构造 PersistableRegionPatch
    → smart_invoke("apply_region_patches", { path, page, patches })
      → Tauri replace.rs::apply_region_patches
      → AppState.history.push(snapshot)
      → core 层改写 lopdf::Document
      → invoke("save_pdf") 落盘
    ← Ok
    → transition_to(Viewing)
    → on_state_change 回调 → TS 清 overlay
```

### 6.3 撤销

```
[TS] Ctrl+Z
  → HistoryController.undo()
    → smart_invoke("undo", { path })
      → Tauri document.rs::undo
      → AppState.history.pop() → AppState.docs.pdf_documents 回滚
    ← Ok
  → renderCurrentPage("undo")
  → on_history_change 回调 → 更新工具栏按钮状态
```

---

## 7. 汇总：8 个行动项优先级

| # | 动作 | 批次 | 工时估算 | 依赖 |
|---|------|:---:|:-----:|------|
| §3 | wasm_api/ 内部引用拆除 | **P0** | 2h | 无 |
| §1 | edit_chain_trace → utils/chain_trace | P1 | 30m | 无 |
| §5 | 合并两个 find_store.rs | P1 | 1h | 无 |
| §6 | review_store 迁回 review/ | P1 | 1h | 无 |
| §2 | 抽 editor/orchestrator.rs | P2 | 2h | §3 完成 |
| §4 | 给 Viewer/Zoom/Find/Review 加 enum State | P2 | 4h | §5 §6 完成 |
| §7 | 去 HOST_ 前缀 | P3 | 1h | §5 §6 完成 |
| §8 | WASM 侧 Application handle | P3 | 4h | §2 §4 完成 |

**总预估**：~15.5 小时，可分 4 个迭代推进。

---

## 8. 下批次预告

**Batch 3（Nutrient 对标）**：
- PSPDFKit Web SDK 的 Instance / ViewState / Document / Annotation API 逐项对照
- 事件模型对比（Nutrient 的 `instance.addEventListener`/observable vs 本项目的 `on_state_change`）
- 多文档支持对比（Nutrient 多 Instance vs 本项目 thread_local 单实例限制）
- 插件扩展机制对比
- 输出迁移/借鉴建议清单

> 请您回复「继续」推进 Batch 3，或回复「做行动项 §X」直接开始重构。本文**未改任何代码**。
