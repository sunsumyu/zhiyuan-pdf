# 开发指南 -- 构建、测试、调试、修复

> Sovereignty PDF Viewer 的实用参考。
> 所有命令均在 `refactor/architecture-improvements` 分支的工作树上验证过。

---

## 1. 环境准备

- **Rust**（stable，且已安装 `wasm32-unknown-unknown` target）
- **Node.js** + npm
- **wasm-pack**（`cargo install wasm-pack`）
- **wasm-bindgen-cli 0.2.120** -- 必须与 `Cargo.lock` 里的 `wasm-bindgen` 版本
  完全一致。版本不同会报 schema 不匹配错误。安装：
  ```
  cargo install wasm-bindgen-cli --version 0.2.120
  ```
  验证：`wasm-bindgen-test-runner -V` 应输出 `0.2.120`。
- **Tauri CLI**（`npx tauri --version`），用于桌面应用
- **Chrome 或 Edge**，用于无头 wasm 测试和 E2E

---

## 2. 构建命令

### WASM（任何 TS 工作之前必须先构建）

```bash
npm run wasm:pdf-viewer-ui
# 等价于: wasm-pack build ./crates/pdf-viewer-ui --target web
# 输出: crates/pdf-viewer-ui/pkg/（自动生成，已 gitignore）
```

### 前端（TS + CSS）

```bash
npm run build
# tsc + vite build -> dist/
```

### 桌面应用

```bash
npm run tauri:dev        # 完整 Tauri + Vite 开发服务器（打开桌面窗口）
# 或
npm run dev              # 仅 Vite，浏览器访问 http://127.0.0.1:5000
                         # 注意：无法打开 PDF（open_pdf 依赖 Tauri 后端）
```

### 后端（Tauri Rust）

```bash
cd src-tauri && cargo build
# 或者直接让 tauri:dev 代劳
```

---

## 3. 测试命令

### `pdf-viewer-core`（纯 Rust，宿主 target）

```bash
cargo test -p pdf-viewer-core
# main 上的结果: 75 通过, 0 失败 (2026-08-15)
```

这个永远能跑--没有任何 wasm 依赖。

### `pdf-viewer-ui`（仅限 wasm target）

```bash
# 必须用 wasm target。宿主 target 按设计就无法编译
# （wasm 依赖都锁在 cfg(target_arch = "wasm32") 里）。

set CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
cargo test -p pdf-viewer-ui --target wasm32-unknown-unknown
# main 上的结果: 9 通过（5 个缩放布局 + 4 个 overlay）(2026-08-15)
```

**坑：** `wasm-bindgen-test-runner`（v0.2.120）不接受 `--headless`、`--node`
这类 flag。把它设为 runner 然后让 cargo 调用即可。如果报：

```
error: unexpected argument '--headless' found
```

说明你的 runner 版本不支持该 flag，或者传参方式不对。正确做法就是设环境变量。

**坑：** 如果报：

```
it looks like the Rust project used to create this Wasm file was linked against
version of wasm-bindgen that uses a different bindgen format than this binary:
  rust Wasm file schema version: 0.2.120
     this binary schema version: 0.2.126
```

说明全局安装的 `wasm-bindgen-cli` 版本与 `Cargo.lock` 不匹配。
装匹配的版本（见"环境准备"）。

### `src-tauri` 测试

```bash
cargo test -p pdf-viewer-standalone  # （Cargo.toml 里的 workspace 包名）
# 17 个集成测试 + infrastructure/pdf 各模块约 150 个单元测试
# 部分测试硬编码了绝对路径（机器相关，换机器会失败）
```

### TypeScript

```bash
npx vitest              # vitest（node 环境，src/**/*.test.ts）
# 目前只有: src/__tests__/diagnostics.test.ts（59 行）
```

### E2E（需要 tauri-driver）

```bash
npm run e2e:build       # tauri build --debug --no-bundle
npm run e2e             # wdio run tests/e2e/wdio.conf.ts
# Specs: load_pdf, hello, page_presentation_runtime, editor_bugs（跳过）
# 需要全局安装 tauri-driver
```

---

## 4. Bug 排查决策树

遇到 bug 时，按这个流程定位所在层和相关文件。

### 第 1 步：看到什么症状？

