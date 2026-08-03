# WASM UI 状态管理企业级方案

> 目标：解决 `pdf-viewer-ui` 中全局 `AppContext` 单 `RefCell` 导致的重入借用 panic，并建立可长期维护、可测试、可演进的状态管理框架。
>
> 本方案面向 `crates/pdf-viewer-ui` 的 WASM UI 层，同时约束 TypeScript bridge、render scheduler、document/editor workflow 的交互方式。

---

## 1. 背景与问题定义

当前 `codex/refactor-split` 分支将多个业务域状态收敛到了统一的 `AppContext`：

```rust
thread_local! {
    pub static APP_CONTEXT: RefCell<AppContext> = RefCell::new(AppContext::default());
}

pub fn with_context<R>(f: impl FnOnce(&AppContext) -> R) -> R {
    APP_CONTEXT.with(|ctx| f(&ctx.borrow()))
}

pub fn with_context_mut<R>(f: impl FnOnce(&mut AppContext) -> R) -> R {
    APP_CONTEXT.with(|ctx| f(&mut ctx.borrow_mut()))
}
```

这带来了一个结构性问题：

```text
with_context_mut(|ctx| {
    // 持有整个 AppContext 的 mutable borrow
    ...
    some_business_function();
        └─ 内部再次调用 with_context(...) 或 with_context_mut(...)
           └─ RefCell already borrowed / already mutably borrowed
});
```

在 WASM 中，Rust panic 通常表现为 `RuntimeError: unreachable`。如果 panic 发生在状态访问路径中，后续多个 UI、render、editor、document 调用会继续触发连锁异常，表现为：

- `RefCell already mutably borrowed`
- `RefCell already borrowed`
- `RuntimeError: unreachable`
- 打开文档失败
- 首帧渲染失败
- reset / close / clearPendingAnchor 等恢复逻辑继续失败

### 1.1 当前问题不是单点 bug

这不是某个 `resolveFramePlan()` 或 `ViewerSession.read()` 的单点错误，而是 **全局状态访问模型不支持业务重入**。

PDF viewer 的真实业务链路天然存在重入需求：

- 打开文档时更新 viewer，同时初始化 page、render、editor。
- render frame plan 计算时读取 viewer/page/zoom，同时写 render scheduler。
- 缩放时读取当前 viewer session，同时更新 zoom、render、presentation。
- reset 流程会清理 document、render、editor、zoom、page turn、pending anchor。
- 编辑流程会读取 legacy snapshot，同时触发页面重绘。

如果所有状态都被一个 `RefCell<AppContext>` 包住，那么任何一个业务域的写操作都会阻塞所有其他业务域的读写。

---

## 2. 企业级设计目标

本方案的目标不是临时绕过 panic，而是建立可扩展的状态管理框架。

### 2.1 必须满足的目标

1. **状态分域**  
   viewer、render、zoom、page、editor、find、review、present 等业务域应拥有独立状态边界。

2. **短 borrow 生命周期**  
   状态借用只允许包裹最小的数据读写，不允许在 borrow 闭包内执行复杂业务编排、异步逻辑或跨域调用。

3. **Action / Reducer / Effect 分层**  
   状态修改与副作用分离，避免在状态 mutation 中直接触发 render、DOM、Tauri IPC 或异步 workflow。

4. **Snapshot 优先**  
   跨域计算必须先读取不可变 snapshot，再进入单域 mutation。

5. **渲染请求队列化**  
   render flow 不允许多入口并发直接进入 WASM 状态系统，应通过 scheduler 合并、去重、串行化。

6. **Safe Rust 优先**  
   不使用 `UnsafeCell` 绕过 borrow 检查作为正式方案。除非有明确内存模型证明和测试覆盖，否则禁止将 UB 风险引入主路径。

7. **可观测性**  
   状态 dispatch、render scheduling、document workflow、editor lifecycle 必须具备诊断日志和 trace id。

---

## 3. 知名框架的参考模式

