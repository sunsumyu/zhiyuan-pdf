# 系统契约索引

> 本文记录当前有效、可验证的系统契约，覆盖跨模块不变量、状态所有权、渲染链、持久化协议与命名规则。
> 与 `api-contract.md`、`architecture-principles.md`、`CONTEXT.md`、`docs/adr/` 分工如下：
>
> - `api-contract.md`：对外 API 清单（签名、稳定性、版本化）。
> - `architecture-principles.md`：最高原则与红线。
> - `CONTEXT.md`：领域词汇表。
> - `docs/adr/`：单项决策历史。
> - **本文**：当前有效的跨模块不变量与修改联动规则。
>
> **优先级**：当前实现 + Accepted ADR 高于历史 overview/context。ADR-0007 已废弃旧 anchor 公式，`CONTEXT.md` 中相关描述视为过时。

---

## 1. 契约模板

每条契约统一使用以下格式，便于后续补充：

```
### C-XXX 契约名称

- **Scope**：契约覆盖的模块/文件。
- **Owner**：唯一负责写入或维护的模块。
- **Allowed writers**：允许修改该状态的入口。
- **Forbidden writers**：禁止写入的路径。
- **Pre**：进入条件。
- **Post**：退出条件。
- **Invariants**：不变量。
- **Failure behavior**：违反时的失败语义。
- **Verification**：测试或验证入口。
- **Related ADR**：相关决策记录。
- **Change coupling**：修改本契约时必须复核的模块。
```

---

## 2. 契约索引

| ID | 契约 | Owner | 验证位置 | 相关 ADR |
|---|---|---|---|---|
| C-001 | Zoom authority 唯一写入源 | `zoom/state.rs` | zoom tests | ADR-0001 |
| C-002 | Rust RAF/SetBox 几何单一写手 | `zoom/raf_loop.rs` + `zoom/raf_dom_cache.rs` | zoom tests + `zoom_wheel_sudden_jump` E2E | ADR-0002 / ADR-0006 |
| C-003 | 单一渲染链 | `core/render` + `ui/render` | render tests | principles |
| C-004 | Frame token 生命周期 | `render/commit.rs` + `present/present_store.rs` | frame tests | overview |
| C-005 | Tile scheduler 与 manager 职责分离 | `tile_scheduler.rs` / `tile_manager.rs` | tile tests | ADR-0005 |
| C-006 | Editor draft 与 cluster/runs 同步 | `editor/session/session.rs` + `editor/editor_controller.rs` | editor tests | overview |
| C-007 | Replacement region 覆盖关系 | `replacement_region.rs` + `source_suppression.rs` + `path_suppression.rs` | effective_page_plan tests | principles |
| C-008 | Patch schema 闭环 | `persistence/models.rs` → Tauri `region_materializer.rs` | persistence + save tests | overview |
| C-009 | Commit 必须 bump revision | `viewer/viewer_controller.rs` | persistence/render tests | overview |
| C-010 | Save 事务一致性 | Tauri `edit_commands.rs` + `save_engine.rs` | integration tests | overview |
| C-011 | WASM/Tauri/TS 命名规则 | facade 层 | `api-contract.md` §2 | principles |

---

## 3. 详细契约

### C-001 Zoom authority 唯一写入源

- **Scope**：`crates/pdf-viewer-core/src/render/zoom/state.rs`、`crates/pdf-viewer-ui/src/zoom/zoom_store.rs`。
- **Owner**：`ZOOM_STATE`（UI crate thread_local 容器）。
- **Allowed writers**：`zoom/state.rs` 中的 `set_target_zoom`、`set_visual_layout`、`set_target_zoom_instant`、`mark_rendered_zoom`。
- **Forbidden writers**：`VIEWER_SESSION.current_zoom` 只能作为派生快照，不允许镜像写入；TS 不得新增直接写容器或 canvas 盒子的路径。
- **Pre**：`set_target_zoom` 接受有限且大于 0 的 zoom；非法值归一化到 `1.0`。
- **Post**：`set_target_zoom_instant` 同时更新 `target_zoom` 和 `visual_zoom`，防止 visual/base 比值爆炸。
- **Invariants**：
  - `ZOOM_STATE.target_zoom` 是唯一 zoom authority。
  - `VIEWER_SESSION.current_zoom` 只是派生快照。
  - `visual_zoom` 可在动画期间领先 `last_rendered_zoom`，但提交后必须通过 `mark_rendered_zoom` 收敛。
