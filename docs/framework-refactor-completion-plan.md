# 框架级重构完成计划

> 基于 `architecture-refactor-plan.md`（原 5 阶段计划）、`docs/architecture-review.md`（后续审计）、
> `docs/naming-and-architecture-refactor-plan.md`（命名审计）以及对当前代码的全面核实。
> 目标：收口所有未完成项，让项目达到"可长期维护、无技术债死角"的状态。
> 编写日期：2026-06-15

---

## 0. 现状核实（与原计划对照）

原计划 `architecture-refactor-plan.md` 的进度标注基本属实，但有一个统计盲区：
**只统计了 pdf-viewer-ui crate，遗漏了 core crate 和 src-tauri 的大文件。**

### 已真实落地（核实通过）

| 项 | 状态 | 验证 |
|---|---|---|
| 命名规范化（store/controller/api 后缀） | 完成 | `*_store.rs` / `*_controller.rs` / `*_api.rs` 命名已统一 |
| viewer/facade.rs、zoom/facade.rs 死码删除 | 完成 | 文件不存在 |
| wasm_api/ 目录 | 完成 | 目录不存在，已拆分到各域 `*_api.rs` |
| core/models.rs 拆分 | 完成 | 已拆为 `models/` 目录（vector/font/geometry/glyph/layout 等） |
| EditorSession 同名冲突 | 完成 | core 改用 `EditorSessionTextPlan` + `ParagraphEditContext` |
| VectorPageModel 重复定义 | 完成 | 仅 `core/models/vector.rs` 一处 |
| AppState 拆为 DocumentStore/CacheStore/HistoryStore/RendererState | 完成 | `src-tauri/src/app_state.rs` |
| 9 个 Session API（159 方法） | 完成 | editor/document/find/review/comment/render/zoom/annotation/history |
| TS bridge Session 化 | 完成 | find/document/viewer/editor/comment/review/annotation/history |
| host_ 别名清理（242 个） | 完成 | 残留 121 处全在 4 个 legacy facade 文件 |
| PdfError 枚举 | 完成 | `src-tauri/src/error.rs` |

### 仍未完成（原计划标注但代码核实未通过）

| 项 | 原计划标注 | 实际状态 | 差距 |
|---|---|---|---|
| thread_local 收口到 AppState | "12 个已封装" | 16 处 thread_local，散布 15 个文件 | 封装不等于收口；只是加了 accessor 函数，仍是分散单例 |
| >500 行 God File | "3 个" | 18 个（core 10 + ui 2 + src-tauri 8，见 1.2） | 原统计只看 UI crate；core 和 src-tauri 完全没碰 |
| host_ 前缀清理 | "剩余 38 个在 facade" | 121 处引用，7 个 host_* 文件 | 清理未完成 |
| facade 命名清理 | "TS 迁移后删除" | 71 处引用，5 个 *facade* 文件 | TS 端仍在用旧 facade 模式 |
| 裸 WASM snake_case 导出 | 未覆盖 | 4 个 free_api.rs 文件，57 个违规裸导出 | 命名审计指出但未处理 |
| 11 个 #[deprecated] 方法 | 未覆盖 | 仍在代码中 | 需确认 TS 是否还调用，否则删除 |

### 原计划完全未覆盖的盲区

1. core crate 的 10 个 God File（最大 1551 行 `effective_page_plan.rs`）
2. src-tauri 的 8 个 God File（最大 1337 行 `pdf_read.rs`）
3. TS 层 11 个 >400 行文件（最大 819 行 `vector_host.ts`）
4. 根目录噪音（15+ 个 log 文件、散落的 plan 文档）
5. 测试覆盖（core 仅 15 个 #[test]，src-tauri 几乎无单元测试）
6. WASM 全局状态收口（架构审计 Phase 3 提出，未动工）
7. 端到端数据流文档缺失（架构审计 Phase 3 提出，未动工）

---

## 1. 消除 God File（Phase A — 高价值，中风险）

> 原则：拆分不是机械按行数切，而是按职责边界。每个拆出的模块有独立语义。
> 拆完后原文件作为 facade re-export，保持外部 API 不变。

### 1.1 执行顺序与拆分方案

按"风险×收益"排序，先做收益最高、风险最低的：

