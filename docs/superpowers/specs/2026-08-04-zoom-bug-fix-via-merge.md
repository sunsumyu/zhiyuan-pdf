# Zoom Bug Fix via Merge codex/refactor-split

> 2026-08-04 · 源自用户反馈的 3 个缩放 bug + `codex/refactor-split` 分支已有的修复

> **状态（2026-08-15）：已实现（经 salvage，非合并）--勿再执行本合并。**
>
> - main 已通过 salvage 批次吸收整条 zoom 修复序列：`578c058`（源自
>   `f968c9b..87bf89a`，含 Rust 布局契约、TS 布局同步、双倍缩放闪烁修复及其单测），
>   已随 salvage 批次推送至 origin/main。
> - 全量合并 `codex/refactor-split` 会产生 33 个冲突并引入一套被 main 深模块重构
>   取代的平行架构（host->platform 重命名、模块拆分等），已评估并放弃。
> - 自动化验证全绿（2026-08-15，main）：core 75 passed；ui wasm 9 passed
>   （5 个 `host::layout` 取消保证测试经 `6f7b614` 修复为 wasm 可运行后全过）；
>   `npm run build`、`npm run wasm:pdf-viewer-ui` 均 exit 0。
> - 剩余唯一未完成项：人工 E2E（场景 A-D），见
>   `fix/zoom-layout-tests-wasm-runnable` 分支上的
>   `docs/runbooks/manual-zoom-e2e-verification.md`。

## Problem Statement

用户在 `main` 分支上打开 PDF 后，缩放时有 3 个 bug：

1. **位置跳**（scroll position jumps）：缩放时滚动位置突然跳变
2. **显示模糊**（blurry rendering）：缩放后内容模糊，分辨率不匹配
3. **不以中心缩放**（zoom anchor not centered）：鼠标位置不固定，缩放时画面偏移

## Solution

**合并 `codex/refactor-split` 分支到 `main`**。

该分支包含 4 个 commit：

| commit | 内容 | 修复 bug |
|---|---|---|
| `f968c9b` | feat(zoom): update Rust layout sync contract for preview-aware DOM sizing | Bug 2 |
| `27c5799` | refactor(zoom): simplify TS layout sync to use Rust domWidth and cssScale | Bug 1, Bug 2 |
| `1af4024` | fix(zoom): remove container width/height overwrite in applyViewportCanvasFrame | Bug 1, Bug 2 |
| `e7bb89e` + `87bf89a` + `a8e7f49` + `c204d36` | 文档驱动改造 + zoom 单测闭环 | （配套文档与测试） |

### 修复原理

- **Bug 1（位置跳）**：`27c5799` 让 TS 布局同步直接使用 Rust 计算的 `domWidth/domHeight/cssScale`，消除 `applyCommittedFrame` 中清除 CSS transform 后又重新调整 DOM 尺寸的不连续跳变。
- **Bug 2（显示模糊）**：`f968c9b` + `27c5799` 让容器尺寸设为 `domWidth = pageWidth * renderZoom`（匹配 canvas 渲染分辨率），而非 `displayWidth = pageWidth * displayZoom`，消除 CSS scale 不匹配。`1af4024` 修复了 `applyViewportCanvasFrame` 中的双重缩放问题。
- **Bug 3（不以中心缩放）**：部分改善（`27c5799` 统一了 CSS transform 应用点），但锚点计算逻辑（`zoom_interaction.rs`）未变，可能仍有残留问题。

## Verification Plan

### 自动化验证（合并前在 codex/refactor-split 已验证）

- `cargo test -p pdf-viewer-ui` → 19 passed
- `cargo test -p pdf-viewer-core` → 119 passed
- `npm run build` → exit 0
- `npm run wasm:pdf-viewer-ui` → exit 0

### 手动验证（合并到 main 后）

1. `npm run wasm:pdf-viewer-ui && npm run dev`（或 `npm run tauri:dev`）
2. 打开 PDF，缩放测试：
   - Dropdown 100% → 125% → 150%：观察无位置跳、无模糊、鼠标位置固定
   - 工具栏 +/- 按钮：同上
   - Ctrl + 鼠标滚轮：连续平滑缩放
3. 如有残留 bug，单独开分支修复

## Out of Scope

- 不重新设计 zoom 系统（现有修复已覆盖大部分问题）
- 不引入 CI 自动跑 E2E
- 不合并其他分支

## Risks

- **锚点缩放残留**：`zoom_interaction.rs` 的锚点计算逻辑未变，可能仍有"不以中心缩放"问题。如出现，需在 main 上单独修。
- **合并冲突**：`main` 与 `codex/refactor-split` 自分叉后可能有其他改动，合并时需解决冲突。
