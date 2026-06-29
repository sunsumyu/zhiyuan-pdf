# pdf-viewer-standalone 文档目录

本目录汇集了项目的架构原则、设计决策与历史经验。

> 项目是从 `nushell-enhanced` 分叉而来的 PDF 阅读/编辑器单体应用。许多核心架构原则（特别是渲染单链原则）继承自 nushell-enhanced 的设计，必须严格遵守，否则会复发"链分叉"类视觉缺陷。

---

## 阅读顺序

### 必读（动手改任何编辑器/渲染相关代码前）

1. **[architecture-principles.md](architecture-principles.md)** — 项目架构铁律：单一渲染链、单一所有者、Rust core/UI/TS 边界。
2. **[editor-render-architecture.md](editor-render-architecture.md)** — 编辑态渲染机制：canvas 单链、textarea 仅做输入捕获、为何不能用浏览器字体。
3. **[editor-core-execution-flow.md](editor-core-execution-flow.md)** — 编辑核心执行流：source geometry、caret/index、删除、marker、overlay/suppression 与离线验证矩阵。
4. **[page-presentation-runtime-architecture.md](page-presentation-runtime-architecture.md)** — 翻页、preview、vector、detail、prefetch、present 的框架级 runtime 方案。
5. **[nushell-divergence-report-2026-05-06.md](nushell-divergence-report-2026-05-06.md)** — 2026-05-06 发现的与 nushell-enhanced 的 4 处分叉点及修复记录。

### 参考

- **[editing-layout-call-formulas.md](editing-layout-call-formulas.md)** — 当前编辑/布局调用链与坐标、caret、hit-test、commit/save 公式速查。
- **[editor-core-execution-flow.md](editor-core-execution-flow.md)** — 编辑核心执行流、marker/删除链路、离线验证矩阵。
- **[origin/](origin/)** — 从 nushell-enhanced 复制的原始权威文档（保留作为来源）。
- **[origin/pdf-engine-naming-guide.md](origin/pdf-engine-naming-guide.md)** — Rust/TS 命名规范。

---

## 文档时效性原则

- 标注日期且 < 6 个月：**当前有效**，按原文执行。
- 标注日期但 > 6 个月：**仅作背景参考**，需要与当前代码对照验证后再采纳。
- 未标注日期或与现状明显冲突的文档：**忽略**。

`origin/` 下的文档大多写于 2026-04，其中 `pdf-blue-bar-investigation` 与 `pdf-viewer-architecture-audit` 仍然是当前架构的最佳描述。其它 `pdf_engine/*` 文档反映 V3 设计意图，部分已实现部分未实现，需要审慎参考。

---

## 一句话原则速记

> **PDF-Glyph 链是唯一视觉链。** 所有可见像素必须通过 Rust canvas painter 输出。浏览器字体、textarea 渲染、HTML 文本节点都不能承担"显示文字"的职责。textarea 只是**屏外的输入捕获器**。

> **每个能力只允许一个 Rust 入口。** 编辑、坐标转换、渲染事务、保存写回 —— TS 不持有领域规则，不"补救"Rust 漏洞。

> **抑制（suppression）由页面层独占。** 编辑层不再画白底遮原文。如果原 PDF path 漏画/多画，到 `effective_page_plan` / `canvas` 去修，不是到编辑层去补。