| 批次 | 文件 | 行数 | 拆分策略 | 风险 |
|---|---|---|---|---|
| A1 | core `effective_page_plan.rs` | 1551 | 抽出 suppression 逻辑到 `suppression_calc.rs`；抽出 overlay 构建到 `overlay_builder.rs`；主文件留编排 | 中（渲染核心） |
| A2 | core `draft_layout.rs` | 1289 | 按 draft 生命周期拆：`draft_init.rs` / `draft_reflow.rs` / `draft_geometry.rs` | 中 |
| A3 | src-tauri `pdf_read.rs` | 1337 | 按 PDF 对象类型拆：`text_operator.rs` / `path_operator.rs` / `image_operator.rs` / `gs_stack.rs` | 中 |
| A4 | src-tauri `vello_renderer.rs` | 1191 | 按 vello scene 构建拆：`glyph_scene.rs` / `path_scene.rs` / `image_scene.rs` | 中 |
| A5 | src-tauri `pdf_write.rs` | 1188 | 按写入对象拆：`write_text.rs` / `write_path.rs` / `write_xref.rs` | 中 |
| A6 | ui `canvas.rs` | 1045 | 抽 draw_* 辅助函数到 `canvas_draw.rs`；主文件留编排 | 低 |
| A7 | core `document_plan.rs` | 1069 | 按 plan 构建阶段拆 | 中 |
| A8 | ui `editor_api.rs` | 1029 | 已是 Session 结构，按方法组拆：`editor_api_text.rs` / `editor_api_format.rs` | 低 |
| A9 | 其余 7 个 500-700 行文件 | — | 视情况内联或小拆 | 低 |

### 1.2 验收标准

每个 God File 拆分后：

- [ ] 单文件不超过 500 行（生产代码）
- [ ] 外部 `use` 路径不变（通过原文件 re-export）
- [ ] `cargo check` 三端通过（core / wasm32 / standalone）
- [ ] `cargo test -p pdf-viewer-core` 全绿
- [ ] 手动验证：打开 PDF → 编辑 → 保存，无回归

### 1.3 不拆的文件（明确豁免）

| 文件 | 行数 | 豁免理由 |
|---|---|---|
| core `page_region_context.rs` | 736 | region 语义内聚，强拆会割裂上下文 |
| ui `page_turn.rs` | 697 | 翻页状态机内聚 |
| src-tauri `pdf_write_font_resolver.rs` | 684 | 字体解析内聚合理 |
| src-tauri `pdf_font.rs` | 668 | 字体内聚 |

---

## 2. WASM 全局状态收口（Phase B — 架构根本改善，中高风险）

> 这是原计划 Phase 1 声称完成但实际未做的核心项。
> 当前 16 处 thread_local 散布在 15 个文件，只是各自加了 accessor 函数，仍是分散单例。

### 2.1 当前状态清单（核实）

```
events.rs::EventBus              → thread_local (设计如此，单例合理)
chain_trace.rs::TRACE_BUFFER     → thread_local (诊断用，单例合理)
editor_store.rs::HOST_EDITOR     → 全局状态
editor/session/session.rs        → 全局状态
editor/host_runtime.rs           → 桥接状态
find/find_store.rs               → 全局状态
find/host_find_store.rs          → 桥接状态
page/page_store.rs               → 全局状态
present/present_store.rs         → 全局状态
presentation/page_turn.rs        → 全局状态
render/render_store.rs           → 全局状态
render/host_runtime.rs           → 桥接状态
review/review_store.rs           → 全局状态
viewer/viewer_store.rs           → 全局状态
zoom/zoom_store.rs               → 全局状态
editor/format/text_geometry.rs   → 局部计数器（不需收口）
```

需收口的是 11 个域状态（排除 EventBus、trace、局部计数器）。

### 2.2 目标设计

```rust
// crates/pdf-viewer-ui/src/app_context.rs
pub struct AppContext {
    pub editor: EditorDomain,        // 包含 editor_store + session
    pub viewer: ViewerDomain,        // 包含 viewer_store
    pub render: RenderDomain,        // 包含 render_store + host_runtime
    pub find: FindDomain,            // 包含 find_store + host_find_store
    pub page: PageDomain,            // 包含 page_store
    pub present: PresentDomain,      // 包含 present_store + page_turn
    pub zoom: ZoomDomain,            // 包含 zoom_store
    pub review: ReviewDomain,        // 包含 review_store
}

thread_local! {
    static APP_CONTEXT: RefCell<AppContext> = RefCell::new(AppContext::new());
}

// 统一访问入口
pub fn with_context<R>(f: impl FnOnce(&AppContext) -> R) -> R { ... }
pub fn with_context_mut<R>(f: impl FnOnce(&mut AppContext) -> R) -> R { ... }
```