- **Failure behavior**：非法 zoom 归一化到 `1.0`；禁止路径写入时记录 trace 并忽略。
- **Verification**：`crates/pdf-viewer-ui/src/zoom/authority_tests.rs`。
- **Related ADR**：ADR-0001（缩放唯一权威）、ADR-0007（anchor 语义收敛）。
- **Change coupling**：修改 zoom state/animation 时必须同步检查 `raf_loop`、`raf_dispatch`、`raf_committed`、`raf_settle`、`zoom_preview`、render frame commit 和 viewer session 投影。

### C-002 Rust RAF/SetBox 几何单一写手

- **Scope**：`crates/pdf-viewer-ui/src/zoom/raf_loop.rs`、`crates/pdf-viewer-ui/src/zoom/raf_dom_cache.rs`、`crates/pdf-viewer-core/src/render/zoom/animation.rs`。
- **Owner**：Rust RAF 循环（UI crate）。
- **Allowed writers**：
  - container 盒（`pdf-page-container`）：`raf_loop.rs` 的 `on_wheel_event`（手势，按 `target_zoom`）与 `raf_committed.rs` 的 `apply_committed_frame`（settle，按 `visual_zoom`）。
  - 渲染表面盒（`pdf-vector-main-canvas`）：同上两处，但两条路径都必须经唯一 helper `raf_dom_cache::set_surface_box(width, height)`。
  - settle 帧的 `content_left/content_top`：唯一权威是 `ZOOM_STATE.visual_layout`（手势经 `animation.rs::compute_anchor_content_offset` 写入）。`apply_committed_frame` 必须**在 settled 分支覆盖 `visual_layout` 之前**先捕获该偏移（`gesture_offsets`），再用捕获值写 DOM，仅当捕获为 `None` 时回退到 `frame.content_left/top`；`width/height` 仍取 `frame`（单一权威是 committed render）。同一函数内先覆盖后读取等于未修复。
  - 手势几何所有权标志 `WHEEL_GESTURE_ACTIVE`：`raf_loop.rs` 私有 thread_local；写入仅 `begin_wheel_gesture()`（`on_wheel_event`）与 `end_wheel_gesture()`（`stop_zoom_raf_loop`）。
  - `zoom/animation.rs` 中的 `advance_zoom_animation_state`、`commit_rendered_zoom`。
- **Forbidden writers**：
  - TS 不得直接写容器或 canvas 盒子；raster 与 vector 呈现面在手势阶段互斥。
  - `WHEEL_GESTURE_ACTIVE` 不得定义在 `zoom_store.rs`（共享缩放状态容器），也不得被 `raf_committed.rs` 直接访问——只能经 `raf_loop::is_wheel_gesture_active()` 只读消费。
- **Pre**：`visual_layout` 中 `display_zoom`、`content_left`、`content_top` 已归一化。
- **Post**：页面内容位置、visual zoom、rendered zoom 和 CSS scale 计算一致；programmatic zoom jump 不得遗留旧 `visual_zoom`；手势期写出的表面盒与 settle 帧写出的表面盒几何一致（settle jump ratio ≈ 1）。
- **Invariants**：
  - I1：视觉尺寸连续——手势期可见面（container 与渲染表面）随 `target_zoom` 逐步缩放，不得把手势的全部缩放量推迟到 settle 那一帧。
  - I2：锚点连续（当前始终居中，ADR-0007）。
  - I3：提交不产生视觉跳变。
  - I4：手势所有权不得超出 RAF 会话——`stop_zoom_raf_loop` 必须释放 `WHEEL_GESTURE_ACTIVE`，避免 abort/reset 后几何永久不可写。
  - I5：手势活跃时 committed frame 只入队不应用（`commit_rendered_frame` 入队，RAF tick 第 3 步跳过应用）。
  - I6：位置连续——settle 不得重居中。`apply_committed_frame` 的 `content_left/top` 必须先于 settled 覆盖从 `visual_layout` 捕获（`gesture_offsets`），不得来自 `frame`（`plan_builder` 的居中值）。验证：wasm unit `raf_loop_tests.rs::settle_frame_keeps_gesture_content_offset`（Z-POSITION-001）与 E2E `zoom_wheel_sudden_jump.spec.ts` 断言 `|containerLeftDeltaSettle| <= 1.0 && |containerTopDeltaSettle| <= 1.0`。
