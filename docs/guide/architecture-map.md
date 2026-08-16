# Sovereignty PDF Viewer -- 架构图谱

> 生成于 2026-08-16，基于 `refactor/architecture-improvements` 分支的工作树。
> 本文档中的每一条论断都可追溯到当前代码树中的具体文件路径。

## 1. 四层总览

```
index.html / main.ts          UI 外壳（DOM、事件接线、?file= 参数）
       |
   src/bridge/                TS 桥接层（约 12,000 行）
   (pdf_runtime.ts = 根)      运行时组装、渲染循环、缩放、编辑
       |
   crates/pdf-viewer-ui/      WASM crate（约 5,500 行 Rust + JS 胶水）
   (pkg/pdf_viewer_ui)        导出结构体: DocumentSession, ViewerSession,
       |                       EditorSession, FindSession, ReviewSession,
       |                       PagePresentationRuntime，以及自由函数
       |                       sync_host_layout, handle_wheel_zoom_host, ...
       |
   crates/pdf-viewer-core/    纯 Rust 库（约 4,000 行）
                               领域模型、文本状态、渲染计划、
                               批注、编辑命令、排版
       |
   src-tauri/                 桌面后端（约 10,700 行 Rust）
   (lib.rs = Tauri 入口)      30 个 IPC 命令、PDF 解析（lopdf + pdf-rs）、
                               字体引擎、读写管线、工作副本、图像缓存
                               （vello GPU 渲染器已于 2026-08-15 移除，见 §7）
```

数据**向下**流（TS 调 wasm 函数，wasm 经 `target_invoke` 调 Tauri 命令），
也**向上**流（Tauri 返回数据，wasm 加工，TS 渲染到 DOM）。

---

## 2. 第 1 层 -- `src-tauri/`（桌面后端）

### 2.1 命令面（30 个命令）

注册于 `src-tauri/src/lib.rs:82-113`。全部位于 `src/interfaces/pdf/` 之下，
并经 `src/interfaces/pdf/mod.rs:22-29` 扁平再导出。

**文档生命周期：**
| 命令 | 文件 | 用途 |
|---|---|---|
| `open_pdf` | `interfaces/pdf/document.rs:8` | 按路径加载 PDF，缓存 `Arc<lopdf::Document>`，返回页数 |
| `clear_cache` | `interfaces/pdf/document.rs:24` | 释放所有文档、工作副本、缓存 |
| `save_pdf` | `interfaces/pdf/document.rs:44` | 应用修改（region patches + text reflows），保存到磁盘 |
| `undo` / `redo` | `interfaces/pdf/document.rs:66/78` | 从历史栈交换文档快照 |
| `pick_file` | `interfaces/pdf/system.rs:44` | 原生文件对话框，返回 `Option<String>` |

**页面数据：**
| 命令 | 文件 | 用途 |
|---|---|---|
| `read_preview` | `interfaces/pdf/page.rs:15` | 轻量页模型：尺寸、扫描/文本类型、光栅预览 URL |
| `read_page_asset_bundle` | `interfaces/pdf/render.rs:18` | 向量模型 + 字形绘制计划的组合包 |
| `read_vector` | `interfaces/pdf/render.rs:78` | 完整向量页模型 |
| `read_glyph_plan` | `interfaces/pdf/render.rs:124` | 文本渲染用的字形绘制计划 |
| `read_images` | `interfaces/pdf/render.rs:158` | 内嵌图片，base64 data URL |
| `diagnose_page` | `interfaces/pdf/render.rs:163` | 调试用 JSON：页字典键、算子计数 |

**编辑操作：**
| 命令 | 文件 | 用途 |
|---|---|---|
| `apply_region_patches` | `interfaces/pdf/replace.rs:9` | 应用文本替换补丁 |

**搜索：**
| 命令 | 文件 | 用途 |
|---|---|---|
| `find_in_page` | `interfaces/pdf/search.rs:10` | 单页 region 级文本搜索 |
| `find_in_document` | `interfaces/pdf/search.rs:35` | 全文档 region 级文本搜索 |

