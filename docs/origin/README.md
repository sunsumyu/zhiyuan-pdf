# origin/ — 来自 nushell-enhanced 的原始权威文档

本目录文件**逐字复制**自 `E:\chain\nushell-enhanced\docs\`。保留作为来源参考与对比基准。修改本项目代码前应先阅读对应文档；任何与原文档冲突的实现都需要明确理由。

---

## 文档清单与时效评估

| 文件 | 来源 | 写于 | 时效 | 优先级 |
|------|------|------|------|--------|
| `pdf-blue-bar-investigation-2026-04-24.md` | 同名 | 2026-04-24 | ✅ 当前有效 | ⭐⭐⭐ 编辑器 owner 模型权威 |
| `pdf-viewer-architecture-audit-2026-04-19.md` | 同名 | 2026-04-19 | ✅ 当前有效 | ⭐⭐⭐ 总体架构图与重构路线 |
| `pdf-engine-naming-guide.md` | `pdf_engine/NAMING_GUIDE.md` | 无日期 | ✅ 风格规则稳定 | ⭐⭐⭐ 命名铁律 |
| `rust-pdf-viewer-rust-first-architecture.md` | `pdf_engine/RUST_PDF_VIEWER_RUST_FIRST_ARCHITECTURE.md` | 无日期 | ✅ Rust-first 原则与 audit 一致 | ⭐⭐ 边界论 |
| `pdf-editing-strategy.md` | `pdf_engine/pdf_editing_strategy.md` | 无日期 | ⚠️ 部分实现 | ⭐ 编辑策略概述 |
| `pdf-render-engine-roadmap.md` | `pdf_engine/pdf_render_engine_roadmap.md` | 无日期 | ⚠️ 路线图，部分已实现 | ⭐ 渲染引擎规划 |
| `pdf-layout-engine-v3-design.md` | `pdf_engine/PDF_LAYOUT_ENGINE_V3_DESIGN.md` | 无日期（V3 时代）| ⚠️ V3 设计意图，已部分落地 | ⭐ 排版引擎设计 |
| `rust-pdf-font-engine-refactor.md` | `pdf_engine/RUST_PDF_FONT_ENGINE_REFACTOR_PLAN.md` | 无日期 | ⚠️ 字体引擎重构计划，未必全部完成 | ⭐ 字体处理参考 |
| `pdf-region-engine-style-preservation.md` | `pdf_engine/PDF_REGION_ENGINE_STYLE_PRESERVATION_UPGRADE.md` | 无日期 | ⚠️ 大文档，含若干已过时假设 | ⭐ 仅按需查阅 |

---

## 阅读建议

- **入门优先级（必读）：**
  1. `pdf-blue-bar-investigation-2026-04-24.md` — 单一所有者模型
  2. `pdf-viewer-architecture-audit-2026-04-19.md` — 三层边界与重构阶段
  3. `pdf-engine-naming-guide.md` — 命名规范

- **专题查阅：**
  - 字体/字形：`rust-pdf-font-engine-refactor.md`
  - 渲染引擎：`pdf-render-engine-roadmap.md` + `pdf-layout-engine-v3-design.md`
  - 编辑策略：`pdf-editing-strategy.md`
  - 区域风格保留：`pdf-region-engine-style-preservation.md`

- **不要读：** nushell-enhanced 中其它 `ALGORITHM_*`、`SUPER_BRAIN_*`、`LCA_*`、`INSERTION_SORT_*`、`TYPESCRIPT_*` 等文档——那些是 nushell 项目早期作为算法可视化器时期的产物，与 PDF Viewer 无关。

---

## 时效性提示

`pdf_engine/` 下的 V3 设计文档大部分**写于本项目分叉之前**，描述了 V3 引擎的设计意图。许多设计已落地为 `pdf-viewer-core` 中的 `models.rs::*V3` / `LayoutInferenceResultV3` / `VectorPageModelV3` 等类型。但也有部分（例如 Bincode/MessagePack 替代 JSON、Dioxus 全 Rust 化前端）**尚未实现**。

阅读时请对照实际代码验证，**不要把"设计文档说要做"等同于"项目已经在这样做"**。

---

## 与上层综合文档的关系

- `../README.md` — 全项目文档索引
- `../architecture-principles.md` — 已综合 `pdf-blue-bar-investigation` + `pdf-viewer-architecture-audit` + `pdf-engine-naming-guide` 的核心要点
- `../editor-render-architecture.md` — 已综合 `pdf-blue-bar-investigation` 的 owner 模型
- `../nushell-divergence-report-2026-05-06.md` — 2026-05-06 的分叉修复实战记录

如果上层综合文档与本目录原文有冲突，**以原文为准**——上层文档可能因综合而省略细节。
