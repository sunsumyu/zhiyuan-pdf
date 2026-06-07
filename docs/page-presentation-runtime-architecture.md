# Page Presentation Runtime 架构方案

> v1 · 2026-06-03 · 针对翻页闪屏、翻页不跟手、相邻页并发抢占、vector extraction 重复启动的问题。

## 0. 结论

翻页闪屏不是单点 canvas bug，而是页面呈现缺少框架级 runtime。

当前 `viewer`、`render scheduler`、`vector host`、Tauri `page_model_service` 各自持有一部分页面状态和副作用。用户翻页后，旧页、目标页、相邻页的 preview/vector/layout 任务会并发运行，且没有一个全局 owner 决定：

- 哪个页是最新用户意图。
- 哪些任务应立即取消或降级。
- 哪些后端结果允许提交到可见 canvas。
- preview、vector、detail tile、editor overlay 的替换顺序。
- 相邻页预热何时可以启动。

应新增 `PagePresentationRuntime`，把页面状态、任务优先级、asset pipeline、present commit、prefetch 和事件日志收口到一条显式工作流。

## 1. 现象与日志证据

终端日志显示一次翻页附近发生了这些事情：

```text
[PDF][resolve_vector_page_model] START page=36
[PDF][resolve_vector_page_model] START page=37
[PDF-V3] Requesting Inference for page=37
[PDF][resolve_vector_page_model] START page=35
[PDF-V3] Requesting Inference for page=35
[PDF][resolve_vector_page_model] START page=37
[PDF-Vector][Cache] HIT (after lock wait) for resolve_paths: key=..._37
[PROF] Vector Model Ready: 76 objects ... Total Time: 459.1852ms
[PROF] Vector Model Ready: 76 objects ... Total Time: 390.5048ms
```

关键判断：

- page 35、36、37 同时参与高成本解析，说明相邻页预热没有被当前页优先级约束。
- page 37 多次 `START` 后又 `HIT (after lock wait)`，说明存在 in-flight 去重不完整或调度入口重复。
- 单页 vector model 需要约 390-459ms，直接等待 vector 完成再呈现不可能跟手。
- preview 路径日志里曾出现约 5ms 级结果，说明翻页首帧应优先使用 preview。

## 2. 参考框架与本项目已有经验

### 2.1 Nutrient / PSPDFKit

本项目已有 `docs/nutrient-comparison.md`，核心可借鉴点是：

- `Instance` 是一个文档/容器级边界。每个实例隔离自己的 view state、事件和任务。
- `ViewState` 采用不可变快照与函数式更新，`setViewState` 一次可以原子改变多个字段。
- 事件模型是 DOM-style `addEventListener`，状态变化通过命名事件广播。
- dirty tracking、save state 和 autosave mode 是显式状态，不散落在编辑或渲染入口中。

本项目不应照搬 Nutrient 的 god object。我们已经选择分域 Session，这是正确方向。但页面呈现需要一个 Nutrient-style 的原子 `ViewState` 和事件事务。

### 2.2 nushell-enhanced 迁移前设计

`docs/nushell-divergence-report-2026-05-06.md` 记录了一个重要教训：视觉问题不能在症状端修。编辑态偏移、tofu、蓝框宽都不是 CSS 小问题，而是可见文字进入了浏览器渲染链。

对翻页同样适用：

- 不要靠散落的 `clearRect`、`visibility`、`setTimeout` 修闪屏。
- 不要让旧页、preview、vector、detail layer 分别由不同模块决定显示/隐藏。
- 可见像素必须经过单一页面呈现链，只有 `PagePresenter` 能提交可见帧。

### 2.3 PDF.js

PDF.js 的方向是：可见页优先队列、渲染状态机、可取消 render task、detail view / visible view 优先级。它证明了成熟 PDF viewer 不会让相邻页随意抢当前页资源。

对本项目的启发：

- current visible page 永远最高优先级。
- 相邻页只能 idle prefetch。
- render task 必须可取消或至少可 stale-ignore。
- detail layer 不能阻塞首帧。

### 2.4 MuPDF

MuPDF 的 `DisplayList` 思路是：先把 PDF page 解释为可复用中间表示，再用于绘制、文本提取、搜索等多种用途。它避免每个视图操作都重新解释 PDF content stream。

对本项目的启发：

- `resolve_vector_page_model` 不应只是一次性 render helper，而应上升为 `DisplayListCache` / `PageIntermediateCache`。
- preview、vector render、text search、editor hit-test 应共享页面 IR。
- 高成本解析结果应按 document revision 和 page index 缓存。

## 3. 当前职责混杂

### Presentation 层混杂

`src/bridge/render/vector_host.ts` 和 `vector_canvas_host.ts` 同时关心：

- stage canvas 和 visible canvas 生命周期。
- frame cache。
- progressive render。
- DOM canvas present。
- stale frame 判断。
- detail layer visibility。

这些属于不同层次。可见提交应集中到 `PagePresenter`，render host 只提供 canvas backend。

### Application 层混杂

`src/bridge/viewer/pdf_runtime.ts` 和 `render_flow.ts` 现在承担：

- open document 编排。
- render loop。
- page indicator 更新。
- preview fallback。
- vector render。
- editor overlay sync。
- zoom commit。

这不是单一 workflow。翻页应有独立的 `PagePresentationRuntime`，由它协调 viewer、render、zoom、editor。

### Backend 层混杂

Tauri 后端当前把 preview、vector model、glyph paint plan、layout inference 分散在多个 command/service 中。缺少统一的 page asset service：

- 不知道当前请求优先级。
- 不知道是否 stale。
- 重复请求只能靠局部 cache/lock。
- 无统一 in-flight task registry。

## 4. 目标架构

```text
用户导航 / 滚动 / 缩放 / 编辑提交
    |
    v
PagePresentationRuntime
    |
    +-- PageTurnCoordinator       最新用户意图，pageTurnId
    +-- PresentationViewState     不可变快照，原子化更新
    +-- PageRenderQueue           优先级队列，最新优先（latest-wins），支持取消
    +-- PageAssetPipeline         资源管道 (preview/vector/displayList/paintPlan/detailTile)
    +-- PagePresenter             唯一的可见帧提交点
    +-- PagePrefetchController    空闲时低优先级预热/预取
    +-- PresentationEventBus      可追溯的状态与渲染事件总线
```

### 分层放置 (Layer placement)

| 模块 | 分层 | 所有权 |
|---|---|---|
| `PagePresentationRuntime` | TS 应用层组装 | 跨域页面工作流协调者 |
| `PageTurnCoordinator` | WASM 或 TS 应用层 | 最新页面意图和 Token 维护者 |
| `PresentationViewState` | WASM 领域状态 | 不可变快照与原子更新 |
| `PageRenderQueue` | WASM 渲染/呈现领域 | 优先级、任务取消、帧准入决策 |
| `PageAssetPipeline` | TS 适配器 | 调用 WASM/Tauri 并携带 Token |
| `PageAssetService` | Tauri 应用层 | 执行中去重、优先级调度、取消分发 |
| `DisplayListCache` | Tauri 基础设施层 | 可复用的已解析页面 IR（中间表示） |
| `PagePresenter` | TS 呈现层 | 仅负责 DOM/Canvas 可见提交 |