- **Failure behavior**：非法值归一化；禁止路径写入时记录 trace 并忽略。
- **Verification**：
  - `crates/pdf-viewer-ui/src/zoom/raf_loop_tests.rs`（Z-GESTURE-001/002/003；Z-POSITION-001 `settle_frame_keeps_gesture_content_offset`，`wasm-pack test --node crates/pdf-viewer-ui`）。
  - `src/__tests__/zoom_raf_contract.test.ts`（Z-RAF-002 几何守卫、Z-RAF-003 标志作用域源码契约）。
  - `tests/e2e/specs/zoom_wheel_sudden_jump.spec.ts`（Z-SURFACE-002，真实 wheel burst 的 settle jump ratio；Z-POSITION-001，settle 位置连续性 `leftDelta/topDelta`）。
- **Related ADR**：ADR-0002（呈现几何单一写手）、ADR-0006（移除 CSS transform zoom）、ADR-0007（anchor 语义）。
- **Change coupling**：修改 RAF/SetBox 时必须同步检查 `zoom_frame.rs`、`zoom_preview.rs`、`raf_dom_cache.rs`、`raf_committed.rs`、`zoom_authority.rs`（reset 路径）、render canvas 适配层，并复核 `docs/ZOOM_ARCHITECTURE_SPEC.md` 的 Z-004.1/Z-004.2/Z-004.3。

### C-003 单一渲染链

- **Scope**：`crates/pdf-viewer-core/src/render/`、`crates/pdf-viewer-ui/src/render/`。
- **Owner**：`core/render` + `ui/render`。
- **Allowed writers**：所有可见像素必须经过 `PDF → pdf-viewer-core → pdf-viewer-ui paint plan → effective_page_plan → Rust canvas painter → DOM canvas`。
- **Forbidden writers**：
  - 浏览器字体承担 PDF 显示职责。
  - TS 使用 `canvas.fillText` 做二次绘制。
  - 编辑层用白底遮罩修复页面层。
  - 可见 textarea 作为主绘制面。
- **Pre**：render request 包含页面、缩放、viewport、当前编辑状态和可能的 source/path suppression。
- **Post**：effective page plan 对原始 text/vector object 做正确抑制，并叠加 draft/persisted editor scene。
- **Invariants**：
  - Core effective page plan、UI effective page plan、canvas draw、tile cache 和 progressive render 必须共享同一 rendered zoom/page identity。
  - 编辑 overlay 的几何坐标必须与 render plan 使用相同 page coordinate system；任何 CSS/visual zoom 变换只能在宿主显示层应用。
- **Failure behavior**：违反渲染链的像素不得出现；测试失败时阻塞合并。
- **Verification**：`crates/pdf-viewer-core/src/render/effective_page_plan/tests.rs`、`crates/pdf-viewer-ui/src/render/` 下的 frame/prepared/progressive/tile 相关模块和测试入口。
- **Related ADR**：ADR-0003（tile-based rendering）、ADR-0004（always vector rendering）。
- **Change coupling**：修改渲染链时必须同步检查 `plan_builder.rs`、`prepared_scene.rs`、`viewport_culling.rs`、`source_suppression.rs`、`path_suppression.rs`、UI canvas renderer。

### C-004 Frame token 生命周期

- **Scope**：`crates/pdf-viewer-ui/src/render/commit.rs`、`crates/pdf-viewer-ui/src/render/workflow.rs`、`crates/pdf-viewer-ui/src/present/`。
- **Owner**：`present/present_store.rs` 与 `render/commit.rs`。
- **Allowed writers**：`schedule_render_frame_request` 生成 token；`commit_render_result` 调用 `settle_render_frame`。
- **Forbidden writers**：编辑事务若不把 frame 返回给 JS，就不得在内部提前申请 frame token。
- **Pre**：`schedule_render_frame_request` 生成的 `frame_token` 必须由 JS host 消费；render result 必须带回 token、rendered zoom 和页面尺寸。
- **Post**：只有 `transition.accepted` 时才调用 `set_page_size`；返回 `RenderCommitResult { accepted, next_frame, page_width, page_height }`。
- **Invariants**：
  - Frame token 单调递增。
  - Stale/cancelled frame 不能更新页面尺寸、rendered zoom 或最终布局。
  - Zoom commit、page size、tile cache、progressive state 必须对同一 frame identity 生效。