### 2.3 执行策略（渐进式，每步可编译）

1. **B1**：创建 `AppContext` 结构体，先放入 1 个域（editor），旧 `thread_local` 保留作为兼容层
2. **B2**：逐域迁移，每迁一个域：删除旧 thread_local → `cargo check` → 验证
3. **B3**：全部迁移后，删除兼容层，旧 accessor 函数改为通过 `with_context` 访问
4. **B4**：更新所有调用点（全局搜索替换）

### 2.4 验收标准

- [ ] 域状态 thread_local 从 11 个降到 1 个（AppContext）
- [ ] EventBus 和 trace 保留独立（设计如此）
- [ ] 所有 Session API 方法通过 `with_context` 访问状态
- [ ] WASM 在浏览器中加载、打开 PDF、编辑、保存全流程无回归

### 2.5 风险与回滚

这是高风险重构——涉及每个域的每次状态访问。建议：

- 每个域迁移是一个独立 commit，可单独 revert
- 迁移顺序：低频域（review → page → present → zoom → find → render → viewer → editor），最后碰编辑器
- 每步迁移后手动验证该域的核心用例

---

## 3. 命名与死码清理（Phase C — 低风险，认知收益高）

### 3.1 裸 WASM 导出收口（P0 命名问题）

4 个 `free_api.rs` 文件的裸 `#[wasm_bindgen]` 函数会暴露 Rust snake_case 到 JS：

- `document/free_api.rs`
- `render/free_api.rs`
- `viewer/free_api.rs`
- `zoom/free_api.rs`

处理：
1. 仍在被 TS 调用的 → 加显式 `#[wasm_bindgen(js_name = camelCaseName)]`
2. 已被 Session API 取代的 → 标 `#[deprecated]` 并确认 TS 无调用后删除
3. 未使用的 → 直接删除

### 3.2 host_ 前缀收尾

剩余 121 处 `host_` 引用集中在：
- `editor/host_runtime.rs`（桥接状态）
- `find/host_find_store.rs`
- `render/host_runtime.rs`
- `host/` 目录（command/layout/scroll）

处理：
1. `host/` 目录 → 重命名为 `platform/`（3 个文件）
2. `host_runtime.rs` → `platform_bridge.rs`
3. `host_find_store.rs` → 合并到 `find_store.rs`
4. 全局替换调用路径

### 3.3 facade 命名收尾

剩余 71 处 `facade` 引用，5 个 `*facade*` 文件：
- TS 端 4 个文件仍用 `xxxFacade*` 命名（find/review）
- Rust 端 `render/wasm_facade.rs`

处理：
1. TS 端 `find_facade.ts` / `review_wasm_facade.ts` → 迁移到 Session API 调用后删除
2. `render/wasm_facade.rs` → `render_api.rs`

### 3.4 deprecated 方法清理

11 个 `#[deprecated(since = "0.2.0")]` 方法：

1. 全局搜索 TS 是否仍调用
2. 有调用的 → TS 迁移到新 API，再删 deprecated
3. 无调用的 → 直接删除

### 3.5 UI crate 根模块清理

lib.rs 里有 7 个根级模块疑似遗留：

| 文件 | 行数 | 处理 |
|---|---|---|
| `runtime.rs` | 2 | 删除 |
| `style_mapper.rs` | 2 | 删除 |
| `dom_projection.rs` | 1 | 删除 |
| `models.rs` | 2 | 删除 |
| `bridge.rs` | 19 | 合并到 `app_controller.rs` 或删除 |
| `ui_state_store.rs` | 305 | 迁移到 `app_context.rs`（Phase B 后） |
| `projection_workflow.rs` | 112 | 评估是否仍在用，否则删除 |

### 3.6 根目录噪音清理

```
删除或 .gitignore：
  *.log (15+ 个，含 505KB 的 dev-current.log、tauri-dev.log)
  build-after-clean.log / build.log / e2e-*.log / frontend-build.log
  tauri-check*.log / vite-dev.*.log / rebuild.log / console-tail.log

移动到 docs/ 或 scratch/：
  architecture-refactor-plan.md（已有 docs/ 版本）
  rebuild.cmd / rebuild.ps1（移到 scripts/）
```

---

## 4. src-tauri 接口层拆分（Phase D — 中风险）

