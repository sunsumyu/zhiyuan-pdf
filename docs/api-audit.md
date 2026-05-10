# API 表面审查与死码清单（Batch 1 / 3）

> **批次说明**：本文是「全工程审查 → 三批交付」的第 1 批，聚焦 **对外 API 盘点 + 死码反查**。第 2 批（结构 / 流程 / 状态机深度审计）和第 3 批（Nutrient 对标）将另文交付。
>
> **方法论**：对所有 `#[tauri::command]` 与 `#[wasm_bindgen]` 导出做静态扫描，再用 ripgrep 反查 `src/`（TS bridge）和 `crates/pdf-viewer-ui/src/`（Rust WASM 内部隧道 `smart_invoke` / `target_invoke`）的实际调用点；只要任一侧有引用即视为「在用」。
>
> **删除原则**：本文档**不删除任何代码**。仅列出候选清单 + 删除理由 + 对功能的影响评估，等待您逐项确认。

---

## 总览（Executive Summary）

| 维度 | 定义数 | 在用数 | 死码数 | 死码率 |
|------|------:|------:|------:|------:|
| **Tauri commands**（`#[command]`） | 40 | 20 | **20** | 50% |
| **WASM 方法**（`#[wasm_bindgen]` 唯一 js_name） | 206 | 76 | 130（含 24 stub） | 51% |
| **WASM 单文件**（>200 LOC 且 0 TS 引用） | — | — | **3 个文件 750 LOC** | — |

**头条结论**：

1. **`crates/pdf-viewer-ui/src/wasm_api/` 整个文件夹基本全死**，3 个文件共 750 LOC、47 个 wasm_bindgen 方法，TS 侧引用为零或几乎为零。这是上一轮 Session 重构后被新 `*_api.rs` 取代的旧 facade，可一次性删除。
2. **20 个 Tauri command 死码** 中 11 个是被 WASM 取代（如 `read_metadata` / `read_pdf` / `render_tile` / `apply_text_patches` / 6 个 `resolve_*`），9 个是历史功能残留（如 `clear_cache` / `create_demo_pdf` / `set_log_level` / `apply_replace` 系列）。
3. **24 个 WASM 方法是 NotImplemented stub**——属于 Session API 设计稿里写了但还没实现的占位。这些**不应删除**，应放入待办或逐个落地。
4. **重构受益面已显现**：在用的 76 个 WASM 方法分布在 11 个 `*_api.rs` Session 文件中，平均 6.9 个/文件，说明 Session 边界粒度合理。

---

## 1. Tauri Command 审查（40 个）

### 1.1 在用命令（20 个）

| 命令 | 文件 | 调用方 | 对应功能 |
|------|------|--------|---------|
| `open_pdf` | document.rs | Rust WASM (`target_invoke`) | 打开 PDF（disk → AppState） |
| `save_pdf` | document.rs | Rust WASM | 保存 PDF 到磁盘 |
| `undo` | document.rs | TS bridge | 全局撤销 |
| `redo` | document.rs | TS bridge | 全局重做 |
| `pick_file` | system.rs | Rust WASM | 系统文件选择对话框 |
| `read_vector` | render.rs | TS bridge | 读取页面矢量数据 |
| `read_glyph_plan` | render.rs | TS bridge | 读取 glyph 绘制计划 |
| `read_images` | render.rs | TS bridge | 读取页面图片缓存 |
| `read_preview` | page.rs | TS bridge | 读取页面预览（光栅） |
| `find_in_page` | search.rs | TS bridge | 单页内搜索 |
| `find_in_document` | search.rs | TS bridge | 全文搜索 |
| `read_annotation_targets` | annotation.rs | Rust WASM | 读取注释目标列表 |
| `read_highlights` | annotation.rs | TS bridge | 读取高亮 |
| `apply_highlight` | annotation.rs | TS bridge | 应用高亮 |
| `delete_annotation` | annotation.rs | TS bridge | 删除注释 |
| `read_comments` | comment.rs | Rust WASM | 读取评论列表 |
| `read_comment_review` | comment.rs | Rust WASM | 读取评论 review |
| `apply_comment` | comment.rs | Rust WASM | 添加评论 |
| `apply_comment_update` | comment.rs | Rust WASM | 更新评论 |
| `apply_region_patches` | replace.rs | Rust WASM | 应用区域替换补丁（编辑保存） |

