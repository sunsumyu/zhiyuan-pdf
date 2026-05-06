# TS → Rust 逻辑迁移计划

> 原则：**TS 只保留 DOM 操作、Canvas 绑定、事件监听**。所有决策逻辑、状态管理、算法均迁至 Rust (WASM/Tauri)。

## 0. TS 架构评估

### 问题 1：死亡的多插件架构 (`src/core/`)

`src/core/` 是从旧多插件平台遗留的基础设施：
- `plugin-loader.ts` (162行) — 加载/卸载插件，但只有 1 个插件 (pdf-viewer)
- `plugin-catalog.ts` (61行) — 列举 13 个不存在的插件 (ai-chat, algorithm-viz, dictionary...)
- `router.ts` (111行) — Hash 路由，但应用只有 1 个页面
- `event-bus.ts` (58行) — 只发射 `plugin:loaded` 事件，无订阅者
- `window-actions.ts` (100行) — 全局命名空间注册，只注册了 1 个 action
- `template-loader.ts` (152行) — HTML 模板加载，未被使用
- `interfaces.ts` (25行) — `IVisualizer`, `IAlgorithmManager` — 完全未实现

**结论**：`src/core/` 全部 ~670 行是死代码，应删除并用直接初始化替代。

### 问题 2：`main.ts` 职责混合

193 行的单一 `init()` 函数混合了：
- 应用初始化
- 20+ 个 DOM 事件绑定
- Toolbar/sidebar UI 逻辑

应拆分为：init + toolbar_bindings + sidebar_bindings

### 问题 3：全局类型安全缺失

`getWasmApi: () => any` 在 16+ 处使用，导致 WASM 接口无类型保护。

### ✅ 良好的部分

- `src/bridge/` 按域分目录（editor/render/viewer/find/comment/review/zoom/ai）— 结构清晰
- Facade 模式在用（每域一个 facade.ts 封装 WASM 调用）
- 依赖注入模式 (createXxxController(deps)) — 可测试性好
- 分层清晰：main.ts → bridge/index.ts → 各域 controller

### 行动计划

1. **立即执行**：删除 `src/core/`，简化初始化流程
2. **随迁移进行**：为 WASM API 添加类型声明 (.d.ts)
3. **后续**：拆分 main.ts UI 绑定为独立模块

## 1. 总览

| 指标 | 当前值 |
|---|---|
| TS 文件数 | 62 |
| TS 总行数 | 12,250 |
| 预估迁移后 TS 行数 | ~3,000 (DOM glue) |
| 迁移目标削减 | -75% |

## 2. 文件分类

### 🔴 必须迁移到 Rust（逻辑/算法/状态）

| 文件 | 行数 | 当前职责 | 迁移目标 |
|---|---|---|---|
| `editor/editor_host.ts` | 1141 | 编辑器状态机、光标同步、UTF16转换、格式动作 | WASM `editor` facade |
| `render/vector_host.ts` | 636 | 渲染帧调度、缓存策略、progressive render | WASM `render` facade |
| `find/pdf_find_controller.ts` | 470 | 搜索算法、匹配遍历、替换逻辑 | WASM `find` facade |
| `zoom/zoom_controller.ts` | 437 | 缩放状态机、wheel 决策、preview tick | WASM `zoom` facade |
| `render/frame_plan.ts` | 435 | 渲染计划构建、zoom plan 计算 | WASM `render` facade |
| `viewer/pdf_runtime.ts` | 478 | 应用级编排（组装各 controller） | 部分迁移（编排留 TS，决策迁 Rust） |
| `review/pdf_review_controller.ts` | 505 | 审阅状态管理、变更接受/拒绝 | WASM `review` facade |
| `document/document_edit_api.ts` | 260 | 文档编辑操作编排、保存/撤销 | WASM `document` facade |
| `editor/editor_host_diagnostics.ts` | 354 | 编辑器诊断状态跟踪 | WASM (合并入 editor) |
| `comment/pdf_comment_controller.ts` | 303 | 评论 CRUD + 状态管理 | WASM `comment` facade |
| `find/find_facade.ts` | 189 | 搜索 WASM bridge 层 | 合并入 find facade |
| **小计** | **~5200** | | |

### 🟡 部分迁移（DOM 操作留 TS，计算逻辑迁 Rust）

| 文件 | 行数 | 保留 TS 部分 | 迁移部分 |
|---|---|---|---|
| `editor/editor_host_view.ts` | 428 | DOM 元素创建、CSS 定位 | hitTest 计算、交互目标解析 |
| `render/vector_canvas_host.ts` | 327 | Canvas 创建、getContext、drawImage | 视口计算、layer 策略 |
| `viewer/pdf_viewer_api.ts` | 316 | 公开 API 壳（调 WASM） | 决策分支 |
| `render/render_flow.ts` | 240 | requestAnimationFrame 调度 | 帧计划优先级排序 |
| `render/layout_trace.ts` | 204 | console 输出 | 可整合入 Rust log |
| `viewer/viewer_geometry_probe.ts` | 172 | DOM 元素尺寸读取 | 几何计算公式 |
| `zoom/zoom_facade.ts` | 65 | WASM 调用壳 | — |
| **小计** | **~1750** | ~800 留 | ~950 迁 |

### 🟢 保留 TS（纯 DOM/UI/视觉）

