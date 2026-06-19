# 路线 B：让 Core 名副其实 — 详细设计

> 2026-05-09
> 前提：当前 `pdf-viewer-core` 不是 core，而是一个"双端碰巧都引用的散文件抽屉"。
> 本文设计一个**真正的领域核心**——打开项目就能看到 PDF 编辑的全部业务逻辑在哪里。

---

## 1. 设计原则

1. **核心 = 业务逻辑，壳 = 平台胶水**
   - `core` 里不出现 `wasm_bindgen`、`tauri`、`thread_local!`、`web_sys`、`lopdf`、`JsValue`
   - `core` 的所有函数都是**纯函数**或**操作自有状态的方法**，可独立单元测试
   - 壳只做：序列化桥接、平台状态持有（thread_local / Mutex）、I/O

2. **不过度抽象**
   - 不引入 trait 抽象层（只有一个 PDF 引擎，不需要可插拔）
   - 不引入事件总线、ECS、Actor
   - 回调/副作用用简单的 `Fn` 闭包参数，不用 trait object

3. **渐进迁移**
   - 不是一次 Big Bang，而是模块逐个搬迁
   - 每搬一个模块就验证 build + E2E

---

## 1.5 当前 vs 目标 — 全景对比图

### 当前架构（问题可视化）

```
┌──────────────────────── 当前状态 ────────────────────────────┐
│                                                              │
│  ┌─────────────────── pdf-viewer-core ────────────────────┐  │
│  │  ╔═══════════════════════════════════════════════════╗  │  │
│  │  ║  models.rs  (782行 大杂烩)                        ║  │  │
│  │  ║  FontHints + StyledRun + EditorSession +          ║  │  │
│  │  ║  GlyphPaintPlan + BoundingBox + FieldHit + ...    ║  │  │
│  │  ╚═══════════════════════════════════════════════════╝  │  │
│  │                                                        │  │
│  │  ┌──── 只有 Tauri 用 ────┐  ┌── 只有 WASM 用 ──┐      │  │
│  │  │ persistence/*         │  │ text/list_sem..   │      │  │
│  │  │ typography/*          │  │ text/editable..   │      │  │
│  │  │ geometry/reflow*      │  │ render/paint_plan │      │  │
│  │  │ document/*            │  └──────────────────-┘      │  │
│  │  │ algorithms/*          │                             │  │
│  │  └───────────────────────┘  ┌── 真正共享 ──────┐      │  │
│  │         ≈70% 单端             │ text/glyph_layout│      │  │
│  │                               │ text/index_conv  │      │  │
│  │                               │ geometry/bbox    │      │  │
│  │                               └─────────────────-┘      │  │
│  │                                    ≈30% 共享            │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌─────────────── pdf-viewer-ui (130 文件) ───────────────┐  │
│  │                                                        │  │
│  │  editor/ (30 文件！)                                    │  │
│  │  ┌─────────────────────────────────────────────────┐   │  │
│  │  │ 纯业务逻辑 ←──┐    ┌──→ 平台绑定               │   │  │
│  │  │ session/       │    │  editor_api.rs (wasm)     │   │  │
│  │  │ commit.rs      │    │  bridge.rs (JS FFI)       │   │  │
│  │  │ draft/        混在   │  runtime.rs (thread_local)│   │  │
│  │  │ format/       一起   │  overlay/visual.rs (Canvas)│  │  │
│  │  │ source/        │    │                           │   │  │
│  │  └─────────────────────────────────────────────────┘   │  │
│  │                                                        │  │
│  │  render/                                                │  │
│  │  ┌─────────────────────────────────────────────────┐   │  │
│  │  │ effective_page_plan.rs (纯计算)  ←混→ canvas.rs │   │  │
│  │  │ source_suppression.rs  (纯计算)     (Canvas2D)  │   │  │
│  │  │ progressive.rs         (纯计算)                  │   │  │
│  │  └─────────────────────────────────────────────────┘   │  │
│  │                                                        │  │
│  │  wasm_api/viewer.rs (65函数 神文件 — 5领域混合)         │  │
│  │  viewer/facade.rs   (死码 ☠)                           │  │
│  │  zoom/facade.rs     (死码 ☠)                           │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌─────────────── src-tauri ─────────────────────────────┐  │
│  │  interfaces/pdf.rs  (40 command 平铺)                  │  │
│  │  AppState { 11 个 Mutex<HashMap> 平铺 }                │  │
│  │  infrastructure/pdf/ (26 文件平铺)                      │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 目标架构（路线 B）

```
┌─────────────────────── 目标状态 ─────────────────────────────┐
│                                                              │
│              ┌──────── pdf-viewer-core ─────────┐            │
│              │  "打开就能看到全部业务逻辑"        │            │
│              │                                   │            │
│              │  model/     ── 领域模型 (6 小文件) │            │
│              │  edit/      ── 编辑操作 (纯状态机) │            │
│              │  render/    ── 渲染规划 (纯计算)   │            │
│              │  text/      ── 文本处理 (纯算法)   │            │
│              │  geometry/  ── 空间计算            │            │
│              │  persistence/ ── 补丁管理          │            │
│              │  diagnostics/ ── 诊断格式化        │            │
│              │                                   │            │
│              │  ⛔ 无 wasm_bindgen               │            │
│              │  ⛔ 无 tauri                       │            │
│              │  ⛔ 无 thread_local                │            │
│              │  ⛔ 无 web_sys / lopdf             │            │
│              │  ✅ cargo test (native target)     │            │
│              └──────────┬────────────────────────┘            │
│                         │                                     │
│            ┌────────────┴────────────┐                        │
│            ▼                         ▼                        │
│  ┌─── pdf-viewer-ui ────┐  ┌──── src-tauri ──────┐          │
│  │  "WASM 薄壳"         │  │  "Native 薄壳"      │          │
│  │                      │  │                      │          │
│  │  wasm_api/ (绑定)    │  │  interfaces/ (命令)  │          │
│  │  runtime/  (状态持有) │  │  state.rs  (状态持有)│          │
│  │  canvas/   (绘制)    │  │  pdf_io/   (读写)   │          │
│  │  bridge/   (IPC)     │  │  render/   (Vello)   │          │
│  │                      │  │                      │          │
│  │  ~60 文件            │  │  ~40 文件            │          │
│  └──────────────────────┘  └──────────────────────┘          │
└──────────────────────────────────────────────────────────────┘
```

---

## 1.6 六边形架构映射（Hexagonal Architecture）

```
                    ┌────────────────────────┐
                    │     TypeScript (UI)     │
                    │   bridge/viewer/editor  │
                    └───────────┬────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              ▼                 ▼                  ▼
    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
    │  WASM API    │  │  Canvas 2D   │  │  Tauri IPC   │
    │  Adapter     │  │  Adapter     │  │  Adapter     │
    │ (serialize)  │  │ (draw calls) │  │ (invoke)     │
    └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
           │                 │                  │
           │      ┌──────────┴──────────┐       │
           │      │    Adapter Layer    │       │
           └──────┤   (pdf-viewer-ui)   ├───────┘
                  │   (src-tauri)       │
                  └──────────┬──────────┘
                             │
                  ╔══════════╧══════════╗
                  ║                     ║
                  ║   pdf-viewer-core   ║
                  ║   ═══════════════   ║
                  ║                     ║
                  ║   Domain Logic      ║
                  ║   (Pure Rust)       ║
                  ║                     ║
                  ║   edit/  render/    ║
                  ║   text/  geometry/  ║
                  ║   model/ persist/   ║
                  ║                     ║
                  ╚══════════╤══════════╝
                             │
                  ┌──────────┴──────────┐
                  │    Port Layer       │
                  │  (core 定义的接口)    │
                  └──────────┬──────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼               ▼
    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
    │ File System  │ │  lopdf       │ │ System Font  │
    │ Adapter      │ │  Adapter     │ │ Adapter      │
    │ (read/write) │ │ (parse PDF)  │ │ (resolve)    │
    └──────────────┘ └──────────────┘ └──────────────┘
