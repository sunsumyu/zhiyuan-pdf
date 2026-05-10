# 全面架构审查

> 2026-05-09 · 对照 Typst / Zed / pdf.js / Bevy / Axum 等框架，从可读性、可扩展性、可维护性三个维度审视项目现状。
> 原则：**指出问题、给出对标、提供可操作路径**，但不过度设计。

---

## 0. 项目身份证

| 维度 | 现状 |
|------|------|
| 产品形态 | Tauri 桌面 PDF 查看器 + 内嵌文本编辑 |
| 技术栈 | Rust (native + WASM) · TypeScript · Vite · WebView2 |
| Crate 数 | 3 (`pdf-viewer-core`, `pdf-viewer-ui`, `src-tauri`) |
| Rust 源文件 | ~130 (.rs) |
| WASM 导出函数 | ~109 `#[wasm_bindgen]` |
| Tauri 命令 | ~40 `#[command]` |
| TS Bridge 文件 | ~45 |
| WASM 产物体积 | ~1.7 MB |

---

## 1. 对标框架的核心设计理念

在审查之前，先提炼四个值得参考的设计理念：

### 1.1 Typst — 管线阶段隔离 (Pipeline Stage Isolation)

Typst 将排版拆为 **解析 → 求值 → 排版 → 导出** 四个阶段，每个阶段的输入/输出是**不可变的中间表示 (IR)**，上下游通过 IR 完全解耦。

```
Source → SyntaxTree → Content → Document → PDF/SVG
         (parse)      (eval)    (layout)   (export)
```

**适用启示：** PDF 查看器也有天然管线 —— `解析 → 布局推理 → 渲染 → 编辑 → 回写`，但当前管线的中间表示和阶段边界不明确。

### 1.2 Zed — 单一数据流 + 最小状态面 (Unidirectional Data Flow)

Zed 的编辑器核心遵循 Elm 架构：状态集中、事件驱动、单向流动。全局状态通过 `Model<T>` 持有，修改只通过 `cx.update_model()` 单一路径。

**适用启示：** 当前项目有 5+ 个全局 `thread_local!` / `OnceLock<RwLock<>>` 状态分散在不同模块，读写路径难以追踪。

### 1.3 Bevy — 关注点按维度正交切分

Bevy 不按"功能模块"（physics/render/audio）组织代码，而是让每个 System 声明自己需要的 Component 查询。同一个 Entity 的数据被多个 System 共享，但不存在"谁拥有谁"的问题。

**适用启示：** 当前 `editor/` 目录下 30 个文件，很多是按"操作"切分（commit.rs, activation.rs, workflow.rs），而非按"关注维度"切分，导致一个用户动作（如"提交编辑"）的数据流横穿多个文件。

### 1.4 Axum — 薄胶水层 + 类型驱动路由

Axum 的 handler 层极薄，只做参数提取和响应封装；业务逻辑全在独立的 service 层，与框架完全解耦。路由通过类型系统约束（`FromRequest` trait）自动完成参数绑定。

**适用启示：** `wasm_api/*.rs` 和 `interfaces/pdf.rs` 应该是**薄胶水层**，只做 JS↔Rust 序列化桥接。但当前有些 wasm_api 函数包含了业务逻辑判断。

---

## 2. 分层审查

### 2.1 Layer 0 — 共享类型层 (`pdf-viewer-core`)

#### 现状

```
pdf-viewer-core/src/
├── models.rs          (782 行，30+ struct 平铺)
├── models/            (2 个子模块)
├── text/              (6 个模块)
├── geometry/          (5 个模块)
├── persistence/       (4 个模块)
├── render/            (3 个模块)
├── document/          (3 个模块)
├── algorithms/        (2 个模块)
├── typography/        (4 个模块)
├── analysis/          (1 个模块)
└── utils/             (2 个模块)
```

#### 问题诊断