**批注：**
| 命令 | 文件 | 用途 |
|---|---|---|
| `read_annotation_targets` | `interfaces/pdf/annotation.rs:10` | 可批注 region 及其包围盒 |
| `read_highlights` / `apply_highlight` / `delete_annotation` | `interfaces/pdf/annotation.rs:22/31/40` | 高亮 CRUD |
| `read_comments` / `read_comment_review` / `apply_comment` / `apply_comment_update` | `interfaces/pdf/comment.rs:11/20/29/38` | 评论 CRUD + 汇总 |

**系统：**
| 命令 | 文件 | 用途 |
|---|---|---|
| `set_log_level` / `clear_pdf_event_log` / `read_pdf_event_log` | `interfaces/pdf/system.rs:12/17/22` | 日志 / 事件环形缓冲 |
| `set_page_asset_test_delay_ms` | `interfaces/pdf/system.rs:27` | 调试：人为加 asset 准入延迟 |
| `terminal_log` / `resolve_asset_url` | `interfaces/pdf/system.rs:32/37` | 前端日志桥 / 文件系统路径转 asset URL |
| `create_demo_pdf` | `interfaces/pdf/system.rs:7` | 写一个硬编码的单页 PDF |

### 2.2 模块地图（`src-tauri/src/infrastructure/pdf/`）

**读取管线：**
| 模块 | 行数 | 职责 |
|---|---|---|
| `pdf_loader.rs` | 361 | 宽容加载器：strict -> load_mem -> trailer 修复 |
| `pdf_read/content_parser.rs` | 654 | 完整 PDF 算子解释器（q/Q/cm、颜色、路径、BT/ET、XObject Do） |
| `pdf_read/image_builder.rs` | 317 | 图片解码（PNG predictor、JPEG 透传） |
| `pdf_read/path_resolver.rs` | 186 | 逐页路径解析，双重检查缓存 + 页锁 |
| `pdf_read/graphics_state.rs` | 42 | 图形状态栈类型 |
| `pdf_read/resource_reader.rs` | 61 | 父链资源展平 |
| `vector_engine.rs` | 454 | Display-list -> `NativeVectorPageModel`：行分组、段落推断、调色板、遮挡剔除（已禁用） |
| `preview_engine.rs` | 392 | 扫描/文本分类 + 最大图 JPEG 抽取 |
| `layout_engine.rs` | 161 | 空间语义 region/段落推断（从 core 移回） |
| `spatial_graph.rs` | 90 | 邻接图 + 连通分量 |
| `glyph_mapping.rs` | 290 | 字形数/位置/glyph-id 解析（vello 移除后保留的部分） |

**写入管线：**
| 模块 | 行数 | 职责 |
|---|---|---|
| `pdf_write/reflow.rs` | 778 | 内容流遍历器、`PdfTextState`、`ReflowCluster` -- 最深的写入逻辑 |
| `pdf_write/annotations.rs` | 264 | 高亮/评论批注字典构建器 |
| `pdf_write/mod.rs` | 290 | `lopdf::Document` 上的 `PdfDocExt` trait：14 种编辑操作 |
| `pdf_write/emitters.rs` | 157 | 延迟行的 PDF 文本算子发射 |
| `pdf_write/pages.rs` | 60 | 页删除/旋转/插入/元数据 |
| `region_materializer.rs` | 577 | region patches + text reflows -> 生效的 `TextReflowPatch` 计划 |

**字体引擎：**
| 模块 | 行数 | 职责 |
|---|---|---|
| `font/parse.rs` | 628 | CMap / `ParsedFont` / 从字体字典取宽度 |
| `font/match_mod.rs` | 510 | 系统字体替换（含 CJK、字重匹配） |
| `font/embed.rs` | 354 | TrueType 子集化并嵌入 PDF |
| `font/mod.rs` | 229 | `PdfTextWriteFont` 决策：复用原字体或嵌入系统回退 |
| `font/catalog.rs` | 243 | Windows 系统字体枚举 |
| `font/face.rs` | 165 | glyph-id 编码 |
| `font/ttc.rs` | 118 | TrueType Collection 抽取 |
| `font/metrics.rs` | 94 | cosmic-text 字形度量缓存 |
| `font/path.rs` | 76 | Douglas-Peucker 路径简化 |

