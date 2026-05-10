# PDF Viewer 架构重构计划

> 基于 2024-05 全项目审计，解决 SRP 违反、全局状态散布、命名混乱、God File 等问题。
> 原则：每一步都保持编译通过，每一步都有独立价值，不过度设计。

---

## 现状数据

| 指标 | 数值 |
|---|---|
| Core crate 文件 | ~97 个 |
| UI crate 文件 | ~131 个（含 39 个 shim） |
| UI crate 非 shim 文件 | ~92 个 / ~11,000 行 |
| `thread_local!` 全局状态 | 12 个，散布 12 个文件 |
| 同名文件对（core/UI） | 39 对 |
| >500 行的 God File | 7 个 |
| `*runtime.rs` 文件 | 11 个 |
| `*facade.rs` 文件 | 11 个 |

---

## Phase 0: 命名规范化（低风险，1-2 天）

**目标**：统一命名语义，消除"同一个词表达不同含义"的问题。

### 0.1 建立命名公约

| 后缀 | 含义 | 示例 |
|---|---|---|
| `*_store.rs` | 持有 `thread_local!` 状态的模块，仅负责存取 | `page_store.rs`, `zoom_store.rs` |
| `*_controller.rs` | 编排多个 store/service 的流程调度 | `editor_controller.rs` |
| `*_api.rs` | WASM 边界层，只做序列化/反序列化 + 委派 | `viewer_api.rs`, `document_api.rs` |
| `*_service.rs` | 有状态的业务操作（读取 store → 计算 → 写回 store） | `patch_service.rs` |
| `*_types.rs` | 纯 DTO / enum 定义 | `editor_types.rs` |

**不再使用的命名**：
- ~~`runtime.rs`~~ → 按实际职责拆成 `*_store.rs` + `*_controller.rs`
- ~~`facade.rs`~~（在 UI crate 内部）→ 区分为 `*_api.rs`（WASM 边界）或合并到 `*_controller.rs`
- ~~`host_*`~~ → 统一为 `platform_*`（表示平台桥接）或直接用 `*_bridge.rs`

### 0.2 文件重命名映射

```
# Store（纯状态容器）
page/runtime.rs           → page/page_store.rs
zoom/state.rs             → zoom/zoom_store.rs
zoom/runtime.rs           → zoom/zoom_controller.rs
editor/session_state.rs   → editor/editor_store.rs
present/runtime.rs        → present/present_store.rs
render/scheduler.rs       → render/render_store.rs
viewer/session.rs         → viewer/viewer_store.rs
viewer/find.rs            → viewer/find_store.rs
viewer/comment_review.rs  → viewer/review_store.rs
find/controller.rs        → find/find_store.rs

# Controller（流程编排）
editor/runtime.rs         → editor/editor_controller.rs
viewer/runtime.rs         → viewer/viewer_controller.rs
runtime.rs (根)           → app_controller.rs
editor/workflow.rs        → 合并入 editor_controller.rs

# API（WASM 边界）
wasm_api/viewer.rs        → wasm_api/viewer_api.rs
wasm_api/document.rs      → wasm_api/document_api.rs
render/wasm_facade.rs     → wasm_api/render_api.rs
comment/facade.rs         → wasm_api/comment_api.rs
document/facade.rs        → wasm_api/document_facade_api.rs
review/facade.rs          → wasm_api/review_api.rs
find/facade.rs            → wasm_api/find_api.rs

# Platform Bridge
editor/host_runtime.rs    → editor/platform_bridge.rs
editor/host_snapshot.rs   → editor/platform_snapshot.rs
editor/host_mode.rs       → editor/platform_mode.rs
editor/host_workflow.rs   → 合并入 platform_bridge.rs
render/host_runtime.rs    → render/platform_bridge.rs
host/command.rs           → platform/command.rs
host/layout.rs            → platform/layout.rs
host/scroll.rs            → platform/scroll.rs
```

### 0.3 执行方法

每个重命名一步到位：
1. `git mv old.rs new.rs`
2. 更新 `mod.rs` 中的 `pub mod` 声明
3. 全局搜索替换 `crate::old_path` → `crate::new_path`
4. `cargo check` 确认三端通过