TS 可以拥有 DOM 和 Canvas API 的调用权。Rust/WASM 应该拥有状态转换、渲染策略、优先级决策、坐标转换以及帧准入逻辑。

**铁律：** 除必要的显示和交互粘合逻辑外，非必要的前端逻辑必须移至 Rust/WASM。TS 可以挂载 DOM 节点、复制 Canvas 像素、读取宿主测量值、绑定浏览器事件并调用 WASM/Tauri 适配器。TS 绝不能拥有翻页准入、渲染优先级、过期帧决策、预取策略、几何规则、文本渲染语义、保存/写回策略或缓存失效。

## 5. 核心概念

### 5.1 `pageTurnId`

每个导航意图都会递增一个单调递增的 Token：

```typescript
type PageTurnId = number;

type PageIntent = {
  documentId: string;
  documentRevision: number;
  pageIndex: number;
  pageTurnId: PageTurnId;
  reason: "next" | "prev" | "jump" | "scroll" | "open" | "editCommit";
  direction: -1 | 0 | 1;
};
```

只有与最新 `pageTurnId` 匹配的结果才允许呈现在可见层上。过期的任务如果成本较低且已接近完成，可以写入缓存以备后用，但绝不能影响当前 UI。

### 5.2 `PresentationViewState`

借鉴 Nutrient 的 ViewState 思想，页面状态更改应该是原子性的：

```rust
pub struct PresentationViewState {
    pub document_id: String,
    pub document_revision: u64,
    pub current_page: u16,
    pub target_page: u16,
    pub page_turn_id: u64,
    pub zoom: f32,
    pub render_phase: RenderPhase,
    pub visible_surface: VisibleSurface,
    pub prefetch: PrefetchState,
}
```

`setCurrentPage + render + zoom layout` 不应再是三个无关的变更。一次翻页被定义为一个状态事务：

```text
Viewing(page=36, vector) 浏览中
  -> Turning(page=37, oldSurfaceRetained) 翻页中（保留旧页面像素）
  -> PreviewVisible(page=37) 预览图可见
  -> VectorVisible(page=37) 矢量图可见
  -> Idle(page=37, prefetchAllowed) 空闲态（允许预取）
```

### 5.3 `RenderPhase`

```rust
pub enum RenderPhase {
    Idle,
    Turning,
    LoadingPreview,
    PreviewVisible,
    RenderingVector,
    RenderingDetail,
    VectorVisible,
    EditingOverlay,
    ErrorRecoverable,
}
```

这使得非法状态转换变得可见。例如，当当前页处于 `LoadingPreview` 阶段时，不能启动 `PrefetchVector` 任务。

### 5.4 页面资产 (Page assets)

```typescript
type PageAssetKind =
  | "preview"
  | "displayList"
  | "vectorModel"
  | "paintPlan"
  | "baseBitmap"
  | "detailTile"
  | "editorOverlay";
```

资产管道必须将预览（preview）和矢量（vector）视为不同的产物：

- **preview**：快速首帧，低保真度，可以是栅格图像。
- **displayList/vectorModel**：高成本、可复用的 IR（中间表示）。
- **paintPlan/baseBitmap**：页面画布的具体绘制内容。
- **detailTile**：视口（viewport）高分辨率精细化调整。
- **editorOverlay**：仅限当前页，且属于同一视觉渲染链。

## 6. 渲染队列策略 (Render queue policy)

优先级顺序：

| 优先级 | 任务 (Job) | 规则 |
|---:|---|---|
| 100 | 当前页预览 | 翻页时立即运行 |
| 90 | 当前页矢量/基底位图 | 预览请求被准入后运行 |
| 80 | 当前页编辑器叠加层 | 绝不能被预取任务抢占 |
| 70 | 当前页高保真 detail 瓦片 | 基础视图可见后运行 |
| 30 | 顺方向下一页预览 | 仅在空闲时运行 |
| 20 | 顺方向下一页 displayList | 仅在空闲时运行，最多一个 |
| 10 | 逆方向页面预热 | 快速导航期间禁用 |

**最新优先规则 (Latest-wins rules)：**

- 新的 `pageTurnId` 会中止来自旧翻页的排队任务。
- 正在运行的当前页任务会接收协同取消检查。
- 正在运行的过期任务只有在已经处于最终的缓存写入阶段时才能继续。
- 当有新的当前页任务到达时，预取任务会被立即丢弃。

## 7. 后端页面资产服务 (Backend page asset service)

添加一个 Tauri 应用服务：

```rust
pub struct PageAssetService {
    in_flight: InFlightPageTaskRegistry,
    cache: PageAssetCache,
    cancellation: PageCancellationRegistry,
}
```

请求结构体：

```rust
pub struct PageAssetRequest {
    pub document_id: String,
    pub document_revision: u64,
    pub page_index: u16,
    pub page_turn_id: u64,
    pub asset_kind: PageAssetKind,
    pub priority: PagePriority,
    pub viewport: Option<PageViewport>,
}
```

### 执行中去重 (In-flight dedupe)

任务 Key 格式为：

```text
document_id + document_revision + page_index + asset_kind + viewport_bucket
```

如果相同的 Key 正在运行，调用者将等待相同的任务。日志必须显示 `dedupe wait`，而不是另一个 `START`。

### 协同取消 (Cooperative cancellation)

每个高成本阶段都会检查：

- 文档版本（document revision）是否仍为当前版本？
- 此 `pageTurnId` 是否仍是可见工作的最新 Token？
- 队列是否已将此任务标记为已取消？

取消操作在阶段 1 不需要立即中断每个 PDF 操作符。它至少必须防止过期结果被呈现，并防止过期任务进入更深的阶段。

## 8. DisplayList / 中间缓存 (DisplayList / intermediate cache)

引入一个可复用的页面 IR，灵感来自 MuPDF DisplayList：

```rust
pub struct PageDisplayList {
    pub document_id: String,
    pub document_revision: u64,
    pub page_index: u16,
    pub page_size: PageSize,
    pub vector_objects: Vec<VectorObject>,
    pub image_refs: Vec<ImageRef>,
    pub text_runs: Vec<TextRunRef>,
    pub source_order: Vec<ObjectRef>,
}
```

这将成为以下模块的共享数据源：

- 矢量绘制 (vector painting)
- 文本提取 (text extraction)
- 搜索 (search)
- 编辑器碰撞检测 (editor hit-test)
- 页面缩略图 (page thumbnails)
- 未来的页面图像导出 (future export page image)

缓存层级：