| 症状 | 可能的层 | 从这里入手 |
|---|---|---|
| 页面渲染空白 / 全白 | Tauri 后端（读取管线）或 TS 渲染循环 | 查 `vector_engine.rs`, `path_resolver.rs`, `render_flow.ts` |
| 页面渲染乱码 / 颜色错误 | 后端内容解析器 | `pdf_read/content_parser.rs` |
| 特定缩放级别下文字发虚 | 缩放布局契约 | `host/layout.rs`, `pdf_layout_sync.ts` |
| 缩放跳动 / 闪烁 | 缩放布局契约（2026-08-15 已解决） | `host/layout.rs`, `zoom_controller.ts`, `vector_canvas_host.ts` |
| 编辑不保存 | 编辑管线 | `editor/editor_api.rs`, `document_edit_api.ts`, `edit_commands.rs` |
| 字体不对（字重、加粗） | 字体引擎 | `font/match_mod.rs`, `font/embed.rs`, `font/parse.rs` |
| 批注 / 高亮丢失 | 批注管线 | `annotation_store.rs`, `page_annotation.rs`, `pdf_annotation_controller.ts` |
| 搜索搜不到 | 搜索管线 | `page_search.rs`, `find_facade.ts`, `pdf_find_controller.ts` |
| PDF 打不开 | 加载器 / Tauri 打开路径 | `pdf_loader.rs`, `document_service.rs:117` |
| Rust 崩溃 / panic | 后端 | 看 `src-tauri/src/` 的堆栈 |
| UI 按钮无反应 | TS 接线 | `main.ts`, `pdf_viewer_api.ts`，确认 handler 存在 |
| 翻页 / 渲染慢 | 预取 / 缓存 | `page_asset.rs`, `vector_page_bundle.ts`, `raster_image_cache.ts` |

### 第 2 步：追踪调用链

确定层之后，从入口开始追：

**渲染 bug：**
```
render_scheduler.ts:175 (requestRender)
  -> pdf_runtime.ts:483 (executeRender)
  -> render_flow.ts:509 (executeActualRender / runRenderLoop)
  -> vector_host.ts:238 (renderVectorPageWithPlan)
    -> vector_page_bundle.ts:267 (resolveVectorPageBundle)
      -> Tauri read_page_asset_bundle
        -> render.rs:18 -> page_intermediate_service.rs -> vector_engine.rs
          -> path_resolver.rs -> content_parser.rs
    -> vector_canvas_host.ts:233 (applyViewportCanvasFrame)
  -> pdf_layout_sync.ts:27 (syncLayoutBox -> wasm syncHostLayout)
```

**缩放 bug：**
```
zoom_controller.ts:397 (bindWheelZoom)
  -> frame_plan.ts:342 (handleWheelZoomHost -> wasm)
  -> zoom_controller.ts:221 (startSmoothZoomPreview -> RAF 循环)
  -> zoom_controller.ts:289 (commitRenderedFrame -> syncLayoutBox)
  -> pdf_layout_sync.ts:27 (syncLayoutBox -> wasm syncHostLayout)
    -> host/layout.rs:44 (SyncHostLayoutRequest -> SyncHostLayoutResult)
       契约: dom_width * css_scale == display_width
```

**编辑/保存 bug：**
```
editor/index.ts:465 (commitEditor -> api.commit)
  -> editor/api.ts:168 (saveSession -> wasm EditorSession)
  -> document_edit_api.ts:79 (refreshDocument)
  -> pdf_runtime.ts:161 (invalidateVectorRenderCache)
  -> 渲染循环
  或
save_pdf 按钮 -> pdf_viewer_api.ts:180 (save)
  -> editor/index.ts:662 (saveEdits)
  -> editor_wasm_api.ts:205 (saveSession -> wasm)
  -> Tauri save_pdf -> document_service.rs:199
    -> region_materializer.rs -> pdf_write/reflow.rs
```

### 第 3 步：加诊断日志

项目内置了诊断系统：

- **Rust 侧：** `pdf_log!()` 宏（`log_service.rs`），0-3 级。
  用 Tauri 命令 `set_log_level(level)` 设置。
- **TS 侧：** `emitPdfDiagnostic()`（`shared/diagnostics.ts`）输出到 console +
  `window.__PDF_DIAGNOSTICS_HISTORY` + Tauri `terminal_log`。
- **事件日志：** `read_pdf_event_log` / `clear_pdf_event_log` 命令暴露一个
  512 条的环形缓冲。