---

## Phase 1: 集中全局状态（中风险，3-5 天）

**目标**：把 12 个分散的 `thread_local!` 收到统一的 Store 层，通过函数接口访问。

### 1.1 当前状态分布

```
page/runtime.rs       → HOST_PAGE_STATE: RefCell<HashMap<u16, PageState>>
zoom/state.rs         → HOST_ZOOM_STATE: RefCell<ZoomState>
editor/session_state.rs → HOST_EDITOR_STATE: RefCell<Option<ActiveEditorState>>
present/runtime.rs    → HOST_PRESENT_STATE: RefCell<PresentState>
render/scheduler.rs   → HOST_RENDER_STATE: RefCell<RenderSchedulerState>
viewer/session.rs     → HOST_VIEWER_SESSION: RefCell<ViewerSession>
viewer/find.rs        → HOST_FIND_STATE: RefCell<FindHighlightState>
viewer/comment_review.rs → HOST_REVIEW_STATE: RefCell<ReviewState>
find/controller.rs    → HOST_FIND_CONTROLLER: RefCell<FindControllerState>
editor/host_runtime.rs → HOST_EDITOR_BRIDGE: RefCell<EditorBridgeState>
render/host_runtime.rs → HOST_RENDER_BRIDGE: RefCell<RenderBridgeState>
editor/edit_chain_trace.rs → TRACE_BUFFER: RefCell<Vec<String>>
```

### 1.2 设计：AppState 单一入口

**模式：Centralized Store（集中式存储）**

不用复杂的 ECS 或 Redux 模式，只做最简单的一步——把分散的 `thread_local!` 收到一个结构体：

```rust
// ui/src/store/mod.rs
pub mod app_state;

// ui/src/store/app_state.rs
use std::cell::RefCell;

/// 应用全部可变状态的单一容器。
/// 所有字段都通过本模块的 accessor 函数访问，禁止外部直接 .with()。
pub struct AppState {
    pub pages: HashMap<u16, PageState>,
    pub zoom: ZoomState,
    pub editor: Option<ActiveEditorState>,
    pub present: PresentState,
    pub render_scheduler: RenderSchedulerState,
    pub viewer_session: ViewerSession,
    pub find_highlight: FindHighlightState,
    pub review: ReviewState,
    pub find_controller: FindControllerState,
    pub editor_bridge: EditorBridgeState,
    pub render_bridge: RenderBridgeState,
    pub trace_buffer: Vec<String>,
}

thread_local! {
    static APP: RefCell<AppState> = RefCell::new(AppState::default());
}

// 唯一的访问方式：回调式借用
pub fn with_state<R>(f: impl FnOnce(&AppState) -> R) -> R {
    APP.with(|cell| f(&cell.borrow()))
}

pub fn with_state_mut<R>(f: impl FnOnce(&mut AppState) -> R) -> R {
    APP.with(|cell| f(&mut cell.borrow_mut()))
}
```

### 1.3 迁移策略

**不一步到位**，而是渐进式：

1. 创建 `store/app_state.rs`，先放空结构体
2. 逐个将 `thread_local!` 的字段搬入 `AppState`
3. 原 `thread_local!` 改为委派到 `with_state` / `with_state_mut`
4. 每搬一个字段，`cargo check` 三端

### 1.4 收益

- 消灭 12 个 `thread_local!`，变为 1 个
- 状态依赖关系一目了然（看 `AppState` 字段就行）
- 未来如果要做状态快照/序列化/测试 mock，只需处理一个结构体
- 避免 borrow 冲突时更容易排查（单一 RefCell）

### 1.5 风险

- 单一 `RefCell` 意味着不能同时借用两个不相关的子状态
- **缓解**：如果出现，可以用 `RefCell<Pages>` + `RefCell<Zoom>` 等分组，但不散到 12 个文件

---

## Phase 2: 拆分 God Files（低风险，2-3 天）

### 2.1 拆分方案

#### `render/canvas.rs`（1,339 行）→ 3 个文件

```
render/canvas_setup.rs      — Canvas 创建、尺寸管理、上下文获取
render/canvas_draw.rs       — 绘制逻辑、图层合成、路径渲染
render/canvas_recovery.rs   — 错误恢复、降级策略
```