**共享状态与顶层工具：**
| 模块 | 行数 | 职责 |
|---|---|---|
| `text_state.rs` | 213 | 共享文本状态字段（Tf/Tc/Tw/Tz/Tr 参数） |
| `text_matrix.rs` | 186 | 矩阵三件套（ctm/tm/tlm），读写两用 |
| `document_service.rs` | 423 | `open_pdf`/`save_pdf`/`rollback`/`redo`/释放；5 个单元测试 |
| `document_resolver.rs` | 132 | 工作副本管理器（`%TEMP%\working_{md5}.pdf`） |
| `cache.rs` | 79 | 3 个 lazy_static 全局缓存 + 失效辅助函数 |
| `color.rs` | 120 | 严格 hex 解析（写入路径）、cmyk/gray 转换（14 个测试） |
| `log_service.rs` | 266 | 分级日志、512 条事件环形缓冲、宏 |
| `commands.rs` | 181 | `PdfEditCommand` trait + 10 种命令类型 |

**同级目录：**
| 模块 | 行数 | 职责 |
|---|---|---|
| `pdf_fallback/`（同级目录） | ~780 | pdf-rs 兜底后端：`scanned_backend.rs`（514）、`classification.rs`（221）。2026-08-16 由 `pdf_read/` 改名，消除与 `pdf/pdf_read/` 的重名 |

> 2026-08-15 已移除：`vello_renderer.rs`（1130 行，死 GPU 渲染器）、
> `pdf_read/facade.rs` + `pdf_read/vector_backend.rs`（无调用方）、
> `page_classifier.rs`（无调用方）。见 §7。

### 2.3 文档生命周期（服务端）

**打开：** `open_pdf` -> `PdfDocumentService::open_pdf`（`document_service.rs:117`）：
缓存命中则直接返回页数。未命中则 `spawn_blocking(load_pdf_lenient)` -> 严格加载
-> load_mem -> trailer 修复重试 -> 按路径插入 `Arc<lopdf::Document>`。
若 lopdf 解出 0 页 -> 回退到 pdf-rs 的 `ScannedReadBackend`。
只返回页数；所有页面数据都是惰性拉取。

**渲染：** 两种产物：
1. **光栅预览**（`read_preview`）：预览缓存 -> `build_light_page_model`
   （扫描/文本分类 + 最大图解码）-> 图片缓存 UUID ->
   `http://pdfasset.localhost/<uuid>`，由 `lib.rs:27` 的自定义协议供图。
2. **向量数据**（`read_page_asset_bundle`）：按 revision 键控的三级缓存 ->
   `resolve_paths`（按文档指针地址键控的全局缓存）-> `parse_content_stream`
   （完整算子遍历）-> `build_vector_page_model_from_display_list`（分组 /
   调色板 / flip_y）。**由前端绘制这些数据。** vello 不在live 路径里。

**保存：** `save_pdf` -> `build_region_materialization_plan`（合并 region_patches
+ text_reflows）-> `apply_batch_reflow_to_doc`（内容流遍历器）->
`doc.save(path)` -> 替换缓存条目 -> 失效 page/layout 缓存。

### 2.4 后端状态

- **`AppState`**（在 `lib.rs:81` manage）：按域分组的 3 个子库--
  `docs`（文档缓存）、`cache`（页面中间产物 / 布局 / 预览缓存）、
  `history`（整个 `lopdf::Document` 的撤销/重做快照栈）。
  （vello 的 `renderer` 槽位已随死代码清扫移除。）
- **lazy_static 全局量：** `PDF_IMAGE_CACHE`、`PDF_FONT_PROGRAM_CACHE`、
  `PDF_RESOLVE_PATHS_CACHE`（按文档指针地址键控）、`WORKING_COPIES`、
  `PAGE_LOCKS`、`PDF_EVENT_LOG`。

---

## 3. 第 2 层 -- `crates/pdf-viewer-core/`（纯 Rust 库）

除 serde + log 外零外部依赖。没有 Tauri，没有 wasm-bindgen。

| 模块 | 职责 |
|---|---|
| `models/` | 核心领域类型：`NativeVectorPageModel`、marker/overlay 类型、批注模型 |
| `text/` | `TextState`、`TextMatrixCore` -- 读写两条路径统一共享字段与算子语义 |
| `render/` | `FramePlanRequest`、`paint_plan`、`effective_page_plan`、路径抑制 -- 渲染帧规划 |
| `edit/` | `document_plan`、段落场景、styled runs、文本变更管线 |
| `annotation/` | 批注/评论/高亮类型定义 |
| `geometry/` | 坐标/矩形基元 |
| `history/` | 文档编辑历史类型 |
| `common/` | 清洗（sanitize）函数、共享工具 |
| `persistence/` | 补丁/region 持久化类型 |
| `typography/` | 匹配器、布局段落函数 |