### 3.1 Redux / Elm

Redux 和 Elm 允许单一状态树，但它们严格约束：

- reducer 是纯函数。
- reducer 中不能执行副作用。
- reducer 中不能递归 dispatch。
- 状态更新完成后再通知订阅者和执行 effect。

抽象流程：

```text
Action
  ↓
Reducer 纯计算 next state
  ↓
Commit state
  ↓
Run effects
  ↓
Render / IPC / async work
```

对本项目的启发：

- 不能在 `with_context_mut` 中调用可能再次读写 store 的高层业务函数。
- document open、render request、editor refresh 应拆成 action + effect。

### 3.2 React

React 不允许 render 阶段随意同步修改状态。状态更新会被排队、批处理，并在 commit 之后执行 effect。

对本项目的启发：

- 打开 PDF 后不应立刻深层调用所有 render/editor/reset 函数。
- 应先提交状态，再将 render/editor refresh 作为 effect 排队。

### 3.3 Vue / Svelte / Signals

响应式框架通过依赖追踪、dirty 标记、microtask flush 防止同步更新链无限重入。

对本项目的启发：

- render 请求应该标记 pending，而不是每个入口立即执行完整 render flow。
- 多个更新应合并到同一帧。

### 3.4 Bevy / ECS

Bevy 将状态拆成资源，通过系统声明访问权限：

```rust
fn render_system(
    viewer: Res<ViewerState>,
    page: Res<PageState>,
    mut render: ResMut<RenderState>,
) {
    ...
}
```

对本项目的启发：

- 状态访问边界要显式。
- `viewer` 的读不应被 `render` 的写阻塞。
- 不要用一个超级全局 `RefCell<AppContext>` 模拟所有资源。

---

## 4. 推荐总体架构

### 4.1 分层架构

```text
TypeScript UI / Tauri Event / DOM Event
                  ↓
           WASM API Boundary
                  ↓
             Command Adapter
                  ↓
             AppAction Dispatch
                  ↓
        ┌─────────┼─────────┐
        ↓         ↓         ↓
   ViewerStore RenderStore EditorStore ...
        ↓         ↓         ↓
           State Snapshots / DTOs
                  ↓
              AppEffects
                  ↓
     Render Scheduler / Tauri IPC / DOM / Canvas
```

### 4.2 状态访问原则

禁止模式：

```rust
with_context_mut(|ctx| {
    ctx.render.pending = true;

    // 高风险：该函数内部可能重新进入 AppContext。
    render_current_page();
});
```

推荐模式：

```rust
let viewer = viewer_store::read_snapshot();
let page = page_store::read_snapshot();
let zoom = zoom_store::read_snapshot();

let effect = render_store::schedule_frame(RenderScheduleInput {
    viewer,
    page,
    zoom,
    reason,
});

app_effects::run(effect);
```

关键区别：

- 读取 snapshot 时没有持有长期 borrow。
- 写 render store 时只写 render。
- 副作用在状态 borrow 释放后执行。

---

## 5. Store 分域设计

### 5.1 替代当前单 RefCell AppContext

当前：

```rust
thread_local! {
    pub static APP_CONTEXT: RefCell<AppContext> = RefCell::new(AppContext::default());
}
```

目标：

```rust
pub struct AppStores {
    pub viewer: RefCell<HostViewerSession>,
    pub zoom: RefCell<HostZoomState>,
    pub review: RefCell<HostCommentReviewSession>,
    pub render: RefCell<HostRenderState<serde_json::Value>>,
    pub render_loop: RefCell<HostRenderLoopState>,
    pub page: RefCell<PageState>,
    pub prepared_scene: RefCell<Option<PreparedPageScene>>,
    pub progressive_task: RefCell<Option<ProgressiveVectorRenderTask>>,
    pub page_turn: RefCell<PageTurnSnapshot>,
    pub present: RefCell<HostPresentState>,
    pub frame_cache: RefCell<HostFrameCacheState>,
    pub viewport_refresh: RefCell<HostViewportRefreshState>,
    pub find_controller: RefCell<FindControllerInner>,
    pub find_session: RefCell<HostFindSession>,
    pub editor_mode: RefCell<EditorModeState>,
    pub editor_host: RefCell<EditorHostRuntimeState>,
    pub editor_session: RefCell<EditorSessionStore>,
}

thread_local! {
    pub static APP_STORES: AppStores = AppStores::default();
}
```

