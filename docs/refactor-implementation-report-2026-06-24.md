# Rust Core & WASM UI 重构实现报告 (2026-06-24)

本报告详细记录了排版与编辑引擎重构计划中已落实的各项重构和算法修复，包括：列表项目符号光标（Caret）偏移修正、`draft_layout.rs` 核心代码模块化拆分、双指针 `build_index_map` 算法修复，以及编译器警告清理与测试加固。

---

## 1. 列表项目符号编辑时光标偏移修正 (Caret Offset Correction)

### 1.1 现状与挑战
* 在编辑带有列表符号（如 `•`、`●` 等）的段落时，列表符号（marker）与正文内容（body）在物理布局上是分离的。
* 编辑器的光标层在计算水平偏移基准 `body_left_offset` 时，使用 `body_session.anchor_bbox.left - shell_bbox.left`。因为该 bbox 包含了 marker，所以光标基准定位到了 marker 的起始位置（即整行的最左端）。
* 然而，活动编辑器的 `caret.left` 是由 `build_draft_render_plan` 产出的，该计划仅针对正文文本（不包含 marker）。
* 这导致在渲染光标（draw_caret）时，光标的水平位置在视觉上产生了向左偏离整个项目符号宽度（`marker_advance`）的偏差，使得光标错绘在 marker 之上，而非正文的起点。

### 1.2 改造方案
* 在 `crates/pdf-viewer-ui/src/editor/overlay/visual.rs` 中的 `body_left_offset` 计算中，动态检测当前编辑目标是否存在 `marker`。
* 若存在 `marker`，则利用 `active_target.scene.marker()` 获取其 `advance` 物理宽度并累加到 `body_left_offset`。
* 这样光标的绘制起始基准就向右修正了整个项目符号的宽度，使光标在编辑列表项时能够精准对齐到正文的起点。

---

## 2. 拆分 `draft_layout.rs` 核心排版引擎 (Phase A / A2)

### 2.1 现状与挑战
* `crates/pdf-viewer-core/src/edit/draft_layout.rs` 原先长达近 800 行，混合了排版初始化模板构建、行基线对齐计算、caret stops 物理坐标转换以及段落重排（Reflow）管线等多种不同层级的职责，难以维护。

### 2.2 模块化拆分方案
为符合单一职责原则，我们将该大文件拆分为 3 个高度内聚的子模块，均置于 `crates/pdf-viewer-core/src/edit/` 下：

1. **初始与模板化子模块 ([draft_init.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_init.rs))**
   * **职责**：负责段落初始模版 run 的解析及空段落编辑渲染计划的构建。
   * **核心函数**：`build_empty_render_plan`
2. **核心重排计算子模块 ([draft_reflow.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_reflow.rs))**
   * **职责**：承载 draft 文本的差分重排管线构建、草稿渲染计划以及持久化渲染计划的构造。
   * **核心函数**：`build_draft_paragraph`、`rebuild_layout_pipeline`、`build_draft_render_plan`、`build_persisted_overlay_render_plan`
3. **几何与光标转换子模块 ([draft_geometry.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_geometry.rs))**
   * **职责**：负责布局的多行基线对齐及物理光标坐标（Caret Stops）与文本索引的转换。
   * **核心函数**：`align_layout_baseline`、`build_editor_draft_caret_plan_from_layout`
4. **统一导出路由器 ([draft_layout.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_layout.rs))**
   * **职责**：作为子模块声明的入口，统一 `pub use` 重导出所有公共 API，保持对外接口的 100% 兼容。同时承载排版引擎的集成测试。

---

## 3. 双指针 `build_index_map` 算法修复 (Step 5 Algorithm Fix)

### 3.1 现状与挑战
* 合成空格是 `normalize_compact_pdf_text` 注入的视觉空格，并不存在于 PDF 的物理 raw runs 中。
* 原 `build_index_map` 重映射算法使用简单的 `if sc == ' '` 进行字符对齐。在遇到以下情况时会发生严重脱节与越界，导致光标跳动与字间距漂移（豆腐块/粘连）：
  1. 真实空格与合成空格混合。
  2. 用户在编辑时删除或插入了非空字符，导致双指针对不齐，而算法没有对齐恢复机制，导致错位一直向后累积。

### 3.2 改造方案
* 重新实现了 `build_index_map` 算法，在 [draft_text_diff.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_text_diff.rs) 中引入双指针对齐与贪婪回溯重新对齐机制：
  1. **精确区分合成空格**：仅当源文本与 Runs 文本中对应字符同时为 `' '` 时，才作为物理空格匹配（双指针均前进）；否则只针对单侧的空格进行单向指针推移。
  2. **非空字符不匹配的局部回溯**：当两个非空字符由于用户的删除或插入操作发生失配时，在局部窗口内进行前瞻性扫描，重新寻找最临近的匹配项进行对齐恢复，防止偏移错位向后续字符传播。