| # | 问题 | 严重度 | 对标 |
|---|------|--------|------|
| C1 | **God Model**: `models.rs` 22KB / 782 行，从字体到编辑器到布局全在一个文件 | 🔴 高 | Typst 的每个 IR 层有独立类型模块 |
| C2 | **名不副实**: 号称"core"但 ~70% 模块只有单端消费（详见前文审计） | 🟡 中 | 共享 crate 应只放真正共享的内容 |
| C3 | **同名冲突**: `EditorSession` 在 core 和 ui 中各有一个，含义完全不同 | 🔴 高 | 类型名应全局唯一或通过模块路径区分 |
| C4 | **重复定义**: `VectorPageModel` 在 core 和 src-tauri 各一份 | 🟡 中 | 单一事实来源 (Single Source of Truth) |

#### 建议

**不删除 core crate**（它的存在有合理性），但进行**瘦身**：

```
pdf-viewer-core/src/
├── types/                  ← 重命名 models.rs，按领域拆小
│   ├── font.rs             (FontHints, ResolvedFontFace, ...)
│   ├── text.rs             (StyledRun, LayoutLine, LayoutParagraph)
│   ├── region.rs           (SemanticRegion, LayoutInferenceResult)
│   ├── glyph.rs            (GlyphPaintPlan, GlyphPaintRun)
│   ├── bbox.rs             (BoundingBox)
│   └── interaction.rs      (FieldHitRequest, FieldProjection, ...)
├── text/                   ← 保留，双端共享的文本计算
│   ├── glyph_layout.rs
│   └── index_convert.rs
└── geometry/
    └── bbox_utils.rs       ← 保留
```

**移出的模块：**
- `persistence/*` → 移入 `src-tauri`（只有 native 端用）
- `typography/*` → 移入 `src-tauri`
- `geometry/{layout_engine, reflow_engine, field_projection}` → 移入 `src-tauri`
- `document/*`, `algorithms/*`, `analysis/*` → 移入 `src-tauri`
- `text/{list_semantics, editable_segments, style_preservation}` → 移入 `pdf-viewer-ui`
- `render/paint_plan` → 移入 `pdf-viewer-ui`

**重命名冲突：**
- `core::EditorSession` → `core::types::ParagraphEditContext`（它实际上就是 anchor_bbox + paragraph）

---

### 2.2 Layer 1 — WASM 前端逻辑层 (`pdf-viewer-ui`)

#### 2.2.1 模块地图

```
pdf-viewer-ui/src/
├── wasm_api/         ← JS↔Rust 胶水层 (应为薄壳)
├── editor/           ← 编辑器核心 (30 文件，最大模块)
├── render/           ← Canvas 渲染
├── present/          ← 帧管理 / 视口布局
├── viewer/           ← 查看器会话
├── zoom/             ← 缩放状态机
├── document/         ← 文档补丁持久化
├── find/             ← 搜索
├── comment/          ← 评论
├── review/           ← 审阅
├── host/             ← 宿主命令 (导航/缩放)
├── page/             ← 页面上下文
├── annotation/       ← 批注
├── utils/            ← 工具函数
├── state_manager.rs  ← 全局补丁状态 (415 行)
├── runtime.rs        ← WASM 运行时桥
├── models.rs         ← UI 层自有类型
└── ...               ← 其他散落文件
```

#### 2.2.2 问题诊断

| # | 问题 | 严重度 | 说明 |
|---|------|--------|------|
| U1 | **API 出口散布** | 🔴 高 | wasm_bindgen 分散在 6 个文件，有 22 个是死码 |
| U2 | **God File**: `wasm_api/viewer.rs` (696 行, 65 函数) | 🔴 高 | 5 个不相关领域混在一个文件 |
| U3 | **全局状态散射** | 🟡 中 | 至少 5 个 `thread_local!`/`OnceLock` 分布在不同文件 |
| U4 | **editor/ 内部过度平铺** | 🟡 中 | 30 个文件 + 5 个子目录 + 大量 re-export |
| U5 | **facade 命名混乱** | 🟡 中 | `viewer/facade.rs`(死码) vs `present/facade.rs`(内部) vs `render/facade.rs`(内部) |
| U6 | **host_ 前缀冗余** | 🟢 低 | 模块路径已表达归属 |