### 5.2 领域 accessor

示例：viewer store

```rust
pub fn with_viewer<R>(f: impl FnOnce(&HostViewerSession) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.viewer.borrow()))
}

pub fn with_viewer_mut<R>(f: impl FnOnce(&mut HostViewerSession) -> R) -> R {
    APP_STORES.with(|stores| f(&mut stores.viewer.borrow_mut()))
}

pub fn read_viewer_snapshot() -> HostViewerSession {
    with_viewer(Clone::clone)
}
```

示例：render store

```rust
pub fn with_render<R>(f: impl FnOnce(&HostRenderState<serde_json::Value>) -> R) -> R {
    APP_STORES.with(|stores| f(&stores.render.borrow()))
}

pub fn with_render_mut<R>(f: impl FnOnce(&mut HostRenderState<serde_json::Value>) -> R) -> R {
    APP_STORES.with(|stores| f(&mut stores.render.borrow_mut()))
}
```

### 5.3 Store 边界建议

| Store | 内容 | 典型 API |
|---|---|---|
| `viewer_store` | path、current_page、page_count、page_width、page_height、document_revision | `read_snapshot`、`set_document`、`set_current_page`、`bump_revision` |
| `zoom_store` | target zoom、wheel zoom、preview anchor | `read_snapshot`、`set_target_zoom`、`clear_pending_anchor` |
| `render_store` | frame token、pending frame、progressive state、cache metadata | `schedule_frame`、`commit_frame`、`abort_frame` |
| `render_loop_store` | render loop queue、committed frame queue | `queue_frame`、`take_ready_frame` |
| `page_store` | vector model、paint plan、viewport、prepared scene、progressive task | `init_page_context`、`update_viewport`、`read_snapshot` |
| `editor_store` | editor mode、host runtime、session state、active block | `read_snapshot`、`reset_for_document`、`commit_edit_state` |
| `find_store` | find controller/session | `open`、`close`、`set_result`、`read_toolbar_state` |
| `review_store` | comment/review session | `read_snapshot`、`apply_review_update` |
| `present_store` | page turn、present state、frame cache、viewport refresh | `request_page_turn`、`resolve_present_decision` |

---

## 6. Action / Effect 模型

### 6.1 Action 定义

```rust
pub enum AppAction {
    DocumentOpened(DocumentOpenedPayload),
    DocumentClosed,
    CurrentPageChanged { page_index: u16, reason: PageChangeReason },
    ZoomChanged { zoom: f32, reason: ZoomChangeReason },
    RenderRequested(RenderRequest),
    RenderCompleted(RenderCommitPayload),
    EditorSnapshotLoaded(EditorSnapshotPayload),
    FindSessionUpdated(FindSessionPayload),
}
```

### 6.2 Effect 定义

```rust
pub enum AppEffect {
    RequestRender(RenderRequest),
    RefreshEditorSnapshot,
    PrefetchAdjacentPages,
    SyncToolbar,
    PersistPatch,
    ClearCanvas,
    None,
}
```

### 6.3 Dispatch 规则

```rust
pub fn dispatch(action: AppAction) -> Vec<AppEffect> {
    match action {
        AppAction::DocumentOpened(payload) => {
            viewer_store::set_document(payload.viewer);
            page_store::reset_for_document(payload.page);
            editor_store::reset_for_document(payload.editor);
            render_store::reset_for_document(payload.render);

            vec![
                AppEffect::RequestRender(RenderRequest::first_page()),
                AppEffect::RefreshEditorSnapshot,
                AppEffect::SyncToolbar,
                AppEffect::PrefetchAdjacentPages,
            ]
        }
        AppAction::ZoomChanged { zoom, reason } => {
            zoom_store::set_target_zoom(zoom);
            vec![AppEffect::RequestRender(RenderRequest::zoom(reason))]
        }
        _ => vec![],
    }
}
```

