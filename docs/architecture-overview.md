# 架构总览

> v1 · 2026-05-06 · 与 `docs/api-contract.md` 配套阅读

## 1. 三层运行时

```
┌──────────────────────────────────────────────────────────────┐
│  TS Frontend (Vite + 浏览器/Tauri WebView)                    │
│  src/                                                         │
│  · UI 组件、事件处理、视觉层                                  │
│  · src/bridge/*_facade.ts → 调 Rust WASM                      │
│  · @tauri-apps/api invoke() → 调 Tauri command                │
└────────────┬───────────────────────────────────┬──────────────┘
             │ wasm_bindgen                      │ Tauri IPC
             │ (camelCase js_names)              │ (snake_case commands)
             ▼                                   ▼
┌──────────────────────────────────────┐  ┌──────────────────────┐
│  Rust WASM Module (in-page UI core)  │  │  Tauri Backend       │
│  crates/pdf-viewer-ui/               │  │  src-tauri/          │
│  · 编辑器、缩放、渲染管线、状态机    │  │  · 文件 IO           │
│  · 9 个域 facade（见 §3）            │  │  · PDF 解析/写出     │
│  · 通过 invoke 异步调用 Tauri        │  │  · 系统对话框        │
└──────────────────────────────────────┘  └──────────────────────┘
```

### 为什么是三层？

- **TS 层**：UI 框架（React-style 但当前是手写 controllers）。负责 DOM、Canvas 2D 命令、事件分发。
- **WASM 核心**：所有"决策"逻辑——光标定位、缩放策略、渲染计划、文本编辑、撤销/重做、补丁队列。前后端共享数据结构通过 `pdf_viewer_core` crate 提供模型。
- **Tauri 后端**：磁盘 IO、PDF 编解码（基于 `lopdf` + `vello`/`wgpu`）、系统集成（文件对话框、shell）。

## 2. 数据流（核心场景）

### 2.1 打开 PDF

```
User → openButton click
  TS: toolbarController.handleOpen()
    → invoke('pick_pdf') ──────────────► Tauri: 文件对话框
                                          ◄── path
    → facadeDocumentOpen({path}) ─────► WASM: 解析、初始化 viewer session
                                          ◄── { pageCount, initialZoom, … }
    → facadeViewerSetDocument(...)    ─► WASM: 绑定 session
    → renderFlow.requestFrame()       ─► WASM: 决策渲染计划 → 触发 canvas 绘制
```

### 2.2 文本编辑（核心已修复）

```
User → 点击段落 → 键入字符 → ⌫
  TS (editor_host.ts): captureKey
    → facadeApplyCommand({command:'deleteContentBackward'}) ─► WASM:
                                                                 1. 应用编辑到 draft
                                                                 2. 重建 cluster runs
                                                                 3. 计算新 caret
                                                                 4. 输出 EditorOpResult
        ◄────────────────────────────────────────────────────── { newText, newCaret, frameRequest }
    → facadePaintCanvas(canvas, zoom, newText, newCaret)        ─► WASM: 直接绘制 editor canvas
    → renderFlow.requestFrame(frameRequest)                     ─► WASM: 后台再渲染整页
```

历史 bug：曾经"删字"用 `apply_editor_input` 路径而非 `applyCommand`，导致 cluster 不重建、render 不刷新。**已通过统一走 facade 修复**。

### 2.3 保存编辑

```
User → Ctrl+S
  TS: facadeSaveSession(path, pageIndex)
    → WASM: 收集所有持久化 patches
    → invoke('apply_region_patches', {path, pageIndex, patches}) ─► Tauri: 写盘
                                                              ◄── 成功/失败
    → WASM: 清空 patch 队列
```

## 3. 域 Facade 全景

每个域的入口都在 `crates/pdf-viewer-ui/src/<domain>/facade.rs` 或 `wasm_facade.rs`，对外 js_name 都是 `<domain>Facade<Verb>`（camelCase）。

| 域 | Rust 文件 | TS 文件 | Stable | Stub |
|---|---|---|---|---|
| `editor.*` | `editor/facade.rs` | `editor_facade.ts` | 22 | 8 |
| `document.*` | `document/facade.rs` | `document_facade.ts` | 13 | 14 |
| `viewer.*` | `viewer/facade.rs` | `viewer_facade.ts` | 9 | 4 |
| `find.*` | `find/facade.rs` | `find_facade_v2.ts` | 4 | 4 |
| `review.*` | `review/facade.rs` | `review_facade_v2.ts` | 5 | 2 |
| `comment.*` | `comment/facade.rs` | `comment_facade.ts` | 17 | 4 |
| `render.*` | `render/wasm_facade.rs` | `render_facade_v2.ts` | 11 | 4 |
| `zoom.*` | `zoom/facade.rs` | `zoom_facade.ts` | 4 | 5 |
| `annotation.*` | `annotation/facade.rs` | `annotation_facade.ts` | 0 | 14 |
| **合计** | — | — | **85** | **59** |

> Stable APIs are FROZEN（命名+签名不能改），Stub APIs 命名 FROZEN（实现可改）。

详见 `docs/api-contract.md` §3。

## 4. 模块组织（Rust）