* **单元测试**：
  * 恢复了原先被 `#[ignore]` 的 `preserves_origins` 回归测试用例。
  * 补充了 `test_build_index_map_middle_deletion`（中途删除）、`test_build_index_map_middle_insertion`（中途插入）以及 `test_build_index_map_mixed_spaces`（真实与合成空格混合）三组严密的单元测试，全面保证映射准确性。

---

## 4. 编译器警告清理与代码优化 (Code Polish & Warning Cleanup)

为了消除多余的技术债并提高编译速度，我们对 Rust 核心层代码进行了深度清理：
1. **弃用警告清理**：废弃了已被弃用的 `EditorDocumentPlan` 别名，全面替换为统一的 `EditContext`。
2. **重复代码清理**：在 `source_geometry.rs` 中，移除了重定义但未被调用的 `bbox_width` 和 `bbox_height` 辅助函数。
3. **死代码与无用导入清理**：
   * 在 `replacement_region.rs` 中，消除了解构 `marker` 却未使用该变量的警告。
   * 清理了 `marker.rs`、`paragraph_scene.rs`、`paint_plan.rs` 等模块中所有不必要的 `use` 导入语句。
   * 删除了 `mod.rs` 根模块中冗余的子模块声明，保证模块树唯一性。

---

## 5. 修改文件清单

| 模块 / 路径 | 变更类型 | 变更描述 |
| :--- | :--- | :--- |
| [draft_init.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_init.rs) | **[NEW]** | 初始化与模版渲染计划模块，使用 `EditContext` |
| [draft_reflow.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_reflow.rs) | **[NEW]** | 排版重排管线与草稿/持久化渲染计划模块，使用 `EditContext` |
| [draft_geometry.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_geometry.rs) | **[NEW]** | 多行基线对齐及光标物理转换模块 |
| [draft_layout.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_layout.rs) | **[MODIFY]** | 作为入口重导出子模块 API，包含主排版测试与 unignore 校验 |
| [draft_text_diff.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_text_diff.rs) | **[MODIFY]** | 重构 `build_index_map` 贪婪双指针对齐算法，添加三组单元测试 |
| [draft_style.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/draft_style.rs) | **[MODIFY]** | 更新别名为 `EditContext` |
| [document_plan/marker.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/document_plan/marker.rs) | **[MODIFY]** | 清理冗余的 `use` 导入警告 |
| [paragraph_scene.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/paragraph_scene.rs) | **[MODIFY]** | 清理冗余的 `use` 导入警告 |
| [paint_plan.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/render/paint_plan.rs) | **[MODIFY]** | 清理未使用的 `BoundingBox` 导入 |
| [replacement_region.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/edit/replacement_region.rs) | **[MODIFY]** | 消除未使用的 `marker` 绑定编译警告 |
| [source_geometry.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-core/src/geometry/source_geometry.rs) | **[MODIFY]** | 移除死代码 `bbox_width` / `bbox_height` |
| [visual.rs](file:///e:/chain/pdf-viewer-standalone/crates/pdf-viewer-ui/src/editor/overlay/visual.rs) | **[MODIFY]** | 修复 `body_left_offset`，在存在列表 marker 时累加 `marker_advance` |

---

## 6. 系统验证与构建指标结果

我们依次对整个工作区的六大构建与测试检查点进行了编译与验证，全部 100% 成功通过：

1. **Rust Core 单元测试**：
   `cargo test -p pdf-viewer-core`
   * **结果**：`Ok`。全库共 78 个单元测试全部成功通过，包括原先 ignore 的 `preserves_origins` 及新增的 `build_index_map` 专项对齐测试。
2. **Rust Core 编译检查**：
   `cargo check -p pdf-viewer-core`
   * **结果**：编译通过，0 警告，0 错误。
3. **WASM UI Target 编译检查**：
   `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown`
   * **结果**：编译通过。
4. **Tauri Native Standalone Backend 编译检查**：
   `cargo check -p pdf-viewer-standalone`
   * **结果**：编译通过。
5. **TS 前端类型安全检查**：
   `npx tsc --noEmit`
   * **结果**：全量 TypeScript 类型检查通过。
6. **Vite 生产环境构建打包**：
   `npx vite build`
   * **结果**：成功构建生产环境 bundle。
7. **全栈本地二进制编译**：
   `cargo build`
   * **结果**：顺利完成全栈构建。