> 架构审计 Phase 1 提出：`interfaces/pdf.rs`（40 个 command）需拆分。
> 但核实发现已经按域拆成了 annotation/comment/document/page/render/replace/search/system 8 个文件。
> 这部分已完成，仅余小幅整理。

### 4.1 剩余项

- [ ] 确认 29 个 command 的参数/返回类型在 `interfaces/pdf/ipc_converters.rs` 中统一序列化
- [ ] 补齐 annotation 命令（add_annotation / update_annotation / read_annotation / flatten / read_all），点亮 AnnotationManager 的 5 个 stub
- [ ] `infrastructure/pdf/document_service.rs`（579 行）评估是否需小拆

---

## 5. TS 层整理（Phase E — 低风险）

### 5.1 God File 拆分

| 文件 | 行数 | 策略 |
|---|---|---|
| `render/vector_host.ts` | 819 | 抽 `vector_host_init.ts` + `vector_host_render.ts` |
| `editor/index.ts` | 669 | 按 editor 生命周期拆（init/active/inactive） |
| `ai/resume_ai_controller.ts` | 616 | 抽 `resume_ai_panel_controller.ts` + `resume_ai_apply_controller.ts` |
| `viewer/pdf_runtime.ts` | 596 | **不拆**（审计已确认是 composition root，职责密度合理） |
| `render/render_flow.ts` | 555 | 抽 `render_flow_scheduling.ts` |
| `review/pdf_review_controller.ts` | 505 | 抽 `review_diff_view.ts` |

### 5.2 TS→WASM 调用收口

确保 TS 只通过 Session API 调用 WASM，不再直接调用 free_api 或 deprecated facade。
迁移完成后删除旧 facade 文件。

---

## 6. 测试与文档（Phase F — 持续，低风险）

### 6.1 测试补强

当前：core crate 15 个 `#[test]`，src-tauri 几乎无单元测试。
目标：覆盖关键管线，不求全覆盖。

优先补测试的模块（按风险排序）：
1. `effective_page_plan` suppression 逻辑（拆分后的小模块各补测试）
2. `coordinate_transform`（坐标转换出错=视觉错位）
3. `draft_layout` 重排逻辑
4. `pdf_write` 写回正确性（round-trip 测试：读入→写出→再读入，比较）
5. 编辑命令（apply/commit/undo/redo）的状态机一致性

### 6.2 端到端数据流文档

架构审计提出的关键缺失：
- [ ] 编辑→保存的完整数据流文档（TS → WASM → Tauri → 磁盘）
- [ ] 打开→渲染的完整数据流文档（磁盘 → Tauri → WASM → canvas）
- [ ] 每个域 Session API 的调用时序图

### 6.3 重构文档归档

完成后更新：
- [ ] `architecture-refactor-plan.md` 标记最终完成状态
- [ ] `architecture-review.md` 更新检查清单
- [ ] 删除过时的中间计划文档（`naming-refactor-review-plan.md` 等）

---

## 7. 执行顺序与依赖关系

```
Phase A (God File 拆分)     ← 无依赖，可立即开始
  └─ A1-A9 可并行（不同 crate 互不阻塞）

Phase B (全局状态收口)      ← 依赖 A6/A8（canvas/editor_api 拆分后再动状态）
  └─ B1→B4 严格串行

Phase C (命名/死码清理)     ← 无依赖，可与 A 并行
  └─ C1-C6 可并行

Phase D (src-tauri 接口)    ← 独立，低优先级

Phase E (TS 层整理)         ← C 完成后做（依赖 facade 清理结果）

Phase F (测试/文档)         ← 持续，A/B 每完成一批就补对应测试
```

### 建议时间线（单人）

| 阶段 | 工时 | 日历周 |
|---|---|---|
| A1-A5（core + src-tauri God File） | 5 天 | 1.5 周 |
| A6-A9（ui + 小文件） | 2 天 | 0.5 周 |
| B1-B4（全局状态收口） | 4 天 | 1.5 周（含验证） |
| C1-C6（命名清理） | 2 天 | 0.5 周 |
| D（src-tauri 接口） | 1 天 | 0.5 周 |
| E（TS 整理） | 2 天 | 0.5 周 |
| F（测试/文档） | 持续 | 1-2 周（并行） |
| **合计** | **~16 工作日** | **5-6 周** |

---

## 8. 全局验收标准

重构全部完成后：

### 代码指标