#### 2.2.3 全局状态清单

| 状态 | 位置 | 类型 | 访问模式 |
|------|------|------|---------|
| 页面状态 | `page/runtime.rs` | `thread_local! RefCell` | 读写 |
| 预备场景 | `page/runtime.rs` | `thread_local! RefCell` | 读写 |
| 渐进渲染任务 | `page/runtime.rs` | `thread_local! RefCell` | 读写 |
| 编辑器模式 | `editor/mode.rs` | `thread_local! RefCell` | 读写 |
| 全局补丁状态 | `state_manager.rs` | `OnceLock<RwLock>` | 读写 |
| 编辑器引擎状态 | `editor/engine_state.rs` | `thread_local! RefCell` | 读写 |

**对标 Zed：** Zed 将所有可变状态收口到 `Model<T>` + `Context`，任何修改必须通过 `cx.update_model()` 路径，保证单向数据流。

**建议（不过度设计版）：** 不需要引入 ECS 或 Elm 架构，但应该：
1. 建立一个 `AppContext` struct，持有所有 `thread_local` 的引用
2. 关键写入路径统一走 `ctx.update(|state| ...)` 风格的闭包，方便将来加日志/事件

#### 2.2.4 API 层重组建议

参照 Axum 的薄胶水层原则：

```
wasm_api/
├── mod.rs
├── editor.rs       ← EditorSession struct (已完成 ✅)
├── viewer.rs       ← ViewerSession struct (从 65 函数瘦身)
├── render.rs       ← RenderPipeline (渐进渲染 + canvas)
├── frame.rs        ← FrameManager (帧计划 + 缓存)
├── page.rs         ← PageContext (页面初始化 + 视口)
├── document.rs     ← DocumentSession (补丁 + 持久化)
├── find.rs         ← FindSession
├── review.rs       ← ReviewSession
└── comment.rs      ← CommentManager
```

每个文件只有 **struct + 方法 + serde 桥接**，无业务逻辑。业务逻辑留在对应的领域模块（`editor/`, `render/`, `zoom/` 等）。

---

### 2.3 Layer 2 — Native 后端 (`src-tauri`)

#### 现状

```
src-tauri/src/
├── lib.rs               ← AppState (11 个 Mutex<HashMap>) + run()
├── interfaces/pdf.rs    ← 40 个 #[command]，1009 行
├── application/pdf/     ← 7 个业务服务
└── infrastructure/pdf/  ← 26 个基础设施文件 (含 47KB 的 vello_renderer)
```

#### 问题诊断

| # | 问题 | 严重度 | 说明 |
|---|------|--------|------|
| T1 | **God File**: `interfaces/pdf.rs` 40 个 command 平铺 | 🔴 高 | 对标 Axum：handler 应按领域分文件 |
| T2 | **AppState 全平铺** | 🟡 中 | 11 个 `Mutex<HashMap>` 无结构，读写不收口 |
| T3 | **interfaces 层过厚** | 🟡 中 | 部分 command 包含业务逻辑（如 `apply_region_patches` 有 100+ 行） |
| T4 | **infrastructure 过平** | 🟡 中 | 26 个文件平铺，无子领域分组 |
| T5 | **VectorPageModel 重复** | 🟡 中 | core 和 src-tauri 各一份 |
| T6 | **无错误类型体系** | 🟢 低 | 全部用 `Result<T, String>`，无结构化错误 |

#### AppState 重组建议

参照 Axum 的 State 分离模式：

