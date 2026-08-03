# Domain 配置

> 由 setup-matt-pocock-skills 于 2026-08-04 生成 · 供 to-spec / wayfinder / triage 技能读取

## C. 上下文

**单上下文**（非 monorepo 信号）：

| 上下文 | 位置 | 说明 |
|---|---|---|
| 根上下文 | `CONTEXT.md` | 当前不存在；未来若创建，作为领域总纲 |
| 架构决策记录（ADR） | `docs/adr/` | 当前无内容；ADR 是 brainstorm / spec 必须尊重的既有决策 |

## 领域基线（当前实际生效）

- `docs/README.md` — 文档索引 + 阅读顺序 + 时效性原则（必读 5 篇）。
- `docs/development-guide.md` — 开发指南（决策树、WASM API 五步流程、Stub/Experimental/Stable、提交前 checklist）。
- `docs/architecture-principles.md` — 架构铁律（单一渲染链、单一 Rust 入口、页面层独占 suppression）。
- `docs/architecture-overview.md` — 三层架构总览（Presentation / Interface / Infrastructure）。

## 说明

- 检测到 monorepo 信号（多个独立子项目）时，迁移为 multi-context（`CONTEXT-MAP.md`），并更新本文件。