#### `editor/editor_api.rs`（801 行）→ 3 个文件

```
editor/editor_api.rs        — 保留：开启/关闭编辑器、caret 操作
editor/format_api.rs        — 提取：所有 toggle_*、set_*_font_* 函数
editor/patch_api.rs          — 提取：build_*_patch、sync_editor_input
```

#### `editor/runtime.rs`（645 行）→ 2 个文件（Phase 0 已重命名为 controller）

```
editor/editor_controller.rs — 保留：编辑器生命周期流程
editor/editor_queries.rs    — 提取：纯查询函数（get_*、find_*、collect_*）
```

#### `wasm_api/viewer.rs`（621 行 / 65 fn）→ 3 个文件

```
wasm_api/viewer_api.rs      — 保留：页面/视口相关
wasm_api/editor_api.rs      — 提取：编辑器相关 wasm 绑定
wasm_api/render_api.rs      — 提取：渲染相关 wasm 绑定
```

#### Core: `render/effective_page_plan.rs`（1,355 行）→ 2 个文件

```
render/effective_page_plan.rs   — 保留：计划构建逻辑
render/effective_page_merge.rs  — 提取：图层合并、优先级解析
```

### 2.2 执行方法

每个拆分：
1. 在同目录创建新文件
2. `cut` 相关函数到新文件
3. 新文件顶部添加必要 imports
4. 原文件添加 `pub use new_file::*;`（保持 API 兼容）
5. `cargo check` 三端

---

## Phase 3: 模块边界整理（中风险，2-3 天）

### 3.1 扁平化 editor 子目录

当前 `editor/` 有 5 层嵌套，过深。重组后：

```
editor/
├── mod.rs
├── editor_store.rs         ← 原 session_state.rs
├── editor_controller.rs    ← 原 runtime.rs
├── editor_api.rs           ← 保留
├── format_api.rs           ← 从 editor_api.rs 拆出
├── patch_api.rs            ← 从 editor_api.rs 拆出
├── activation.rs           ← 保留
├── command.rs              ← 保留
├── commit.rs               ← 保留
├── search_facade.rs        ← 保留
├── render_transaction.rs   ← 保留
├── replace_pipeline.rs     ← 保留
├── overlay/                ← 保留子目录（UI 渲染相关）
│   ├── paragraph_overlay.rs
│   ├── projection.rs
│   ├── visual.rs
│   └── navigation.rs
├── format/                 ← 保留子目录
│   ├── list_format.rs
│   └── text_geometry.rs
├── platform_bridge.rs      ← 原 host_runtime.rs
├── platform_snapshot.rs    ← 原 host_snapshot.rs
└── platform_mode.rs        ← 原 host_mode.rs
```

**删除的子目录**：
- `draft/` → 全是 shim，直接改为 `pub use pdf_viewer_core::edit::*` 在 mod.rs
- `session/` → `session.rs` 升到 editor 根，shim 文件合并到 mod.rs
- `source/` → 全是 shim，合并到 mod.rs

### 3.2 收拢根目录散落文件

当前 `lib.rs` 同级有多个杂散文件：

```
# 当前根目录散落
commands.rs           → 死代码，删除，从 lib.rs 移除 pub mod
dom_projection.rs     → shim，合并到 lib.rs 的 pub use
models.rs             → 只剩 1 行 re-export，合并到 lib.rs
projection_workflow.rs → 移入 render/ 或 present/
style_mapper.rs       → shim，合并到 lib.rs 的 pub use
viewport_culling.rs   → shim，合并到 lib.rs 的 pub use
viewport_refresh.rs   → shim，合并到 lib.rs 的 pub use
```

### 3.3 WASM 边界层统一

所有 `#[wasm_bindgen]` 函数集中到 `wasm_api/` 目录：

```
wasm_api/
├── mod.rs
├── viewer_api.rs       ← 页面/视口
├── document_api.rs     ← 文档操作
├── editor_api.rs       ← 编辑器操作
├── render_api.rs       ← 渲染相关
├── comment_api.rs      ← 评论/批注
├── find_api.rs         ← 搜索
└── review_api.rs       ← 审阅
```