```rust
// 当前：11 个 Mutex<HashMap> 平铺
pub struct AppState {
    pub pdf_documents: Mutex<HashMap<String, Arc<lopdf::Document>>>,
    pub pdf_light_page_cache: Mutex<HashMap<...>>,
    pub pdf_page_cache: Mutex<HashMap<...>>,
    // ... 还有 8 个
}

// 建议：按领域分组
pub struct AppState {
    pub documents: DocumentStore,
    pub cache: CacheStore,
    pub history: HistoryStore,
    pub renderer: RendererState,
}

pub struct DocumentStore {
    docs: Mutex<HashMap<String, Arc<lopdf::Document>>>,
    loading: Mutex<HashMap<String, LoadingStatus>>,
}

pub struct CacheStore {
    light_pages: Mutex<HashMap<String, Arc<LightPageModel>>>,
    vector_pages: Mutex<HashMap<String, Arc<VectorPageModel>>>,
    layout: Mutex<HashMap<String, Arc<LayoutInferenceResult>>>,
    previews: Mutex<HashMap<String, PagePreview>>,
    metadata: Mutex<HashMap<String, ReadDocumentMeta>>,
}
```

**好处：** 每个 command handler 只需要注入自己需要的子 store，不用传整个 AppState。

#### interfaces 拆分建议

```
interfaces/
├── mod.rs
├── document.rs       ← open / read / save / clear_cache / undo / redo
├── render.rs         ← read_vector / read_glyph_plan / read_images / render_tile
├── layout.rs         ← resolve_layout / resolve_caret / resolve_hit / ...
├── search.rs         ← find_in_page / find_in_document
├── annotation.rs     ← read_annotation_targets / read_highlights / apply_highlight / ...
├── comment.rs        ← read_comments / apply_comment / ...
├── replace.rs        ← apply_text_patches / apply_region_patches / apply_replace / ...
└── system.rs         ← set_log_level / get_asset_url / create_demo_pdf / pick_file
```

---

### 2.4 Layer 3 — TypeScript Bridge (`src/bridge/`)

#### 现状

```
bridge/
├── index.ts              ← 插件入口
├── viewer/               ← 7 文件 (pdf_runtime 17KB 是主控)
├── editor/               ← 5 文件
├── render/               ← 8 文件 (vector_host 23KB 是最大)
├── ai/                   ← 7 文件
├── comment/              ← 7 文件
├── find/                 ← 2 文件
├── review/               ← 2 文件
├── document/             ← 2 文件
├── annotation/           ← 1 文件
├── zoom/                 ← 1 文件
└── shared/               ← 2 文件 (wasm_loader)
```

#### 评价

TS 层结构相对合理，按领域分目录，每个目录内文件不多。主要问题是：

| # | 问题 | 严重度 |
|---|------|--------|
| B1 | `pdf_runtime.ts` 17KB 做了太多事（初始化/渲染/缩放/键盘/编辑） | 🟡 中 |
| B2 | `vector_host.ts` 23KB，canvas 渲染 + DOM 操作混合 | 🟡 中 |
| B3 | WASM 函数调用无类型安全（`wasm.xxx()` 都是 any） | 🟡 中 |
| B4 | Tauri invoke 调用无类型安全（`invoke('read_pdf', {...})` 字符串命令名） | 🟢 低 |

---

## 3. 跨层设计问题

### 3.1 数据流不清晰

一个典型操作 **"用户编辑文本并保存"** 的数据流当前是：

```
用户输入 (TS)
  → editor_host_view.ts (DOM 事件)
  → EditorSession.commit() [wasm_api]
  → editor/commit.rs (链路追踪)
  → state_manager.rs (全局补丁状态)
  → document/patch_persistence.rs (序列化)
  → bridge.rs → targetInvoke (TS → Tauri IPC)
  → interfaces/pdf.rs apply_region_patches
  → application/pdf/region_patch_service.rs
  → infrastructure/pdf/region_materializer.rs
  → infrastructure/pdf/pdf_write.rs (lopdf 操作)
```

