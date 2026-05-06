# PDF Viewer / Editor — 对外 API 契约 v1

> **状态**：草稿冻结候选 (2026-05-06)
> 一旦这份契约批准，下面列出的任何 API **不得改名、不得改签名**，只允许新增字段或新增 API。

---

## 1. 设计原则

1. **三层 API、单一入口**
   - **WASM facade**（前端 ↔ pdf-viewer-ui crate）：camelCase，所有入口集中在 `crates/pdf-viewer-ui/src/<domain>/facade.rs`
   - **Tauri commands**（前端 ↔ Tauri 后端）：snake_case，所有入口集中在 `src-tauri/src/interfaces/multimedia/pdf.rs`
   - **TS facade**（前端调用者 ↔ wasm/tauri）：camelCase，集中在 `src/bridge/<domain>_facade.ts`

2. **域名命名空间**：每个 API 用域前缀，方便定位与文档化
   - `editor.*`：编辑会话（活动段落 / draft / caret / format）
   - `document.*`：文档级（打开 / 保存 / 撤销 / 元数据 / 页面增删）
   - `page.*`：单页操作（注释 / 链接 / 图像）
   - `render.*`：渲染调度
   - `viewer.*`：查看器会话（路径 / 当前页 / 缩放）
   - `zoom.*`：缩放交互
   - `find.*`：查找
   - `comment.*`：批注
   - `annotation.*`：注释（高亮、墨迹、图章）
   - `review.*`：变更审阅

3. **稳定性等级**
   - `Stable`：契约冻结，不可改名/不可改签名
   - `Experimental`：可能变化，必须在文档中明确标注
   - `Stub`：API 已预留但未实现，调用返回 `not_implemented`

4. **破坏性变更**：必须升 v2 命名空间（如 `editorV2.*`），保留 v1 至少 2 个 release 周期。

---

## 2. 命名规范

| 层 | 风格 | 规则 |
|---|---|---|
| WASM facade（js_name） | `camelCase`，`<domain>.<verb><Object>` | `editor.openAtPoint`、`document.undo` |
| WASM facade（Rust fn） | `snake_case` | `fn open_at_point()` |
| Tauri command | `snake_case` | `pub async fn open_pdf` |
| TS export | `camelCase` | `export function openEditor()` |
| Rust struct/enum | `PascalCase` | `EditorOpenRequest` |
| TS type | `PascalCase` | `EditorOpenRequest` |
| Result type 后缀 | `Result` | `EditorOpenResult` |
| Request type 后缀 | `Request` | `EditorOpenRequest` |

**禁用**：
- ❌ 同义词混用：`get/read/fetch/load/resolve` 全用 `read`（查询） / `get`（同步纯读）
- ❌ 动词模糊：`handle*`、`process*`、`do*`、`apply*`（除非是命令模式）
- ❌ 实现细节泄漏：`_workflow`、`_internal`、`_tx`、`_runtime` 不能出现在公开 API 名

---

## 3. 域 API 全清单（v1 冻结候选）

### 3.1 `editor.*`（编辑会话）

| API | 状态 | 说明 |
|---|---|---|
| `editor.setMode(enabled: bool)` | Stable | 开启/关闭编辑模式 |
| `editor.toggleMode()` | Stable | 切换 |
| `editor.openAtPoint(req)` | Stable | 在客户端坐标处打开段落编辑器 |
| `editor.openByRegion(req)` | Stable | 通过 region_id 打开（搜索结果跳转） |
| `editor.close()` | Stable | 关闭编辑器（不提交） |
| `editor.commit(req)` | Stable | 提交并触发 render |
| `editor.commitSilent(req)` | Stable | 提交不 render（保存前用） |
| `editor.applyCommand(req)` | Stable | 输入命令：insert/backspace/delete/navigate |
| `editor.syncInput(req)` | Stable | 同步前端 textarea 状态到 Rust |
| `editor.moveCaretToPoint(req)` | Stable | 鼠标点击移动光标 |
| `editor.applyFormat(action)` | Stable | bold/italic/font/color/align/list |
| `editor.readSnapshot(zoom)` | Stable | 读取活动编辑器快照（含 targets） |
| `editor.readFormatState()` | Stable | 读取当前段落格式状态 |
| `editor.hasUnsavedChanges()` | Stable | 是否有未提交修改 |
| `editor.paintCanvas(canvas, zoom, draftText, caret)` | Stable | 绘制编辑器内 canvas |
| `editor.utf16ToCharIndex(text, offset)` | Stable | UTF-16 → 字符索引转换 |
| `editor.charToUtf16Offset(text, char)` | Stable | 字符索引 → UTF-16 转换 |
| `editor.beginCommit()` / `editor.finishCommit()` | Stable | 提交事务边界 |
| `editor.readDiagnostics()` | Stable | 调试信息 |
| `editor.readRuntime()` | Stable | 内部运行时状态（仅 debug） |
| **预留 (Stub)** | | |
| `editor.selectRange(start, end)` | Stub | 选区操作 |
| `editor.cut()` / `copy()` / `paste(text)` | Stub | 剪贴板 |
| `editor.undo()` / `editor.redo()` | Stub | 编辑层撤销（区别于文档层） |
| `editor.findInActive(query)` | Stub | 当前段落内查找 |
| `editor.replaceInActive(query, replacement)` | Stub | 当前段落内替换 |