### 6.4 Effect 执行规则

Effect 必须在所有状态 borrow 释放后执行。

```rust
pub async fn run_effects(effects: Vec<AppEffect>) {
    for effect in effects {
        match effect {
            AppEffect::RequestRender(request) => {
                render_scheduler::request_render(request).await;
            }
            AppEffect::RefreshEditorSnapshot => {
                editor_lifecycle::refresh_snapshot().await;
            }
            _ => {}
        }
    }
}
```

禁止在 reducer / store mutation 内直接执行 effect。

---

## 7. Render Scheduler 企业级约束

### 7.1 问题

当前打开文档后可能由多个路径同时触发：

- `resolveFramePlan`
- `scheduleRenderFrame`
- `queueRenderLoopFrame`
- `ViewerSession.read`
- `EditorSession.readLegacySnapshot`
- `DocumentSession.close`
- `clearPendingAnchor`

如果这些流程直接互相调用，就会导致状态系统重入。

### 7.2 目标模型

```rust
pub struct RenderSchedulerState {
    pub current_token: u64,
    pub running: bool,
    pub pending: Option<RenderRequest>,
    pub last_committed_zoom: f32,
    pub last_reason: Option<RenderReason>,
}
```

所有入口只允许调用：

```rust
request_render(RenderRequest)
```

不允许直接调用完整 render flow。

### 7.3 合并规则

| 场景 | 策略 |
|---|---|
| 当前没有 render running | 生成 token，返回 `Effect::StartRender(token)` |
| 当前已有 render running，同页同 zoom | 合并 reason，忽略重复请求 |
| 当前已有 render running，但 zoom/page 变化 | 写入 pending，当前帧结束后启动最新 pending |
| 文档 revision 变化 | abort 旧 token，清空 pending，启动新文档首帧 |
| editor commit 触发 render | 合并为当前文档 revision 下的高优先级 request |

### 7.4 Token 验证

所有异步 render 结果提交前必须检查：

```rust
if !render_store::is_frame_current(token) {
    return RenderCommitResult::Stale;
}
```

---

## 8. TypeScript Bridge 约束

### 8.1 WASM 入口不得并发轰炸

TypeScript 侧应对关键流程做串行化：

```ts
let documentOpenPromise: Promise<void> | null = null;

async function openPdfFile(path: string) {
  if (documentOpenPromise) {
    await documentOpenPromise;
  }

  documentOpenPromise = doOpenPdfFile(path).finally(() => {
    documentOpenPromise = null;
  });

  return documentOpenPromise;
}
```

### 8.2 Render 请求只走 scheduler

禁止 TS 直接在多个地方调用：

```ts
resolveFramePlan();
scheduleRenderFrame();
queueRenderLoopFrame();
renderPage();
```

推荐统一入口：

```ts
renderScheduler.requestRender({ reason, pageIndex, zoom });
```

### 8.3 Controller 初始化不得触发重型状态写

`plugin.initialize()` 阶段只允许：

- 初始化 WASM。
- 注册事件监听。
- 读取轻量状态。
- 准备 controller。

不应主动触发 document open、render、editor snapshot 等重型状态链路。

---

## 9. 分阶段实施计划

### Phase 0：止血与定位

目标：确认当前 panic 源头和最小复现路径。

任务：

- [ ] 在 `app_context.rs` 的 accessor 上增加 debug trace，记录 `with_*` 入口名称。
- [ ] 复现打开 PDF 首帧 panic。
- [ ] 记录首个 panic，而不是后续连锁 panic。
- [ ] 将 `favicon.ico 404` 标记为非阻塞问题，不纳入主线排查。