| 文件 | 行数 | 原因 |
|---|---|---|
| `viewer/pdf_viewer_dom.ts` | 157 | 纯 DOM 操作 |
| `viewer/pdf_keyboard.ts` | 65 | 键盘事件绑定 → 调 WASM |
| `comment/pdf_comment_dom.ts` | 80 | DOM 创建 |
| `comment/pdf_comment_overlay_view.ts` | 120 | Overlay DOM 渲染 |
| `comment/pdf_comment_review_view.ts` | 168 | 审阅面板 DOM |
| `render/render_facade.ts` | — | 薄壳，无逻辑 |
| `render/render_facade_v2.ts` | — | 薄壳 |
| `shared/wasm_loader.ts` | 80 | WASM 加载器 |
| `shared/diagnostics.ts` | 50 | console.warn 封装 |
| `annotation/pdf_annotation_controller.ts` | 237 | 主要是 DOM 事件分发 |
| `annotation/annotation_facade.ts` | 50 | 薄壳 |
| `ai/*` (7 files) | 1022 | UI 面板，后续独立迁移 |
| `main.ts` | 161 | 入口 |
| `core/template-loader.ts` | 152 | HTML 模板 |
| **小计** | **~2500** | |

### 🔵 已有 Rust 实现、TS 仅为 thin bridge

| 文件 | 状态 |
|---|---|
| `editor/editor_facade.ts` (255行) | 已是 WASM 调用壳 |
| `editor/editor_wasm_api.ts` (185行) | 已是 WASM 调用壳 |
| `render/render_wasm_api.ts` (244行) | 已是 WASM 调用壳 |
| `document/document_facade.ts` | 已是 WASM 调用壳 |
| `find/find_facade_v2.ts` | 已是 WASM 调用壳 |
| `review/review_facade_v2.ts` | 已是 WASM 调用壳 |
| `comment/comment_facade.ts` | 已是 WASM 调用壳 |
| `viewer/viewer_facade.ts` | 已是 WASM 调用壳 |
| `viewer/viewer_session.ts` (86行) | 已是 WASM 调用壳 |
| `comment/pdf_comment_wasm_bridge.ts` | 已是 WASM 调用壳 |

## 3. 迁移优先级

| 优先级 | 域 | 行数 | 难度 | 收益 | 依赖 |
|---|---|---|---|---|---|
| **P0** | 编辑器 (editor_host) | 1141+354 | 高 | 极高（消除 TS↔WASM 频繁往返） | 无 |
| **P1** | 搜索 (find_controller) | 470+189 | 中 | 高（算法全在 TS 不合理） | 无 |
| **P2** | 缩放 (zoom_controller) | 437 | 中 | 高（状态机 + 决策全在 TS） | 依赖 render |
| **P3** | 渲染调度 (vector_host + frame_plan) | 636+435 | 高 | 高（性能关键路径） | 依赖 zoom |
| **P4** | 审阅 (review_controller) | 505 | 中 | 中 | 无 |
| **P5** | 评论 (comment_controller) | 303 | 低 | 中 | 无 |
| **P6** | 文档编辑API (document_edit_api) | 260 | 低 | 中 | 依赖 editor |

## 4. P0 编辑器迁移细化

### editor_host.ts 内部职责拆解

| 职责 | 行数 (估) | 迁移策略 |
|---|---|---|
| UTF16↔char index 转换 | ~30 | **已在 WASM** (facadeUtf16ToCharIndex) |
| textarea 光标同步 | ~50 | 保留 TS (DOM API) |
| 编辑器 open/commit/close 状态机 | ~200 | → WASM editor session |
| keyboard/input 事件处理 | ~100 | 事件捕获留 TS，决策迁 WASM |
| format action 分发 | ~80 | → WASM applyFormat |
| 交互目标扫描 (scanBlueRun) | ~60 | → WASM hitTest |
| caret 位置计算 | ~80 | → WASM caret resolver |
| save/commit 编排 | ~120 | 部分迁移 |
| DOM 显示 (shell 定位、overlay) | ~200 | 保留 TS |
| 诊断 | ~354 | → WASM (合并) |

### 迁移后 TS 剩余 (~250 行)：
- textarea 创建 + focus/blur 管理
- keyboard event listener → 调 WASM `processKeyEvent()`
- input event listener → 调 WASM `processInputEvent()`
- shell DOM 定位 (CSS transform)
- overlay DOM 渲染

### 新增 Rust (WASM) 接口：
```rust
// crates/pdf-viewer-ui/src/editor/facade.rs 新增
fn editorProcessKeyEvent(key: String, modifiers: u8) -> EditorKeyResult;
fn editorProcessInputEvent(text: String, caretUtf16: u32) -> EditorInputResult;
fn editorGetInteractionTargets(pageIndex: u16, displayZoom: f32) -> Vec<InteractionTarget>;
fn editorResolveHitTarget(x: f32, y: f32) -> Option<HitResult>;
```

## 5. 执行原则

1. **每次迁移一个 "切片"**：不要一次性重写整文件，而是按职责逐函数迁移
2. **保持 TS 编译通过**：每个 PR 迁移后 TS 仍可编译运行
3. **先写 WASM facade 接口 → 再让 TS 调用它 → 最后删除 TS 旧代码**
4. **测试策略**：Rust 单测 + TS 集成测试双覆盖
5. **回退能力**：保留 TS 旧实现为 `_legacy` 后缀，feature flag 切换

## 6. 预估工时

| 阶段 | 预估 |
|---|---|
| P0 编辑器 | 3-5 个 session |
| P1 搜索 | 1-2 个 session |
| P2 缩放 | 1-2 个 session |
| P3 渲染 | 3-4 个 session |
| P4-P6 | 各 1 session |
| **总计** | ~12-16 session |