### 3.2 `document.*`（文档级）

| API | 状态 | 说明 |
|---|---|---|
| `document.open(req)` | Stable | 打开 PDF（管线版） |
| `document.pick(req)` | Stable | 弹文件选择对话框 + 打开 |
| `document.close()` | Stable | 关闭当前文档 |
| `document.undo()` | Stable | 文档级撤销（patch 粒度） |
| `document.redo()` | Stable | 文档级重做 |
| `document.rotate(delta)` | Stable | 整文旋转 |
| `document.requestRefresh(reason, req)` | Stable | 请求重渲染 |
| `document.bumpRevision(reason)` | Stable | 标记文档已变更 |
| `document.applyPatch(patch)` | Stable | 应用持久化补丁 |
| `document.applyRegionReplacements(reqs, frame)` | Stable | 批量区域替换 |
| `document.buildRegionPatch(...)` | Stable | 构造区域补丁 |
| `document.openSession(req)` | Stable | 初始化 host session |
| `document.resetSession(w, h)` | Stable | 重置 session |
| `document.setSize(w, h)` | Stable | 设置当前文档页面尺寸 |
| `document.readSession()` | Stable | 读取 session 状态 |
| **预留 (Stub)** | | |
| `document.insertPage(index, source)` | Stub | 插入页面 |
| `document.removePage(index)` | Stub | 删除页面 |
| `document.movePage(from, to)` | Stub | 移动页面 |
| `document.rotatePage(index, delta)` | Stub | 单页旋转 |
| `document.readMetadata()` / `setMetadata(meta)` | Stub | 元数据 |
| `document.exportPages(indices, format)` | Stub | 导出页面（图片/子 PDF） |
| `document.setPassword(pwd)` / `removePassword()` | Stub | 加密 |
| `document.getOutline()` / `setOutline(tree)` | Stub | 大纲 |

### 3.3 `find.*`（查找）

| API | 状态 |
|---|---|
| `find.clearSession()` | Stable |
| `find.readSession()` | Stable |
| `find.setSession(query, scope, pages, preferred)` | Stable |
| `find.moveMatch(step)` | Stable |

### 3.4 `comment.*` / `annotation.*` / `review.*`

（详见现有 wasm_api/document.rs，命名将批量改为域前缀。略）

### 3.5 `render.*`（渲染调度）

| API | 状态 | 说明 |
|---|---|---|
| `render.beginFrame(req)` | Stable | 开始一帧 |
| `render.scheduleFrame(req)` | Stable | 调度 |
| `render.commitFrame(token, zoom)` | Stable | 提交 |
| `render.settleFrame(token, zoom)` | Stable | 结算 |
| `render.abortFrame(token)` | Stable | 中止 |
| `render.isFrameCurrent(token)` | Stable | 是否当前帧 |
| `render.queueLoopFrame(frame)` / `advanceLoopFrame(frame)` | Stable | 主循环 |
| `render.commitResult(token, zoom, w, h)` | Stable | 提交渲染结果 |
| `render.startProgressive()` / `stepProgressive(...)` / `cancelProgressive()` | Stable | 渐进渲染 |
| `render.renderPage(canvasId, cache)` | Stable | 整页渲染 |
| `render.resolvePolicy(req)` | Stable | 决策渐进渲染策略 |
| `render.resolveLayerExecutionPlan(...)` | Stable | 图层执行计划 |
| `render.resolveLayerPresentDecision(...)` | Stable | 图层呈现决策 |
| `render.resolveExecutionPlan(...)` | Stable | 渲染执行计划 |
| `render.resolveFollowUp(...)` / `scheduleFollowUp(...)` | Stable | 后续跟进 |
| `render.touchFrameCache(...)` / `storeFrameCache(...)` / `resetFrameCache()` | Stable | 帧缓存 |