| 缓存名称 | Key 组成 | 失效策略 |
|---|---|---|
| `PreviewCache` | 文档版本 + 页码 + 预览缩放比例 | 文档版本变化 |
| `DisplayListCache` | 文档版本 + 页码 | 文档版本变化 |
| `PaintPlanCache` | 文档版本 + 页码 + 编辑版本 | 编辑版本变化 |
| `FrameBitmapCache` | 文档版本 + 页码 + 缩放区间 + 瓦片索引 | 缩放/瓦片逐出 |
| `TextPageCache` | 文档版本 + 页码 | 文档版本变化 |

## 9. Presenter 规则 (Presenter rules)

`PagePresenter` 是唯一允许更改可见 canvas 可见性或将像素复制到可见表面的模块。

**规则：**

1. 保留旧的可见表面，直到目标页的预览（preview）至少准备就绪。
2. 渲染开始时绝不清除 detail 图层。
3. 预览呈现和矢量呈现都需要验证最新的 `pageTurnId`。
4. 矢量呈现以原子方式替换预览。
5. 仅在匹配的基底位图（base bitmap）可见后，才叠加 detail 瓦片（detail tile）。
6. 编辑器叠加层（editor overlay）不能透过浏览器文本或 textarea 绘制。它遵循 nushell 单一渲染链规则。

图层模型：

```text
页面容器 (page container)
  main canvas        基础 preview/vector 绘制表面
  detail canvas      可选的、对应相同 pageTurnId 的高分辨率 detail 瓦片
  editor canvas      Rust 绘制的编辑器叠加层
  interaction layer  仅用于事件碰撞检测的交互层
```

## 10. 事件模型与诊断 (Event model and diagnostics)

使用 Nutrient 风格的命名空间事件：

```text
viewState.change
pageTurn.intent
pageTurn.cancel
renderQueue.enqueue
renderQueue.dropStale
asset.preview.start
asset.preview.ready
asset.vector.start
asset.vector.dedupeWait
asset.vector.cancel
present.preview.accept
present.vector.accept
present.rejectStale
prefetch.start
prefetch.drop
```

要求的日志字段：

```text
documentId, documentRevision, pageIndex, pageTurnId, assetKind,
priority, phase, elapsedMs, cacheHit, dedupe, accepted
```

修复后的日志示例如下：

```text
[pageTurn.intent] page=37 turn=42 direction=1
[renderQueue.dropStale] page=36 turn=41 reason=newer-intent
[asset.preview.ready] page=37 turn=42 elapsedMs=7 cacheHit=true
[present.preview.accept] page=37 turn=42
[asset.vector.start] page=37 turn=42 priority=90
[asset.vector.dedupeWait] page=37 turn=42 key=doc:rev:37:displayList
[present.vector.accept] page=37 turn=42 elapsedMs=211
[prefetch.start] page=38 priority=30 reason=idle-directional
```

应该消失的糟糕日志：

```text
START page=37
START page=35
START page=37
HIT (after lock wait) page=37
present page=35 while current page=37
```

## 11. 迁移计划 (Migration plan)

### Phase 0: 文档和插桩 (Document and instrumentation)

- 将此架构方案添加到必备的渲染阅读列表中。
- 首先将 `pageTurnId` 添加到 TS 诊断日志中，不改变行为。
- 添加事件名称和日志字段。
- 增加一个调试面板或控制台过滤规则，专门过滤 `pageTurn`, `asset`, `present`。

**准入标准：**
- 快速的 next/prev 翻页序列可以通过 `pageTurnId` 进行端到端追溯。
- 能够证明是哪个任务提交了具体的帧。

### Phase 1: 准入和过期呈现防御 (Admission and stale present guard)

- 引入 `PageTurnCoordinator`。
- 将 `prevPage`, `nextPage`, 页码输入, AI 应用的页面切换全部路由通过它。
- 在 `commitVectorRenderResult` 和栅格 fallback 呈现中，加入最新 Token 准入校验。
- 保留旧表面直到预览/矢量渲染被接受。
- 丢弃队列中过期的导航渲染任务。

**准入标准：**
- 过期的页面像素绝不能呈现。
- 保留旧的可见页面，而不是显示白屏过渡。
- 绝不在 `index.ts` 或庞大的单体入口中直接添加新的渲染代码。

### Phase 2: 预览优先的翻页 (Preview-first page turn)

- 使预览图（preview）成为翻页时首个被准入的资产。
- 预览图准备就绪后立即呈现。
- 在预览图准入之后才启动矢量渲染，而不是之前。
- 快速导航模式：在用户翻页停顿 120-200ms 之前，仅请求/呈现预览图。

**准入标准：**
- 页码指示器在事件 Tick 内立即更改。
- 目标页面的首帧预览图在矢量模型计算完成之前到达。
- 矢量渲染的缓慢不再导致可见的白屏过渡。

### Phase 3: 队列优先级和预取控制 (Queue priority and prefetch control)

- 实现 `PageRenderQueue`。
- 只有在当前页达到 `PreviewVisible` 或 `VectorVisible` 状态后，才开始预取。
- 最大并发预取任务数限制为 1 个。
- 仅做顺方向预取；快速翻页时，禁用逆方向页面预热。

**准入标准：**
- 当当前页面还未渲染完毕时，日志中不再出现 page-1 和 page+1 的高成本解析。
- 当前页面的高优先级任务能够抢占预取任务。

### Phase 4: 后端执行中去重 (Backend in-flight dedupe)

- 引入 `PageAssetService`。
- 添加任务注册表，以文档版本、页码、资产种类、视口分桶作为联合 Key。
- 重构 `resolve_vector_page_model` 接口以使用此注册表。
- 添加 `dedupeWait` 诊断日志。

**准入标准：**
- 同一页面资产在单次修订版中不会多次打出 `START` 日志。
- 重复的调用者会共享同一个正在执行的结果。

### Phase 5: DisplayList 缓存 (DisplayList cache)

- 引入 `DisplayListCache`。
- 将昂贵的 PDF 解释过程与具体的 paint-plan 生成剥离开来。
- 将矢量渲染、搜索、编辑器碰撞几何尽可能路由通过同一个已解析页面 IR（中间表示）。

**准入标准：**
- 重复访问某页可以免于执行完整的 `Pure Vector Extraction`（纯矢量提取）。
- 搜索或编辑器的几何检测不再触发独立的 PDF 内容解析。

### Phase 6: ViewState 和 EventBus 收敛 (ViewState and EventBus convergence)

- 将 `PresentationViewState` 提升为 WASM 维护的状态。
- 在可行的地方，增加 `getState` / `setState(updater)` 风格的原子更新。
- 每次翻页事务仅触发一次 `viewState.change` 事件。
- 除非多实例（multi-instance）成为产品目标，否则保留单文档 thread_local 限制。

**准入标准：**
- 页码、缩放、渲染阶段以单个事务原子更新。
- 订阅者通过 EventBus 订阅，而不是使用散落的单个回调槽。

