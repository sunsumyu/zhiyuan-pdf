# 架构变体与特征树

> 本文用论文中的 feature tree / implementation variants 思路描述本项目的合法运行形态、模块特征树与外部参考"借配方"登记规则。
> 与 `CONTRACTS.md` 分工如下：
>
> - `CONTRACTS.md`：跨模块不变量与修改联动规则。
> - **本文**：运行时变体、模块特征树、兼容/迁移状态、外部参考登记。

---

## 1. 变体规则

- 每个变体必须有明确进入条件。
- 每个变体必须有唯一 owner。
- 变体之间的状态转换必须可追踪。
- 不允许两个变体同时写同一几何状态。
- 变体必须有明确退出条件。

---

## 2. 运行时变体矩阵

| 变体 | 进入条件 | 主 surface | 几何 owner | 渲染 owner | 退出条件 | 相关 ADR |
|---|---|---|---|---|---|---|
| Idle vector | 普通稳定显示 | vector | render pipeline | vector renderer | wheel/input | ADR-0003 / ADR-0004 |
| Zoom animation | wheel gesture | vector | Rust RAF loop | incremental render | settle | ADR-0002 / ADR-0006 |
| Settle | animation 落定 | vector | Rust RAF loop | final render | idle | ADR-0002 |
| Preview/tile | 快速预览可用 | raster/vector | preview path | raster/tile | vector commit | ADR-0003 |
| Progressive render | 大页面/多对象 | vector | render pipeline | progressive renderer | complete | ADR-0004 |
| Editor active | 编辑器打开 | vector + editor canvas | editor/render path | effective plan | close/commit | principles |
| Saving | 用户保存 | existing surface | persistence owner | no visual fork | write complete | overview |

---

## 3. 变体详细记录

### 3.1 Idle vector

- **进入条件**：无用户交互、无动画、无编辑。
- **退出条件**：wheel gesture、programmatic zoom、editor open。
- **主 surface**：vector canvas。
- **几何 owner**：`core/render` + `ui/render`。
- **渲染 owner**：vector renderer。
- **Token/revision**：frame token 单调递增；document revision 稳定。
- **禁止组合**：不得与 zoom animation、editor active 同时写几何。
- **实现文件**：`crates/pdf-viewer-core/src/render/`、`crates/pdf-viewer-ui/src/render/`。
- **验证入口**：`render/effective_page_plan/tests.rs`。
- **相关 ADR**：ADR-0003、ADR-0004。

### 3.2 Zoom animation

- **进入条件**：wheel gesture 或 programmatic zoom。
- **退出条件**：visual zoom 与 target zoom 差值小于 `ZOOM_SETTLED_THRESHOLD`（0.0008）。
- **主 surface**：vector canvas。
- **几何 owner**：Rust RAF loop（`zoom/raf_loop.rs`）。
- **渲染 owner**：incremental render（tile scheduler）。
- **Token/revision**：frame token 单调递增；document revision 稳定。
- **禁止组合**：不得与 editor active 同时写几何；raster 与 vector 呈现面在手势阶段互斥。
- **实现文件**：`crates/pdf-viewer-ui/src/zoom/raf_loop.rs`、`crates/pdf-viewer-core/src/render/zoom/animation.rs`。
- **验证入口**：`crates/pdf-viewer-ui/src/zoom/authority_tests.rs`。
- **相关 ADR**：ADR-0002、ADR-0006、ADR-0007。

### 3.3 Settle

- **进入条件**：zoom animation 落定。
- **退出条件**：final render 完成。
- **主 surface**：vector canvas。
- **几何 owner**：Rust RAF loop。
- **渲染 owner**：final render（full-page or tile）。
- **Token/revision**：frame token 单调递增；document revision 稳定。
- **禁止组合**：不得与 zoom animation 同时写几何。
- **实现文件**：`crates/pdf-viewer-ui/src/zoom/raf_loop.rs`、`crates/pdf-viewer-core/src/render/zoom/zoom_decide.rs`。
- **验证入口**：`zoom/authority_tests.rs`。
- **相关 ADR**：ADR-0002。

### 3.4 Preview/tile