```
crates/pdf-viewer-ui/src/
├── lib.rs              # 模块入口 + 历史兼容别名
│
├── annotation/         # ① 域：annotation.* facade
├── comment/            # ② 域：comment.*
├── document/           # ③ 域：document.*
├── editor/             # ④ 域：editor.*（最大，~50 文件）
├── find/               # ⑤ 域：find.*
├── review/             # ⑥ 域：review.*
├── viewer/             # ⑦ 域：viewer.* + comment_review/find 子模块
├── render/             # ⑧ 域：render.* + 渲染管线核心
├── zoom/               # ⑨ 域：zoom.* + 交互/preview/state
│
├── host/               # 全局会话状态（HOST_VIEWER_SESSION 等 thread_local）
├── page/               # 页面级数据结构
├── present/            # 帧呈现计划（FramePlanRequest 等）
├── runtime/            # WASM runtime hooks
├── state_manager.rs    # 文档/补丁全局状态
├── style_mapper.rs     # 字体/样式映射
├── utils/              # 通用工具
├── viewport_culling.rs # 视口剔除
├── viewport_refresh.rs # 视口刷新
├── wasm_api/           # 历史 wasm_bindgen 入口（部分被 facade 取代，标 LEGACY）
└── bridge/             # （已废弃，逻辑已搬移到上述域）
```

### 命名约定

- **历史的 lib.rs 别名**：30+ 个 `pub use editor::activation as editor_activation_workflow`，让其他模块使用扁平路径。**新代码请使用规范的 `crate::editor::activation` 路径**，别名将在 Phase 3 清理。
- **Workflow vs Domain**：旧设计把每个动作叫 "*_workflow"，新设计按域归并到 `<domain>::<topic>::<verb>` 三段命名。

## 5. 模块组织（TS）

```
src/
├── main.ts             # 应用入口
├── style.css
├── pdf-app/            # 主组件
│
├── bridge/             # WASM facade + Tauri invoke 封装
│   ├── *_facade.ts     # 每域一个：editor / document / viewer / ...
│   ├── editor_host.ts  # 编辑器视图 controller（最复杂）
│   ├── render_flow.ts  # 渲染调度
│   ├── viewer_session.ts
│   ├── zoom_controller.ts
│   ├── pdf_*_controller.ts
│   └── wasm_loader.ts  # 单例 WASM 实例 getter
│
└── components/         # UI 组件
    ├── toolbar/
    ├── sidebar/
    └── ...
```

Bridge 已按域重组为子目录（`editor/` `render/` `viewer/` `find/` `comment/` `review/` `ai/` `document/` `zoom/`）。

## 5.5 模块组织（Tauri 后端）

```
src-tauri/src/
├── lib.rs              # AppState + Tauri builder + command 注册
├── state.rs            # LoadingStatus enum
│
├── interfaces/         # Tauri command 层（IPC 入口）
│   └── pdf.rs          # 31 个 #[command] 函数
│
├── application/        # 应用业务层
│   └── pdf/
│       ├── page_annotation.rs
│       ├── page_replace.rs
│       ├── page_search.rs
│       ├── page_context.rs
│       ├── comment_review.rs
│       └── region_patch_service.rs
│
└── infrastructure/     # 基础设施层
    ├── pdf/            # PDF 处理核心
    │   ├── cache.rs            # 统一缓存管理（page/layout/image/font）
    │   ├── document_service.rs # 文档打开/保存/撤销/重做
    │   ├── page_model_service.rs # 页面模型构建
    │   ├── geometry_service.rs # 编辑器几何计算
    │   ├── engine.rs           # 兼容性 re-export
    │   ├── font/               # 字体子系统
    │   │   ├── catalog.rs      # 系统字体枚举
    │   │   ├── matching.rs     # 字体匹配
    │   │   ├── metrics.rs      # 字体度量
    │   │   ├── ttc.rs          # TTC 解析
    │   │   └── embedded_program.rs # 嵌入字体规范化
    │   ├── models.rs           # infra 层数据结构
    │   ├── pdf_read.rs         # lopdf 读取
    │   ├── pdf_write.rs        # lopdf 写出
    │   ├── vector_engine.rs    # 矢量页面模型构建
    │   ├── vello_renderer.rs   # GPU 渲染
    │   └── ...                 # 其余辅助模块
    └── pdf_read/               # 只读阅读模式
        ├── facade.rs
        ├── backend.rs
        ├── scanned_backend.rs
        └── vector_backend.rs
```

## 6. 关键不变量

| 不变量 | 强制位置 | 备注 |
|---|---|---|
| 编辑器 draft 文本与 cluster runs 必须同步 | `editor/runtime.rs::sync_active_editor_input` | 否则光标错位 |
| 每次 commit 必须 bump revision | `viewer/runtime.rs::note_document_mutation` | 帧缓存以此判失效 |
| Display zoom != Render zoom | `zoom/state.rs` | display 是 CSS scale，render 是 raster scale |
| 帧 token 是单调递增的 u32 | `present/runtime.rs` | abort 用 token 比较 |
| Tauri command 命名为 snake_case | `src-tauri/src/lib.rs` | Tauri 框架要求 |
| WASM js_name 命名为 camelCase | `<domain>/facade.rs` | 与 TS 风格统一 |
| Stub API 返回 `{implemented:false, error:string}` | 所有 facade 内 `stub()` 函数 | 标准化前端检测 |

## 7. 构建与发布

```pwsh
# 1. 重新生成 WASM 包
npm run wasm:pdf-viewer-ui      # → crates/pdf-viewer-ui/pkg/

# 2. 类型检查 + Vite 构建
npm run build                    # → dist/

# 3. 开发服务器
npm run dev                      # → localhost:6173

# 4. Tauri 桌面打包
npm run tauri build
```

## 8. 进一步阅读

- `docs/api-contract.md` — API 命名规范、稳定性等级、所有域 API 清单、弃用流程
- `docs/development-guide.md` — 如何添加 API、新模块、测试
- `docs/architecture-principles.md` — 架构演进史与设计决策
- `docs/architecture-review.md` — 当前架构审查 + phase checkboxes（最新进度记录于此）
- `docs/archive/` — 历史 ADR 与设计文档归档（含 `progress-2026-05-06-facade-era.txt`）