## 12. 验证计划 (Verification plan)

### 单元测试 (Unit tests)
- 队列的最新优先（latest-wins）任务替换逻辑。
- 当前页预览未就绪时，不能运行预取任务。
- 过期的呈现请求被拒绝。
- 执行中任务注册表能正确合并重复的页面资产请求。
- DisplayList 缓存可以随着文档版本的变化正确失效。

### 集成测试 (Integration tests)
- 模拟在 100ms 内快速翻页 next-next-next。
- 断言只有最后一页能被呈现。
- 断言预览图呈现在矢量渲染接受之前发生。
- 断言同一页面资产不会启动重复的矢量提取任务。

### 视觉测试 (Visual tests)
- 翻页后的像素采样不能是全白的（除非 PDF 页面本身就是白色的）。
- 过渡期间旧页保持可见，不出现中间白屏帧。
- 最终呈现的预览/矢量图必须与当前页码指示器一致。

### 性能目标 (Performance targets)

| 指标 | 目标值 |
|---|---:|
| 页码指示器更新延迟 | < 16ms |
| 预览呈现（若已缓存） | < 50ms |
| 预览呈现（未缓存但可用） | < 120ms |
| 过期呈现拒绝率 | 100% |
| 单个页面资产的重复矢量提取次数 | 0 |
| 当前页任务抢占预取任务成功率 | 100% |

## 13. 日志与可追溯性计划 (Logging and traceability plan)

### 参考模型 (Reference model)
- 借鉴 Spring Boot 日志，保持一致的日志格式，并在启用追踪时支持关联 ID（correlation ID）。
- 类似 OpenTelemetry，将请求建模为包含多个带属性 span 的 trace。
- 类似 Rust `tracing`，使用 span 和结构化事件，而不是散落在各处的 free-form print 语句。

### 项目规则 (Project rule)
- 核心工作流必须使用框架 span，禁止使用零散的 `console.log` 或 `log_step!`。
- 高价值工作流必须打印 `*.begin` 和 `*.end`，并附带 `result` 和 `elapsedMs`。
- 提前返回或过期拒绝也必须生成终端结束事件（terminal event）。
- 默认的 INFO 级别日志只包含生命周期、呈现拒绝、去重等待和工作流结束事件。
- DEBUG 级别日志可包含准入通过和详细的缓存决策。
- TRACE/AUDIT 级别日志包含低级别的 PDF 算子/图像/字体诊断。

### 当前实现 (Current implementation)
- `log_pdf_event(level, event, fields)` 负责输出结构一致的键值对事件。
- `PdfEventSpan` 是 Rust 中的一个拦截器/切面：负责打出 `event.begin`、`event.end`、`elapsedMs` 以及在提前丢弃时输出 `result=aborted`。
- `read_page_asset_bundle` 已使用 `PdfEventSpan`，因此页面资产工作流是可追溯的，无需在业务逻辑中手写成对的日志。
- 文档生命周期命令（`document.open`, `document.save`, `document.undo`, `document.redo`, `document.clearCache`）在接口边界处均使用了 `PdfEventSpan`。
- `PageAssetAdmissionService` 会打出结构化的准入事件：
  - `pageAsset.dedupeWait.begin`
  - `pageAsset.dedupeWait.end`
  - `pageAsset.prefetchRejected`
  - `pageAsset.currentSuperseded`
  - 仅在 DEBUG 级别打印 `pageAsset.admit`
- 后端 `log_pdf_event` 会保留最近 512 条结构化 PDF 事件；测试可通过 `clear_pdf_event_log` / `read_pdf_event_log` 读取，不再依赖解析 WDIO stdout。
- 后端 page asset 命令支持测试延迟钩子 `set_page_asset_test_delay_ms`，用于在 E2E 中稳定制造慢 vector/page asset 场景；默认值为 0，且实际延迟只在 debug build 生效，不影响 release 路径。
- 前端高频的诊断性 `console.log` 调用已尽可能移至 `logPdfLayoutTrace` / 详细诊断。
- 嘈杂的 open/save 内部日志在生命周期 span 已经覆盖 INFO 事件的前提下，已降级为 DEBUG 级别。
- 低级别的 `resolve_paths`、矢量提取和布局推理日志在页面资产 span 已覆盖工作流的前提下，均已被移至 DEBUG/TRACE 级别。

### 规范的页面资产链 (Canonical page asset chain)

```text
pageTurn.intent (翻页意图)
  -> pageAsset.bundle.begin role=current page=37 revision=2 (启动当前资产加载)
  -> pageAsset.dedupeWait.begin/end (去重等待开始/结束，仅在发生重复等待时)
  -> pageAsset.prefetchRejected 或 pageAsset.currentSuperseded (过期路径上的拦截)
  -> pageAsset.bundle.end result=accepted elapsedMs=... (资产加载结束，被接受)
  -> preview-first-frame.presented (首帧预览图呈现)
  -> render-loop.present.after (渲染循环完成)
  -> pageTurn.visible (新页完全可见)
```

### 剩余日志清理 (Remaining logging cleanup)
- 添加快速翻页的本地日志黄金测试（golden test）：确保所需事件存在，吵闹的冗余事件缺失。

## 14. 实现状态 (Implementation status)

**更新时间：** 2026-06-07