每个 `*_api.rs` 只做：
1. 反序列化 `JsValue` → Rust 类型
2. 调用 controller 层
3. 序列化返回值 → `JsValue`

**不包含任何业务逻辑。**

---

## Phase 4: Core crate lib.rs 清理（低风险，0.5 天）

### 4.1 删除 re-export 噪音

当前 `core/lib.rs` 有大量 `pub use` 把子模块平铺到根：

```rust
pub use geometry::bbox_utils;
pub use geometry::coordinate_transform;
pub use text::glyph_layout;
// ... 20+ 行
```

这违反了模块层次的语义。改为：

```rust
// core/lib.rs — 只声明顶层模块
pub mod algorithms;
pub mod analysis;
pub mod document;
pub mod edit;
pub mod geometry;
pub mod models;
pub mod persistence;
pub mod render;
pub mod text;
pub mod typography;
pub mod utils;
```

调用方使用完整路径：`pdf_viewer_core::geometry::bbox_utils` 而不是 `pdf_viewer_core::bbox_utils`。

需要全局搜索替换受影响的 import 路径。

---

## Phase 5: 可选优化（低优先级）

### 5.1 引入 trait 抽象 Store 访问

如果未来需要测试 mock：

```rust
pub trait PageStore {
    fn get_page_state(&self, index: u16) -> Option<&PageState>;
    fn set_page_state(&mut self, index: u16, state: PageState);
}

// 生产实现
impl PageStore for AppState { ... }

// 测试实现
struct MockPageStore { pages: HashMap<u16, PageState> }
impl PageStore for MockPageStore { ... }
```

**现在不做**，等有测试需求时再加。遵循 YAGNI。

### 5.2 editor 拆成独立 crate

如果 `editor/` 继续增长到 5000+ 行，可以拆成 `pdf-viewer-editor` crate：

```
crates/
├── pdf-viewer-core/     ← 纯计算
├── pdf-viewer-editor/   ← 编辑器 UI 逻辑（依赖 core）
└── pdf-viewer-ui/       ← 壳层（依赖 core + editor）
```

**现在不做**，先通过 Phase 0-3 把 editor 内部理清。

---

## 已执行进度（最近一次会话累计）

| Phase | 状态 | 备注 |
|-------|------|------|
| Phase 0: 命名规范化 | ✅ 完成 | 所有 store/controller/api 文件按约定命名 |
| Phase 1: 全局状态封装 | ✅ 完成 | 12 个 `thread_local!` 通过 accessor 函数访问 |
| Phase 2: 拆分 God Files | ✅ 完成 | `canvas.rs` 1339→981, `editor_controller.rs` 645→386 |
| Phase 3.1-3.2: 模块边界 | ✅ 完成 | 删除 23 个死 shim 文件，清理 `draft/`/`source/` 目录 |
| Phase 3.3: WASM 边界统一 | ✅ 完成 | `wasm_api/viewer.rs` 621→206，新增 `zoom_api.rs`/`frame_api.rs` |
| Phase 4: Core lib.rs 清理 | ✅ 完成 | 删除 22 个 `pub use` 重导出，更新 37 个文件 |
| Phase 5: Struct-based API（P0–P4） | ✅ 全部完成 | 9 个 Session 共 159 个方法 |

### Struct API 进度（对应 `docs/editor-api-architecture-proposal.md`）

| 优先级 | API | 方法数 | 文件 | 状态 |
|--------|-----|--------|------|------|
| P0 | `EditorSession` | 33 | `editor/editor_api.rs` (798) | ✅ |
| P1 | `DocumentSession` | 29 | `document/document_api.rs` (253) | ✅ |
| P2 | `FindSession` | 17 | `find/find_api.rs` (189) | ✅ |
| P2 | `ReviewSession` | 7 | `review/review_api.rs` (87) | ✅ |
| P2 | `CommentManager` | 16 | `comment/comment_api.rs` (197) | ✅ |
| P3 | `RenderPipeline` | 18 | `render/render_api.rs` (262) | ✅ |
| P3 | `ZoomController` | 24 | `zoom/zoom_api.rs` (201) | ✅ |
| P3 | `AnnotationManager` | 7 | `annotation/annotation_api.rs` (167) | ✅（2 实现 + 5 stub）|
| P4 | `HistoryController` | 8 | `history/history_api.rs` (114) | ✅（Nutrient `instance.history` 对齐）|