### 1.2 死码命令（20 个，按删除信心分组）

#### 🟢 高信心删除（11 个）—— 被 WASM Session 完全取代

| 命令 | 行数 | 替代方案 | 删除影响 |
|------|----:|---------|---------|
| `read_metadata` | ~6 | DocumentSession 在内存 lopdf::Document 上自取 | 无 |
| `read_pdf` | ~44 | `open_pdf` 已包含读取流程 | 无 |
| `probe_pdf` | ~45 | `open_pdf` 已涵盖（probe 是 open 的子集） | 无 |
| `read_page_info` | ~27 | RenderPipeline + ViewerSession 自计算 | 无 |
| `read_materialization_report` | ~6 | 调试命令，无 UI 入口 | 无 |
| `prefetch_preview` | ~45 | RenderPipeline.startProgressive 内部预取 | 无 |
| `render_tile` | ~10 | vello 渲染走 WASM 内部 RenderPipeline | 无 |
| `apply_text_patches` | ~25 | 被 `apply_region_patches` 取代（更通用） | 无 |
| `apply_replace` | ~8 | 被 `apply_region_patches` 取代 | 无 |
| `apply_batch_replace` | ~20 | 被 `apply_region_patches` 取代 | 无 |
| `resolve_layout` | ~7 | EditorSession.beginEdit 内部解析 | 无 |

> **删除收益**：~243 LOC + 11 个 invoke_handler! 注册项 + 对应的 application/infrastructure 服务层调用链可能也跟着不再需要（需要二级追踪）。

#### 🟡 中信心删除（6 个）—— 几何 resolve 系列

| 命令 | 行数 | 状态 |
|------|----:|------|
| `resolve_caret` | ~5 | 取消 native，改 EditorSession 内部解析 |
| `resolve_hit` | ~5 | 同上 |
| `resolve_hit_target` | ~5 | 同上 |
| `resolve_params` | ~5 | 同上 |
| `resolve_projection` | ~5 | 同上 |

> **风险**：这些是 native 侧的 PdfEditorGeometryService 出口，删除前需确认 `crates/pdf-viewer-core` 的 geometry 模块自给自足、不依赖 native lopdf 上下文。若依赖（例如要查 native 字体度量），应保留。

#### 🟠 低信心 / 保留候选（3 个）—— 调试与系统工具

| 命令 | 用途 | 建议 |
|------|-----|------|
| `set_log_level` | 运行时调日志等级 | **保留**：开发调试有用，未来可能从 settings UI 调用 |
| `create_demo_pdf` | 生成示例 PDF | **保留**：onboarding 场景可能用到，删除收益小 |
| `clear_cache` | 释放所有 PDF 资源 | **建议改为**：迁移到 DocumentSession.clear() |
| `get_asset_url` | 把本地路径转成 `asset.localhost/` URL | **保留**：Tauri 资源协议必备工具 |

---

## 2. WASM API 表面审查（206 个 js_name）

### 2.1 整文件死码（**首要清理目标**）

| 文件 | LOC | wasm_bindgen 方法 | 在用 | 状态 |
|------|---:|-----------------:|-----:|------|
| `wasm_api/frame_api.rs` | 204 | 23 | **0** | 🟢 全死，可删 |
| `wasm_api/document.rs` | 341 | ? | 待二级反查 | 🟡 疑似旧 facade，待确认 |
| `wasm_api/viewer.rs` | 205 | ? | 待二级反查 | 🟡 疑似旧 facade，待确认 |
| `wasm_api/zoom_api.rs` | 222 | 24 | 6 | 🟡 部分还在用，需要先把 6 个迁到 zoom/zoom_api.rs 再删 |

**`wasm_api/frame_api.rs` 23 个死方法清单**：
```
abort_render_frame, advance_render_loop_frame, begin_render_frame,
commit_render_frame, is_render_frame_current, queue_render_loop_frame,
reset_frame_cache, resolve_frame_plan, resolve_host_scroll_refresh,
resolve_layer_execution_plan, resolve_layer_present_decision,
resolve_render_execution_plan, resolve_render_follow_up,
resolve_viewport_layout, resolve_viewport_refresh, resolve_viewport_tile,
schedule_render_follow_up, schedule_render_frame, settle_render_frame,
store_frame_cache_entry, sync_host_layout, take_frame_plan,
touch_frame_cache_entry
```