### 已实现 (Implemented)：
- WASM 中的 `PagePresentationRuntime` 现在已拥有翻页意图、最新页面准入、可见表面标记、当前/预取资产准入以及相邻预取决策的控制权。
- TS 的 `prefetchAdjacentPages` 不再计算 `currentPage +/- 1`；它直接消费 Rust 的 `decideAdjacentPrefetch`，只启动已被 Runtime 准入的目标页。
- 矢量 Bundle 加载在 IPC 之前、IPC 之后以及图像缓存水合（hydration）之前，都会询问 Rust 准入状态。
- 导航/默认渲染可以首先呈现被准入的预览首帧，然后继续在后台离屏进行矢量渲染。
- Tauri 渲染命令现在接收 `requestRole` 并将当前/预取的准入工作委派给 `PageAssetAdmissionService`。
- 后端预取（prefetch）不再覆写 `active_pages`；只有当前可见页面的资产才会更新活动页面状态。
- 后端页面资产现在具有基于 文档/页面/资产类型 的执行中锁（in-flight locks），因此重复的同页 `read_vector` / `read_glyph_plan` 请求会先等待，然后复用已有缓存。
- Tauri 暴露了 `read_page_asset_bundle` 接口，使矢量模型和字形绘制计划（glyph paint plan）能在一个被准入且被锁保护的后台工作流后一并解析。
- TS 中的矢量 Bundle 加载现在直接调用 `read_page_asset_bundle`，不再并行启动两个独立的 `read_vector` 和 `read_glyph_plan` IPC 调用。
- 后端执行中锁的获取现在会打印 `dedupeWait.begin` / `dedupeWait.end` 诊断日志，包含 role、资产类型、页码和已耗时毫秒数。
- 页面资产锁的 Key 包含了 WASM 视图会话的 `documentRevision`，该参数通过 TS IPC 传入 Tauri。
- Tauri 矢量模型和布局推理缓存现在使用版本感知（revision-aware）的 Key 来响应页面资产请求。
- WASM `PageRenderQueue` 已开始接管渲染队列策略；TS scheduler 现在通过 `resolveRenderQueueAction` 获取 suppress / dispatch / replace pending navigation / replace non-navigation 决策，以及 `pendingQueueEffect` 队列清理策略。TS 仅保留 timer、RAF、数组存储和 Promise adapter。
- 执行中的导航渲染采用 latest-wins：WASM `PageRenderQueue` 返回 `replacePendingNavigation`，TS 释放被替换的 Promise，只保留最新 pending navigation。
- scroll debounce window 已由 WASM `PageRenderQueue` 作为 `scrollDebounceMs` 返回，TS 只负责用该值挂载浏览器 timer/RAF。
- 导航渲染完成后会再次校验 `pageTurnId`；过期请求打出 `page-turn.stale-render-skipped`，不再被记录为 accepted visible-ready。
- 后端页面资产锁已有聚焦单测覆盖：相同文档/页/revision/kind 会等待既有执行中工作，不同 revision 不互相阻塞，`invalidate_pdf_page_cache` 会清理对应文档的 asset locks 且保留其他文档的 locks。
- 后端已引入第一层 `PageDisplayList` / `PdfPageIntermediateService`：页面 bundle 工作流会先按 document revision 解析并缓存页面中间表示，缓存命中/未命中通过 `pageIntermediate.displayListCache` 事件可观测，文档失效路径会同步清理该中间缓存。
- `vector_engine` 已将 PDF content stream 解析与输出派生拆开：`NativeVectorPageModel` 和 `LayoutInferenceResult` 都可以从同一份 `PageDisplayList` 构建；`read_page_asset_bundle`、`read_vector`、`read_glyph_plan` 已路由到 `PdfPageIntermediateService`，并会回填既有 vector/layout cache。
- 搜索和 annotation 目标解析已接入页面中间服务：`find_in_page` / `find_in_document` 以及高亮、评论、annotation target 的区域定位现在复用 `PdfPageIntermediateService`，不再从接口/应用层直接调用旧 page model service。
- 旧基础设施包装已委派到中间服务：`PdfPageModelService` / `PdfReadService` / `PdfEditorGeometryService` 的 vector/layout/glyph 入口不再绕开 `PageDisplayList`，`diagnose_page` 的对象/文本统计也复用 `PdfPageIntermediateService`。除底层 `pdf_read::resolve_paths` 定义和 `vector_engine::resolve_page_display_list_with_doc` 外，后端已无直接 `resolve_paths` 调用。

### 收口状态 (Closure status)：
- 结构化诊断链已覆盖 `pageAsset.prefetchRejected`、`pageAsset.currentSuperseded`、`page-turn.stale-render-skipped`、`pageAsset.dedupeWait.begin/end` 与 current bundle begin/end；页面呈现 E2E 已直接读取后端 event ring buffer 做断言。
- `PageRenderQueue` 已拥有 commit suppression、执行中导航 latest-wins、非导航替换、scroll debounce window 与 pending queue effect 的策略判断；TS 保留 timer、RAF、数组存储和 Promise adapter 这些浏览器/异步粘合职责。
- 快速导航、慢 page asset、重复资产请求和真实多页 PDF 主路径均已有自动化覆盖；如需继续加固，可以另开任务补更大的真实 PDF fixture 或手工性能 trace。
- DisplayList 风格页面中间表示已完成当前可行范围：page bundle、vector model、glyph/layout plan、搜索、annotation 目标定位、诊断统计和旧基础设施包装都复用 `PdfPageIntermediateService`。preview 首帧路径主要读取 XObject 图片，不走 `resolve_paths`，保持独立；editor hit-test 位于 `pdf-viewer-core` / WASM 几何路径；当前未发现独立 thumbnail 后端解析绕路。

### 恢复快照（2026-06-06）

- 已验证：`cargo check`、`cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown`、`npm run wasm:pdf-viewer-ui`、`npm run build` 均通过。
- 已补充 `crates/pdf-viewer-ui/src/presentation/page_turn.rs` 的聚焦 Rust 单测，覆盖 latest page-turn token、stale visible 拒绝、方向性预取、current/prefetch asset admission。
- 已补充 `tests/e2e/fixtures/multipage.pdf`，并让 `tests/e2e/specs/page_presentation_runtime.spec.ts` 使用真实 4 页 fixture 跑 next-next-next-prev 快速导航。
- 已验证 `npx tsc --noEmit -p tests/e2e/tsconfig.json` 通过。
- 已完成 `PageRenderQueue` 第一段接线：`src/bridge/render/render_scheduler.ts` 不再本地决定 commit suppression 和队列替换策略，而是委派给 WASM runtime；`npm run build` 已再次通过。
- 已修复本机 E2E driver 端口配置：Windows excluded TCP range 覆盖了 4444/4445，后续 4723 也出现 tauri-driver bind 失败；WDIO/tauri-driver 改用 5210/5211 后，单独运行 `npm run e2e -- --spec tests/e2e/specs/page_presentation_runtime.spec.ts` 通过。
- 已将快速翻页 E2E 调整为同步连发导航请求，覆盖 pending navigation latest-wins：旧运行中请求必须产生 `page-turn.stale-render-skipped`，中间 pageTurnId 不能 `visible-ready`，最新 pageTurnId 必须可见。
- 已补充 `src-tauri/src/application/pdf/page_asset.rs` 后端单测，覆盖 page asset in-flight lock 的同 key 等待、revision 隔离以及页面缓存失效时的锁清理；`cargo test --manifest-path src-tauri\Cargo.toml page_asset --lib` 通过。
- 已将 scroll debounce window 从 TS 常量迁入 WASM `RenderQueueAction.scrollDebounceMs`；已验证 `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown`、`npx tsc --noEmit -p tests/e2e/tsconfig.json`、`npm run wasm:pdf-viewer-ui`、`npm run build`、页面呈现 E2E 均通过。
- 已将 pending queue 清理策略从 TS action-name 推断迁入 WASM `RenderQueueAction.pendingQueueEffect`；TS 仍保存 pending request/Promise，但 queue effect 的语义由 WASM 返回。已再次验证 `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown`、`npx tsc --noEmit -p tests/e2e/tsconfig.json`、`npm run wasm:pdf-viewer-ui`、`npm run build`、页面呈现 E2E 均通过。
- 已增加后端 PDF 事件 ring buffer，并暴露 `clear_pdf_event_log` / `read_pdf_event_log`；页面呈现 E2E 现在会断言后端 `pageAsset.bundle.begin/end role=current page=2 result=accepted`，已验证 `cargo check --manifest-path src-tauri\Cargo.toml`、`cargo build --manifest-path src-tauri\Cargo.toml`、`cargo test --manifest-path src-tauri\Cargo.toml page_asset --lib`、E2E TS 编译和页面呈现 E2E 均通过。
- 已增强 `src-tauri/src/application/pdf/page_asset.rs` 后端单测：重复同 key asset 请求在等待既有 in-flight lock 时必须记录 `pageAsset.dedupeWait.begin`，释放后必须记录 `pageAsset.dedupeWait.end` 和 `elapsedMs`；`cargo test --manifest-path src-tauri\Cargo.toml page_asset --lib` 通过。
- 已增加慢路径页面呈现 E2E：通过 debug-only 的 `set_page_asset_test_delay_ms=250` 稳定延迟 asset bundle，快速 `next-next` 后旧目标页必须产生 `page-turn.stale-render-skipped`，最新目标页必须 `visible-ready`；页面呈现 E2E 两条用例均通过。
- 注意：普通 native `cargo test -p pdf-viewer-ui` 当前不是有效验证命令，因为该 crate 包含 WASM-only 的 `web_sys` / `js_sys` / async wasm-bindgen surface，native test target 下会缺失这些依赖。该层应以 WASM target check、wasm-pack build 和浏览器/E2E 验证为准。
- 诊断状态：后端已经发出 `pageAsset.prefetchRejected` 和 `pageAsset.currentSuperseded`；TS 已记录 prefetch reject；慢 page asset 与快速导航 E2E 已确认 stale/admission 终态事件可观测。