验收：

- 能明确第一个 reentrant borrow 的业务路径。

### Phase 1：拆分 AppContext 为 AppStores

目标：移除单 `RefCell<AppContext>` 的超级锁。

任务：

- [ ] 新建 `AppStores`。
- [ ] 将 viewer、zoom、render、page、editor、find、review、present 等字段改为领域级 `RefCell`。
- [ ] 为每个领域提供 `with_xxx` / `with_xxx_mut` accessor。
- [ ] 修改现有 store 文件，不再依赖 `with_context` / `with_context_mut`。
- [ ] 保留 `app_context.rs` 作为统一 store registry，不作为统一 borrow root。

验收：

- `findstr /s /n "with_context" crates/pdf-viewer-ui/src/*.rs` 只允许出现在 `app_context.rs` 或兼容层。
- 打开 PDF 不再出现 `RefCell already borrowed`。
- `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown` 通过。

### Phase 2：Snapshot 化跨域计算

目标：消除持有某个 store borrow 时调用高层业务函数的模式。

任务：

- [ ] 为 viewer/page/zoom/render/editor 提供 snapshot DTO。
- [ ] 修改 frame plan 计算：先读 snapshot，再 compute，再 commit。
- [ ] 修改 document open workflow：状态提交与 effect 分离。
- [ ] 修改 reset workflow：按 store 顺序短 borrow 清理，不在 borrow 内调用高层函数。

验收：

- 禁止在 `with_*_mut` 闭包中调用 `*_api`、`*_workflow`、`render_current_page`、`open_*_flow` 等高层函数。
- 打开文档、翻页、缩放、编辑刷新均无 borrow panic。

### Phase 3：引入 AppAction / AppEffect

目标：将业务流程框架化，避免未来继续引入重入问题。

任务：

- [ ] 定义 `AppAction`。
- [ ] 定义 `AppEffect`。
- [ ] 实现 `dispatch(action) -> Vec<AppEffect>`。
- [ ] 将 document open、zoom、page turn、editor commit 的状态修改迁移到 dispatch。
- [ ] effect 在 TS 或 WASM async workflow 层执行。

验收：

- document open 主流程不再直接深层调用 render/editor/reset 高层函数。
- 所有副作用都有明确 effect 类型。
- diagnostics 能按 action/effect 输出 trace。

### Phase 4：Render Scheduler 统一入口

目标：收敛所有渲染请求，避免多入口并发。

任务：

- [ ] 定义 `RenderRequest`、`RenderReason`、`RenderToken`。
- [ ] 所有 render 请求统一走 `request_render`。
- [ ] 实现 running/pending 合并策略。
- [ ] 所有 render commit 检查 token 和 document revision。

验收：

- 缩放滚轮连续触发时，render 请求被合并。
- 打开文档时只产生一条首帧 render 主链路。
- stale render result 不会污染当前画面。

### Phase 5：测试与回归防线

目标：让状态重入问题不可回归。

任务：

- [ ] 为每个 store 添加 unit test。
- [ ] 为 dispatch 添加 action/effect 快照测试。
- [ ] 为 render scheduler 添加 token/pending 合并测试。
- [ ] 增加 WASM integration smoke test：open → render first page → zoom → page turn → edit snapshot。
- [ ] 增加 lint/grep 检查：禁止新增全局 `RefCell<AppContext>` 模式。

验收：

- CI 中 `cargo check`、核心单元测试、WASM check 均通过。
- 首帧打开、缩放、翻页、编辑 smoke test 通过。

---

## 10. 编码规范

### 10.1 禁止项

禁止在 store mutation 内执行以下操作：

```rust
with_xxx_mut(|state| {
    some_async_or_workflow_function();
    render_current_page();
    open_document_flow();
    editor_lifecycle_refresh();
    tauri_invoke();
});
```

禁止直接暴露大范围上下文：