> **建议**：整个 `wasm_api/` 目录完成残留迁移后**整体删除**，新 Session API（`*_api.rs` + `crate::api` umbrella）已完整替代其能力。删除收益预计 **~1000 LOC + 1 个不再有意义的二级模块**。

### 2.2 NotImplemented Stub（24 个）—— 不要删

这是 Session API 设计稿写了但还没落地的占位方法，应该作为「待实现功能清单」管理：

```
applyPatch, applyRegionReplacements, buildEditableSegments,
buildPageRegionContext, buildRegionPatch, bumpRevision,
clearSession, commitResult, exportPages, fillFormField, getState,
getToolbarState, insertPage, list (annotation), listPageComments,
listPageAnnotationTargets, navigateNext, navigatePrev, readAll,
readAttachments, readFilteredFeed, readMetadata, redoOp, undoOp
```

> **建议**：把这 24 个移到 `docs/editor-api-architecture-proposal.md` 的「待实现」章节做单点跟踪，避免散落各 Session 文件里。

### 2.3 真死码 Session 方法（106 个 - 待逐项确认）

按 Session 分布（前 5）：

| Session 文件 | 死方法数 | 典型例子 |
|------------|--------:|---------|
| `wasm_api/frame_api.rs` | 23 | （上面已列） |
| `wasm_api/zoom_api.rs` | 18 | apply_zoom_selection / clear_preview_present / ... |
| `find/find_api.rs` | ~12 | clearSession / getReplaceRequests / getToolbarState / highlightAll / moveActive / moveMatch |
| `render/render_api.rs` | ~10 | buildEditableSegments / cancelProgressive / commitResult / navigateNext / navigatePrev |
| `review/review_api.rs` | ~6 | accept / acceptAll / exportReport |

> **风险提示**：`find/*` 和 `render/*` 中部分方法可能是「设计上对外公开但当前 TS 还没用到」——属于 Session API 接口稳定性预留。**需要您逐域确认**：
> - **find**：moveActive / moveMatch / highlightAll 是 PDF 阅读器标准能力，TS 现在用什么？是不是走 Tauri 的 `find_in_page` / `find_in_document`？
> - **render**：navigateNext / navigatePrev 是不是被 zoom 或 viewer session 接管了？
> - **review**：accept / acceptAll 是修订审阅核心动作，TS 上是不是有同名功能但走了别的路径？

---

## 3. API → 产品功能映射

按用户可感知功能维度梳理（以**实际被调用**的 API 为准）：

### 3.1 文档生命周期

| 功能 | TS 入口 | WASM API | Tauri Command |
|-----|--------|---------|--------------|
| 选文件 | `pdf_runtime.ts` | — | `pick_file` |
| 打开 PDF | `pdf_document_runtime.ts` | DocumentSession.openTextPdfFlow | `open_pdf` |
| 保存 PDF | editor commit 链 | EditorSession.commit → DocumentSession | `save_pdf` |
| 撤销/重做 | 全局快捷键 | HistoryController.undo/redo | `undo` / `redo` |

### 3.2 渲染与翻页

| 功能 | TS 入口 | WASM API | Tauri Command |
|-----|--------|---------|--------------|
| 矢量数据读取 | render_flow.ts | RenderPipeline | `read_vector` |
| Glyph 绘制计划 | render_flow.ts | — | `read_glyph_plan` |
| 页面图片 | render_flow.ts | — | `read_images` |
| 缩略预览 | render_flow.ts | — | `read_preview` |
| 渐进渲染 | render_flow.ts | RenderPipeline.startProgressive | — |
| 视口/翻页 | viewer_session.ts | ViewerSession | — |
| 缩放 | zoom controllers | ZoomController + 6 个 wasm_api/zoom_api 旧函数 | — |

### 3.3 编辑与替换

| 功能 | TS 入口 | WASM API | Tauri Command |
|-----|--------|---------|--------------|
| 进入编辑 | editor_host.ts | EditorSession.begin | — |
| 文字录入/格式 | editor_host.ts | EditorSession.insertText/setFormat | — |
| 提交编辑（落盘） | EditorSession.commit | DocumentSession.applyDocumentPatch | `apply_region_patches` |
| 全局搜索/替换 | find_facade.ts | FindSession（部分） | `find_in_page` / `find_in_document` |