- **Failure behavior**：非法 token 归一化；stale frame 丢弃并记录 trace。
- **Verification**：`crates/pdf-viewer-ui/src/render/commit.rs`、`render/workflow.rs` 与 `present/present_store.rs` 的内联测试。
- **Related ADR**：overview §2.3。
- **Change coupling**：修改 frame token 时必须同步检查 `render_transaction.rs`、`workflow.rs`、`commit.rs`、`present/present_store.rs`、JS host 的 `renderCurrentPage` 调用。

### C-005 Tile scheduler 与 manager 职责分离

- **Scope**：`crates/pdf-viewer-core/src/render/tile_scheduler.rs`、`crates/pdf-viewer-core/src/render/tile_manager.rs`。
- **Owner**：`tile_scheduler.rs`（纯调度）、`tile_manager.rs`（协调器）。
- **Allowed writers**：scheduler 只负责优先级与增量调度；manager 负责队列、缓存和 scheduler 委托。
- **Forbidden writers**：scheduler 不得持有 `TileCache`；manager 不得重复实现调度逻辑。
- **Pre**：tile key、tile grid 计算属于稳定领域规则。
- **Post**：scheduler 可独立测试；manager 协调渲染队列与缓存。
- **Invariants**：
  - Tile Key：`{page}|{zoom}|{dpr}|{x}|{y}`。
  - Tile Size：`512×512`。
  - Scheduler 纯调度，不管理缓存。
- **Failure behavior**：职责越界时测试失败；阻塞合并。
- **Verification**：`tile_scheduler.rs` 内联测试、`tile_manager.rs` 内联测试。
- **Related ADR**：ADR-0005（tile manager scheduler split）。
- **Change coupling**：修改 tile 架构时必须同步检查 `tile_v2.rs`、`tile_cache_legacy.rs`、`tile_bridge.ts`。

### C-006 Editor draft 与 cluster/runs 同步

- **Scope**：`crates/pdf-viewer-ui/src/editor/session/session.rs`、`crates/pdf-viewer-ui/src/editor/editor_controller.rs`、`crates/pdf-viewer-core/src/edit/`。
- **Owner**：`editor/session/session.rs` 与 `editor/editor_controller.rs`（UI crate）、`edit/`（core crate）。
- **Allowed writers**：编辑器 session/controller 中的输入同步入口、`build_editor_document_plan_for_target`（core）。
- **Forbidden writers**：UI 层不得自行重建 core 的文本目标或几何规则；应复用 `pdf_viewer_core::edit` 的结果。
- **Pre**：`active_editor_draft_text()` 有值时，编辑器处于 live editing 状态。
- **Post**：draft 文本与 cluster runs 同步；core 产出可被 UI 和渲染层消费的编辑目标、替换区域、段落场景、文档计划和持久化 patch。
- **Invariants**：
  - Core 编辑状态与 UI 编辑会话必须使用一致的 `region_id`、`paragraph_id`、`target_indices` 和文本规范化规则。
  - `source_body_text()` 保留 canonical source text，不得包含由 glyph/run 分割引入的伪空格。
- **Failure behavior**：同步失败时记录 trace 并阻塞 commit；测试失败时阻塞合并。
- **Verification**：`crates/pdf-viewer-core/src/edit/document_plan/tests.rs`、`draft_layout/tests.rs`。
- **Related ADR**：overview §2.2。
- **Change coupling**：修改 editor draft 时必须同步检查 `editor_controller.rs`、`session/`、`activation.rs`、`orchestrator/replace_pipeline.rs`、`overlay/`、`render/effective_page_plan.rs`。

### C-007 Replacement region 覆盖关系

- **Scope**：`crates/pdf-viewer-core/src/edit/replacement_region.rs`、`crates/pdf-viewer-core/src/render/source_suppression.rs`、`crates/pdf-viewer-core/src/render/path_suppression.rs`、`crates/pdf-viewer-core/src/render/viewport_culling.rs`。
- **Owner**：`replacement_region.rs`（编辑几何）、`source_suppression.rs` + `path_suppression.rs`（渲染抑制）。
- **Allowed writers**：`paragraph_replacement_region`（编辑）、`suppresses_run` / `should_suppress`（渲染）。
- **Forbidden writers**：编辑层不得用白底遮罩修复页面层；应由 `effective_page_plan` 统一抑制。
- **Pre**：`ReplacementRegion` 必须由有效编辑目标和 source bbox 构建。
- **Post**：
  - `text_clear_bbox` 覆盖原文本。
  - `path_suppression_bbox` 位于文字清除范围内或与其保持预期包含关系。
  - `cull_bbox` 覆盖完整 row band/page 范围，避免原始内容在编辑 overlay 外泄漏。