```

---

## 2. 核心该有什么？

### 2.1 核心定义：PDF 查看/编辑的"纯业务逻辑"

问一个简单的问题：**如果把 WASM 换成 Qt、把 Tauri 换成 Electron，哪些代码完全不用改？**

那些完全不用改的代码 = 核心。

### 2.2 各模块归属判定

| 当前位置 | 逻辑内容 | 依赖平台？ | → 归属 |
|----------|---------|:----------:|--------|
| **ui** `editor/session/` | 编辑器会话状态机、文档计划构建 | ❌ 纯逻辑 | → **core** |
| **ui** `editor/commit.rs` | 提交管线（构建 patch、调用持久化） | ⚠️ 调 thread_local 状态 | → **core**（状态作为参数传入） |
| **ui** `editor/draft/` | 草稿排版、编辑目标解析 | ❌ 纯计算 | → **core** |
| **ui** `editor/format/` | 列表格式化、文本几何、索引计算 | ❌ 纯计算 | → **core** |
| **ui** `editor/source/` | 源文本几何、源 runs 解析 | ❌ 纯计算 | → **core** |
| **ui** `editor/overlay/paragraph_overlay` | Overlay 数据收集 | ❌ 纯计算 | → **core** |
| **ui** `editor/overlay/visual.rs` | Canvas 绘制代码 | ✅ web_sys Canvas2D | → **留在 ui** |
| **ui** `editor/bridge.rs` | JS 桥接（targetInvoke） | ✅ WASM FFI | → **留在 ui** |
| **ui** `editor/runtime.rs` | 编辑器运行时（thread_local 状态管理） | ✅ thread_local | → **留在 ui**（薄壳） |
| **ui** `editor/editor_api.rs` | wasm_bindgen 入口 | ✅ wasm_bindgen | → **留在 ui** |
| **ui** `render/effective_page_plan.rs` | 有效渲染计划生成 | ❌ 纯计算 | → **core** |
| **ui** `render/progressive.rs` | 渐进渲染任务规划 | ❌ 纯计算 | → **core** |
| **ui** `render/canvas.rs` | Canvas 2D 绘制 | ✅ web_sys Canvas2D | → **留在 ui** |
| **ui** `render/source_suppression.rs` | 源文本抑制判断 | ❌ 纯计算 | → **core** |
| **ui** `render/path_suppression.rs` | 路径抑制判断 | ❌ 纯计算 | → **core** |
| **ui** `state_manager.rs` | 全局补丁状态 + undo/redo | ⚠️ OnceLock | → **core**（状态结构），**ui** 持有实例 |
| **ui** `viewport_culling.rs` | 视口裁剪判断 | ❌ 纯计算 | → **core** |
| **ui** `style_mapper.rs` | 样式映射 | ❌ 纯计算 | → **core** |
| **tauri** `vector_engine.rs` | 从 lopdf 构建 VectorPageModel | ✅ lopdf I/O | → **留在 tauri** |
| **tauri** `pdf_write.rs` | PDF 回写 | ✅ lopdf I/O | → **留在 tauri** |
| **tauri** `vello_renderer.rs` | Vello 渲染 | ✅ Vello/GPU | → **留在 tauri** |
| **tauri** `pdf_font.rs` | 系统字体解析 | ✅ 文件系统 | → **留在 tauri** |
| **core** `models.rs` | 共享类型 | ❌ | → **core**（拆小） |
| **core** `text/glyph_layout` | 字形排版计算 | ❌ 纯计算 | → **core** ✅ 已在 |
| **core** `geometry/layout_engine` | 布局推理 | ❌ 纯计算 | → **core** ✅ 已在 |
| **core** `persistence/*` | 补丁持久化模型 | ❌ 纯数据 | → **core** ✅ 已在 |

### 2.3 模块迁移流向图

```
┌──────────── pdf-viewer-ui ──────────────────────────────────────────┐
│                                                                     │
│   editor/                          render/                          │
│   ┌──────────────────────┐         ┌──────────────────────┐        │
│   │ session/       ──────┼────┐    │ effective_plan.rs ───┼───┐    │
│   │ commit.rs      ──────┼────┤    │ source_suppress. ───┼───┤    │
│   │ draft/         ──────┼────┤    │ path_suppress.   ───┼───┤    │
│   │ format/        ──────┼────┤    │ progressive.rs  ───┼───┤    │
│   │ source/        ──────┼────┤    │                     │   │    │
│   │ overlay/para.. ──────┼────┤    │ canvas.rs        留  │   │    │
│   │ replacement*.  ──────┼────┤    │ prepared_scene   留  │   │    │
│   │ engine_state   ──────┼────┤    └──────────────────────┘   │    │
│   │                      │    │                               │    │
│   │ editor_api.rs  留 ✋ │    │    viewport_culling.rs ───────┤    │
│   │ bridge.rs      留 ✋ │    │    style_mapper.rs ───────────┤    │
│   │ runtime.rs     留 ✋ │    │    state_manager.rs ──────────┤    │
│   │ visual.rs      留 ✋ │    │                               │    │
│   └──────────────────────┘    │                               │    │
│                               │                               │    │
│   留 ✋ = 平台绑定，留在 ui    │                               │    │
└───────────────────────────────┼───────────────────────────────┼────┘
                                │                               │
                    ════════════╪═══════════════════════════════╪════
                    ║           ▼           ▼                   ▼   ║
                    ║   ┌──────────────────────────────────────┐   ║
                    ║   │         pdf-viewer-core (新)          │   ║
                    ║   │                                      │   ║
                    ║   │   edit/          render/              │   ║
                    ║   │   ├── session    ├── effective_plan   │   ║
                    ║   │   ├── commit     ├── overlay          │   ║
                    ║   │   ├── draft      ├── progressive      │   ║
                    ║   │   ├── format     ├── source_suppress  │   ║
                    ║   │   ├── source     ├── path_suppress    │   ║
                    ║   │   ├── engine     ├── viewport_cull    │   ║
                    ║   │   └── style      └── paint_plan ✅    │   ║
                    ║   │                                      │   ║
                    ║   │   model/ (拆自 models.rs)             │   ║
                    ║   │   text/  (已在 ✅)                    │   ║
                    ║   │   geometry/ (已在 ✅)                  │   ║
                    ║   │   persistence/                        │   ║
                    ║   └──────────────────────────────────────┘   ║
                    ════════════════════════════════════════════════
```

---

## 2.5 Crate 依赖图（当前 vs 目标）

### 当前：Core 是被动共享包

```
┌─────────────┐         ┌─────────────┐
│ pdf-viewer  │         │  src-tauri   │
│    -ui      │         │  (native)    │
│  (WASM)     │         │              │
└──────┬──────┘         └──────┬───────┘
       │                       │
       │  依赖 (wasm feature)  │  依赖
       │                       │
       └───────────┬───────────┘
                   ▼
         ┌─────────────────┐
         │ pdf-viewer-core │
         │  (散文件抽屉)    │
         │                 │
         │  models.rs (全) │  ← 两端都要，但太大
         │  text/*   (部分) │  ← 有的只 WASM 用
         │  geom/*   (部分) │  ← 有的只 Tauri 用
         │  persist/* (全)  │  ← 只 Tauri 用但也放这
         │  typo/*   (全)  │  ← 只 Tauri 用但也放这
         └─────────────────┘

箭头表示"依赖"，但依赖内容不清晰
```

### 目标：Core 是领域心脏

```
┌─────────────┐         ┌─────────────┐
│ pdf-viewer  │         │  src-tauri   │
│    -ui      │         │  (native)    │
│  (WASM 壳)  │         │  (Native 壳) │
│             │         │              │
│  wasm_api/  │         │  interfaces/ │
│  canvas/    │         │  pdf_io/     │
│  runtime/   │         │  render/     │
└──────┬──────┘         └──────┬───────┘
       │                       │
       │  只取 model + edit    │  只取 model + geometry
       │  + render + text      │  + text + persistence
       │                       │
       └───────────┬───────────┘
                   ▼
         ╔═════════════════╗
         ║ pdf-viewer-core ║
         ║  (领域核心)      ║
         ║                 ║
         ║  model/  (6文件) ║  ← 类型清晰,按领域拆分
         ║  edit/   (纯逻辑) ║  ← 编辑器状态机+提交
         ║  render/ (纯计算) ║  ← 渲染计划生成
         ║  text/   (纯算法) ║  ← 文本处理
         ║  geometry/(纯数学) ║  ← 空间计算
         ║  persistence/    ║  ← 补丁管理
         ╚═════════════════╝

所有模块每个消费者都可以按需使用
Cargo.toml 无平台依赖
```

---

## 2.6 models.rs 拆分图

```
┌─────────── 当前: models.rs (782 行, 30+ struct) ────────────────────┐
│                                                                     │
│  FontHints, FontSourceKind, SymbolClass, ResolvedFontIdentity,     │
│  ResolvedFontFace, StyledRun, BoundingBox, LayoutRun, LayoutLine,  │
│  LayoutAlignment, LayoutParagraph, EditorSession, SemanticRegion,  │
│  LayoutInferenceResult, PaintMode, GlyphPaintRun,                  │
│  GlyphPaintParagraph, GlyphPaintPlan, VectorPageModel,             │
│  VectorRenderObject, PageState, FieldHitRequest,                   │
│  FieldProjectionRequest, FieldHitResolution, ...                   │
│                                                                     │
│  全部平铺在一个文件  ← 不可维护                                      │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼ 拆分为
┌──── model/ ─────────────────────────────────────────────────────────┐
│                                                                     │
│  ┌── font.rs ──────┐  ┌── text.rs ─────────────┐                  │
│  │ FontHints       │  │ StyledRun              │                  │
│  │ FontSourceKind  │  │ LayoutRun              │                  │
│  │ SymbolClass     │  │ LayoutLine             │                  │
│  │ ResolvedFont*   │  │ LayoutAlignment        │                  │
│  └─────────────────┘  │ LayoutParagraph        │                  │
│                        └────────────────────────┘                  │
│  ┌── bbox.rs ──────┐  ┌── region.rs ───────────┐                  │
│  │ BoundingBox     │  │ SemanticRegion         │                  │
│  │ (+ flip_y 等)   │  │ LayoutInferenceResult  │                  │
│  └─────────────────┘  │ LayoutRole, LayoutMode │                  │
│                        └────────────────────────┘                  │
│  ┌── glyph.rs ─────┐  ┌── page.rs ────────────┐                  │
│  │ GlyphPaintRun   │  │ VectorPageModel       │                  │
│  │ GlyphPaintPara  │  │ VectorRenderObject    │                  │
│  │ GlyphPaintPlan  │  │ PageState             │                  │
│  │ PaintMode       │  └────────────────────────┘                  │
│  └─────────────────┘                                               │
│  ┌── patch.rs ─────┐  ┌── interaction.rs ──────┐                  │
│  │ PersistableReg  │  │ FieldHitRequest       │                  │
│  │  ionPatch       │  │ FieldProjection*      │                  │
│  │ ParagraphRegion │  │ FieldHitResolution    │                  │
│  │  Snapshot       │  │ FieldEditorParams*    │                  │
│  └─────────────────┘  └────────────────────────┘                  │
│                                                                     │
│  mod.rs: pub use font::*; pub use text::*; ...  (保持向后兼容)      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. 新的 Core 结构

```
pdf-viewer-core/
├── Cargo.toml              (无 wasm_bindgen、无 tauri、无 lopdf 依赖)
│
├── model/                  ── 领域模型（当前 models.rs 拆分）
│   ├── mod.rs
│   ├── font.rs             FontHints, ResolvedFontFace, FontSourceKind
│   ├── text.rs             StyledRun, LayoutRun, LayoutLine, LayoutParagraph
│   ├── page.rs             VectorPageModel, VectorRenderObject, PageState
│   ├── region.rs           SemanticRegion, LayoutInferenceResult
│   ├── glyph.rs            GlyphPaintPlan, GlyphPaintRun, GlyphPaintParagraph
│   ├── bbox.rs             BoundingBox
│   ├── patch.rs            PersistableRegionPatch, ParagraphRegionSnapshot
│   └── interaction.rs      FieldHitRequest, FieldProjection, ...
│
├── edit/                   ── 编辑器领域（从 ui/editor/ 迁入纯逻辑部分）
│   ├── mod.rs
│   ├── session.rs          ActiveEditorTarget, EditorDocumentPlan 构建
│   ├── commit.rs           commit 管线（纯状态变换，无 thread_local）
│   ├── engine_state.rs     LiveEditorParagraphState（结构 + 方法）
│   ├── draft_layout.rs     草稿排版计算
│   ├── edit_target.rs      编辑目标解析
│   ├── text_model.rs       EditorTextModel
│   ├── style_mapper.rs     StyleMapper
│   ├── source_text.rs      源文本提取
│   ├── source_runs.rs      源 Runs 匹配
│   ├── source_geometry.rs  源几何计算
│   ├── replacement_region.rs  替换区域计算
│   └── format/
│       ├── list_format.rs
│       ├── text_geometry.rs
│       ├── text_index.rs
│       └── text_model.rs
│
├── render/                 ── 渲染计划（从 ui/render/ 迁入纯计算部分）
│   ├── mod.rs
│   ├── effective_plan.rs   EffectiveVectorRenderEntry 生成
│   ├── overlay.rs          ParagraphRenderOverlay 收集
│   ├── progressive.rs      ProgressiveVectorRenderTask 规划
│   ├── paint_plan.rs       GlyphPaintPlan 构建（已在 core）
│   ├── source_suppression.rs  文本抑制判断
│   ├── path_suppression.rs    路径抑制判断
│   └── viewport_culling.rs    视口裁剪
│
├── text/                   ── 文本处理（已在 core，保持）
│   ├── glyph_layout.rs
│   ├── index_convert.rs
│   ├── list_semantics.rs
│   ├── editable_segments.rs
│   └── style_preservation.rs
│
├── geometry/               ── 空间计算
│   ├── bbox_utils.rs
│   ├── layout_engine.rs    布局推理
│   ├── reflow_engine.rs    回流排版
│   ├── field_projection.rs 字段投影
│   └── coordinate_transform.rs
│
├── persistence/            ── 补丁状态管理（纯状态结构 + 操作方法）
│   ├── patch_state.rs      GlobalPatchState struct + 方法（从 ui/state_manager.rs 迁入）
│   ├── history.rs          undo/redo 逻辑
│   └── engine.rs           已有
│
└── diagnostics/            ── 诊断/调试（可选）
    └── trace.rs            chain_trace! 宏定义（纯格式化，不做 console.log）
```

---

## 4. 新的 Shell 结构

### 4.1 pdf-viewer-ui（WASM 薄壳）

```
pdf-viewer-ui/src/
├── wasm_api/               ── JS 绑定层（薄）
│   ├── editor.rs           EditorSession struct + wasm_bindgen 方法
│   ├── viewer.rs           拆分后的查看器 API
│   ├── render.rs           渲染 API
│   ├── document.rs         文档 API
│   └── ...
│
├── runtime/                ── WASM 平台状态持有
│   ├── page_state.rs       thread_local! HOST_PAGE_STATE
│   ├── editor_state.rs     thread_local! 编辑器状态
│   ├── patch_state.rs      持有 core::persistence::PatchState 的实例
│   └── mod.rs
│
├── canvas/                 ── Canvas 2D 绘制（平台相关）
│   ├── renderer.rs         CanvasRenderer（当前 render/canvas.rs）
│   ├── overlay_draw.rs     编辑器覆盖层绘制（当前 overlay/visual.rs）
│   └── text_draw.rs        文本渲染辅助
│
├── bridge/                 ── JS 互操作
│   ├── target_invoke.rs    Tauri IPC 桥接
│   └── on_debug.rs         调试输出 sink
│
├── host/                   ── 宿主交互（导航、缩放）
├── present/                ── 帧管理
├── zoom/                   ── 缩放交互（DOM 事件 → core 状态）
└── lib.rs
```

**关键变化：** `editor/` 目录不再有 30 个文件，纯逻辑部分全在 core。剩下的只有：
- `wasm_api/editor.rs` — JS 绑定
- `runtime/editor_state.rs` — thread_local 持有
- `canvas/overlay_draw.rs` — Canvas 绘制

### 4.2 src-tauri（Native 薄壳）

```
src-tauri/src/
├── interfaces/             ── Tauri 命令层（按领域拆分）
│   ├── document.rs
│   ├── render.rs
│   ├── layout.rs
│   ├── search.rs
│   ├── annotation.rs
│   └── ...
│
├── state.rs                ── AppState（按领域分组）
│
├── pdf_io/                 ── PDF 文件 I/O（平台相关）
│   ├── reader.rs           lopdf 解析
│   ├── writer.rs           lopdf 回写
│   ├── font_resolver.rs    系统字体匹配
│   └── vector_builder.rs   构建 VectorPageModel（从 lopdf → core 类型）
│
└── render/                 ── 本地渲染（平台相关）
    └── vello.rs            Vello 渲染
```

---

## 5. 最关键的接口设计：Core 如何不依赖平台

### 5.1 编辑器提交：不再依赖 thread_local

```rust
// ── core/edit/commit.rs ──────────────────────────────
// 纯函数：接收状态，返回结果，不访问任何全局变量

pub struct CommitResult {
    pub patch: Option<PersistableRegionPatch>,
    pub should_close_editor: bool,
}

pub fn commit_active_editor(
    active_state: &LiveEditorParagraphState,
    draft_text: &str,
    patch_state: &mut GlobalPatchState,     // 传入，不从全局读
) -> CommitResult {
    let patch = build_patch(active_state, draft_text);
    if let Some(ref p) = patch {
        apply_patch(patch_state, p);
    }
    CommitResult {
        patch,
        should_close_editor: true,
    }
}

// ── ui/wasm_api/editor.rs ──────────────────────────
// 薄壳：从 thread_local 取出状态，调用 core，存回去

#[wasm_bindgen]
impl EditorSession {
    pub fn commit(&self, request: JsValue) -> JsValue {
        let result = EDITOR_STATE.with(|editor| {
            let editor = editor.borrow();
            PATCH_STATE.with(|patches| {
                let mut patches = patches.borrow_mut();
                core::edit::commit::commit_active_editor(
                    &editor, &draft_text, &mut patches
                )
            })
        });
        to_value(&result).unwrap()
    }
}
```

### 5.2 渲染计划生成：纯输入 → 纯输出

```rust
// ── core/render/effective_plan.rs ──────────────────
// 纯函数：给定页面模型 + overlay + 视口 → 返回渲染计划

pub fn build_effective_vector_render_plan(
    vector_model: &VectorPageModel,
    prepared_scene: Option<&PreparedPageScene>,
    viewport: &BoundingBox,
    overlays: &[ParagraphRenderOverlay],
) -> Vec<EffectiveVectorRenderEntry> {
    // 纯计算，无副作用
}

// ── ui/canvas/renderer.rs ──────────────────────────
// 壳：拿到计划后用 Canvas2D 执行绘制

impl CanvasRenderer {
    pub fn render_page(&self, state: &PageState, plan: &PaintPlan) {
        let overlays = core::render::overlay::collect_overlays(plan, ...);
        let render_plan = core::render::effective_plan::build(...);
        
        // 以下是平台代码：Canvas2D 调用
        for entry in render_plan {
            match entry {
                EffectiveVectorRenderEntry::Object { .. } => self.draw_vector_object(...),
                EffectiveVectorRenderEntry::ParagraphOverlay(o) => self.draw_overlay(...),
            }
        }
    }
}
```

### 5.3 诊断日志：Core 只格式化，Shell 决定输出

```rust
// ── core/diagnostics/trace.rs ──────────────────────
// Core 只负责格式化，通过回调输出

pub type TraceSink = fn(&str);

static TRACE_SINK: std::sync::OnceLock<TraceSink> = std::sync::OnceLock::new();

pub fn set_trace_sink(sink: TraceSink) {
    let _ = TRACE_SINK.set(sink);
}

#[macro_export]
macro_rules! chain_trace {
    ($step:expr $(, $key:expr => $val:expr)* $(,)?) => {
        if let Some(sink) = $crate::diagnostics::trace::TRACE_SINK.get() {
            let msg = format!("[CHAIN] {} {}", $step,
                [$( format!("{}={}", $key, $val) ),*].join(" "));
            sink(&msg);
        }
    };
}

// ── ui/bridge/on_debug.rs ──────────────────────────
// WASM 壳：注册输出到 browser console
pub fn init_trace() {
    pdf_viewer_core::diagnostics::trace::set_trace_sink(|msg| {
        web_sys::console::log_1(&msg.into());
    });
}

// ── src-tauri ──────────────────────────────────────
// Native 壳：注册输出到 log crate
pub fn init_trace() {
    pdf_viewer_core::diagnostics::trace::set_trace_sink(|msg| {
        log::debug!("{}", msg);
    });
}
```

---

## 5.5 核心数据流：编辑→提交→保存 管线

### 当前（10 文件跨 3 crate，无法追踪）

```
用户按键
  │
  ▼
┌──────────────────── TypeScript ────────────────────────────┐
│ editor_host_view.ts  →  EditorSession.commit()  [WASM]    │
└──────────────────────────┬────────────────────────────────-┘
                           │ wasm_bindgen
                           ▼
┌──────────────────── pdf-viewer-ui ─────────────────────────┐
│                                                            │
│  editor_api.rs ──→ commit.rs ──→ runtime.rs                │
│       │                │              │                    │
│       │                │   ┌──────────┘                    │
│       │                ▼   ▼                               │
│       │         state_manager.rs (OnceLock 全局状态)        │
│       │                │                                   │
│       │                ▼                                   │
│       │    document/patch_persistence.rs                   │
│       │                │                                   │
│       │                ▼                                   │
│       │         bridge.rs → targetInvoke()                 │
└───────┼────────────────┼───────────────────────────────────┘
        │                │ JS IPC
        │                ▼
┌───────┼──────── src-tauri ─────────────────────────────────┐
│       │  interfaces/pdf.rs  apply_region_patches           │
│       │         │                                          │
│       │         ▼                                          │
│       │  region_materializer.rs → pdf_write.rs → 磁盘       │
└───────┼────────────────────────────────────────────────────┘
        │
        │  问题：
        │  ❌ 10 个文件、3 个 crate、2 次 FFI
        │  ❌ 状态散落 thread_local + OnceLock
        │  ❌ 无法 cargo test 验证提交逻辑
```

### 目标（Core 拥有全部提交逻辑）

```
用户按键
  │
  ▼
┌──────────── TypeScript ──────────────┐
│ editor_host_view.ts                  │
│   → EditorSession.commit() [WASM]    │
└──────────────┬───────────────────────┘
               │ wasm_bindgen
               ▼
┌──── pdf-viewer-ui (薄壳) ────────────┐
│                                      │
│  wasm_api/editor.rs                  │
│    │                                 │
│    │  ① 从 thread_local 取出状态     │
│    │  ② 调用 core                    │
│    │  ③ 存回 thread_local            │
│    │  ④ 触发 IPC (如需保存到磁盘)     │
│    │                                 │
│    ▼                                 │
│  runtime/editor_state.rs             │
│    │  thread_local! { RefCell<..> }  │
│    │                                 │
└────┼─────────────────────────────────┘
     │
     │  &mut state (参数传入，不是全局读)
     ▼
╔════════════ pdf-viewer-core ═══════╗
║                                    ║
║  edit/commit.rs                    ║
║    │                               ║
║    ├─→ build_patch()  (纯计算)     ║
║    ├─→ apply_patch()  (修改传入的   ║
║    │     &mut PatchState)          ║
║    └─→ CommitResult   (纯返回值)   ║
║                                    ║
║  ✅ 可 cargo test                  ║
║  ✅ 无全局状态                      ║
║  ✅ 所有逻辑在一个 crate            ║
╚════════════════════════════════════╝
     │
     │  CommitResult { patch, .. }
     ▼
┌──── pdf-viewer-ui (薄壳) ────┐     ┌──── src-tauri (薄壳) ────┐
│  bridge/target_invoke.rs     │────→│  interfaces/document.rs  │
│  (序列化 patch → IPC)        │     │  pdf_io/writer.rs → 磁盘  │
└──────────────────────────────┘     └──────────────────────────┘
```

---

## 5.6 核心数据流：页面渲染管线

### 当前

```
TS: renderCurrentPage()
  │
  ▼ wasm_bindgen
┌──────────────── pdf-viewer-ui ─────────────────────────┐
│                                                        │
│  wasm_api/viewer.rs  render_page()                     │
│       │                                                │
│       ▼                                                │
│  page/runtime.rs  HOST_PAGE_STATE (thread_local)       │
│       │                                                │
│       ▼                                                │
│  render/canvas.rs  render_page()                       │
│       │                                                │
│       ├──→ paragraph_overlay.rs  (纯计算 → 应在 core)  │
│       ├──→ effective_page_plan.rs (纯计算 → 应在 core)  │
│       ├──→ viewport_culling.rs    (纯计算 → 应在 core)  │
│       ├──→ source_suppression.rs  (纯计算 → 应在 core)  │
│       │                                                │
│       └──→ ctx.fillText() / ctx.fillRect()  (平台)     │
│                                                        │
│  问题：纯计算和 Canvas 绑定混在同一个调用链              │
└────────────────────────────────────────────────────────┘
```

### 目标

```
TS: renderCurrentPage()
  │
  ▼ wasm_bindgen
┌──── pdf-viewer-ui (薄壳) ───────────────────────────┐
│                                                      │
│  wasm_api/render.rs                                  │
│    ① 从 thread_local 取出 PageState                  │
│    ② 调用 core 生成渲染计划                           │
│    ③ 用 CanvasRenderer 执行绘制                       │
│                                                      │
│         ┌─────────────┐                              │
│    ②    │             │                              │
│    ┌────┼─────────────┼──────────────────────┐       │
│    │    ▼             ▼                      │       │
│    │  ╔═══════ pdf-viewer-core ═══════════╗  │       │
│    │  ║                                   ║  │       │
│    │  ║  render/overlay::collect()        ║  │       │
│    │  ║       │                           ║  │       │
│    │  ║       ▼                           ║  │       │
│    │  ║  render/effective_plan::build()   ║  │       │
│    │  ║       │                           ║  │       │
│    │  ║       ├─ viewport_culling         ║  │       │
│    │  ║       ├─ source_suppression       ║  │       │
│    │  ║       └─ path_suppression         ║  │       │
│    │  ║       │                           ║  │       │
│    │  ║       ▼                           ║  │       │
│    │  ║  Vec<EffectiveRenderEntry>        ║  │       │
│    │  ║  (纯数据，描述"画什么"而非"怎么画") ║  │       │
│    │  ║                                   ║  │       │
│    │  ╚═══════════════════════════════════╝  │       │
│    └─────────────────┬───────────────────────┘       │
│                      │                               │
│    ③                 ▼                               │
│    ┌─────────────────────────────────┐               │
│    │  canvas/renderer.rs  (平台层)   │               │
│    │                                 │               │
│    │  for entry in plan {            │               │
│    │    match entry {                │               │
│    │      Object {..}  → draw_obj() │  ← Canvas2D   │
│    │      Overlay(..)  → draw_ovl() │  ← Canvas2D   │
│    │    }                            │               │
│    │  }                              │               │
│    └─────────────────────────────────┘               │
└──────────────────────────────────────────────────────┘
```

---

## 5.7 状态持有模式：Core 定义结构，Shell 持有实例

```
╔═══════════ pdf-viewer-core ══════════╗
║                                      ║
║  // 纯 struct + 方法，无全局状态      ║
║                                      ║
║  pub struct PatchState {             ║
║      patches: HashMap<..>,           ║
║      history: Vec<PatchCommand>,     ║
║      redo_stack: Vec<PatchCommand>,  ║
║  }                                   ║
║                                      ║
║  impl PatchState {                   ║
║      pub fn apply(&mut self, ..)     ║
║      pub fn undo(&mut self) -> ..    ║
║      pub fn redo(&mut self) -> ..    ║
║  }                                   ║
║                                      ║
║  pub struct EditorEngine {           ║
║      pub active: Option<Target>,     ║
║      pub mode: EditorMode,           ║
║  }                                   ║
║                                      ║
║  impl EditorEngine {                 ║
║      pub fn begin(&mut self, ..)     ║
║      pub fn commit(&mut self, ..)    ║
║  }                                   ║
╚══════════════╤═══════════════════════╝
               │
       ┌───────┴───────┐
       ▼               ▼
┌─── WASM 壳 ───┐  ┌─── Tauri 壳 ──────────────────┐
│                │  │                                │
│  thread_local! │  │  AppState {                    │
│  {             │  │    patches: Mutex<PatchState>,  │
│    PATCH:      │  │    // ...                      │
│      RefCell<  │  │  }                             │
│      PatchState│  │                                │
│    >,          │  │  // Tauri: 多线程 → Mutex       │
│    EDITOR:     │  │  // 但操作同一个 core struct    │
│      RefCell<  │  │                                │
│      Editor    │  └────────────────────────────────┘
│    >,          │
│  }             │
│                │
│  // WASM: 单线程 → RefCell 即可
└────────────────┘
```

---

## 5.8 诊断日志流

```
╔══════════ pdf-viewer-core ═════════════════╗
║                                            ║
║  chain_trace!("commit.start", ...)         ║
║       │                                    ║
║       ▼                                    ║
║  diagnostics::trace::TRACE_SINK            ║
║  (OnceLock<fn(&str)>)                      ║
║       │                                    ║
║       │  在 init 时由 shell 注册回调        ║
╚═══════╪════════════════════════════════════╝
        │
   ┌────┴─────────────────┐
   ▼                      ▼
┌──── WASM ────┐   ┌──── Tauri ────────┐
│ set_trace_sink│   │ set_trace_sink    │
│ (|msg| {     │   │ (|msg| {          │
│   console    │   │   log::debug!(..) │
│   ::log_1()  │   │ })                │
│ })           │   │                   │
│              │   │ → 写入文件/stderr  │
│ → 浏览器控制台│   └──────────────────-┘
└──────────────┘
```

---

## 6. 迁移路径（渐进式，不是 Big Bang）

### 6.0 迁移时间线与依赖

```
              Day 1          Day 2-3          Day 4-6        Day 7
              ──────         ──────────       ──────────     ──────
Phase B0 ─── ████ ─┐
 删死码+拆models    │
                    │
Phase B1 ──────────└── ████████ ─┐
 迁移纯计算模块                   │
 (无风险,直接move)                │
                                 │
Phase B2 ────────────────────────└── ████████████ ─┐
 去 thread_local,                                   │
 迁移有状态模块                                      │
 (中风险,需重构接口)                                  │
                                                    │
Phase B3 ───────────────────────────────────────────└── ████
 统一类型 + 整理壳
```

### 6.0.1 Phase 依赖关系

```
B0 ──→ B1 ──→ B2 ──→ B3 ──→ B4
│      │      │      │      │
│      │      │      │      └─→ 整理壳 (wasm_api 拆分, interfaces 拆分)
│      │      │      │
│      │      │      └─→ 统一 VectorPageModel, 错误类型
│      │      │
│      │      └─→ commit.rs / session / engine_state 迁入 core
│      │          (需要改造: thread_local → 参数传入)
│      │
│      └─→ viewport_culling / source/ / format/ 等直接 move
│          (零风险: 纯计算, 无平台依赖)
│
└─→ 删死码 + models.rs 拆为 model/
    (基础准备, 不影响功能)

⚡ 安全回退点：
   B0 完成后 → 项目已经更干净，可以停
   B1 完成后 → Core 有了真正的 render 计算, 可以停
   B2 完成后 → Core 有了完整编辑逻辑, 可以停
```

### Phase B0：准备工作（半天）

- [ ] 删除死码（viewer/facade.rs, zoom/facade.rs, wasm_api/editor.rs）
- [ ] `core/models.rs` 拆为 `core/model/` 下的 6 个文件
- [ ] 验证 build + E2E

### Phase B1：迁移纯计算模块（1-2 天）

这些模块完全无平台依赖，直接 move：

- [ ] `ui/viewport_culling.rs` → `core/render/viewport_culling.rs`
- [ ] `ui/render/source_suppression.rs` → `core/render/source_suppression.rs`
- [ ] `ui/render/path_suppression.rs` → `core/render/path_suppression.rs`
- [ ] `ui/style_mapper.rs` → `core/edit/style_mapper.rs`
- [ ] `ui/editor/format/*` → `core/edit/format/*`
- [ ] `ui/editor/source/*` → `core/edit/source/*`
- [ ] `ui/editor/replacement_region.rs` → `core/edit/replacement_region.rs`
- [ ] 每个模块迁移后 `cargo check --target wasm32-unknown-unknown` + `npm run e2e`

### Phase B2：迁移有状态依赖的模块（2-3 天）

这些模块需要**去除 thread_local 依赖**，改为参数传入：

- [ ] `ui/state_manager.rs` → 拆为 `core/persistence/patch_state.rs`（结构+方法）+ `ui/runtime/patch_state.rs`（持有实例）
- [ ] `ui/editor/commit.rs` → 改为纯函数（状态作为参数）
- [ ] `ui/editor/session/*` → 纯数据结构和构建函数搬到 core
- [ ] `ui/editor/engine_state.rs` → 纯结构搬到 core，thread_local 持有留在 ui
- [ ] `ui/render/effective_page_plan.rs` → `core/render/effective_plan.rs`
- [ ] `ui/render/progressive.rs` → `core/render/progressive.rs`

### Phase B3：迁移 Tauri 端可共享逻辑（1 天）

- [ ] 确认 `geometry/layout_engine` 等已在 core（✅ 已在）
- [ ] `tauri/infrastructure/pdf/models.rs` 中重复的 `VectorPageModel` → 统一用 core 的
- [ ] 统一错误类型

### Phase B4：整理壳（1 天）

- [ ] `ui/editor/` 从 30 文件缩减到 ~8 文件（API + runtime + canvas）
- [ ] `ui/wasm_api/viewer.rs` 拆为 5 个领域文件
- [ ] `tauri/interfaces/pdf.rs` 拆为 8 个领域文件
- [ ] 更新 mod.rs 和 re-exports

---

## 7. 预期成果

| 指标 | 当前 | Phase B 完成后 |
|------|------|---------------|
| Core 文件数 | ~15（大部分是单端散文件） | **~35**（真正的领域逻辑） |
| Core 可独立单元测试？ | ❌ 很多模块依赖 wasm | ✅ `cargo test`（native target） |
| UI shell 文件数 | ~130 | **~60**（纯平台胶水） |
| 新人理解路径 | "core 里是什么？什么都有又什么都不是" | **"core = PDF 编辑的全部业务逻辑"** |
| 添加新编辑功能 | 要同时改 core + ui + 可能改 tauri | **只改 core，壳自动获得** |
| WASM 体积 | ~1.7 MB | 基本不变（代码总量不变，只是搬家） |

---

## 8. 风险与诚实评估

| 风险 | 概率 | 缓解 |
|------|------|------|
| 迁移过程中引入回归 | 中 | 每步验证 E2E |
| thread_local 解耦复杂度超预期 | 中 | B2 阶段可以先迁最简单的 1-2 个模块试水 |
| 总工期超预期（目前估 5-7 天） | 中 | Phase 先做 B0+B1（低风险），评估后再决定 B2 |
| Core 仍然被单端功能污染 | 低 | CI 检查：core 不允许依赖 wasm_bindgen/tauri |

---

## 9. 判断标准：什么时候该停？

路线 B 的目标不是"把所有代码搬到 core"，而是达到一个**清晰的分层**：

> **打开 `pdf-viewer-core/`，你能看到 PDF 编辑器的全部业务逻辑。**
> **打开 `pdf-viewer-ui/`，你只看到 WASM 绑定 + Canvas 绘制。**
> **打开 `src-tauri/`，你只看到文件 I/O + Tauri 命令。**

当这三句话成立时，重构就完成了。

---

## 10. 代码分布 Before / After

```
                    当前                              目标
            ┌─────────────────┐               ┌─────────────────┐
            │  pdf-viewer-core │               │  pdf-viewer-core │
            │                 │               │                 │
  15 files  │  ░░░░░          │    35 files   │  ████████████   │
            │  (散文件,30%共享) │               │  (领域核心,100%) │
            └─────────────────┘               └─────────────────┘

            ┌─────────────────┐               ┌─────────────────┐
            │  pdf-viewer-ui   │               │  pdf-viewer-ui   │
            │                 │               │                 │
 130 files  │  ████████████████│    60 files   │  ████████       │
            │  (业务+平台混合)  │               │  (纯平台胶水)    │
            └─────────────────┘               └─────────────────┘

            ┌─────────────────┐               ┌─────────────────┐
            │   src-tauri      │               │   src-tauri      │
            │                 │               │                 │
  55 files  │  █████████████  │    40 files   │  ████████       │
            │  (业务+平台混合)  │               │  (纯平台胶水)    │
            └─────────────────┘               └─────────────────┘

           总计 200 files                     总计 135 files
           core 占 7.5%                       core 占 26%
           业务逻辑散落三处                    业务逻辑集中一处
```

### 新人阅读路径对比

```
当前：
  新人 → "core 是什么？" → 看到散文件 → 困惑
       → "编辑逻辑在哪？" → ui/editor/ (30文件) → 迷路
       → "渲染怎么工作？" → ui/render/ (混合计算+Canvas) → 分不清边界

目标：
  新人 → "项目核心？" → pdf-viewer-core/
       → "编辑逻辑？" → core/edit/   (纯状态机, 可 cargo test)
       → "渲染逻辑？" → core/render/ (纯计划生成, 可 cargo test)
       → "怎么上屏？" → ui/canvas/   (Canvas2D 绘制)
       → "怎么读 PDF？" → tauri/pdf_io/ (lopdf 解析)
```

---

## 11. 架构审查：对照代码的实际问题发现

> 以架构师视角，对照 §5.5-§5.8 的目标图审查当前代码的真实流程和状态流转。

### 11.1 问题一：双状态机并行，无同步保证

当前编辑器存在**两套独立的状态机**，分别持有在不同的 `thread_local!` 中：

```
状态机 A (新 API): session_state.rs
┌─────────────────────────────────────────────────────┐
│  thread_local! SESSION_STATE: Cell<SessionState>    │
│  thread_local! ACTIVE_BLOCK_ID: RefCell<Option>     │
│                                                     │
│  Viewing ──→ Editing ──→ EditingBlock ──→ Saving    │
│                              │                      │
│                              └──→ Viewing           │
└─────────────────────────────────────────────────────┘

状态机 B (旧基础设施): session/session.rs
┌──────────────────────────────────────────────────────┐
│  thread_local! HOST_EDITOR_MODE: RefCell<            │
│    EditorModeState {                                 │
│      text_edit_enabled: bool,                        │
│      active_paragraph_id: Option<String>,            │
│      live_state: Option<LiveEditorParagraphState>,   │
│    }                                                 │
│  >                                                   │
└──────────────────────────────────────────────────────┘

第三层状态: host_runtime.rs
┌──────────────────────────────────────────────────────┐
│  thread_local! HOST_EDITOR_HOST_RUNTIME_STATE:       │
│    RefCell<EditorHostRuntimeState {                   │
│      committing: bool,    ← commit 重入锁            │
│      last_display_zoom,                              │
│    }>                                                │
└──────────────────────────────────────────────────────┘
```

**实际问题：**

```
editor_api.rs::commit() 调用链:

    ① guard_state!(EditingBlock)          ← 检查状态机 A
    ② begin_commit()                      ← 检查状态机 C (committing flag)
    ③ commit_editor_tx(draft_text, ...)
        └→ commit_active_editor_text()
            ├→ get_active_editor_state()   ← 读状态机 B (live_state)
            ├→ build_active_editor_patch() ← 读状态机 B (live_state)
            ├→ apply_document_patch_direct()
            │   └→ record_patch() ← 写 GLOBAL_PATCH_STATE (OnceLock)
            └→ close_active_editor()       ← 写状态机 B
    ④ finish_commit()                     ← 写状态机 C
    ⑤ transition_to_viewing()             ← 写状态机 A
    ⑥ set_text_edit_mode(false)           ← 写状态机 B

风险：如果 ③ panic，状态机 A 和 C 不会回滚
      状态机 A 仍在 EditingBlock，状态机 C 的 committing=true
      → 死锁：后续所有 commit 被 begin_commit() 拒绝
```

**⚠️ 这是一个真实的 bug 风险。**
在路线 B 中，应该**合并为一个状态机**（Core 只有一个 `EditorEngine` struct），
让 commit 成为原子操作。

---

### 11.2 问题二：commit 流程中 4 个状态存储的写入顺序

```
当前 commit 的写入流:

  editor_api.rs                         状态存储
  ══════════════                        ════════════
  commit()
    │
    ├─ build_active_editor_patch()  ──→ 读 HOST_EDITOR_MODE.live_state
    │                                   (状态机 B)
    │
    ├─ apply_document_patch_direct() ─→ 写 GLOBAL_PATCH_STATE    ④
    │   └─ record_patch()   (patch state)
    │
    ├─ remember_replacement_target() ──→ 写 GLOBAL_PATCH_STATE    ④
    │                                   (.paragraph_replacement_targets)
    │
    ├─ close_active_editor()  ─────────→ 写 HOST_EDITOR_MODE     ②
    │                                   (live_state = None,
    │                                    active_paragraph_id = None)
    │
    ├─ finish_commit()  ───────────────→ 写 HOST_RUNTIME_STATE   ③
    │                                   (committing = false)
    │
    ├─ transition_to_viewing()  ───────→ 写 SESSION_STATE        ①
    │                                   (→ Viewing)
    │
    └─ set_text_edit_mode(false)  ─────→ 写 HOST_EDITOR_MODE     ②
                                        (text_edit_enabled = false,
                                         live_state = None ← 已经是)
```

**问题清单：**

- **a) 4 个存储、6 次写入操作不是事务** — 任意一步 panic 都会导致状态不一致
- **b) `set_text_edit_mode(false)` 重复清理** — `close_active_editor()` 已经清了 `live_state`，
  然后 `set_text_edit_mode(false)` 又检查一次并打 warning — 这是防御性代码在为架构缺陷兜底
- **c) `GLOBAL_PATCH_STATE` 用 `OnceLock<RwLock<>>` 而其他用 `thread_local! RefCell`**
  — 混用两种线程安全策略（WASM 是单线程，OnceLock+RwLock 完全多余）

---

### 11.3 问题三：mode.rs 是纯代理层，零逻辑

```rust
// mode.rs — 每个函数都只是转发到 session.rs
pub fn get_active_editor_state() -> .. { host_active_editor_state() }
pub fn close_active_editor() -> ..     { host_close_active_editor(); .. }
pub fn is_text_edit_mode_enabled() ..  { host_is_text_edit_enabled() }
```

**这一层只做 rename import，没有任何逻辑。** 它的存在增加了代码跳转深度
（commit.rs → mode.rs → session.rs → thread_local）。应该直接消除。

---

### 11.4 问题四：编辑→提交图中遗漏的「渲染副作用」

§5.5 图中画的是 `commit → patch → IPC`，但实际代码中 commit 还有一条**隐式渲染路径**：

```
实际流程（图中没体现的部分）:

  editor_api.rs::commit()
    └→ commit_editor_tx()                    ← render_transaction.rs
        ├→ commit_active_editor_text()       ← 业务逻辑
        └→ schedule_render_frame_request()   ← ⚡ 触发重绘！
            └→ present/runtime.rs
                └→ HOST_RENDER_SCHEDULE      ← 又一个 thread_local!

  也就是说 commit 不只产生 patch，还会直接调度一次渲染帧。
  这个副作用在目标图中没有体现。
```

**修正后的目标流应该是：**

```
Core 的 commit():
  输入: &EditorState + draft_text + &mut PatchState
  输出: CommitResult {
            patch: Option<Patch>,
            should_render: bool,    ← 告诉壳"要不要重绘"
            closed_paragraph_id,    ← 告诉壳"要关哪个"
        }

Shell 的 wasm_api::commit():
  ① 调 core commit
  ② 如果有 patch → apply 到 thread_local PatchState
  ③ 如果 should_render → schedule_render_frame()   ← 平台副作用
  ④ 如果需要持久化 → bridge::target_invoke()       ← 平台副作用
```

---

### 11.5 问题五：渲染管线中的「场景准备」遗漏

§5.6 图中画的是 `core::render::effective_plan::build() → Canvas`，
但实际代码中还有一步 **PreparedPageScene**，它是 overlay 收集的前提：

```
实际渲染调用链:

  wasm_api/viewer.rs  render_page()
    │
    ├→ init_page_context()                    ← page/runtime.rs
    │   └→ PreparedPageScene::build()         ← render/prepared_scene.rs
    │       ├→ paragraph_overlay 收集          ← 依赖 PatchState + EditorState
    │       └→ replacement_region 收集         ← 依赖 PatchState
    │
    └→ canvas.rs  render_page()
        └→ build_effective_render_plan()       ← render/effective_page_plan.rs
            └→ 使用 PreparedPageScene 作为输入
```

**`PreparedPageScene::build()` 会读取 `GLOBAL_PATCH_STATE` 和 `HOST_EDITOR_MODE`。**
这意味着：

1. 场景准备（PreparedPageScene）也是"纯计算"，但被绑在了 `init_page_context()` 里
   （后者还会写 `HOST_PAGE_STATE`），导致纯计算和状态写入混合
2. 迁移时需要把 `PreparedPageScene::build()` 的输入显式化：
   传入 `&PatchState` 和 `&EditorModeState` 而不是从全局读

---

### 11.6 问题六：14 个 thread_local 的完整清单

```
                         ┌─────────────────────────────────────────┐
                         │        thread_local! 全局状态清单        │
                         ├─────────────────────────────────────────┤
                         │                                         │
  编辑器群 (5 个):        │  ① SESSION_STATE     (Cell<SessionState>)│
                         │  ② ACTIVE_BLOCK_ID   (RefCell<Option>)  │
                         │  ③ HOST_EDITOR_MODE   (RefCell<         │
                         │     EditorModeState>)                   │
                         │  ④ HOST_EDITOR_HOST_  (RefCell<         │
                         │     RUNTIME_STATE>)   EditorHostRuntime)│
                         │  ⑤ DEBUG_TRACE_*      (RefCell<Vec>)    │
                         │                                         │
  渲染群 (4 个):          │  ⑥ HOST_PAGE_STATE   (RefCell<PageState>)│
                         │  ⑦ HOST_PREPARED_    (RefCell<Option>)  │
                         │     SCENE                               │
                         │  ⑧ HOST_PROGRESSIVE  (RefCell<Option>)  │
                         │     _RENDER_TASK                        │
                         │  ⑨ HOST_RENDER_*     (present/runtime)  │
                         │                                         │
  查看器群 (3 个):        │  ⑩ HOST_ZOOM_STATE  (RefCell<ZoomState>)│
                         │  ⑪ VIEWER_SESSION   (viewer/session)    │
                         │  ⑫ RENDER_HOST_*    (render/host_runtime)│
                         │                                         │
  功能群 (2 个):          │  ⑬ FIND_CONTROLLER  (find/controller)   │
                         │  ⑭ COMMENT_REVIEW   (viewer/comment)    │
                         │                                         │
  全局 OnceLock (1 个):   │  ⑮ GLOBAL_PATCH_    (OnceLock<RwLock<  │
                         │     STATE>)           GlobalPatchState>) │
                         └─────────────────────────────────────────┘

  问题总结：
  ● 编辑器相关就占了 5 个 → 合并为 1 个 EditorEngine
  ● ①② 和 ③ 是重叠的状态机 → 应该合一
  ● ⑮ 用 OnceLock<RwLock> 在 WASM 单线程环境完全多余
    → 应改为 thread_local! RefCell 或直接合并到 core struct
  ● 14+1 = 15 个全局状态互相读写，没有文档说明依赖关系
```

---

### 11.7 问题总结与路线 B 的修正

| # | 问题 | 严重度 | 路线 B 如何修正 |
|---|------|--------|---------------|
| 1 | 双状态机无同步，commit panic 可致死锁 | **高** | Core 统一为一个 `EditorEngine` struct |
| 2 | commit 涉及 4 个存储 6 次写入非事务 | **高** | Core commit 是纯函数，返回结果由壳一次性 apply |
| 3 | mode.rs 纯代理层无逻辑 | 低 | 消除，直接引用 core |
| 4 | commit 有隐式渲染副作用 | **中** | CommitResult 增加 `should_render` 字段，壳决定调度 |
| 5 | PreparedPageScene 混合状态写入和纯计算 | **中** | 场景准备入参显式化，迁入 core |
| 6 | 15 个全局状态无依赖文档 | **中** | 编辑器 5→1，渲染 4→2，总计 15→~8 |

### 修正后的 commit 流程图

```
╔═══════════════════ pdf-viewer-core ═══════════════════╗
║                                                       ║
║  pub struct EditorEngine {                            ║
║      state: SessionState,     ← 合并原 ①②③           ║
║      active_block: Option<BlockContext>,               ║
║      live_state: Option<LiveEditorParagraphState>,     ║
║      committing: bool,        ← 合并原 ④              ║
║  }                                                    ║
║                                                       ║
║  impl EditorEngine {                                  ║
║      pub fn commit(                                   ║
║          &mut self,                                   ║
║          draft_text: &str,                            ║
║          patch_state: &mut PatchState,                ║
║      ) -> CommitResult {                              ║
║                                                       ║
║          // 1. 状态检查                                ║
║          if self.state != EditingBlock { return Err }  ║
║          if self.committing { return Err }             ║
║                                                       ║
║          // 2. 原子操作                                ║
║          self.committing = true;                      ║
║          let patch = build_patch(&self.live_state, ..);║
║          if let Some(ref p) = patch {                 ║
║              patch_state.apply(p);  ← 唯一写入点      ║
║          }                                            ║
║                                                       ║
║          // 3. 状态转换                                ║
║          self.live_state = None;                      ║
║          self.active_block = None;                    ║
║          self.state = Viewing;                        ║
║          self.committing = false;                     ║
║                                                       ║
║          // 4. 纯返回值，无副作用                       ║
║          CommitResult {                               ║
║              patch,                                   ║
║              should_render: true,                     ║
║              closed_paragraph_id,                     ║
║          }                                            ║
║      }                                                ║
║  }                                                    ║
║                                                       ║
║  ✅ 如果 build_patch panic:                           ║
║     self.committing = true → 但状态机整体一致          ║
║     调用方可以 catch_unwind 或者检查 committing 复位    ║
║  ✅ 不可能出现 A=EditingBlock + C=committing 的死锁    ║
╚═══════════════════════════════════════════════════════╝
         │
         │ CommitResult
         ▼
┌──── pdf-viewer-ui (薄壳) ────────────────────────────┐
│                                                      │
│  EDITOR.with(|engine| {                              │
│      PATCHES.with(|patches| {                        │
│          let result = engine.borrow_mut()             │
│              .commit(draft_text, &mut patches);       │
│                                                      │
│          // 壳处理副作用:                              │
│          if result.should_render {                    │
│              schedule_render_frame();  ← 平台副作用    │
│          }                                           │
│          if let Some(patch) = result.patch {          │
│              bridge::target_invoke("save", patch);    │
│          }                                           │
│      })                                              │
│  })                                                  │
└──────────────────────────────────────────────────────┘
```

这样，**核心保证了状态一致性**，壳只负责"把结果投递到平台"。

---

## 12. 线程优化：WASM 侧阻塞问题与 Web Worker 路线图

### 12.1 现状审计

#### Tauri 端 ✅ 已正确处理

18 处 `tokio::task::spawn_blocking`，所有重 CPU 操作不阻塞事件循环：

```
spawn_blocking 使用分布 (src-tauri/):
├── interfaces/pdf.rs          (6处) lopdf 加载、PDF读取、预览
├── page_model_service.rs      (3处) vector model 构建
├── pdf_write_service.rs       (3处) PDF 回写
├── page_annotation.rs         (2处) 注解处理
├── document_service.rs        (2处) 文档操作
├── geometry_service.rs        (1处) 布局推理
└── pdf_read_service.rs        (1处) PDF 读取
```

#### WASM 端 ❌ 完全阻塞主线程

```
搜索结果:
  spawn / Worker / web_worker / spawn_local → 0 命中

整个 WASM 侧没有任何 Web Worker。
所有计算都在浏览器主线程上同步执行。
```

### 12.2 主线程阻塞热点

```
浏览器主线程
 │
 ├─ render_page() ─────────────────── 同步！全量渲染
 │   ├─ init_page_context()           同步
 │   │   └─ PreparedPageScene::build() 同步（遍历所有 overlay）
 │   └─ canvas.render_page()          同步（所有 Canvas2D 调用）
 │
 ├─ commit() ──────────────────────── 同步！
 │   ├─ build_active_editor_patch()   同步
 │   ├─ record_patch()    同步
 │   └─ schedule_render_frame()       同步（仅调度，但计划生成是同步的）
 │
 ├─ start_progressive_render() ────── 同步！
 │   └─ ProgressiveVectorRenderTask::build()  同步（生成全量计划）
 │
 └─ step_progressive_render() ─────── 同步，但有预算控制
     └─ render_vector_slice(budget_ms) ← 唯一的缓解措施
         └─ 用 requestAnimationFrame 在帧间让出
```

**唯一的缓解措施**是渐进渲染：TS 层用 `requestAnimationFrame` 把绘制分成多帧。
但**计划生成本身**（overlay 收集、有效渲染计划构建、视口裁剪计算）都是同步完成的。

### 12.3 大文档影响估算

```
大 PDF（100+ 页，每页 500+ 元素）时:

  用户点击"编辑" 
    → openBlock() → overlay 收集 + scene 构建 → ⏱️ 50-200ms 卡顿
  用户按键输入
    → syncInput() → scene 重建 → ⏱️ 20-80ms 卡顿
  用户点击"保存"
    → commit() → patch 构建 + apply + render schedule → ⏱️ 30-100ms 卡顿
  页面渲染
    → render_page() → 全量同步 → ⏱️ 100-500ms 卡顿

  在此期间 UI 完全冻结（无法滚动、无法点击）
```

### 12.4 为什么当前架构无法引入 Web Worker

```
当前（不可能 offload）:

  WASM 主线程:
    wasm_api → thread_local 状态 → 纯计算 → Canvas → 全部耦合
                    ↑
                    └─ thread_local 不能跨线程传递
                    └─ Canvas2D 只能在主线程操作
                    └─ 纯计算和平台调用混在同一个调用链

  结论: 无法把任何计算挪到 Worker，因为它们和平台状态纠缠在一起
```

### 12.5 路线 B 如何解锁 Web Worker

路线 B 的 "Core 无平台依赖" 设计**天然支持** Web Worker offload：

```
路线 B 完成后:

  主线程 (UI thread):              Worker 线程 (Compute thread):
  ┌────────────────────┐          ┌───────────────────────────┐
  │ wasm_api/ (薄壳)   │          │ pdf-viewer-core.wasm (纯)  │
  │ canvas/ (绘制)     │          │                           │
  │ runtime/ (状态持有) │          │  无 thread_local           │
  │                    │  ──msg→  │  无 Canvas/DOM             │
  │ 1. 序列化状态快照   │          │  无 wasm_bindgen           │
  │ 2. postMessage     │          │                           │
  │                    │  ←msg──  │  build_effective_plan()    │
  │ 3. 收到计划        │          │  collect_overlays()        │
  │ 4. Canvas 绘制     │          │  commit()                  │
  └────────────────────┘          │  build_scene()             │
                                  └───────────────────────────┘

  因为 core 是纯函数（无平台依赖），
  可以在 Worker 里单独实例化一份 core WASM，
  主线程只负责状态持有和 Canvas 绘制。
```

### 12.6 分阶段实施计划

```
Phase W0: 路线 B 完成后可立即评估
─────────────────────────────────
  前提: Core 已经是纯函数，无 thread_local
  评估: 用 Performance API 度量各操作实际耗时
  判断: 哪些操作超过 16ms（一帧）需要 offload

Phase W1: 渐进计划生成（不需要 Worker）
─────────────────────────────────────
  把 build_effective_plan() 改为分块/增量:
  - 输入不变时缓存结果
  - overlay 变化时只更新受影响的部分
  收益: 消除计划生成的卡顿（最常见的热点）
  风险: 低（纯算法优化，不改架构）

Phase W2: Web Worker offload 纯计算
─────────────────────────────────────
  把 core 在 Worker 中实例化:
  - 主线程 postMessage(state_snapshot)
  - Worker 调用 core 函数
  - Worker postMessage(render_plan) 回主线程
  - 主线程用 plan 驱动 Canvas 绘制
  收益: 计算完全不阻塞主线程
  风险: 中（需要序列化开销，可能需 SharedArrayBuffer）

Phase W3: OffscreenCanvas（可选）
─────────────────────────────────
  把 Canvas 绘制也挪到 Worker:
  - 使用 OffscreenCanvas API
  - 主线程只负责 DOM 事件和状态
  收益: 绘制也不阻塞主线程
  风险: 高（浏览器兼容性、API 限制）
```

### 12.7 优先级矩阵

| 优化 | 收益 | 依赖路线 B？ | 建议时机 |
|------|------|------------|---------|
| 渐进计划生成（分块 build） | 消除计划生成卡顿 | ❌ 不依赖 | 可现在做 |
| commit 纯函数化 | 消除提交卡顿 + 可测试 | ✅ 依赖 B2 | B2 完成后 |
| Web Worker offload 纯计算 | 消除所有计算卡顿 | ✅ 依赖 B 完成 | B 完成后 |
| OffscreenCanvas | 消除绘制卡顿 | ✅ 依赖 W2 | W2 验证后 |

> **路线 B 的隐藏价值**：不只是代码组织更清晰，还为性能优化打开了大门。
> 当前架构因为平台耦合，根本无法做 Worker offload。