---

## 4. 第 3 层 -- `crates/pdf-viewer-ui/`（WASM crate）

依赖 `pdf-viewer-core` 的 `"wasm"` feature。经 `wasm-bindgen`
（target: web）导出给 JS。**宿主 target 上无法编译**（wasm 门控依赖：
`web-sys`、`js-sys`、`wasm-bindgen-futures`）。

### 4.1 导出的 WASM 结构体（单例会话句柄）

| 结构体 | 文件 | JS 类 | 用途 |
|---|---|---|---|
| `DocumentSession` | `document/document_api.rs` | `api.DocumentSession` | 打开/关闭 PDF、撤销/重做/旋转、region 补丁 |
| `ViewerSession` | `viewer/viewer_api.rs` | `api.ViewerSession` | 页面导航、缩放、当前页跟踪 |
| `EditorSession` | `editor/editor_api.rs` | `api.EditorSession` | 文本编辑：begin/hitTest/openBlock/commit/save |
| `FindSession` | `find/find_api.rs` | `api.FindSession` | 搜索编排 |
| `ReviewSession` | `review/review_api.rs` | `api.ReviewSession` | 审阅面板状态 |
| `PagePresentationRuntime` | `presentation/presentation_api.rs` | `api.PagePresentationRuntime` | 翻页准入、预取决策 |

### 4.2 关键自由函数（导出给 JS）

| 函数 | 文件 | 用途 |
|---|---|---|
| `sync_host_layout` | `host/layout.rs:44` | **缩放契约**：由 display+render 缩放算出 domWidth/domHeight/cssScale |
| `handle_wheel_zoom_host` | `zoom/wheel_host.rs` | 滚轮缩放决策：目标缩放、渲染决策、预览变换 |
| `step_preview_host` | `zoom/preview_step.rs` | RAF 驱动的平滑缩放预览插值 |
| `build_frame_plan` 及相关 | `present/plan_builder.rs` | 帧计划生命周期（peek/take/schedule/commit） |
| `build_glyph_paint_plan` | `render/paint_plan.rs` | 文本绘制用的字形渲染计划 |

### 4.3 模块地图

| 模块 | 职责 |
|---|---|
| `bridge.rs` | JS FFI 绑定（`target_invoke` -> `window.__TAURI__.core.invoke`） |
| `host/` | `layout.rs` -- sync_host_layout（缩放抵消契约：`dom_width * css_scale == display_width`） |
| `zoom/` | 滚轮缩放宿主、预览步进、缩放控制器状态 |
| `document/` | 会话 API、打开/关闭管线、编辑变更管线、补丁持久化 |
| `editor/` | 文本编辑会话、块命中测试、光标同步 |
| `render/` | Canvas overlay 渲染、marker/段落 overlay 绘制 |
| `present/` | 帧计划构建器、视口布局 |
| `presentation/` | 翻页准入、预取决策、cancel-gate |
| `viewer/` | 查看器会话状态、滚动/视口刷新 |
| `find/` | 搜索会话 |
| `review/` | 审阅会话 |
| `annotation/` | 批注会话 |
| `comment/` | 评论会话 |
| `common/` | 清洗函数、共享工具 |
| `models.rs` | 从 core 再导出 + UI 本地类型 |

### 4.4 测试（仅 wasm target）

| 模块 | 数量 | 覆盖 |
|---|---|---|
| `host/layout.rs` | 5 个 `#[wasm_bindgen_test]` | 缩放抵消保证、无闪烁、回退、清洗 |
| `editor/overlay/paragraph_overlay.rs` | 4 个 `#[wasm_bindgen_test]` | overlay 补丁/提交/marker/carries |
| **合计** | **9** | |

---

## 5. 第 4 层 -- `src/bridge/`（TypeScript 桥接层）

### 5.1 运行时组装根

`src/bridge/viewer/pdf_runtime.ts`（`createPdfViewerRuntime`，第 81 行）是
组装根。它通过晚绑定的 `let ... !` 赋值 + 依赖注入闭包（147-152、278、370、
458、480 行）构建并接线所有子运行时：

