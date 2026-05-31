# 项目架构现状审计

> 生成时间：2026-05-09
> 目的：盘清当前三个 Rust crate + TypeScript bridge 的职责边界、API 出口散布情况、类型重复等问题，为下一步整体重构提供依据。

---

## 1. 项目全景

```
pdf-viewer-standalone/
├── crates/
│   ├── pdf-viewer-core/   ← 纯 Rust 库，无 wasm_bindgen（可选 feature）
│   └── pdf-viewer-ui/     ← WASM crate，编译为 .wasm 给浏览器 WebView 使用
├── src-tauri/             ← Tauri native 后端（Rust，异步，文件 I/O + PDF 解析 + 渲染）
└── src/                   ← TypeScript 前端（Vite + WebView）
```

### 1.1 数据流

```
┌─────────────────────────────────────────────────────────┐
│                   TypeScript (src/)                      │
│                                                         │
│  bridge/viewer/  bridge/editor/  bridge/render/  ...    │
│       │                │               │                │
│       ▼                ▼               ▼                │
│  ┌─────────┐    ┌───────────┐   ┌─────────────┐        │
│  │ Tauri   │    │ WASM      │   │ WASM        │        │
│  │ invoke  │    │ (sync)    │   │ (sync)      │        │
│  └────┬────┘    └─────┬─────┘   └──────┬──────┘        │
└───────┼───────────────┼────────────────┼────────────────┘
        │               │                │
        ▼               ▼                ▼
  ┌───────────┐   ┌───────────────────────────┐
  │ src-tauri │   │      pdf-viewer-ui        │
  │ (native)  │   │       (WASM)              │
  │           │   │                           │
  │ interfaces│   │  wasm_api/  ← 主 API 出口 │
  │ /pdf.rs   │   │  viewer/facade.rs ← 死码！│
  │ (42 cmd)  │   │  zoom/facade.rs   ← 死码！│
  │           │   │  editor/editor_api.rs     │
  └─────┬─────┘   └─────────────┬─────────────┘
        │                       │
        ▼                       ▼
  ┌─────────────────────────────────────┐
  │         pdf-viewer-core             │
  │  models / algorithms / text /       │
  │  geometry / persistence / render    │
  └─────────────────────────────────────┘
```

---

## 2. API 出口散布问题

### 2.1 WASM 出口（pdf-viewer-ui → 浏览器 JS）

| 声明位置 | `#[wasm_bindgen]` 数量 | TS 调用方 | 状态 |
|----------|----------------------|-----------|------|
| `wasm_api/viewer.rs` | **65** | `bridge/viewer/`, `bridge/render/` | ✅ 主力，但是"神文件" |
| `wasm_api/document.rs` | **39** | `bridge/document/`, `bridge/editor/` | ✅ 主力 |
| `editor/editor_api.rs` | **2** (`EditorSession` struct) | `bridge/editor/new/api.ts` | ✅ 已重构 |
| `viewer/facade.rs` | **13** | **无** | ❌ **全部死码** |
| `zoom/facade.rs` | **9** | **无** | ❌ **全部死码** |
| `bridge/mod.rs` | 1 (`on_debug`) | 内部回调 | ✅ |
| `render/canvas.rs` | 1 (`CanvasRenderer`) | 内部 | ✅ |
| `runtime.rs` | 1 | 初始化 | ✅ |

**问题：**
- `viewer/facade.rs` 和 `zoom/facade.rs` 共 22 个 wasm_bindgen 函数完全无人调用，是之前"frozen v1"设计的产物，但 TS 实际走的是 `wasm_api/viewer.rs`
- `wasm_api/viewer.rs` 65 个函数混合了 5 个不相关的领域（渲染、缩放、页面上下文、帧缓存、编辑器投影），是最严重的"神文件"
- `wasm_api/editor.rs` 已清空（只剩注释），可删除
- `wasm_api/search_facade.rs` 只有一行 `pub use`

### 2.2 Tauri 命令出口（src-tauri → 前端 invoke）

| 声明位置 | `#[command]` 数量 | 分类 |
|----------|-----------------|------|
| `interfaces/pdf.rs` | **~40** | 文档打开/保存、页面读取、搜索、批注、评论、替换、几何计算 |