### 3.6 `viewer.*`（查看器会话）

实现：`crates/pdf-viewer-ui/src/viewer/facade.rs`，TS：`src/bridge/viewer_facade.ts`

| API | 状态 | 说明 |
|---|---|---|
| `viewer.readSession()` | Stable | 读 session（path/pageCount/currentPage/zoom/pageSize） |
| `viewer.resetSession()` | Stable | 重置到默认 |
| `viewer.setDocument(path, pageCount, initialZoom)` | Stable | 绑定新文档 |
| `viewer.setCurrentPage(index)` | Stable | 设置当前页 |
| `viewer.setCurrentZoom(zoom)` | Stable | 设置当前缩放 |
| `viewer.setPageSize(w, h)` | Stable | 设置页面尺寸 |
| `viewer.navigatePrev()` / `navigateNext()` | Stable | 翻页 |
| `viewer.applyZoomSelection(zoom)` | Stable | 应用缩放选择 |
| `viewer.goToPage(index, anchor)` | Stub | 跳页（含锚点） |
| `viewer.goToNamedDestination(name)` | Stub | 跳到命名目的地 |
| `viewer.setPresentationMode(enabled)` | Stub | 演示模式 |
| `viewer.setLayoutMode(mode)` | Stub | 单页/连续/对开 |

### 3.7 `find.*`（同 3.3）

实现：`crates/pdf-viewer-ui/src/find/facade.rs`，TS：`src/bridge/find_facade_v2.ts`

加 stub：`setOptions / replaceCurrent / replaceAll / highlightAll`

### 3.8 `review.*`

实现：`crates/pdf-viewer-ui/src/review/facade.rs`，TS：`src/bridge/review_facade_v2.ts`

| API | 状态 |
|---|---|
| `review.readFeed()` | Stable |
| `review.accept(patchKey)` / `reject(patchKey)` | Stable |
| `review.acceptAll()` / `rejectAll()` | Stable |
| `review.exportReport(format)` | Stub |
| `review.readFilteredFeed(filter)` | Stub |

### 3.9 `comment.*`

实现：`crates/pdf-viewer-ui/src/comment/facade.rs`，TS：`src/bridge/comment_facade.ts`

包含 17 个 Stable API（session / listings / review pipeline / mutation）+ 4 个 Stub（`replyComment / setResolved / export / import`）。

### 3.10 `zoom.*`、`page.*`、`render.*`、`annotation.*`

待 phase 5 实施（后续会话）。`render.*` 和 `annotation.*` 已部分存在于 `wasm_api/render_facade.rs` / `wasm_api/annotation_facade.rs`，需对齐命名规范。

---

## 4. Tauri command 清单（snake_case，与上述 wasm 域分离）

`src-tauri/src/interfaces/multimedia/pdf.rs` 已有 ~40 条命令，整体命名 OK，建议只做小幅调整：

| 当前 | 建议改名 | 理由 |
|---|---|---|
| `init_demo` | `create_demo_pdf` | init 不表达"创建" |
| `read_pdf` | 保留 | OK |
| `read_metadata` | 保留 | OK |
| `commit_edits` | `apply_text_patches` | commit 在 wasm 端有不同语义 |
| `apply_patches` | `apply_region_patches` | 区分上一条 |
| `find_in_page` / `find_in_document` | 保留 | OK |
| `read_glyph_plan` | 保留 | OK |

---

## 5. 类型契约

所有 Request / Result 类型必须：
1. 在 Rust 用 `#[serde(rename_all = "camelCase")]`
2. 在 TS 同名定义
3. 字段允许新增（必须 Optional），不允许重命名/删除

---

## 6. 弃用流程

1. 新加 API 标 `#[deprecated(note = "use editor.xxx instead")]`
2. TS facade 同步标 JSDoc `@deprecated`
3. 维持 2 个 release 周期
4. 第 3 个 release 才删除

---

## 7. 待办（实施）

参考 `progress.txt` Phase 2B / 2C / 3 / 4。