```
viewerSession（单例）
pagePresentationRuntime（wasm PagePresentationRuntime 的适配器）
framePlanAdapter（wasm 帧计划自由函数的适配器）
layoutSync（syncLayoutBox -> wasm syncHostLayout）
documentEditApi（变更面）
editorHost（段落编辑器生命周期）
zoomController（滚轮缩放宿主 + 平滑预览 + 提交）
renderFlow（渲染循环）
renderScheduler（渲染请求串行化）
documentRuntime（打开/重置/渲染）
resumeAiController, findController, commentController,
reviewController, annotationController
```

### 5.2 端到端流程

#### 打开（按钮 / URL 参数 / 拖拽）

```
index.html:242-264（内联脚本）
  -> window.__pdfOpenHandler (main.ts:49-63)
  -> Tauri invoke('pick_file') 或隐藏 input 点击
  -> handleFileOpen(path) (main.ts:38-46)
  -> api().openPdfFile(path) (pdf_viewer_api.ts:72)
  -> deps.openTextPdfFlow(path) (pdf_runtime.ts:577)
  -> documentRuntime.openTextPdfFlow(path) (pdf_document_runtime.ts:85)
    -> clearVectorHost() + clearEditorHost()  [取消渲染中的请求]
    -> session.open({path, ...})  [wasm DocumentSession]
      -> target_invoke("open_pdf") -> window.__TAURI__.core.invoke -> src-tauri
    -> renderCurrentPage() (pdf_document_runtime.ts:123)
```

#### 渲染

```
renderScheduler.requestRender(source, reason, ctx) (render_scheduler.ts:175)
  -> executeRender (pdf_runtime.ts:483-570)
    -> documentRuntime.renderCurrentPage(reason) (line 498)
      -> renderFlow.renderCurrentPage (render_flow.ts:509)
        -> framePlanAdapter.scheduleRender -> runRenderLoop
          -> 扫描件预览快速路径: presentRaster -> #pdf-render-target
          -> 向量路径: renderVectorPageWithPlan (vector_host.ts:238)
            -> resolveVectorPageBundle (Tauri read_page_asset_bundle)
            -> applyViewportCanvasFrame（以 render-zoom 单位重新装箱画布）
            -> 逐层 renderLayer（worker 或主线程）
            -> 延迟 present 入队
          -> commitVectorRenderResult
            -> beforePresent: zoomController.commitRenderedFrame -> syncLayoutBox
            -> 拷贝像素，置为可见
    -> markPageVisible + 预取相邻页
```

#### 缩放（578c058 修复后）

**核心契约**：容器永远以 *render-zoom* 单位定尺寸；display 与 render 的差距
只通过 wasm `syncHostLayout` 算出的 CSS `scale()` 表达。

```
在 #pdf-scroll-container 上 Ctrl+滚轮
  -> zoomController.bindWheelZoom (zoom_controller.ts:397)
  -> 构建完整请求（视口点、滚动、边界）
  -> wasm handleWheelZoomHost（Rust 拥有目标缩放 + 渲染决策）
  -> syncZoomSelect() [更新下拉框]
  -> startSmoothZoomPreview() [RAF 循环]
    -> wasm stepPreviewHost 返回 {previewPresent: {translateX/Y, cssScale}}
    -> applyPreviewFrame 把瞬态 transform 写到容器
  -> scheduleWheelZoomRender [防抖 -> 立即渲染或继续预览]

布局同步（即修复本身）:
  pdf_layout_sync.ts:27-131
  -> syncLayoutBox(displayZoom, renderedZoom, layoutOverride)
  -> wasm syncHostLayout 返回 {domWidth, domHeight, cssScale, hostWidth, ...}
  -> wrapper 尺寸为 hostWidth x hostHeight
  -> #pdf-page-container: 定位在 contentLeft/Top，尺寸 domWidth x domHeight
     加 transform: scale(cssScale)  [displayWidth = domWidth * cssScale]
  -> applyViewportCanvasFrame 只以 render-zoom 单位重新装箱画布
     （不再把 displayWidth 写进容器）

提交:
  zoomController.commitRenderedFrame (line 289)
  -> 清除 transform -> syncLayoutBox(displayZoom, renderZoom) -> 恢复滚动
```

#### 编辑