```rust
pub fn with_context_mut<R>(f: impl FnOnce(&mut AppContext) -> R) -> R
```

禁止正式主路径使用 `UnsafeCell` 绕过 borrow panic。

### 10.2 推荐项

推荐短 borrow：

```rust
pub fn set_current_page(page_index: u16) {
    with_viewer_mut(|viewer| {
        viewer.current_page = page_index;
    });
}
```

推荐 snapshot：

```rust
let viewer = viewer_store::read_snapshot();
let zoom = zoom_store::read_snapshot();
let page = page_store::read_snapshot();
let plan = compute_frame_plan(&viewer, &zoom, &page);
render_store::commit_plan(plan);
```

推荐 action/effect：

```rust
let effects = app_dispatch::dispatch(AppAction::ZoomChanged { zoom, reason });
app_effects::run(effects).await;
```

---

## 11. 迁移风险与控制

| 风险 | 说明 | 控制措施 |
|---|---|---|
| 拆分后调用点多 | 当前约 78 处状态访问 | 按 store 文件逐个迁移，保持 public API 不变 |
| 跨 store 仍可能嵌套 | 拆分不能解决同一 store 内重入 | Phase 2 snapshot 化，禁止 mutation 内高层调用 |
| render 行为变化 | render scheduler 合并可能改变时序 | token/revision 验证 + smoke test |
| TS/WASM 入口不一致 | TS 仍可能直接调用多个 render API | 收敛到 renderScheduler.requestRender |
| 重构过程中功能回退 | document/editor/render 交叉复杂 | 分阶段，每阶段有独立验收 |

---

## 12. 当前建议落地顺序

针对当前控制台报错，建议优先级如下：

1. **Phase 1：拆分 AppContext 为 AppStores**  
   先解决 `RefCell already borrowed` 的结构性触发条件。

2. **Phase 2：Snapshot 化 frame plan / document open / reset**  
   解决同一 store 内潜在重入。

3. **Phase 4：Render Scheduler 统一入口**  
   解决打开文档后 render 多入口并发问题。

4. **Phase 3：Action / Effect 框架化**  
   在主流程稳定后进行系统性收敛。

5. **Phase 5：测试与回归防线**  
   将当前 bug 固化为测试用例，防止回归。

---

## 13. 验收清单

### 功能验收

- [ ] 应用启动无白屏。
- [ ] 打开 PDF 无 `RefCell already borrowed`。
- [ ] 首帧页面正常渲染。
- [ ] 翻页正常。
- [ ] 缩放正常。
- [ ] 文本编辑入口正常。
- [ ] 查找面板正常。
- [ ] reset / close document 不触发 panic。

### 架构验收

- [ ] 不再存在单一 `RefCell<AppContext>` 作为所有状态的 borrow root。
- [ ] store accessor 按领域划分。
- [ ] 跨域计算使用 snapshot。
- [ ] render 请求统一进入 scheduler。
- [ ] 状态 mutation 与 side effect 分离。
- [ ] 新代码不引入 `UnsafeCell` 绕过 borrow 检查。

### 工程验收

- [ ] `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown` 通过。
- [ ] WASM build 通过。
- [ ] Tauri dev 打开文档 smoke test 通过。
- [ ] diagnostics 可定位 action/effect/render token。

---

## 14. 最终结论

当前 `AppContext` 单 `RefCell` 方案属于过度收口：它减少了 thread_local 数量，但把所有业务域绑定到同一个运行时 borrow 锁上，不适合 PDF viewer 这种高交互、多管线、重渲染调度的应用。

企业级方案不是用 `UnsafeCell` 绕过 panic，而是：

1. 拆分状态边界。
2. 缩短 borrow 生命周期。
3. 使用 snapshot 组合跨域数据。
4. 引入 action/effect 分层。
5. 将 render 请求队列化和 token 化。
6. 用测试和 diagnostics 固化行为。

该方案可以同时解决当前 panic，并为后续 editor、render、document workflow 的长期维护提供稳定基础。