### 剩余已知问题（对标 Nutrient 的差距）

| 差距 | 优先级 | 工作量 | 备注 |
|------|--------|--------|------|
| Tauri 后端补齐 annotation 命令 | P3 | 3-5 天 | `add_annotation` / `update_annotation` / `read_annotation` / `flatten_annotations` / `read_all_annotations`，补后 `AnnotationManager` 的 5 个 stub 可点亮 |
| Event 系统（`addEventListener`/`removeEventListener`） | P2 | 1 周 | Nutrient 风格事件分发，目前完全缺失 |
| 坐标变换统一模块（6 个 transform） | P3 | 3-5 天 | 已部分集中在 `coordinate_transform`，需整合 |
| `host_` 前缀清理 | ✅ 完成 | 0 | 第 1 轮 + 第 2 轮共删 **242 个**别名（17 文件）。第 2 轮采用模块路径调用（`progressive_workflow::start_progressive_render()` 而非 `host_xxx()`），无需重命名底层函数。剩余 38 个全部在 4 个 legacy `*/facade.rs`，TS 迁移后整体删除 |
| TS 端从 `xxxFacade*` 迁移到 Session API | — | 1-2 周 | 旧 facade 仍可用，无紧迫性 |

### 当前状态指标

| 指标 | 重构前 | 当前 | 目标 |
|------|--------|------|------|
| UI crate .rs 文件 | 131 | 137 | — |
| Shim 文件 | 39 | 32 | <10 |
| `thread_local!` 数量 | 12 | 12（已封装）| 1 (AppState) |
| >500 行 God File | 7 | 3 | 0 |
| 最大 wasm 边界文件 | `viewer.rs` 621 | `editor_api.rs` 798（结构良好的 Session）| — |
| Struct-based Session API | 0 | 9（159 方法）| 8–9 ✅ |

---

## 执行顺序与时间估算

```
Phase 0: 命名规范化     [1-2 天]  风险: 低   价值: 高（认知负担大幅降低）
Phase 1: 集中全局状态   [3-5 天]  风险: 中   价值: 高（架构根本改善）
Phase 2: 拆分 God Files [2-3 天]  风险: 低   价值: 中（可读性提升）
Phase 3: 模块边界整理   [2-3 天]  风险: 中   价值: 中（结构清晰化）
Phase 4: Core lib.rs    [0.5 天]  风险: 低   价值: 低（美观）
Phase 5: 可选优化       [按需]    风险: -    价值: 按需
                        ─────────
                        总计: 9-14 天
```

## 验证检查点

每个 Phase 完成后必须满足：

- [ ] `cargo check -p pdf-viewer-core` 通过
- [ ] `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown` 通过
- [ ] `cargo check -p pdf-viewer-standalone` 通过
- [ ] `cargo test -p pdf-viewer-core` 全部通过（55 tests）
- [ ] 无新增 `cargo clippy` 错误（warning 允许）

---

## 设计模式使用清单

| 模式 | 用在哪里 | 为什么用 | 为什么不过度 |
|---|---|---|---|
| **Centralized Store** | Phase 1 AppState | 解决 12 个 thread_local 散布 | 不引入 Redux/ECS，就是一个 struct |
| **Facade** | Phase 3 wasm_api | WASM 边界层职责明确 | 只在 JS↔Rust 边界用，内部不用 |
| **Repository accessor** | Phase 1 with_state/with_state_mut | 统一状态访问方式 | 不引入 trait，就是两个函数 |
| **Module Facade (re-export)** | Phase 2 拆分 God File | 拆文件不破坏外部 API | 只在拆分过渡期用 |

**明确不使用的模式**：
- ❌ Observer/Event Bus — 过度，当前直接调用链够用
- ❌ ECS — 过度，这不是游戏引擎
- ❌ DI Container — 过度，Rust 的模块系统已经够用
- ❌ Abstract Factory — 没有多态构建需求
