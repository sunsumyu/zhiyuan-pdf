# Sovereignty PDF Viewer -- 开发者指南

> **从这里开始。** 本目录是本代码库的权威工作文档。全部内容基于当前工作树
> （2026-08-16）生成，反映的是代码今天的真实状态。

## 快速上手

1. **先读 `architecture-map.md`** —— 逐层的完整模块地图：每个模块、每个已注册的
   Tauri 命令、每个 WASM 导出、每条端到端流程（打开、渲染、缩放、编辑），
   全部带 file:line 锚点。

2. **再读 `development.md`** —— 构建/测试命令与确切的坑（wasm target 要求、
   bindgen 版本必须严格匹配、tauri:dev 与 vite-only 的区别）、bug 排查决策树，
   以及必须保持的关键不变量。

3. **需要某个具体模块时**，去架构图的模块表（第 2-4 层）里查。每个模块都有
   1-2 行职责说明和行数。

---

## 架构图（`architecture-map.md`）

覆盖完整的四层技术栈：

| 层 | 目录 | 行数 | 职责 |
|---|---|---|---|
| UI 外壳 | `index.html`, `src/main.ts` | ~300 | DOM 事件、?file= 参数、按钮接线 |
| TS 桥接层 | `src/bridge/` | ~12,000 | 运行时组装、渲染循环、缩放、编辑 |
| WASM crate | `crates/pdf-viewer-ui/` | ~5,500 | wasm-bindgen 导出、排版契约、缩放宿主 |
| 纯 Rust 库 | `crates/pdf-viewer-core/` | ~4,000 | 领域模型、文本状态、渲染计划 |
| 桌面后端 | `src-tauri/` | ~10,700 | 30 个 IPC 命令、PDF 解析、字体引擎、读写 |

另附：死代码清单、重复逻辑、命名危害。

## 开发指南（`development.md`）

- **构建命令**及各自的坑（wasm-pack、bindgen 版本、vite-only 的限制）
- 每一层的**测试命令**（附实际验证过的输出）
- **bug 排查决策树**（症状 -> 所在层 -> 调用链）
- **常见修复模式**（缩放契约、字体、编辑补丁、缓存失效）
- **分支指南**（main vs architecture-improvements vs fix 分支）

---

## 文档清单 -- 哪些现行、哪些过期

`docs/` 目录在项目整个生命周期里积累了约 45 个文件。多数写于 2026 年 8 月的
"抢救式重构"（salvage）之前，不反映当前代码库。以下是一次如实的评估。

### 现行（用这些）

| 文件 | 日期 | 状态 | 备注 |
|---|---|---|---|
| `docs/guide/README.md` | 2026-08-16 | **现行** | 本文件 |
| `docs/guide/architecture-map.md` | 2026-08-16 | **现行** | 基于工作树的完整模块地图 |
| `docs/guide/development.md` | 2026-08-16 | **现行** | 构建/测试/调试参考 |
| `CONTEXT.md` | 2026-08-15 | **现行** | 领域词汇表（本分支新建） |
| `docs/superpowers/specs/2026-08-04-zoom-bug-fix-via-merge.md` | 2026-08-15 | **现行**（已关闭） | 缩放规格书，标记为已通过 salvage 实现 |
| `.scratch/UBIQUITOUS_LANGUAGE.md` | 2026-08-15 | **现行** | 领域术语定义 |
| `docs/runbooks/manual-zoom-e2e-verification.md` | 2026-08-16 | **现行** | E2E 手动验证手册（在 `fix/zoom-layout-tests-wasm-runnable` 分支上） |
| `docs/bug-postmortems/blue-block-overlay-artifact.md` | ? | **现行** | 具体 bug 的复盘 |
| `docs/bug-postmortems/vector-text-rendering-blank-issue.md` | ? | **现行** | 具体 bug 的复盘 |

### 部分过期（可当背景读，细节勿信）