- **进入条件**：zoom gesture 或大页面加载。
- **退出条件**：vector commit 完成。
- **主 surface**：raster preview 或 vector tile。
- **几何 owner**：preview path 或 tile manager。
- **渲染 owner**：raster/tile renderer。
- **Token/revision**：frame token 单调递增；document revision 稳定。
- **禁止组合**：不得与 zoom animation 同时写几何。
- **实现文件**：`crates/pdf-viewer-core/src/render/tile_manager.rs`、`crates/pdf-viewer-core/src/render/tile_v2.rs`。
- **验证入口**：`tile_manager.rs` 内联测试、`tile_v2.rs` 内联测试。
- **相关 ADR**：ADR-0003、ADR-0005。

### 3.5 Progressive render

- **进入条件**：大页面/多对象、超过一次性阈值。
- **退出条件**：全部条目渲染完成。
- **主 surface**：vector canvas。
- **几何 owner**：render pipeline。
- **渲染 owner**：progressive renderer。
- **Token/revision**：frame token 单调递增；document revision 稳定。
- **禁止组合**：不得与 zoom animation 同时写几何。
- **实现文件**：`crates/pdf-viewer-core/src/render/progressive.rs`。
- **验证入口**：`progressive.rs` 内联测试。
- **相关 ADR**：ADR-0004。

### 3.6 Editor active

- **进入条件**：用户点击段落或 programmatic open。
- **退出条件**：用户关闭编辑器或 commit。
- **主 surface**：vector canvas + editor canvas。
- **几何 owner**：editor/render path。
- **渲染 owner**：effective plan。
- **Token/revision**：frame token 单调递增；document revision 在 commit 时 bump。
- **禁止组合**：不得与 zoom animation 同时写几何。
- **实现文件**：`crates/pdf-viewer-ui/src/editor/`、`crates/pdf-viewer-core/src/edit/`。
- **验证入口**：`edit/document_plan/tests.rs`、`edit/draft_layout/tests.rs`。
- **相关 ADR**：principles §2。

### 3.7 Saving

- **进入条件**：用户触发保存（Ctrl+S 或 menu）。
- **退出条件**：写盘完成或失败。
- **主 surface**：existing surface（无视觉分叉）。
- **几何 owner**：persistence owner。
- **渲染 owner**：no visual fork。
- **Token/revision**：frame token 单调递增；document revision 在 save 后 bump。
- **禁止组合**：不得与 editor active 同时写 patch。
- **实现文件**：`src-tauri/src/application/pdf/edit_commands.rs`、`src-tauri/src/infrastructure/pdf/save_engine.rs`。
- **验证入口**：`src-tauri/tests/integration_tests.rs`。
- **相关 ADR**：overview §2.3。

---

## 4. 模块特征树

按以下维度构建项目专属特征树，帮助决定验证范围和实现选择：

```
变更类型
├── Core pure computation
│   ├── 是否影响 PDF 解析？
│   ├── 是否影响页面计划？
│   ├── 是否有 native unit test？
│   └── 是否需要 fixture/regression test？
├── UI/WASM
│   ├── 是否影响 wasm ABI？
│   ├── 是否影响 RAF/zoom/tile？
│   ├── 是否需要 wasm clippy/check？
│   └── 是否需要浏览器/Tauri E2E？
├── Tauri/native
│   ├── 是否触及 wgpu/vello？
│   ├── 是否增加冷编译成本？
│   ├── 是否影响 IPC 或 asset scheme？
│   └── 是否需要启动性能探针？
├── API/compat
│   ├── 是否只是 re-export/shim？
│   ├── 是否有 TS 调用方？
│   ├── 是否触及 WASM 边界？
│   └── 是否需要兼容期？
└── Performance
    ├── 编译时间
    ├── 首次打开时间
    ├── 首帧/页面渲染
    ├── RAF 调度
    └── 内存/GPU 初始化
```

---

## 5. 兼容/迁移变体

### 5.1 Legacy facade

- **状态**：部分 facade 仍被 TS 或 WASM 边界调用，属于兼容层。
- **规则**：不能直接删除；必须先做全局引用搜索，确认无生产调用后再删除。
- **验证**：删除前必须通过 `cargo check`、`cargo test`、`tsc`、`vitest`、E2E smoke。

### 5.2 Re-export shim

- **状态**：部分 re-export 用于历史兼容，实际已无调用方。
- **规则**：全局引用搜索无生产调用时可删除。
- **验证**：删除前必须通过 `cargo check`、`cargo test`。