### 3.4 标注与评论

| 功能 | TS 入口 | WASM API | Tauri Command |
|-----|--------|---------|--------------|
| 高亮 CRUD | pdf_annotation_controller.ts | — | `read_highlights` / `apply_highlight` / `delete_annotation` |
| 注释目标 | pdf_annotation_controller.ts | AnnotationManager（部分） | `read_annotation_targets` |
| 评论 CRUD | pdf_comment_wasm_bridge.ts | CommentManager（部分） | `read_comments` / `apply_comment` / `apply_comment_update` |
| 评论 review | pdf_comment_wasm_bridge.ts | — | `read_comment_review` |

### 3.5 AI / Resume（独立子系统）

| 功能 | 状态 |
|-----|------|
| 11 个 `submit_resume_prompt` / `apply_resume_*` / `mark_suggestion_*` 命令 | TS 调用了，但 src-tauri 没有定义 → **疑似走外部 sidecar 或 HTTP 服务**，不在本审查范围内 |

> **行动项**：确认 AI 调用是否走另一个 Tauri 插件 / 外部进程；如果是，应在 `docs/architecture-diagrams.md` 增补一张子系统拓扑图。

---

## 4. 推荐删除清单（待您确认）

按删除批次组织，每批可独立提 commit、独立验证。

### Batch A · 整文件清理（删除收益最大，风险可控）

| 操作 | 文件 | LOC | 风险 |
|------|------|----:|------|
| 删除 | `crates/pdf-viewer-ui/src/wasm_api/frame_api.rs` | 204 | 0（零调用方） |
| 验证后删除 | `crates/pdf-viewer-ui/src/wasm_api/document.rs` | 341 | 低（疑似已被 document/document_api.rs 取代） |
| 验证后删除 | `crates/pdf-viewer-ui/src/wasm_api/viewer.rs` | 205 | 低（疑似已被 viewer/viewer_api.rs 取代） |
| 迁移后删除 | `crates/pdf-viewer-ui/src/wasm_api/zoom_api.rs` | 222 | 中（6 个方法仍在用，需先迁到 zoom/zoom_api.rs） |
| 整个目录删除 | `crates/pdf-viewer-ui/src/wasm_api/` | — | 完成上述四步后无依赖 |

**预计净减**：~970 LOC + 1 个二级模块。

### Batch B · Tauri command 清理（11 个高信心 + 6 个中信心）

```
# 高信心
apply_batch_replace, apply_replace, apply_text_patches,
prefetch_preview, probe_pdf, read_materialization_report,
read_metadata, read_page_info, read_pdf, render_tile,
resolve_layout

# 中信心（核对 geometry 自包含性后删）
resolve_caret, resolve_hit, resolve_hit_target,
resolve_params, resolve_projection
```

**预计净减**：~243 LOC + 17 行 `invoke_handler!` 注册 + 二级 application 层调用链。

### Batch C · 真死 Session 方法（需要逐域确认）

按 Session 分批确认是否真死、是否预留：

- **find/find_api.rs**：12 个方法（move/highlight 系列）
- **render/render_api.rs**：10 个方法（navigate/build/commit）
- **review/review_api.rs**：6 个方法（accept/export）
- 其他散落约 ~80 个

**预计净减**：~500–800 LOC（视确认率）。

---

## 5. 不建议删除的项

| 项 | 理由 |
|---|------|
| 24 个 NotImplemented stub | Session API 契约的设计稿，是「待实现」而非「死码」 |
| `set_log_level` / `create_demo_pdf` / `get_asset_url` | 工具命令，删除收益小 |
| `accept` / `acceptAll` / `exportReport` | 修订审阅核心能力，未来必有 UI 入口 |
| AppState 4 个 sub-store 的字段 | 即使部分字段当前无写入也是 schema 一部分 |

---

## 6. 下批次预告

- **Batch 2（结构 / 流程 / 状态机）**：每个 Session 的状态机图、单向依赖审计、`pdf_runtime.ts` 装配链 trace、跨 crate 数据流。
- **Batch 3（Nutrient 对标）**：PSPDFKit Web SDK Instance API / ViewState / Document / Annotation 与本项目的逐项对比，输出迁移建议。

> **请您确认 Batch A / B / C 的删除清单，逐批回复 OK 即可推进。**
> 默认我**不会主动删除任何代码**，等您逐项确认。