- **布局追踪：** `src/bridge/render/layout_trace.ts` 在 mismatch/transform/verbose
  时记录 DOM 几何。
- **编辑器自检：** DevTools console 里执行 `window.verifyEditorBugs()`
  （`src/dev/verify_editor_bugs.ts`）。

### 第 4 步：修复前先写测试

**Rust（core/ui crate）：** 写一个能复现 bug 的 `#[test]` 或
`#[wasm_bindgen_test]`。模式参考 `host/layout.rs` 的测试（108-200 行）：
纯函数调用 + `assert_close`。

**src-tauri：** 写 `#[cfg(test)]` 单元测试。很多模块已有（`color.rs` 22 个、
`glyph_mapping.rs` 18 个、`pdf_write/reflow.rs` 15 个等）。

**TS：** 在 `src/__tests__/` 里写 vitest。测试架子已就位（`vitest.config.ts`），
但覆盖还很薄。

**E2E：** 在 `tests/e2e/specs/` 里加 spec（需要 tauri-driver）。

### 第 5 步：常见修复模式

**缩放时渲染错位：**
`host/layout.rs` 里的缩放布局契约（`dom_width * css_scale == display_width`）
是唯一事实源。视觉与预期不符时，打印 `SyncHostLayoutResult` 的各字段，
看哪条不变量被破坏。

**字体渲染错误：**
查 `font/match_mod.rs` 的系统字体替换--它做字重匹配和 CJK 回退。
字体解析链是 `parse.rs` -> `face.rs` -> `embed.rs`。

**编辑补丁未生效：**
追 `region_materializer.rs::build_region_materialization_plan`--它把
region_patches + text_reflows 合成生效的 `TextReflowPatch` 条目。
查 `cache.pdf_materialization_reports` 里的物化报告。

**编辑后页面数据过期：**
显式失效缓存：`cache.rs` 里的 `invalidate_pdf_page_cache`（按前缀）。
`edit_commands.rs` 的编辑路径会自动做；手工改数据的必须调 `requestRefresh`。

---

## 5. 在 `main` 还是 `refactor/architecture-improvements` 上工作

**`main`** 是生产分支。包含全部抢救回来的 bug 修复（marker、字体、缩放、
对话框）以及深度模块重构（TextState、TextMatrixCore、模块删除）。
PR 一律对着 main。

**`refactor/architecture-improvements`** 是当前活跃的架构分支。
近期提交：
- 领域词汇表（`CONTEXT.md`）和缩放规格书（`2026-08-04-zoom-bug-fix-via-merge.md`）
- Vitest 前端测试架子（`diagnostics.test.ts`）
- 浅层模块删除，委托给 `pdf-viewer-core`
- 依赖倒置修复

**`fix/zoom-layout-tests-wasm-runnable`** 是个小分支（2 个提交），把缩放抵消
测试修成 wasm 可运行，并移植了手动 E2E 手册。是 main 的 PR 候选。

**`codex/refactor-split`** 是历史分支。不要合入 main--缩放修复和字体/marker/
对话框修复已经通过批量抢救进入 main。整体合并会经由 33 处冲突引入一套平行
架构。

---

## 6. 必须保持的关键不变量

1. **缩放契约：** `dom_width * css_scale == display_width`（由 `host/layout.rs`
   的 5 个 wasm 测试守护）。绝不把 `displayWidth` 直接写进容器；
   一律走 `syncHostLayout`。

2. **编辑顺序：** `clearVectorHost()` 必须在 `session.open()` **之前**调用，
   以取消渲染中的请求。这只靠 `pdf_document_runtime.ts:96-100` 的调用顺序
   保证，API 形状本身不强制。

3. **缓存失效：** 任何文档变更后，失效：
   `pdf_page_cache`、`pdf_page_intermediate_cache`、`pdf_layout_cache`、
   `PDF_RESOLVE_PATHS_CACHE`（按文档指针前缀匹配）。

4. **Wasm 单例：** `DocumentSession`、`ViewerSession`、`EditorSession`、
   `ReviewSession`、`PagePresentationRuntime` 是惰性构造的单例，背后是 wasm
   的 thread_local 状态。TS 对象只是句柄--真实状态在 wasm 里。

5. **工作副本生命周期：** `resolve_working_path` 会复制到
   `%TEMP%\working_{md5}.pdf`。所有保存都写**原始路径**。工作副本在编辑后
   过期，只有在文件不存在时才会重建。