### 5.3 Stub API

- **状态**：命名已冻结，实现待做。
- **规则**：不能改名；实现时走"添加新 API"流程。
- **验证**：实现后必须通过 `cargo check`、`cargo test`、`tsc`、`vitest`。

### 5.4 Session API 迁移

- **状态**：部分 TS 调用尚未迁移到 Session API。
- **规则**：迁移时必须保持兼容期；旧 API 标记 `@deprecated`，保留两个 release 周期。
- **验证**：迁移后必须通过 `cargo check`、`cargo test`、`tsc`、`vitest`、E2E smoke。

---

## 6. 外部参考"借配方"登记规则

每个外部参考方案必须记录以下信息，避免盲目照搬：

| 字段 | 说明 |
|---|---|
| **Reference** | 来源（论文、项目、文档）。 |
| **Problem solved** | 解决的问题。 |
| **Mapped to** | 映射到本项目的模块。 |
| **Not portable** | 不可照搬的假设（性能数值、并发模型、渲染模型、事件模型等）。 |
| **Evidence** | 本项目验证证据（测试、trace、benchmark）。 |
| **Adopted?** | 是否采纳。 |
| **Follow-up** | 后续行动。 |

### 6.1 已登记参考

#### SpecDB（arXiv 2605.31097）

- **Reference**：SpecDB: LLM-Generated Customized Databases via Feature-Oriented Decomposition。
- **Problem solved**：按目标工作负载生成定制数据库，避免单体系统背负全部子系统。
- **Mapped to**：
  - 跨模块不变量 → `CONTRACTS.md`。
  - 实现变体 → 本文 §2-3。
  - 精化闭环 → `ARCHITECTURE_REFINING_LOOP.md`。
  - 依赖分层 → `ARCHITECTURE_REFINING_LOOP.md` §5。
- **Not portable**：
  - 数据库性能数值（tpmC、延迟）不适用于 PDF viewer。
  - 从零生成整套系统的思路不适用于存量演进项目。
  - 跨系统重组变体（论文只验证了 PG 单一来源）不适用于本项目多源参考。
- **Evidence**：本文档、`CONTRACTS.md`、`ARCHITECTURE_REFINING_LOOP.md`。
- **Adopted?**：方法论采纳，具体数值和实现不采纳。
- **Follow-up**：无。

#### Nutrient（PDF SDK）

- **Reference**：Nutrient 文档与 API 设计。
- **Problem solved**：商业 PDF SDK 的 API 设计与事件模型。
- **Mapped to**：
  - Session API 命名 → `docs/editor-api-architecture-proposal.md`。
  - Facade 分层 → `docs/architecture-overview.md` §3。
- **Not portable**：
  - Nutrient 完整事件系统不适用于本项目手写 controllers。
  - Nutrient annotation/history/document 生命周期不完全匹配本项目 persistence 模型。
  - React/Web PDF viewer 渲染调度不适用于本项目 Rust canvas painter。
- **Evidence**：`docs/editor-api-architecture-proposal.md`、`docs/architecture-overview.md`。
- **Adopted?**：命名和分层采纳，事件系统和生命周期不采纳。
- **Follow-up**：无。

#### PDF.js / pdfium / mupdf

- **Reference**：开源 PDF 渲染/解析库。
- **Problem solved**：PDF 解析、渲染、文本提取。
- **Mapped to**：
  - PDF 解析 → `src-tauri/src/infrastructure/pdf/`（基于 `lopdf`）。
  - 渲染 → `crates/pdf-viewer-core/src/render/`（基于 `vello`/`wgpu`）。
- **Not portable**：
  - PDF.js 浏览器 worker 模型不适用于本项目 Tauri native shell。
  - pdfium/mupdf C API 不适用于本项目 Rust-first 架构。
  - 其他项目的 GPU renderer 生命周期不适用于本项目 vello/wgpu。
- **Evidence**：`src-tauri/src/infrastructure/pdf/`、`crates/pdf-viewer-core/src/render/`。
- **Adopted?**：不直接采纳，仅作为参考。
- **Follow-up**：无。

---

## 7. 参考

- `docs/CONTRACTS.md`
- `docs/api-contract.md`
- `docs/architecture-principles.md`
- `docs/CONTEXT.md`
- `docs/architecture-overview.md`
- `docs/adr/`
