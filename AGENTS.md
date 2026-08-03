# AGENTS.md — 纸鸢 PDF 阅读器 · 开发代理指引

> 本文件面向所有开发代理（Claude / Codex / ZCode 等）。**任何创造性工作（新功能、组件、行为修改、重构）都必须先走文档驱动流水线，禁止直接写代码。**

## 文档驱动开发流水线

```
想法
  │
  ▼
① brainstorming ──产出──▶ docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md（已提交、设计获批）
  │                        （HARD-GATE：设计未获批，禁止任何实现动作）
  ▼
② writing-plans ──产出──▶ docs/superpowers/plans/YYYY-MM-DD-<feature>.md
  │                        （含 Goal / Architecture / Global Constraints / Task 粒度 / checkbox 步骤，禁占位符）
  ▼
③ executing-plans（内联） 或 subagent-driven-development（多任务/需评审，推荐）
  │                        （执行前用 using-git-worktrees 建隔离；绝不在 main 分支实现）
  ▼
④ verification-before-completion ──▶ finishing-a-development-branch
  │                        （每条成功声明都必须先在本轮跑过验证命令）
  ▼
收尾 / 合并 / PR
```

- **入口判断**：`to-spec`（把已讨论内容合成 PRD）与 `to-tickets`（把 plan/spec 拆成带 `Blocked by` 边的垂直切片）是流水线的旁路桥，用于把本地文档体系发布到 issue tracker。超大型模糊任务先 `wayfinder` 照清路线，再进入 ①。
- **执行子技能选择**：任务彼此独立且需逐任务评审 → `subagent-driven-development`；内联轻量执行 → `executing-plans`。两者末尾都强制走 `finishing-a-development-branch`。

## Agent skills

> 配置来源：`docs/agents/`（由 setup-matt-pocock-skills 生成）。本文档引用流水线所需的 agent skills。

### Issue tracker

**配置声明：`docs/agents/issue-tracker.md`**

- 当前 tracker 类型：**Local markdown**。
- 约定：spec 与 tickets 为本地 markdown 文件，位于 `.scratch/<feature-slug>/`。
  - spec：`.scratch/<feature-slug>/spec.md`
  - tickets：`.scratch/<feature-slug>/issues/<NN>-<slug>.md`（`01` 起按依赖顺序编号）
  - blocking 边：票文件内的 `Blocked by: NN, NN` 行
  - triage 状态：票文件顶部 `Status:` 行
  - 评论：追加到 `## Comments` 节
- 发布时对 spec/tickets 打 `ready-for-agent` 标签（标签词汇见 `docs/agents/triage-labels.md`）。
- 改用其它 tracker（GitHub gh / GitLab glab / Other）前，先更新此文件。

### Triage labels

**配置声明：`docs/agents/triage-labels.md`**

五个 canonical 角色 → 实际标签字符串的映射。`to-spec`/`to-tickets`/`triage`/`wayfinder` 使用此映射发布状态标签。

### Domain

**配置声明：`docs/agents/domain.md`**

- 单上下文（非 monorepo）：根 `CONTEXT.md` + `docs/adr/`。
- `docs/adr/` 为架构决策记录（ADR），是 brainstorm/spec 阶段必须尊重的既有决策。
- 本项目没有 `CONTEXT.md`、`docs/adr/` 尚无内容时，以 `docs/README.md` 的「必读」文档与 `docs/development-guide.md` 为领域基线。

## 分层架构硬约束（源自 `.cursorrules` / `.windsurfrules`）

- 三层：**Presentation 层** `src/bridge`（TS）· **Interface/Controller 层** `src-tauri/interfaces` · **Infrastructure 层** `src-tauri/infrastructure` + `crates`。
- **六大硬性构建检查点**（改动后必须跑）：`cargo check`（core / ui / standalone）、`cargo test`（core）、`tsc && vite build`、`cargo build`。
- 编码规范、提交前 checklist 见 `docs/development-guide.md` §7、§8。

## 文档时效性原则（`docs/README.md`）

- 标注日期且 < 6 个月：**当前有效**，按原文执行。
- 标注日期但 > 6 个月：**仅作背景参考**，需与当前代码对照验证后再采纳。
- 未标注日期或与现状明显冲突的文档：**忽略**。