- [ ] 没有单文件超过 500 行（豁免清单除外）
- [ ] thread_local 域状态 ≤ 2（AppContext + EventBus）
- [ ] 无裸 WASM snake_case 导出
- [ ] 无 host_ 前缀（platform_ 替代）
- [ ] 无 facade 命名（Session API 替代）
- [ ] 无 #[deprecated] 方法
- [ ] UI crate 根模块 ≤ 3 个（app_context + app_controller + events）

### 编译与测试

- [ ] `cargo check -p pdf-viewer-core` 通过
- [ ] `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown` 通过
- [ ] `cargo check -p pdf-viewer-standalone` 通过
- [ ] `cargo test -p pdf-viewer-core` 全绿（目标 ≥ 40 tests）
- [ ] `cargo clippy` 无新增 error
- [ ] `npx tsc --noEmit` 通过
- [ ] `npm run build` 通过
- [ ] `npm run e2e` 全绿

### 架构原则

- [ ] 同一能力只有一个 Rust 入口（单一所有者原则）
- [ ] TS 只做宿主适配，不持有领域规则
- [ ] PDF-Glyph 链是唯一视觉链
- [ ] 坐标转换只通过统一 mapper
- [ ] 手动编辑、AI 编辑、撤销重做、保存使用同一条链

---

## 9. 明确不做的事（防止过度设计）

继承自 `architecture-principles.md` 和 `architecture-review.md`：

- ❌ 不引入 ECS / Actor / 重型事件总线
- ❌ 不重写 TS 层为框架化（React/Solid），当前 vanilla TS + DOM 合理
- ❌ 不引入 trait 抽象层（PDF 查看器不需要可插拔后端）
- ❌ 不把 thread_local 改为 Arc<Mutex>（WASM 单线程）
- ❌ 不做微服务化拆分
- ❌ 不大规模移动 src-tauri 基础设施到 crates（暂缓）
- ❌ 不在重构期间扩展新功能（Word/论文工作台等）

---

## 10. A2 拆分、build_index_map 算法修复与列表光标修正落实记录（2026-06-24）

已成功推进重构计划中 **A2 批次大文件拆分**、**第一步及第四步/第五步算法与光标偏移动态修正**：

1. **A2 `draft_layout.rs` 大文件拆分与 Facade 重映射完成**：
   - 原 ~800 行的排版计划核心模块已成功解耦并拆分为 `draft_init.rs` (初始空模版构造)、`draft_geometry.rs` (基线对齐与物理光标坐标转换) 和 `draft_reflow.rs` (重排/重算 pipeline 驱动)。
   - 原 `draft_layout.rs` 仅作为 router 重导出所有公共 API，保持外部 100% 兼容。
   - 彻底将 `EditorDocumentPlan` 替换为 `EditContext`，移除了整个业务模块中所有的弃用（Deprecated）警告。
   - 清理了 `replacement_region.rs`、`document_plan/marker.rs`、`paragraph_scene.rs` 等文件中未使用的变量和冗余导入。

2. **双指针映射算法 `build_index_map` 重构（Step 5 算法修复）完成**：
   - 重构 `draft_text_diff.rs` 中的 `build_index_map` 算法，在双指针匹配时能够精确区分合成空格与物理真实空格。
   - 引入非空字符不匹配时的局部回溯重新对齐机制，彻底修复了用户在段落中途删除/插入字符时因匹配位置向后偏移导致的 caret 跳跃与“字距粘连/漂移”问题。

3. **列表项目符号光标偏移修正（Step 4 光标修正）完成**：
   - 修复了在编辑含有列表项目符号的段落时，光标水平位置偏左错绘在项目符号上的缺陷。
   - 修改了 `crates/pdf-viewer-ui/src/editor/overlay/visual.rs` 中的 `body_left_offset`，动态检测当前编辑目标是否存在 marker，并在存在时累加 `marker_advance` 宽度，使光标基准精准对齐至正文起点。

4. **测试加固与六大检查点通过**：
   - 恢复了 `preserves_origins` 回归测试用例（移除了 `#[ignore]` 标志），并补充了 middle deletion, middle insertion, mixed spaces 三组针对增删与混合空格映射的单元测试，78 个测试全部通过。
   - 重新验证了 tsc、cargo check WASM 目标、Tauri 后端及 standalone 整体二进制构建，全部 100% 成功。

有关本次重构的详细代码级变更和指标验证情况，请参见：[Rust Core & WASM UI 重构实现报告 (2026-06-24)](file:///e:/chain/pdf-viewer-standalone/docs/refactor-implementation-report-2026-06-24.md)。