**问题：**
- 全部 40 个 command 平铺在一个 1009 行的文件中，无按领域分组
- `AppState` 包含 11 个 `Mutex<HashMap>` 字段，全部平铺在 `lib.rs`

---

## 3. pdf-viewer-core 审计

### 3.1 被谁使用？

| 消费者 | 引用数 | 文件数 | 主要使用内容 |
|--------|--------|--------|-------------|
| `pdf-viewer-ui` (WASM) | 105 | 44 | models, text/glyph_layout, geometry/bbox, render/paint_plan |
| `src-tauri` (native) | 63 | 16 | models, geometry/layout_engine, persistence, text/* |

### 3.2 core 中各模块的使用者分布

| core 模块 | WASM 端用？ | Tauri 端用？ | 说明 |
|-----------|:-----------:|:-----------:|------|
| `models.rs` (22KB) | ✅ | ✅ | **核心共享类型**：GlyphPaintPlan, EditorSession, LayoutParagraph 等 |
| `text/glyph_layout` | ✅ | ✅ | 字形布局计算 |
| `text/index_convert` | ✅ | ✅ | UTF-16 ↔ char 索引转换 |
| `text/list_semantics` | ✅ | ❌ | 列表语义检测 — 只 WASM 用 |
| `text/editable_segments` | ✅ | ❌ | 可编辑段提取 — 只 WASM 用 |
| `text/style_preservation` | ✅ | ❌ | 样式保持 — 只 WASM 用 |
| `geometry/bbox_utils` | ✅ | ✅ | 包围盒计算 |
| `geometry/layout_engine` | ❌ | ✅ | 布局推理 — 只 Tauri 用 |
| `geometry/reflow_engine` | ❌ | ✅ | 回流排版 — 只 Tauri 用 |
| `geometry/field_projection` | ❌ | ✅ | 字段投影 — 只 Tauri 用 |
| `persistence/*` | ❌ | ✅ | 持久化引擎 — 只 Tauri 用 |
| `render/paint_plan` | ✅ | ❌ | 绘制计划 — 只 WASM 用 |
| `render/renderer` | ❌ | ✅ | 渲染器 — 只 Tauri 用 |
| `document/*` | ❌ | ✅ | 文档模型 — 只 Tauri 用 |
| `algorithms/*` | ❌ | ✅ | 算法 — 只 Tauri 用 |
| `typography/*` | ❌ | ✅ | 字体解析 — 只 Tauri 用 |
| `analysis/*` | ❌ | ✅ | 分析 — 只 Tauri 用 |

### 3.3 core 的真正作用

**双端共享的**只有：
- `models.rs`（类型定义）
- `text/glyph_layout`（字形计算）
- `text/index_convert`（索引转换）
- `geometry/bbox_utils`（包围盒）

**其余 ~70% 的模块只有单端使用**。特别是：
- `geometry/layout_engine`, `geometry/reflow_engine`, `persistence/*`, `document/*`, `algorithms/*`, `typography/*` → **只有 Tauri 用**
- `text/list_semantics`, `text/editable_segments`, `render/paint_plan` → **只有 WASM 用**

### 3.4 类型重复问题

| 类型名 | 定义位置 | 说明 |
|--------|---------|------|
| `EditorSession` | `core/models.rs`（数据结构）+ `ui/editor/editor_api.rs`（WASM API struct） | **同名不同义**：core 的是段落数据，ui 的是 API 入口 |
| `VectorPageModel` | `core/models.rs` + `src-tauri/infrastructure/pdf/models.rs` | **重复定义**，字段可能有偏差 |
| `GlyphPaintPlan` | 只在 `core/models.rs` | ✅ 正确共享 |

---

## 4. 问题总结

### P0 — 立即删除的死码

| 文件 | 行数 | 问题 |
|------|------|------|
| `viewer/facade.rs` | 122 | 13 个 wasm_bindgen，零外部调用 |
| `zoom/facade.rs` | 89 | 9 个 wasm_bindgen，零外部调用 |
| `wasm_api/editor.rs` | 11 | 只有注释，空文件 |
| `wasm_api/search_facade.rs` | 2 | 一行 re-export，可内联 |

### P1 — 架构散布

| 问题 | 严重度 | 说明 |
|------|--------|------|
| `wasm_api/viewer.rs` 神文件 | 高 | 65 个函数覆盖 5 个领域，无法维护 |
| `interfaces/pdf.rs` 神文件 | 高 | 40 个 Tauri command 平铺 |
| facade 命名混乱 | 中 | `viewer/facade.rs`（死码）vs `wasm_api/viewer.rs`（实际 API）vs `present/facade.rs`（内部辅助） |
| `host_` 前缀冗余 | 低 | 模块路径已表达归属 |

### P2 — core 边界模糊

| 问题 | 说明 |
|------|------|
| ~70% 模块单端使用 | core 名义上是"共享"，实际大部分只给 Tauri 或只给 WASM 用 |
| `models.rs` 22KB 大杂烩 | 编辑器、渲染、文档、持久化类型全在一个文件 |
| `EditorSession` 同名冲突 | core 的数据结构 vs ui 的 WASM API struct |
| `VectorPageModel` 重复定义 | core 和 src-tauri 各一份 |

---

## 5. 建议的重构路径

### Phase A：清除死码（~1 小时）

1. 删除 `viewer/facade.rs`、`zoom/facade.rs`
2. 删除 `wasm_api/editor.rs`（空文件）
3. 内联或删除 `wasm_api/search_facade.rs`
4. 更新 `mod.rs` 声明
5. 验证 build + E2E

### Phase B：拆分神文件（~2-3 小时）

1. 将 `wasm_api/viewer.rs` 的 65 个函数按领域拆分为：
   - `wasm_api/render.rs`（渐进渲染 ~8 个）
   - `wasm_api/frame.rs`（帧管理/缩放 ~15 个）
   - `wasm_api/page_context.rs`（页面上下文 ~5 个）
   - `wasm_api/viewer.rs`（瘦身后 ~20 个纯查看器函数）
   - `wasm_api/projection.rs`（编辑器投影 ~3 个）
2. 将 `interfaces/pdf.rs` 拆分为多个文件

### Phase C：core 边界重新划分（需讨论）

两个方向：

**方案 C1：保留 core，但瘦身**
- 只留真正双端共享的：`models`（拆小）、`text/{glyph_layout, index_convert}`、`geometry/bbox_utils`
- 将只有 Tauri 用的模块移回 `src-tauri`
- 将只有 WASM 用的模块移回 `pdf-viewer-ui`

**方案 C2：删除 core，按需内联**
- 共享类型放到 workspace-level `shared-types` crate（纯 struct + serde，无逻辑）
- 算法/引擎各归各家
- 优点：消除"边界不清"问题
- 缺点：需要大量 move + 修改 import path

### Phase D：结构体化 API（长期，沿用 EditorSession 模式）

将 `wasm_api/` 中的平铺函数逐步改为 struct 方法：
- `RenderPipeline`、`ZoomController`、`PageContext`、`FrameManager`
- 参照已完成的 `EditorSession` 模式

---

## 6. 之前重构方案的覆盖范围

| 范围 | 方案中提到？ | 实际执行？ |
|------|:-----------:|:---------:|
| `editor/facade.rs` → `EditorSession` | ✅ 提到 | ✅ 已完成 |
| `viewer/facade.rs` → `Viewer` struct | ✅ 提到（P1） | ❌ 未执行，且是死码 |
| `zoom/facade.rs` → `ZoomController` | ✅ 提到（P3） | ❌ 未执行，且是死码 |
| `wasm_api/viewer.rs` 拆分 | ✅ 提到（P1） | ❌ 未执行 |
| `document/facade.rs` → `DocumentSession` | ✅ 提到（P1） | ❌ 未执行 |
| `interfaces/pdf.rs` 拆分 | ❌ 未提到 | — |
| `pdf-viewer-core` 边界问题 | ❌ 未提到 | — |
| `src-tauri` 架构 | ❌ 未提到 | — |
| TypeScript bridge 层 | 部分提到 | 部分完成 |

**结论：之前的方案只覆盖了 `pdf-viewer-ui` 中的 editor 模块（P0）。`src-tauri`、`pdf-viewer-core`、以及 `pdf-viewer-ui` 中非 editor 的部分（viewer、zoom、render）均未涉及。**