**问题：** 这条链路经过 **10 个文件、3 个 crate、2 次 FFI 边界**。没有一个地方能看到完整的管线定义。

**对标 Typst：** Typst 的 `compile()` 函数就是完整管线的入口，每个阶段返回明确的 IR。

**建议（务实版）：** 不需要重写架构，但应该在文档中画出每个核心用户操作的**端到端数据流图**，并在代码中用命名约定标记管线阶段：
- `editor/commit.rs` → 阶段名 `commit`
- `state_manager.rs` → 阶段名 `persist_local`
- `bridge.rs → targetInvoke` → 阶段名 `ipc_materialize`

这些阶段名已经通过 `chain_trace!` 部分实现了，可以进一步规范。

### 3.2 类型在边界处的序列化开销

每次 WASM↔JS 或 JS↔Tauri 通信都要经过 `serde_wasm_bindgen::to_value` / `serde_json`。这是 Tauri + WASM 架构的固有成本，但可以通过以下方式减轻：

- **批量操作**：合并小消息（当前已部分做了）
- **增量更新**：只传变更部分（如编辑 delta 而非全文）
- **共享内存**：Canvas 渲染可以直接操作 WASM 线性内存的 ImageData（当前已在做）

### 3.3 错误处理

当前全项目统一使用 `Result<T, String>` 作为错误类型。

**对标 Axum / Typst：** 使用 `thiserror` 定义枚举错误类型，在 handler 层用 `Into<Response>` 自动转换。

**建议（最小改动版）：** 在 `src-tauri` 引入错误枚举即可，WASM 端因为要过 JS 边界，`String` 错误反而是最简方案。

```rust
// src-tauri/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("Document not found: {path}")]
    DocumentNotFound { path: String },
    #[error("Page {index} out of range (total: {total})")]
    PageOutOfRange { index: u16, total: u16 },
    #[error("PDF parse error: {0}")]
    ParseError(#[from] lopdf::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// 在 command handler 中自动转为 String
impl From<PdfError> for String {
    fn from(e: PdfError) -> String { e.to_string() }
}
```

---

## 4. 设计原则检查清单

| 原则 | 现状 | 评级 | 改进方向 |
|------|------|:----:|---------|
| **单一职责 (SRP)** | `wasm_api/viewer.rs` 混合 5 领域；`interfaces/pdf.rs` 混合所有命令 | ❌ | 按领域拆文件 |
| **开闭原则 (OCP)** | 添加新 PDF 功能需要修改 `interfaces/pdf.rs` + `lib.rs` 注册 | ⚠️ | command 注册可以模块化 |
| **依赖倒置 (DIP)** | `pdf-viewer-ui` 直接依赖 `pdf-viewer-core` 具体类型 | ✅ | 对于纯数据类型，直接依赖是合理的 |
| **接口隔离 (ISP)** | TS 端拿到整个 wasm 对象，可以调用所有 109 个函数 | ⚠️ | struct 方法分组已改善（EditorSession） |
| **最少知识 (LoD)** | command handler 拿到整个 AppState | ⚠️ | 拆分为子 store |
| **DRY** | `VectorPageModel` 重复；`EditorSession` 同名 | ❌ | 消除重复 |
| **管线清晰性** | 端到端数据流跨 10 文件 3 crate，无集中描述 | ⚠️ | 文档 + 命名约定 |
| **死码零容忍** | 22 个 wasm_bindgen 死码 + 空文件 | ❌ | 立即清除 |

---

## 5. 推荐行动计划

### Phase 0：清除噪音（半天）

- [ ] 删除 `viewer/facade.rs`（13 个死码 wasm_bindgen）
- [ ] 删除 `zoom/facade.rs`（9 个死码 wasm_bindgen）
- [ ] 删除 `wasm_api/editor.rs`（空文件）
- [ ] 删除/内联 `wasm_api/search_facade.rs`
- [ ] 修正 `EditorSession` 同名：core 中重命名为 `ParagraphEditContext`
- [ ] 验证 build + E2E