- **Invariants**：
  - `text_clear_bbox.left > shell_bbox.left`。
  - `text_clear_bbox.right < shell_bbox.right`。
  - `text_clear_bbox.bottom > shell_bbox.bottom`。
  - `path_suppression_bbox` 不超出 `text_clear_bbox`。
  - `cull_bbox` 覆盖页面宽度及 row band。
- **Failure behavior**：违反覆盖关系时渲染泄漏；测试失败时阻塞合并。
- **Verification**：`replacement_region.rs` 内联测试、`source_suppression.rs` 内联测试、`path_suppression.rs` 内联测试。
- **Related ADR**：principles §2。
- **Change coupling**：修改编辑 bbox 时必须同步检查 `source_suppression.rs`、`path_suppression.rs`、`effective_page_plan/`、UI `overlay/` 与 canvas draw。

### C-008 Patch schema 闭环

- **Scope**：`crates/pdf-viewer-core/src/persistence/models.rs`、`crates/pdf-viewer-core/src/persistence/engine.rs`、`crates/pdf-viewer-core/src/persistence/patch_store.rs`、`src-tauri/src/infrastructure/pdf/region_materializer.rs`、`src-tauri/src/infrastructure/pdf/save_engine.rs`。
- **Owner**：`persistence/models.rs`（core）、`region_materializer.rs`（Tauri）。
- **Allowed writers**：`build_paragraph_patch`（core）、`execute_region_patches`（Tauri）。
- **Forbidden writers**：UI 层不得绕过 core persistence 直接构造 patch；Tauri 不得绕过 `region_materializer` 直接写盘。
- **Pre**：patch 至少需要稳定的 `patch_key`、`page_index`、`region_id`、`original_text`、`new_text`、`source` 和 `target_indices`。
- **Post**：patch 能被 Tauri 的 `execute_region_patches` 转换为 materialization plan，再生成 `BatchTextReflowCommand`。
- **Invariants**：
  - `region_id` 必须贯穿 UI active editor、replacement target、Core patch、Tauri materialization 和保存后的缓存失效。
  - `original_text` 用于判断当前 patch 是否基于正确的源文档；`new_text` 是最终持久化内容。
  - `new_runs` 的布局信息必须与 `new_text` 和 `wrap_width` 保持一致。
- **Failure behavior**：schema 不匹配时保存失败并回滚；测试失败时阻塞合并。
- **Verification**：`persistence/models.rs` 内联测试、`region_materializer.rs` 内联测试、`save_engine.rs` 内联测试。
- **Related ADR**：overview §2.3。
- **Change coupling**：变更 `PersistableRegionPatch` 字段时需同步检查 UI 序列化、Tauri `PersistableRegionPatch` 消费、`region_materializer::build_region_materialization_plan`、`save_engine` 和前端替换流程。

### C-009 Commit 必须 bump revision

- **Scope**：`crates/pdf-viewer-ui/src/viewer/viewer_controller.rs`、`crates/pdf-viewer-ui/src/document/mutation_pipeline.rs`。
- **Owner**：`viewer/viewer_controller.rs` 的 `note_document_mutation`。
- **Allowed writers**：`document/mutation_pipeline.rs` 通过 `note_document_mutation` 记录文档变更。
- **Forbidden writers**：其他模块不得绕过 `note_document_mutation` 直接修改 revision。
- **Pre**：commit 操作已完成。
- **Post**：document revision 单调递增；渲染链感知到 revision 变化。
- **Invariants**：
  - 每次 commit 必须 bump revision。
  - Revision 变化触发渲染链刷新。
- **Failure behavior**：revision 未 bump 时渲染不刷新；测试失败时阻塞合并。
- **Verification**：`viewer/viewer_controller.rs` 与 `document/mutation_pipeline.rs` 的测试入口。
- **Related ADR**：overview §2.2。
- **Change coupling**：修改 revision 逻辑时必须同步检查 `document/mutation_pipeline.rs`、`render/commit.rs`、`present/present_store.rs`、`editor/orchestrator/commit.rs`。

### C-010 Save 事务一致性