| 文件 | 日期 | 过期程度 |
|---|---|---|
| `docs/architecture-overview.md` | 2026-05-06 | salvage 之前写的。高层流程大体正确，但模块名和模块关系已变（浅层模块已删、新增 TextState）。 |
| `docs/architecture-principles.md` | 2026-05-06 | 原则仍然有效；示例引用的是旧模块名。 |
| `docs/page-presentation-runtime-architecture.md` | 2026-06-03 | 写得很细，但早于缩放修复和模块重组。 |
| `docs/edit-save-architecture.md` | ? | 编辑流程描述大体正确；缺 salvage 后 `edit_commands.rs` 和 `region_materializer.rs` 的变化。 |
| `docs/editor-api-architecture-proposal.md` | ? | 125KB 的设计提案。部分已实现、部分已放弃。仅作历史背景读。 |
| `docs/route-b-core-redesign.md` | ? | 84KB 的核心库抽取设计文档。部分落地（`pdf-viewer-core` 已存在）。理解设计意图仍有用。 |
| `docs/development-guide.md` | 2026-05-06 | 旧开发指南。已被上面的 `docs/guide/development.md` 取代。 |
| `docs/origin/`（10 个文件） | 各时期 | 项目初期的原始设计文档。历史资料。 |
| `docs/naming-and-architecture-refactor-plan.md` | ? | 重构计划。大部分已执行；个别条目被 salvage 取代。 |
| `docs/naming-refactor-review-plan.md` | 27KB | 命名审计。部分执行。 |

### 已过期（不要用于当前开发）

| 文件 | 日期 | 过期原因 |
|---|---|---|
| `docs/architecture-audit.md` | 2026-05-09 | salvage 前的审计。模块结构已大变。 |
| `docs/architecture-diagrams.md` | ? | 图引用的是旧模块布局。 |
| `docs/architecture-review.md` | ? | 22KB 的 salvage 前架构评审。 |
| `docs/api-audit.md` | ? | salvage 前 API 面的审计。 |
| `docs/api-contract.md` | ? | salvage 前 API 面的契约。 |
| `docs/method-inventory.md` | 268KB | 超大方法清单——生成的，全部是 salvage 前的。 |
| `docs/method-constraint-audit.md` | 106KB | 生成的，salvage 前。 |
| `docs/structure-flow-audit.md` | ? | salvage 前的结构审计。 |
| `docs/framework-refactor-completion-plan.md` | ? | 重构计划，大部分已执行或被取代。 |
| `docs/startup-performance-plan.md` | ? | 项目早期的性能计划。 |
| `docs/ts-to-rust-migration-plan.md` | ? | TS 优先时代的迁移计划。已不再相关。 |
| `docs/nutrient-comparison.md` | ? | 项目初期的 PDF 库选型对比。 |
| `docs/nushell-divergence-report-2026-05-06.md` | 2026-05-06 | 一次性差异报告。 |

### 归档（仅历史参考）

| 目录 | 内容 |
|---|---|
| `docs/archive/`（10 个文件，650KB+） | 早期架构计划、方法映射、命名规范。全部为 salvage 前。 |
| `docs/origin/`（10 个文件） | 原始设计文档。历史背景。 |
| `docs/images/`（4 张 PNG） | 架构/编辑/渲染/UI 图。可能显示旧布局。 |

### 草稿区（工作产物，不是文档）

| 目录 | 内容 |
|---|---|
| `.scratch/tickets/`（5 个文件） | 进行中的工单队列（01-05） |
| `.scratch/zoom-layout-refactor-verification/` | 缩放验证子工单 |
| `.scratch/unify-dispatch/` | TextState 统一子工单 |
| `.scratch/wayfinder-map.md` + `wayfinder-README.md` | 早期架构工作的 Wayfinder 探索记录 |
| `.scratch/*.diff`, `.scratch/*.rs`, `.scratch/*.log` | 临时工作文件 |