### 最终收口状态

1. P0：诊断链与快速翻页验收已完成自动化覆盖。
   - 覆盖：同步 next-next-next-prev 快速导航、慢 page asset stale 路径、后端 current bundle begin/end、旧 pageTurnId 不进入 visible-ready。
   - 后续可选：针对真实大 PDF 做手工性能 trace，不再阻塞本轮重构完成。
2. P1：`src/bridge/render/render_scheduler.ts` 中的队列策略已迁入 WASM `PageRenderQueue`。
   - 已完成：commit suppression、执行中导航 latest-wins 替换、非导航替换、scroll debounce window、pending queue effect 这些策略判断已由 WASM 返回。
   - 当前 TS 保留 scroll timer/RAF、executing flag、pending queue 存储与 Promise resolve，属于浏览器运行时和 Promise adapter 职责。
   - 后续可选：如果需要更细的可观测性，可以让 WASM 返回 pending queue 快照和 queue flush 诊断事件。
3. P1：快速翻页和重复 asset 请求的集成/E2E 覆盖已完成。
   - 覆盖 `pageTurnId`、`pageAsset.dedupeWait.begin/end`、`pageAsset.prefetchRejected`、`pageAsset.currentSuperseded`。
   - 已完成：多页 fixture 与快速翻页 E2E 主路径，当前断言 next-next-next-prev 最终停在最新目标页，并确认旧导航不会 visible-ready；E2E 已能直接读取后端 page asset 事件并断言当前页 bundle begin/end；慢 page asset 场景已通过测试延迟钩子覆盖 stale/admission 终态。
   - 后续可选：如需继续加固，可增加更接近真实大 PDF 的慢 fixture。
4. P2：`DisplayListCache` / 页面中间表示已完成。
   - 已完成：后端新增 revision-aware `PageDisplayList` 缓存和 `PdfPageIntermediateService::resolve_page_display_list`，page bundle 工作流已先经过该中间缓存；缓存失效已有后端聚焦测试覆盖。
   - 已完成：vector model 与 layout inference 的构建逻辑已拆成“从 `PageDisplayList` 派生输出”；`read_vector` / `read_glyph_plan` 已接入该中间服务路径。
   - 已完成：搜索和 annotation target/highlight/comment 区域定位已从旧 page model service 迁到 `PdfPageIntermediateService`。
   - 已完成：诊断命令和旧 `PdfPageModelService` / `PdfReadService` / `PdfEditorGeometryService` 包装已委派到 `PdfPageIntermediateService`；直接 `resolve_paths` 调用已收敛到 `vector_engine` 的 display-list 构建入口。
   - 已完成：补充 display-list 派生缓存聚焦测试，验证 seeded `PageDisplayList` 可派生 vector/layout，并回填既有 vector/layout cache。
   - 已完成：补充 search/annotation 聚焦覆盖，验证 annotation targets 和 search 可消费由 seeded `PageDisplayList` 派生出的 page model。
   - 已完成：补充真实 PDF fixture E2E，`multipage.pdf` 通过 `find_in_page` 命中 `Page 2`，`read_annotation_targets` 也能从同页派生 annotation target。
   - 已完成：确认 editor hit-test 当前位于 `pdf-viewer-core` / WASM 几何路径，不存在 Tauri 后端 `resolve_paths` 绕路；thumbnail 当前未发现独立后端解析路径，preview 继续作为 XObject 快速首帧独立保留。
   - 已完成：清理后端 warning backlog，`cargo check --manifest-path src-tauri\Cargo.toml` 和聚焦后端测试当前均无 warning。
   - 验收：vector render、search、editor hit-test、thumbnails 在可行处共享同一份 parsed page IR。

### 最终验收快照（2026-06-07）

- `cargo fmt --manifest-path src-tauri\Cargo.toml` 通过。
- `cargo check --manifest-path src-tauri\Cargo.toml` 通过，当前无 warning。
- `cargo test --manifest-path src-tauri\Cargo.toml page_intermediate --lib` 通过，当前无 warning。
- `cargo test --manifest-path src-tauri\Cargo.toml page_asset --lib` 通过，当前无 warning。
- `cargo build --manifest-path src-tauri\Cargo.toml` 通过，当前无 warning。
- `npx tsc --noEmit -p tests\e2e\tsconfig.json` 通过。
- `npm run e2e:build` 通过；普通 sandbox 下 Vite/esbuild 可能因 `spawn EPERM` 失败，需要在本机允许执行。
- `npm run e2e -- --spec tests/e2e/specs/page_presentation_runtime.spec.ts` 通过。

### §15 P0/P1 快速翻页性能收敛落地快照（2026-06-07）