- **Scope**：`src-tauri/src/application/pdf/edit_commands.rs`、`src-tauri/src/infrastructure/pdf/region_materializer.rs`、`src-tauri/src/infrastructure/pdf/save_engine.rs`、`src-tauri/src/app_state.rs`。
- **Owner**：`edit_commands.rs`（Tauri 应用层）。
- **Allowed writers**：`apply_region_patches`（Tauri command）。
- **Forbidden writers**：其他模块不得绕过 `apply_region_patches` 直接写盘。
- **Pre**：`ensure_document_loaded` 成功；`app_state.docs.pdf_documents` 中存在对应路径；patch 能构建有效 materialization plan。
- **Post**：
  - 磁盘文档、内存 `pdf_documents` 和后续读取缓存代表同一版本。
  - 保存失败时不会完成内存缓存替换和缓存失效后的成功状态。
- **Invariants**：
  - Cache key 使用 `"{path}::"` 前缀；任何保存流程改动必须保持三类 cache 的统一失效。
  - Undo history 保存的是保存前的文档快照；redo 在新编辑提交时清空。
  - `page_index` 在 Core/UI 中是 0-based，而注释命令构造 `page_num` 时显式使用 `(page_index + 1) as u32`。
- **Failure behavior**：保存失败时回滚内存状态并记录错误；测试失败时阻塞合并。
- **Verification**：`src-tauri/tests/integration_tests.rs`、`src-tauri/src/infrastructure/pdf/tests_reflow.rs`。
- **Related ADR**：overview §2.3。
- **Change coupling**：修改 save 事务时必须同步检查 `edit_commands.rs`、`region_materializer.rs`、`save_engine.rs`、`app_state.rs`、cache invalidation 逻辑。

### C-011 WASM/Tauri/TS 命名规则

- **Scope**：`crates/pdf-viewer-ui/src/<domain>/facade.rs`、`src-tauri/src/interfaces/`、`src/bridge/<domain>_facade.ts`。
- **Owner**：各域 facade。
- **Allowed writers**：facade 函数、Tauri command、TS facade 函数。
- **Forbidden writers**：禁止模糊动词、实现细节泄漏、同义词混用。
- **Pre**：API 设计遵循 `api-contract.md` §2。
- **Post**：
  - WASM JS API：`camelCase`。
  - Rust：`snake_case`。
  - Tauri command：`snake_case`。
  - Rust/TS 类型：`PascalCase`。
  - Request 类型以 `Request` 结尾。
  - Result 类型以 `Result` 结尾。
- **Invariants**：
  - Rust 使用 `#[serde(rename_all = "camelCase")]`。
  - TS 使用同名定义。
  - 字段可以新增，但必须 Optional。
  - 不允许重命名或删除。
- **Failure behavior**：命名违规时 clippy/tsc 失败；阻塞合并。
- **Verification**：`cargo clippy`、`npx tsc --noEmit`。
- **Related ADR**：principles §4。
- **Change coupling**：修改命名规则时必须同步检查所有 facade、Tauri command、TS facade。

---

## 4. 修改联动矩阵

修改以下模块时，必须复核对应契约：

| 修改目标 | 必须复核的契约 |
|---|---|
| Zoom state/animation | C-001、C-002、C-004 |
| Editor draft/cluster/runs | C-006、C-007、C-008 |
| Replacement region / suppression | C-007、C-003 |
| Patch schema | C-008、C-010 |
| Render plan / effective page plan | C-003、C-004、C-007 |
| Frame token / commit | C-004、C-009 |
| Save transaction | C-010、C-008 |
| Facade naming | C-011 |

---

## 5. 验证入口

- **Unit tests**：各模块内联 `#[cfg(test)] mod tests`。
- **Integration tests**：`src-tauri/tests/integration_tests.rs`。
- **CI**：`.github/workflows/ci.yml`（cargo clippy、cargo test、cargo fmt、tsc、vitest）。
- **E2E**：`tests/e2e/`（WebdriverIO + tauri-driver + Edge WebView2）。

---

## 6. 待补充契约

以下契约已识别但尚未详细记录，后续补充：

- 跨层错误结构和错误码。
- 同步/异步调用语义。
- Session 生命周期。
- 文档打开、关闭、保存期间的状态限制。
- 并发与取消规则。
- Stub 的统一返回结构和前端处理要求。
- API 清单与当前实现的自动核对机制。

---

## 7. 参考

- `docs/api-contract.md`
- `docs/architecture-principles.md`
- `docs/CONTEXT.md`
- `docs/architecture-overview.md`
- `docs/adr/`