### Phase 1：分拆神文件（1-2 天）

- [ ] `wasm_api/viewer.rs` (65 fn) → 5 个领域文件
- [ ] `interfaces/pdf.rs` (40 cmd) → 8 个领域文件
- [ ] `AppState` 拆分为 `DocumentStore` / `CacheStore` / `HistoryStore` / `RendererState`

### Phase 2：core 瘦身（1 天）

- [ ] `models.rs` (782 行) → 按领域拆为 6 个小文件
- [ ] 单端模块归位（persistence → src-tauri，list_semantics → pdf-viewer-ui）
- [ ] 消除 `VectorPageModel` 重复定义

### Phase 3：渐进优化（持续）

- [ ] `src-tauri` 引入 `PdfError` 错误枚举
- [ ] WASM 全局状态收口到 `AppContext`
- [ ] 关键管线（编辑→保存、打开→渲染）的端到端数据流文档
- [ ] TS 侧拆分 `pdf_runtime.ts` (17KB) 和 `vector_host.ts` (23KB)

### 不做的事（避免过度设计）

- ❌ 不引入 ECS / Actor / 事件总线等重型架构
- ❌ 不重写 TS 层为框架化（React/Solid），当前 vanilla TS + DOM 操作是合理的
- ❌ 不引入 trait 抽象层（PDF 查看器不需要可插拔的 PDF 后端）
- ❌ 不把所有 `thread_local!` 改为 `Arc<Mutex<>>`（WASM 是单线程的）
- ❌ 不做微服务化拆分（桌面应用不需要）

---

## 6. 之前重构方案的补全

| 维度 | 之前方案覆盖？ | 本次审查新增 |
|------|:-------------:|-------------|
| `pdf-viewer-ui / editor` | ✅ 详细（P0） | — |
| `pdf-viewer-ui / wasm_api` | ✅ 提及（P1） | 死码分析、拆分方案细化 |
| `pdf-viewer-ui / viewer, zoom` | ✅ 提及（P2-P3） | 确认 facade.rs 全部是死码 |
| `pdf-viewer-ui` 全局状态 | ❌ | 全局状态清单 + 收口建议 |
| `pdf-viewer-core` | ❌ | 完整消费者分析 + 瘦身方案 |
| `src-tauri` | ❌ | AppState / interfaces / 错误处理 |
| TypeScript bridge | 部分 | 大文件识别 + 类型安全建议 |
| 跨层数据流 | ❌ | 端到端管线分析 |
| 设计原则对照 | ❌ | SOLID + 框架对标 |

---

## 附录：文件体积 Top 10（需要关注的大文件）

| 文件 | 大小 | 所属层 | 建议 |
|------|------|--------|------|
| `infrastructure/pdf/vello_renderer.rs` | 47 KB | src-tauri | 渲染器内聚，暂不拆 |
| `infrastructure/pdf/pdf_write.rs` | 35 KB | src-tauri | PDF 写入，可提取子函数 |
| `editor/editor_api.rs` | 32 KB | WASM | 已重构为 EditorSession ✅ |
| `editor/runtime.rs` | 25 KB | WASM | 编辑器运行时，可拆事件处理 |
| `infrastructure/pdf/pdf_read.rs` | 24 KB | src-tauri | PDF 解析，内聚合理 |
| `infrastructure/pdf/pdf_write_font_resolver.rs` | 23 KB | src-tauri | 字体解析，内聚合理 |
| `render/canvas.rs` | ~22 KB | WASM | 可拆 draw_* 辅助函数 |
| `core/models.rs` | 22 KB | core | 🔴 必须拆分 |
| `infrastructure/pdf/region_materializer.rs` | 20 KB | src-tauri | 区域物化，内聚合理 |
| `bridge/viewer/pdf_runtime.ts` | 17 KB | TS | 可拆初始化/渲染/事件 |
