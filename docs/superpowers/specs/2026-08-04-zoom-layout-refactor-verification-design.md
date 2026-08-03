# Zoom & Layout Refactor — Verification & Close-out Design

> 2026-08-04 · 源自 `docs/zoom-layout-refactor-design.md` 的验证计划 + `docs/plans/2026-07-30-zoom-layout-refactor-plan.md` Task 3。
> 本设计是 brainstorming 阶段产物，评审通过后由 writing-plans 派生实现计划。

## Problem Statement

`zoom-layout-refactor-design.md` 与 `2026-07-30-zoom-layout-refactor-plan.md` 定义的 zoom/layout 架构重构，**代码实现已全部落地**（Rust 契约 `f968c9b`、TS 简化 `27c5799`、canvas host 根因修复 `1af4024`），但其**验证计划从未闭环**：

1. 设计文档验证计划第 1 条要求的 Rust 单元测试（`dom_width * css_scale == display_width`，preview vs committed 态）**未写**。
2. 实现计划 Task 3 的手动 E2E 验证（dropdown / buttons / Ctrl+滚轮，观察无闪图）**未执行**。
3. 计划文档 checkbox 全部停留在 `- [ ]`，无法反映真实完成度。

## Solution

### 1. Rust 单元测试（已完成于本会话 2026-08-04）

在 `crates/pdf-viewer-ui/src/platform/layout.rs` 新增 `#[cfg(test)] mod tests`，覆盖：

- **committed 态**：`render_zoom == display_zoom` → `css_scale == 1.0`、`dom_width == display_width`。
- **preview 态**：`render_zoom < display_zoom` → 核心不变量 `dom_width * css_scale == display_width`（数学消去保证）。
- **防二次放大闪图**：`visual_width` 不得等于原缺陷值 `W * Z_display² / Z_rendered`。
- **fallback**：`render_zoom` 缺省 → 回退到 `display_zoom`。
- **sanitize**：非法（NaN）输入 → 回退默认，不 panic。

验证命令：`cargo test --package pdf-viewer-ui`（2026-08-04 实测 19 passed）。

### 2. 手动 E2E 验证（仍待人工）

以下无法在自动化环境完成，需交互式运行 `npm run tauri:dev` 并人工观察：

- dropdown 100% → 125% → 150%：无瞬态 double-scaling 闪图、平滑过渡。
- 工具栏 +/− 按钮：同左。
- Ctrl + 鼠标滚轮：连续平滑缩放、无 pop/flash、布局稳定。

### 3. 计划回填（已完成于本会话 2026-08-04）

`2026-07-30-zoom-layout-refactor-plan.md` checkbox 按实际完成度回填：Task 1/2 全部 `- [x]`，Task 3 自动化证据部分 `- [x]`、手动 E2E 保持 `- [ ]` 并注明待办。

## Implementation Decisions

- **D1**：单测以设计文档验证计划为唯一需求源；若与实际实现有偏差，以代码为准并回填偏差说明（本次无偏差）。
- **D2**：不修改 `sync_host_layout` 的实现逻辑（除新增测试外零改动），避免回归风险。
- **D3**：手动 E2E 不在自动化范围，作为独立待办（local tracker ticket）保留。

## Testing Decisions

- 自动化：`cargo test --package pdf-viewer-ui` + `npx tsc --noEmit`（`tsc` 验证 TS 侧无回归）。
- 手动：真实桌面 app 交互（dropdown / buttons / Ctrl+滚轮）。

## Out of Scope

- 不改 zoom 算法/契约、不合并分支、不推送远端。
- 不引入 CI 自动跑 E2E。

## Further Notes

- 手动 E2E 完成前，Task 3 Step 3/4（Ctrl+滚轮验证、验证提交）保持 `- [ ]`。
- 后续若启用 GitHub tracker，本设计的 tickets 可迁移（见 `docs/agents/issue-tracker.md`）。