- **fast-flip 状态追踪**已实现：`page_turn.rs` 的 `PageTurnSnapshot` 新增 `last_turn_at_ms: f64` 和 `fast_flip_mode: bool`；`request_page_turn` 接受 `now_ms: f64`（由 TS 传入 `performance.now()`），检测两次翻页间隔 < 100ms 时自动进入 fast-flip 模式。
- **fast-flip 下预取策略动态调整**：`decide_adjacent_prefetch` 在 fast-flip 模式时将 vector prefetch window 降为 0（暂停），逆方向 preview runway 降为 1 页；normal 模式维持 vector=2、reverse preview=2 不变；顺方向 preview runway 在两种模式下均保持 8 页。
- **WASM → TS 全链路已接线**：`presentation_api.rs` 增加 `now_ms` 参数 → `page_presentation_runtime.ts` 增加 `nowMs?` 透传 → `pdf_viewer_api.ts` `prevPage`/`nextPage` 传入 `performance.now()` → `bridge/index.ts` 同步更新。
- **`PagePresenter` ready-only commit** 已实现：新增 `commitReadySurfaceOrFallback`，preview-first 路径改用该函数；cache hit 时 <1ms 内直接提交，cache miss 时打 `current-raster-miss-ready-only-fallback` 性能违规日志并返回 false，不再等待 40-50ms decode 阻塞当前可见路径。
- **Rust 单测已覆盖**：`fast_flip_mode_activates_when_turns_are_rapid`、`fast_flip_pauses_vector_prefetch_and_reduces_reverse_preview`、`normal_mode_includes_vector_prefetch` 三个新测试；既有 4 个测试均已更新 `now_ms` 参数。
- **验收命令**：
  - `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown` 通过（仅预存 deprecated warning）。
  - `npx tsc --noEmit -p tests\e2e\tsconfig.json` 通过。
  - `npm run wasm:pdf-viewer-ui` 通过。
  - `npm run build` 通过。
  - `npm run e2e -- --spec tests/e2e/specs/page_presentation_runtime.spec.ts` 通过（3 passed）。

## 15. 高速翻页性能收敛方案 (Fast page-turn performance plan)

本节是对前文 `PagePresentationRuntime` / `PageRenderQueue` / `PageAssetPipeline` / `PagePresenter` 架构的继续收敛，不引入新的并行架构。目标是把“当前页可见路径”从解析、解码和重计算中隔离出来，让当前页只提交已经准备好的 `ready surface`。

### 15.1 性能目标与口径

| 指标 | 可接受 | 优秀 | 工程目标 |
|---|---:|---:|---:|
| 自动连续翻页 press -> visible-ready | <= 40ms | <= 20ms | <= 10ms |
| ready surface commit | <= 10ms | <= 5ms | <= 3ms |
| current cache miss 后首帧 | <= 40ms | <= 20ms | <= 10ms 低清首帧 |
| 高清 raster/vector 补帧 | 不阻塞首帧 | 后台完成 | 后台完成 |

诊断必须拆开以下时间：

```text
page-turn.bench-press
  -> pageAsset.admit current
  -> surface.cache-hit / surface.cache-miss
  -> raster.decode-start/end 或 vector.render-start/end
  -> present.commit
  -> page-turn.visible-ready
```

其中 `page-turn.visible-ready.elapsedMs` 代表单次翻页内部耗时；连续两个 `visible-ready` 的差值代表自动连续翻页体感间隔；`bench-press -> visible-ready` 才是压测下最接近用户按键到可见的指标。

### 15.2 Ready surface 定义

`ready surface` 指已经满足“可直接提交到可见层”的页面画面。它可以是低清预览、已解码 raster、已完成的 vector frame 或已绘制 canvas，但不能是仍需 PDF 解析、图片 decode、IPC 等待或高成本计算的原始资产。

| 资产 | 是否 ready surface | 允许在 current visible path 中等待 |
|---|---|---|
| PDF content stream | 否 | 否 |
| JPEG/XObject bytes | 否 | 否 |
| `PageDisplayList` | 否，属于 IR | 否 |
| 未 decode 的 image URL | 否 | 否 |
| decoded `HTMLImageElement` / `ImageBitmap` | 是 | 是，仅提交 |
| 已绘制 preview canvas / raster img | 是 | 是，仅提交 |
| 已完成 vector frame | 是 | 是，仅提交 |

硬约束：

```text
PagePresenter 只能提交 ready surface。
PagePresenter 不允许触发 current raster decode、PDF parse 或 IPC。
```

过渡期允许 current miss 继续 fallback，但必须打印性能违规事件，直到所有 current miss 都被低清首帧或预热命中消除。

### 15.3 Current miss 处理规则

高速翻页时，如果高清 raster 未命中，不应阻塞等待高清 decode：

```text
pageTurn.intent
  -> current high-res raster miss
  -> try ready preview / low-res surface
      -> hit: commit low-res surface, visible-ready
      -> miss: retain old surface or show page frame, log missing-ready-surface
  -> background decode high-res raster
  -> latest-wins admission
  -> if still current, swap high-res surface
```

首帧策略：

| 场景 | 当前页显示策略 | 后台任务 |
|---|---|---|
| ready preview hit | 立即显示 preview | decode high-res / render vector |
| decoded raster hit | 立即显示 raster | render vector/detail |
| vector frame hit | 立即显示 vector | render detail |
| 所有 ready surface miss | 保留旧 surface 或显示轻量 page frame | 优先生成 preview，再高清 |

### 15.4 Fast-flip 模式

当连续翻页 press 间隔进入 100ms 内，进入 fast-flip mode。该模式仍由 `PageRenderQueue` / `PagePrefetchController` 统一决策，不允许 TS 私自决定预取策略。

| 模式 | 触发 | 策略 |
|---|---|---|
| normal | 普通翻页 | 预取当前页前后 1-2 页 |
| fast-flip | 连续 press 间隔 < 100ms | 顺方向 preview runway 预取 8 页，逆方向最多 1-2 页 |
| jump | 页码跳转、搜索跳转 | 只准备目标页 + 近邻 preview |
| settle | 停止翻页 100-200ms | 补 vector、detail、editor overlay |

fast-flip 下任务优先级：

| 优先级 | 任务 | 规则 |
|---:|---|---|
| 100 | 最新 current ready preview/raster commit | 只能提交 ready surface |
| 90 | 最新 current preview 生成 | 如果没有 ready surface，优先生成低清首帧 |
| 80 | 顺方向 preview runway | 最多 8 页，低优先级后台 |
| 60 | 最新 current vector/baseBitmap | preview 可见后运行 |
| 30 | 近邻 vector/displayList | 空闲时运行，最多 1-2 页 |
| 10 | detail/editor overlay | settle 后运行 |

### 15.5 资源池化边界

| 资源 | 是否池化 | 池化方式 | 注意事项 |
|---|---|---|---|
| Stage canvas / offscreen canvas | 是 | `CanvasPool` 按尺寸复用 | 回收时清 transform 和像素；避免无限持有 GPU buffer |
| Decoded raster image | 是 | LRU，按文档版本 + 页码 + zoom bucket | 当前实现按 src LRU；后续应加入内存预算和 page key |
| `ImageBitmap` | 是 | LRU + 显式 `close()` | 比 `HTMLImageElement` 更适合 ready surface，但要控制 GPU 内存 |
| `PageDisplayList` | 是 | revision-aware cache | 文档版本变化必须失效 |
| Vector bundle / paint plan | 是 | page bundle cache + revision key | 编辑版本变化必须失效 |
| Preview metadata | 是 | page preview cache | 低清首帧的 runway 应优先复用它 |
| In-flight page asset lock | 是 | document + revision + page + kind | 防止重复 decode/parse |
| DOM 可见节点 | 少量固定复用 | main/back/stage canvas 固定节点 | 不要频繁 create/remove |

