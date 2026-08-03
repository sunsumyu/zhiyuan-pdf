# Zoom & Layout Refactor — Verification Close-out Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the verification gap of the zoom/layout refactor: add Rust unit tests proving the `dom_width * css_scale == display_width` cancellation guarantee, run automated checks, and record status in the original plan document.

**Architecture:** Rust (`pdf-viewer-ui`) unit tests live in `platform/layout.rs`; TS verification is `tsc --noEmit`. The original plan (`docs/plans/2026-07-30-zoom-layout-refactor-plan.md`) checkbox states are backfilled from observed evidence.

**Tech Stack:** Rust (`pdf-viewer-ui`), TypeScript (tsc), git (conventional commits).

**Global Constraints** (copied verbatim from `docs/superpowers/specs/2026-08-04-zoom-layout-refactor-verification-design.md`):
- 自动化：`cargo test --package pdf-viewer-ui` + `npx tsc --noEmit`。
- 不修改 `sync_host_layout` 的实现逻辑（除新增测试外零改动），避免回归风险。
- 不改 zoom 算法/契约、不合并分支、不推送远端。
- 手动 E2E 完成前，Task 3 Step 3/4 保持 `- [ ]`。

---

### Task 1: Add `sync_host_layout` cancellation unit tests

**Files:**
- Modify: `crates/pdf-viewer-ui/src/platform/layout.rs`

**Interfaces:**
- Consumes: `sync_host_layout(SyncHostLayoutRequest) -> SyncHostLayoutResult` (existing signature).
- Produces: `#[cfg(test)] mod tests` with 5 test cases.

- [x] **Step 1: Add test module to `layout.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    const PAGE_W: f32 = 612.0;
    const PAGE_H: f32 = 792.0;
    const VIEWPORT_W: f32 = 800.0;
    const VIEWPORT_H: f32 = 600.0;

    fn request(display_zoom: f32, render_zoom: Option<f32>) -> SyncHostLayoutRequest { /* … */ }
    fn assert_close(actual: f32, expected: f32, msg: &str) { /* … */ }

    #[test]
    fn committed_state_has_identity_css_scale() { /* … */ }
    #[test]
    fn preview_state_cancels_css_scale_against_render_zoom() { /* … */ }
    #[test]
    fn zoom_in_preview_does_not_flash_larger() { /* … */ }
    #[test]
    fn render_zoom_defaults_to_display_zoom() { /* … */ }
    #[test]
    fn sanitizes_invalid_zoom_inputs() { /* … */ }
}
```

- [x] **Step 2: Run `cargo test --package pdf-viewer-ui`**
  - 2026-08-04 evidence: `test result: ok. 19 passed; 0 failed` (5 new `platform::layout::tests` included).

- [x] **Step 3: Run `npx tsc --noEmit`** — exit 0.

- [x] **Step 4: Commit Task 1**

```bash
git commit -m "test(zoom): add sync_host_layout cancellation unit tests and backfill plan checkboxes"
```

---

### Task 2: Backfill original plan checkboxes

**Files:**
- Modify: `docs/plans/2026-07-30-zoom-layout-refactor-plan.md`

**Interfaces:**
- Consumes: execution evidence from Task 1.
- Produces: `- [x]` for Task 1/2 and automated Task 3 evidence; `- [ ]` kept for manual E2E steps.

- [x] **Step 1: Mark Task 1 Step 1–4 as `- [x]`**
- [x] **Step 2: Mark Task 2 Step 1–4 as `- [x]`**
- [x] **Step 3: Update Task 3 Step 1–2 with automated evidence; keep Step 3–4 as `- [ ]` (manual E2E pending).**

---

### Task 3: Manual E2E verification (human-in-the-loop, still pending)

**Files:**
- Test: run `npm run tauri:dev` interactively.

- [ ] **Step 1: Zoom via dropdown (100% → 125% → 150%)** — observe no transient double-scaling flash.
- [ ] **Step 2: Zoom via toolbar +/− buttons** — smooth transition.
- [ ] **Step 3: Ctrl + Mouse Wheel zoom** — continuous smooth scaling without pop/flash.
- [ ] **Step 4: Commit verification marker**

```bash
git commit --allow-empty -m "test(zoom): verify smooth zoom without double-scaling flash"
```

> 说明：本任务需要人工交互，不可由 agent 自动完成；完成后回填 `docs/plans/2026-07-30-zoom-layout-refactor-plan.md` 的剩余 `- [ ]`。