```
#pdf-add-text-btn 点击 -> toggleTextEditMode()
  -> EditorSession.setEditMode (wasm)
  -> syncTargets: 渲染交互目标

在目标上 pointerdown -> api.begin() -> api.hitTest -> api.openBlock
  -> setupActiveEditor: 定位外壳、聚焦 textarea、绘制光标

beforeinput -> onBeforeInputRequested
  -> api.syncInput（光标 Rust <-> JS 同步）
  -> api.applyCommand
  -> 回写 draftText + 光标，重绘
  -> renderCurrentPage('editorVisibility') [立即出帧，不入队]

blur/Escape -> commitEditor -> api.commit({draftText, caretIndex})
  -> Rust 构建补丁

保存（#pdf-save-btn）:
  -> api().save() -> editorHost.saveEdits
  -> documentEditApi.saveEdits('manual-save')
  -> 失效缓存 -> wasm requestRefresh -> 渲染帧
```

### 5.3 WASM 加载

`shared/wasm_loader.ts:1` 导入 `crates/pdf-viewer-ui/pkg/pdf_viewer_ui`
（由 `npm run wasm:pdf-viewer-ui` 构建）。初始化：

1. `installTargetInvokeBridge()`（第 14 行）-- 在 wasm 初始化**之前**装好
   Rust `target_invoke` 的 JS 垫片（Rust 在初始化期间就会回调 JS）。
2. `await init()`（第 58 行）-- wasm 模块加载。
3. 再次安装桥（第 64 行）-- 确保拿到最新绑定。
4. 指纹校验：`'pdf-viewer-rust-single-chain-20260429'`。

TS 桥接层从不直接调 Tauri 命令；wasm 经 `targetInvokeV3` 桥调
`window.__TAURI__.core.invoke`。

### 5.4 UI 元素 -> 桥接映射

| HTML 元素 | 处理器 | 桥接 API |
|---|---|---|
| `#open-btn` | 内联脚本 -> `__pdfOpenHandler` | `invoke('pick_file')` -> `openPdfFile` |
| `#pdf-save-btn` | `main.ts:89` | `api().save()` -> `editorHost.saveEdits` |
| `#pdf-undo/redo-btn` | `main.ts:99-100` | `undo()/redo()` |
| `#pdf-prev/next-page-btn` | `main.ts:103-108` | `prevPage()/nextPage()` |
| `#pdf-zoom-select` | `main.ts:112-115` | `setZoom(val)` |
| `#pdf-zoom-in/out-btn` | `main.ts:117-129` | `setZoom(...)` |
| `#pdf-scroll-container` 上的 Ctrl+滚轮 | `zoomController.bindWheelZoom` | wasm `handleWheelZoomHost` |
| `#pdf-add-text-btn` | `main.ts:167-171` | `toggleTextEditMode()` |
| `#pdf-search-btn` / Ctrl+F | `findController.open()` | wasm `FindSession` |
| 方向键/PageUp/Down | `handlePdfViewerKeydown` | `prevPage()/nextPage()` |

---

## 6. 跨层流动的领域类型

| 类型 | 定义处 | 消费方 | 用途 |
|---|---|---|---|
| `SyncHostLayoutRequest/Result` | `pdf-viewer-ui/host/layout.rs` | TS `pdf_layout_sync.ts` | 缩放抵消契约 |
| `FramePlanRequest` | `pdf-viewer-core/render` | `pdf-viewer-ui/present/plan_builder.rs` | 渲染帧规划 |
| `TextState` / `TextMatrixCore` | `pdf-viewer-core/text/` | src-tauri 的读取（content_parser）与写入（reflow）两侧 | 共享 PDF 文本算子状态 |
| `NativeVectorPageModel` | `src-tauri/infrastructure/pdf/models.rs`（core 的再导出） | TS `vector_page_bundle.ts` | 渲染用完整向量页 |
| `NativePageModel` / 批注类型 | `pdf-viewer-core/models/` | `src-tauri/application/pdf/page_annotation.rs` | 页面 region 上下文 |
| `PersistableRegionPatch` | `pdf-viewer-core/persistence/` | `src-tauri/infrastructure/pdf/region_materializer.rs` | 编辑补丁物化 |
| `PdfEditCommand` | `src-tauri/infrastructure/pdf/commands.rs` | `edit_commands.rs` | 编辑事务抽象 |

---

## 7. 已知问题 / 死代码