不应池化：

| 资源 | 原因 |
|---|---|
| 带旧文档引用的编辑 overlay 状态 | 易跨文档污染 |
| 活跃 textarea / selection 状态 | 浏览器 selection 与 IME 状态不可安全复用 |
| 已失效 document revision 的任何 surface | 会显示旧版本页面 |
| 大量隐藏高清 canvas | GPU/内存占用高，容易拖慢主线程和合成 |

### 15.6 线程与任务隔离

原则：会占用 10ms 以上、可能阻塞输入或定时器的任务，不得与 UI 提交路径共用主线程。

| 任务 | 推荐执行位置 | 是否可共享线程 | 原因 |
|---|---|---|---|
| UI 输入、键盘、PagePresenter commit | 浏览器主线程 | 不共享高成本任务 | 必须低延迟 |
| `HTMLImageElement.decode()` / `createImageBitmap` | 浏览器解码线程/worker 能力 | 与 UI 分离 | 日志显示 current decode 可达 40-50ms |
| Preview runway 预热 | 独立低优先级队列 | 可共享 preview worker 限流 | 不应抢 current commit |
| Vector render / Vello / canvas draw | stage canvas 或 dedicated worker（可行时） | 不与 preview decode 无限并发 | GPU/CPU 资源竞争会拖慢可见提交 |
| PDF content stream -> `PageDisplayList` | Tauri `spawn_blocking` 专用池 | 不与轻量 IPC/状态锁共享 | 解析可能长时间占 CPU |
| Search / annotation target | 后台 CPU 池 | 可共享 DisplayList 派生池 | 依赖 IR，但不应阻塞翻页 |
| Save/write-back | 独立串行队列 | 不与渲染/preview 共用 | IO + PDF 写回不应影响阅读 |
| OCR / AI / 大文本分析 | 独立 worker/进程 | 不共享渲染池 | 高 CPU/内存，必须隔离 |

需要隔离的冲突：

- preview decode 与 current commit 冲突：decode 可能拖慢 JS timer 和 visible-ready，应后台限流。
- vector render 与 preview decode 冲突：二者都可能占 CPU/GPU，fast-flip 下 preview runway 优先，vector 延后。
- PDF parse 与 search/annotation 冲突：都依赖页面 IR，应先共享 `DisplayListCache`，避免重复解析。
- save/write-back 与 page asset read 冲突：写回期间应冻结或提升 document revision，旧 surface 不得继续标记 current。

### 15.7 分阶段落地

| 阶段 | 内容 | 验收 |
|---|---|---|
| P0 | current raster miss 打性能违规日志；bench 日志统计 press -> visible-ready；backend preview 预取窗口和 WASM preview runway 分离 | 能定位所有 current miss |
| P1 | fast-flip 下顺方向 preview runway 预取 8 页；vector 仍只预取近邻 1-2 页 | 16ms 自动翻页时 current miss 明显下降 |
| P1 | `PagePresenter` 支持 ready-only commit：current miss 不等待 decode，先 fallback ready preview | current visible path 不再出现 40-50ms decode |
| P2 | `ImageBitmapSurfaceCache` 替代部分 `HTMLImageElement` ready surface | commit 降到 1-3ms 区间 |
| P2 | worker/offscreen canvas 隔离 preview decode 与 vector draw | 自动 press 间隔接近设定 interval |
| P3 | 内存预算驱动的 SurfaceCache：按文档版本、页、zoom bucket、surface kind 逐出 | 大 PDF 连续翻页无内存抖动 |

## 16. 有意推迟的事项 (What remains intentionally deferred)

- **真正的多文档多实例支持**：当前文档设计选择单文档 thread_local 模式。在修复翻页问题时，不要承担此项重构。
- **PDF 算子级别的硬性取消（Hard cancellation）**：阶段 1 仅需要协同阶段取消和过期呈现拒绝。
- **重写渲染器后端**：核心问题是准入控制、优先级、缓存和 Present 的所有权，而不是渲染后端本身。
- **替换 TS DOM 外壳**：TS 应当保持仅作为 DOM/Canvas 粘合层；Rust/WASM 拥有渲染决策。
- **更改编辑器文本渲染**：nushell 的单一渲染链规则保持完好。

## 17. 设计模式所有权 (Design-pattern ownership)

| 关注点 | 使用设计模式 | 所有权归属 |
|---|---|---|
| 翻页意图与状态转换 | State + Command | `PageTurnCoordinator`, `PresentationViewState` |
| 队列准入与抢占 | Scheduler / Priority Queue | `PageRenderQueue` |
| 预览/矢量/Detail 加载 | Pipeline / Adapter | `PageAssetPipeline`, `PageAssetService` |
| 解析后页面的复用 | Cache + Intermediate Representation | `DisplayListCache` |
| 画布后端隔离 | Bridge | `PagePresenter` -> canvas host |
| 事件分发 | EventBus | `PresentationEventBus` |
| 编辑渲染视觉正确性 | 单一渲染链 (Single rendering chain) | Rust canvas painter |

## 18. 架构红线/准则 (Architectural guardrails)

- 绝不能在不提取 `PagePresentationRuntime` 的情况下，直接向 `pdf_runtime.ts` 或 `render_flow.ts` 添加更多导航规则。
- 绝不能在 `PagePresenter` 之外清除或隐藏可见的 canvas 图层。
- 绝不能让预取任务（prefetch）调用与当前页渲染相同的高优先级路径。
- 绝不能让后端结果在没有通过 `pageTurnId` 校验的情况下直接呈现。
- 绝不能将浏览器文本渲染放回到可见的编辑器路径中。
- 绝不能让 TS 去凭空设计那些本应由 Rust/WASM 渲染规划拥有的视觉规则。
- 绝不能在 TS 中保留非必要的页面/渲染/编辑器决策。如果逻辑可以用不直接依赖 DOM/浏览器 API 的方式表达，就必须归 Rust/WASM 所有。
- 在修改编辑器叠加层时，绝不能将列表标记语义（list marker semantics）混合到可编辑的主体文本中。

## 19. 阅读地图 (Reading map)

- `docs/nutrient-comparison.md`：了解 ViewState, EventBus, 脏检查（dirty tracking）, 单文档限制。
- `docs/nushell-divergence-report-2026-05-06.md`：单一渲染链与迁移回归问题。
- `docs/editor-render-architecture.md`：编辑器 canvas 叠加层规则。
- `docs/architecture-overview.md`：三层运行时边界。
- `docs/ts-to-rust-migration-plan.md`：TS 作为 DOM 外壳、Rust 作为决策所有者的设计。
- `docs/structure-flow-audit.md`：当前的隐式状态和跨域编排热点。