### 2026-08-15 死代码清扫中移除（分支 `refactor/architecture-improvements`）
- `vello_renderer.rs` + `RendererState`（1130 行）-- `VelloRenderer::new()` 零调用点。
  同时移除 `vello` + `wgpu` 依赖（Cargo.lock -642 行）；本项目*以及*源项目
  （`nushell-enhanced` 的 `render_vector_tile` 注册了但从未被 TS 调用）的前端
  都从未调用过 vello/wgpu。真实渲染是 wasm Canvas 2D 路径
  （`pdf-viewer-ui/src/render/canvas.rs`），经 WebView2/Skia 获得 GPU 加速。
  可用 `git show fdde982^:src-tauri/src/infrastructure/pdf/vello_renderer.rs` 找回。
- `pdf_read/facade.rs`、`pdf_read/vector_backend.rs` -- 无调用方
- `page_classifier.rs` -- 无调用方（preview_engine 自带内联版本）
- `interfaces/pdf/ipc_converters.rs` -- 再导出垫片；调用方现直接使用
  `application::pdf::edit_commands`（同时修复了 `page_annotation.rs` 里
  Application -> Interfaces 的依赖倒置）
- `PDF_OPS_LOCK`、`read_document_meta_cache`、旧版 `PageModel`/`PageTextInfo`/`TextObjectInfo`
- `color.rs` 孤儿函数 `blend`/`parse_rgb`/`parse_vello`（随 vello 渲染器成为孤儿；
  严格的 `parse_pdf` 写入路径保留）
- `vector_engine.rs` 里的遮挡剔除块 -- 有扫描和日志但 drain 被注释掉
  （纯死计算）
- `index.html` 里的 `#vector-render-container` div -- TS 从不读取
- `main.ts` 里的 `window.pdfSetToolMode()` 调用 -- 定义不存在（经 `?.` 一直是空操作）

### 重复逻辑
- ~~`pdf_utils.rs` 与 `edit_commands.rs` 里的 `truncate_for_log`~~ -- 2026-08-15
  去重（`edit_commands.rs` 现从 `pdf_utils` 导入）
- ~~`/Rotate` 父链遍历~~ -- 唯一的真重复（`path_resolver.rs`）现改为调用
  `pdf_utils::read_page_rotation`。`resource_reader.rs` / `preview_engine.rs`
  遍历父链是为了 `/Resources` 继承的 XObject/Font 查找--模式相似但语义不同；
  统一它们需要一个泛型"继承属性遍历器"（价值低，不做）。
- ~~`DocumentSession` / `ReviewSession` 的 TS 单例~~ -- 现在只经
  `src/bridge/shared/session_singletons.ts` 构造（原先有 2 / 3 份模块级副本）
- ~~撤销/重做历史上限 `HISTORY_LIMIT=20` 与硬编码 `20`~~ -- 单一常量在
  `app_state::HISTORY_LIMIT`，`document_service.rs` 和 `edit_commands.rs` 共用

### 命名危害
- ~~两个 `pdf_read/` 目录~~ -- 2026-08-16 已解决：`src-tauri/src/infrastructure/pdf_read/`
  改名为 `infrastructure/pdf_fallback/`（它是扫描件 PDF 的 pdf-rs *兜底*后端；
  lopdf 解析器仍在 `infrastructure/pdf/pdf_read/`）
- `font/layout.rs` "遮蔽" std 的 `layout` 名字 -- **非问题**：stable Rust 根本
  不存在 `std::layout`。该模块只是委托给
  `pdf-viewer-core::geometry::layout_engine::layout_paragraph` 的薄包装。保持原样。
- **`font/catalog.rs` + `font/match_mod.rs` 里的 GBK 乱码字体名字符串字面量是
  刻意的--不要"修复"它们。** PDF 常携带被 GBK 误解码的字体名（如
  `寰蒋闆呴粦` = `微软雅黑`）；`name_variants()` 把它们映射到规范的 Windows
  字体名（`微软雅黑` -> `Microsoft YaHei`）。*注释*中的乱码已于 2026-08-16
  修复（约 30 行，涉及 `canvas.rs`、`layout_analyzer.rs`、`vector_engine.rs`、
  `save_engine.rs`、`catalog.rs`、`match_mod.rs`）；只剩这些功能性字面量，
  属有意保留。
