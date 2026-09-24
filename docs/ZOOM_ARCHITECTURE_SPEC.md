# 缩放架构详细规格

> 状态：Draft / Implementation-ready
>
> 目标：以文档驱动、测试驱动的方式收敛缩放子系统，明确跨模块不变量、模块边界、方法职责、变量范围、日志格式和测试证据。
>
> 本文不是对已有实现的“全部正确”声明。凡标记为 **Required** 的内容，表示开发目标；凡标记为 **Observed** 的内容，表示当前代码已观察到的事实；凡标记为 **Gap** 的内容，表示需要通过实现或测试补齐。

---

## 1. 问题陈述

当前缩放链路已经完成了从 TS/CSS transform 主导到 Rust/WASM RAF + SetBox 主导的迁移，但以下跨模块边界仍未形成可验证的闭环：

1. `target_zoom`、`visual_zoom`、`last_rendered_zoom` 的所有权和收敛关系分散在 Core、UI、TS bridge。
2. `FrameToken` 在 Rust render store、Core scheduler、TS async render flow、vector/tile renderer 之间传递，但完整 stale-frame 生命周期缺少端到端测试。
3. `SetBox` 是架构语义，但 wheel 路径仍直接调用 DOM `style.set_property`，没有统一操作模型。
4. `immediateMutationFrame` 的名称暗示绕过 RAF，但实现仍进入 Rust committed-frame queue，语义需要统一。
5. ADR-0007 已决定当前缩放始终居中，但 anchor 字段、请求和兼容入口仍存在，未明确哪些是废弃字段、哪些是兼容数据。
6. 测试多数只验证返回值和布尔结果，缺少带 `frame_token`、三种 zoom、布局、surface 和 stale 原因的结构化日志。
7. 测试结果没有统一保存到文档，后续无法学习“哪个方法由什么不变量保护、运行结果是什么”。

---

## 2. 目标与非目标

### 2.1 目标

- 建立缩放子系统的单一领域模型和跨模块契约。
- 将每个缩放模块和公开方法纳入方法级测试矩阵。
- 每个关键测试同时验证外部行为和输出结构化日志。
- 将测试说明、执行命令、结果摘要、日志样例和失败记录保存到版本化文档。
- 统一 `FrameToken`、stale frame、settle、SetBox、surface ownership 的语义。
- 明确 anchor 遗留字段的迁移/删除策略。
- 以最少的跨层测试 seam 验证完整链路，而不是为每个内部函数复制大量集成测试。

### 2.2 非目标

- 本规格不重新设计 PDF 渲染器、tile 算法或编辑器数据模型。
- 本规格不把 SpecDB 的数据库性能数字移植为 PDF viewer 指标。
- 本规格不要求一次性删除所有 legacy facade 或 anchor 字段。
- 本规格不把日志输出变成生产环境无限量逐帧日志；生产采样策略另行决定。

---

## 3. 领域模型

### 3.1 缩放状态

`HostZoomState` 是 UI/WASM 侧缩放事实容器，包含：

| 变量 | 所有者 | 语义 | 可写者 | 生命周期 |
|---|---|---|---|---|
| `target_zoom` | `ZOOM_STATE` | 用户当前意图的目标缩放 | `set_target_zoom`、`set_target_zoom_instant`、授权入口 | wheel/programmatic 到 reset |
| `visual_zoom` | `ZOOM_STATE` | 当前帧实际呈现的视觉缩放 | `advance_zoom_animation_state`、instant 入口 | animation 到 settle |
| `last_rendered_zoom` | `ZOOM_STATE` | 最近被接受并提交的渲染缩放 | `mark_rendered_zoom` / commit 路径 | document session 生命周期 |
| `last_animation_timestamp_ms` | `ZOOM_STATE` | 动画插值时间基准 | RAF tick | animation 生命周期 |
| `visual_layout` | `ZOOM_STATE` | display zoom、content left/top 的布局快照 | `set_visual_layout` | 每次布局更新 |
| `preview_host.preview_active` | `ZOOM_STATE` | 是否处于 preview 阶段 | preview host 方法 | wheel 到 settle |
| `preview_host.wheel_render_pending` | `ZOOM_STATE` | wheel 后是否等待 render | preview host 方法 | 单次 wheel/render |
| `preview_host.pending_committed_frame` | `ZOOM_STATE` | 等待 RAF 应用的提交帧 | `queue_committed_frame` / `take_ready_committed_frame` | frame queue 生命周期 |
| `preview_host.cancel_pending_render` | `ZOOM_STATE` | 是否取消待处理渲染 | preview host 方法 | 单次取消请求 |
| `drawing_delay.active` | `ZOOM_STATE` | settle 后高清绘制延迟是否启用 | drawing-delay 方法 | settle cleanup |
| `drawing_delay.started_at_ms` | `ZOOM_STATE` | 延迟起始时间 | settle 入口 | 单次 settle |
| `drawing_delay.delay_ms` | `ZOOM_STATE` | 延迟预算 | 配置/状态入口 | 配置生命周期 |
| `VIEWER_SESSION.current_zoom` | viewer session | `target_zoom` 的派生投影，不是事实源 | projection path | session 生命周期 |

#### 状态不变量 Z-STATE-001

```text
target_zoom > 0 且有限
visual_zoom > 0 且有限
last_rendered_zoom > 0 且有限
visual_zoom 与 target_zoom 的差值低于 settle threshold 时 settled = true
last_rendered_zoom 只能由已接受的 render commit 更新
VIEWER_SESSION.current_zoom == projection(target_zoom)
```

### 3.2 Frame identity

每个异步渲染请求必须携带：

```text
frame_token
page identity
rendered_zoom
visual/layout snapshot
source document revision
```

`frame_token` 用于判断请求是否仍是当前工作；`document revision` 用于判断内容是否仍是当前版本。二者不可互换：

- 缩放变化通常改变 `frame_token`，不一定改变 document revision。
- 编辑提交改变 document revision，并通常触发新的 frame token。

### 3.3 Surface

当前缩放 surface 至少包括：

- vector surface：主呈现面。
- raster/preview surface：手势或预览阶段的辅助呈现面。
- editor canvas/overlay：编辑态附加呈现面。

**Required**：同一时刻可以有多个 surface 存在，但对于同一几何字段只能有一个 writer；surface 的显示/隐藏状态必须可由日志重建。

---

## 4. 跨模块契约

### Z-001 唯一缩放事实源

- `ZOOM_STATE` 是 `target_zoom`、`visual_zoom`、`last_rendered_zoom` 的唯一事实源。
- `VIEWER_SESSION.current_zoom` 只能投影，不得独立写入。
- TS 可以读取和提交输入，但不得实现第二套 zoom reducer。
- Core 纯函数不得持有 UI session 状态，只接收 request 并返回 result。

**保护方法**：

- UI：`read_zoom_state`、`with_zoom_state`、`with_zoom_state_mut`、`reset_zoom_state`。
- Authority：`set_target_zoom`、`set_target_zoom_instant`、`set_target_zoom_authoritative`、`mark_rendered_zoom`、`set_visual_layout`。
- Core：`advance_zoom_animation_state`、`commit_rendered_zoom`。

### Z-002 三种 zoom 的收敛关系

- wheel：修改 `target_zoom`，由 RAF 推进 `visual_zoom`。
- animation：`visual_zoom` 可以暂时领先 `last_rendered_zoom`。
- render commit：只有接受的 frame 才能更新 `last_rendered_zoom`。
- settle：`visual_zoom` 收敛到 `target_zoom`，随后安排最终渲染。
- instant：必须同时更新 `target_zoom` 与 `visual_zoom`，并清理旧动画时间基准。

禁止：

- 以 `last_rendered_zoom` 覆盖用户刚设置的 target。
- 以 stale frame 的 rendered zoom 更新 `last_rendered_zoom`。
- 在 TS 中直接写 `current_zoom` 作为第二事实源。

### Z-003 RAF 单一驱动

- `start_zoom_raf_loop` 只能建立一个有效 RAF loop。
- 重复 start 必须幂等。
- `stop_zoom_raf_loop` 必须取消 handle、closure、DOM cache 和 settle cleanup。
- stop 后旧 closure 不得修改 zoom state、surface 几何或 frame queue。
- wheel 在 loop 停止时必须重新唤醒 loop。

### Z-004 几何单一写手

几何字段包括：

```text
container width
container height
container left
container top
canvas/page width
canvas/page height
visual_layout.content_left
visual_layout.content_top
visual_layout.display_zoom
```

- Rust RAF/zoom 路径是几何 owner。
- TS 只采集 DOM 输入、组装 request、调用 WASM facade，不直接写几何。
- Rust 内部的即时 wheel SetBox 和 committed-frame SetBox 必须共享同一 `SetBox` 语义和日志 schema。
- 若继续使用直接 `style.set_property`，必须将其封装为唯一 helper，并禁止其他调用点扩散。

#### Z-004.1 手势 SetBox 必须覆盖渲染表面

`canvas/page width|height` 这一组字段在两次 SetBox 中都必须写到，且两次盒必须几何一致：

| 写入时机 | 触发者 | 目标元素 | 盒尺寸 |
|---|---|---|---|
| 手势中（每次 wheel） | `on_wheel_event` | container + 渲染表面 | `page × target_zoom` |
| settle（committed frame） | `apply_committed_frame` | container + 渲染表面 | `page × base_render_zoom` |

- 渲染表面是用户实际看到的 `#pdf-vector-main-canvas`，引用由 `raf_dom_cache::init_dom_cache` 缓存到 `DomCache.main_canvas`。
- 手势写表面的入口是 `raf_dom_cache::set_surface_box(display_width, display_height)` —— 唯一 helper，也是表面盒的唯一直写点。
- 理由：ADR-0006 移除了手势期的 CSS transform。若手势只写 container 而表面保留上次 committed 的盒，则手势期间可见面不缩放，全部缩放量落到 settle 那一帧，表现为"松开滚轮后页面突然放大到 N 倍"。手势写出的盒与 settle 时 `resolve_canvas_css_box` 写出的盒一致，故 settle 帧不再改变表面尺寸（settle jump ratio = 1，见 ZOOM_TEST_EVIDENCE 的 `[zoom-jump]` 证据）。
- 手势期表面保持已提交位图分辨率，由浏览器拉伸 —— 这是对已删除 CSS transform 的替代，不引入第二套缩放机制。

#### Z-004.2 手势几何所有权的最小作用域

- `WHEEL_GESTURE_ACTIVE` 是 **RAF 会话状态**，不是 zoom 状态，因此其 thread_local 定义在 `raf_loop.rs` 内部且为模块私有。
- 它不属于 `zoom_store.rs`。放进共享缩放状态容器会让每个读写 ZOOM_STATE 的模块都可能影响几何写入门槛。
- 唯一 API：
  - `begin_wheel_gesture()`（私有，仅 `on_wheel_event` 调用）；
  - `end_wheel_gesture()`（`pub(super)`，仅 `stop_zoom_raf_loop` 调用 —— 释放路径只有"循环停止"一条，reset 经 `reset_zoom_runtime` → `stop_zoom_raf_loop` 间接触发）；
  - `is_wheel_gesture_active()`（`pub(super)`，`raf_committed` 只读消费）。
- `raf_committed` 不得直接访问该 thread_local，只能经 `is_wheel_gesture_active()` 读取；`zoom_store.rs` 不得出现该符号。

#### Z-004.3 `content_left/content_top` 的单一权威是 `visual_layout`

`content_left/content_top` 有两个生产者：

1. 手势期间：`animation.rs::compute_anchor_content_offset` 写入 `ZOOM_STATE.visual_layout.content_left/top`（锚点偏移，保持鼠标下点不动）；
2. settle：`plan_builder.rs::compute_viewport_layout_result` 生成 `frame.content_left/top`（居中值）。

若 settle 直接应用 `frame.content_left/top`，页面会在松手时横向跳跃（实测 182.5px）。**修复**：`raf_committed.rs::apply_committed_frame` 在 settled 分支覆盖 `visual_layout` **之前**先捕获手势的锚点偏移 `gesture_offsets`，再用同一组捕获值同时写状态与写 DOM。仅当 `gesture_offsets` 不存在时才回退到 `frame.content_left/top`。`width/height` 仍从 frame 读取（单一权威是 committed render）。

- 理由：手势的锚点偏移是用户意图（鼠标下点不动），settle 的居中值是渲染器的默认行为。两者冲突时，用户意图优先。更关键的是读取顺序：同一函数内先覆盖后读取是无效修复，必须在覆盖前捕获。
- 验证：
  - E2E `zoom_wheel_sudden_jump.spec.ts` 断言 `|containerLeftDeltaSettle| <= 1.0 && |containerTopDeltaSettle| <= 1.0`；修复前红测 `leftDelta = -182.5`，修复后绿测 `leftDelta = 0, topDelta = 0`（见 ZOOM_TEST_EVIDENCE Run 2026-09-22）。
  - wasm unit `raf_loop_tests.rs::settle_frame_keeps_gesture_content_offset`（Z-POSITION-001）：红测 15/1（旧代码），绿测 16/0（捕获偏移版本）。

### Z-005 FrameToken stale 防护

- schedule 分配新 token。
- 所有异步边界前后都检查 token current。
- stale frame 不得：
  - 更新 page size；
  - 更新 `last_rendered_zoom`；
  - 应用 committed layout；
  - 清除更新的 in-flight 状态；
  - 覆盖更晚的 queued frame。
- accepted frame 才能进入 committed queue 和 settle。
- tile request 和 vector request 使用同一 frame identity 语义。

### Z-006 document revision 与 frame token 分离

- `document revision` 表示内容版本。
- `frame_token` 表示渲染工作版本。
- commit 必须通过 `note_document_mutation` bump revision。
- zoom-only frame 不得伪造 document mutation。
- document mutation 必须使旧 render frame 失效或被 current 检查拒绝。

### Z-007 settle 与 immediate mutation 语义一致

`editorVisibility` 和 `documentMutation` 可以拥有更高优先级，但“immediate”不能含糊：

- 若定义为“绕过 RAF 直接应用”，实现必须直接 apply，并有独立测试。
- 若定义为“优先进入 Rust queue，等待下一个 RAF”，名称应改为 `priority_render_frame` 或等价名称。

本规格推荐第二种：保留统一 queue/单写手，只提高优先级；避免绕过 FrameToken 和 SetBox owner。

### Z-008 anchor 语义收敛

ADR-0007 定义当前缩放始终居中。必须在规格和代码中区分：

- 已废弃的 cursor-anchor 行为。
- 仍为兼容序列化或 API 形状保留的 anchor 字段。
- 当前实际生效的居中布局。

在未完成迁移前，anchor 字段不能被删除；但必须有测试证明它们不会改变最终居中布局。

### Z-009 日志 correlation

所有关键事件必须至少输出：

```text
correlation_id
phase
frame_token
page_index
source_revision
target_zoom
visual_zoom
last_rendered_zoom
settled
active_surface
display_width
display_height
content_left
content_top
outcome
reason
```

事件阶段固定为：

```text
zoom.input
zoom.target-updated
zoom.raf.tick
zoom.setbox.applied
render.frame-scheduled
render.frame-current-check
render.frame-rejected
render.frame-committed
zoom.settle
zoom.surface-switch
```

---

## 5. 模块与方法规格

### 5.1 Core：`crates/pdf-viewer-core/src/render/zoom/state.rs`

| 方法/类型 | 责任 | 输入边界 | 输出/副作用 | 必测行为 |
|---|---|---|---|---|
| `VisualLayoutState` | 布局值对象 | finite zoom/offset | 纯数据 | 非有限值归一化、边界值 |
| `PendingCommittedFrame` | 待提交布局快照 | frame geometry | 纯数据 | 字段完整性、token 关联 |
| `PreviewHostState` | preview 状态值对象 | flags/frame | 纯数据 | reset/clear 不变量 |
| `DrawingDelayState` | settle 延迟值对象 | timestamp/delay | 纯数据 | start/cancel/expire |
| `HostZoomState` | 统一缩放状态 | 合法 zoom/layout | 纯数据 | default/reset/projection |
| `ZoomAnimationStep` | tick 结果 | visual/settled | 纯数据 | settle threshold |

### 5.2 Core：`crates/pdf-viewer-core/src/render/zoom/animation.rs`

| 方法 | 责任 | 必测场景 | 日志字段 |
|---|---|---|---|
| `clamp_zoom` | 将 zoom 限制在合法范围 | NaN、±Inf、低于 min、高于 max | input/min/max/output/reason |
| `clamp_f32` | 安全区间截断 | 非限值、反向边界 | input/output |
| `clamp_unit` | [0,1] 截断 | 负数、超 1、非限值 | input/output |
| `centered_offset` | 计算小内容居中偏移 | content < viewport、=、> | content/viewport/offset |
| `resolve_wheel_zoom_request` | wheel 输入到 target/layout request | zoom in/out、边界、非法输入、连续 wheel | delta/current/target/layout |
| `resolve_zoom_limits_result` | 解析安全最大 zoom | 页面尺寸、DPR、画布上限、非法值 | page/dpr/max_zoom/reason |
| `advance_zoom_animation_state` | visual 向 target 插值 | 首帧、长间隔、负时间、settle | before/after/delta/settled |
| `commit_rendered_zoom` | 接受渲染 zoom | 合法、非法、stale caller | previous/input/output |
| `build_zoom_preview_frame` | 组合 tick、layout、preview frame | preview、settle、无布局 | token/zoom/layout/outcome |

### 5.3 Core：`crates/pdf-viewer-core/src/render/zoom/zoom_decide.rs`

| 方法 | 责任 | 必测场景 |
|---|---|---|
| `resolve_preview_render_zoom` | 选择 visual/render zoom | animation、settled、invalid |
| `resolve_wheel_render_decision` | wheel 是否立即渲染 | settled、preview、idle、pending |
| `resolve_preview_tick_decision` | tick 是否继续、flush 或 render | preview active、settle、stale |
| `resolve_render_follow_up_decision` | 是否补渲染最新 target | target 与 rendered 不同/相同 |
| `resolve_zoom_commit_decision` | commit 是否接受 | stale ratio、settled、pending |
| `resolve_flush_decision` | flush 是否接受 | stale/valid |

### 5.4 Core：`crates/pdf-viewer-core/src/render/zoom/zoom_layout.rs`

| 方法 | 责任 | 必测场景 |
|---|---|---|
| `resolve_layout_fallback` | 缺布局数据时构造安全布局 | 缺失尺寸、极值、非法值 |
| `resolve_fit_to_width` | 计算适宽 zoom | 页面宽小于/等于/大于 viewport |
| `resolve_canvas_css_box` | 计算 canvas CSS box | display/render zoom 比值、DPR、极值 |
| `is_immediate_mutation_frame` | 判断高优先级 frame | editorVisibility/documentMutation/其他 reason |

### 5.5 Core：`crates/pdf-viewer-core/src/render/zoom/zoom_render.rs`

| 方法 | 责任 | 必测场景 |
|---|---|---|
| `should_reknock_preview_render` | preview 重敲门 | blur threshold、interval、in-flight |
| `predict_render_target` | 预测 render target | 正/负 velocity、边界、settle |
| `should_render` | Yes/Soon/Skip | blur 三档、settled、in-flight |

### 5.6 Core：`crates/pdf-viewer-core/src/render/zoom/zoom_tick.rs`

| 方法 | 责任 | 必测场景 |
|---|---|---|
| `tick_zoom_state_core` | 单帧决策编排 | animation、settle、delay、stop/start、DOM/async op 顺序 |
| `DomOp` | 同步几何操作 | UpdateLayout/SetScroll/SetWrapperSize 序列 |
| `AsyncOp` | 异步调度操作 | RequestRender/ScheduleNextFrame/StopRafLoop 顺序 |

### 5.7 UI：zoom store/authority

模块：

- `crates/pdf-viewer-ui/src/zoom/zoom_store.rs`
- `crates/pdf-viewer-ui/src/zoom/zoom_authority.rs`
- `crates/pdf-viewer-ui/src/zoom/zoom_preview.rs`
- `crates/pdf-viewer-ui/src/zoom/zoom_frame.rs`

每个公开方法必须有：

1. 正常路径测试。
2. reset/非法输入测试。
3. projection 不变量测试。
4. 结构化日志测试。

重点方法：

- `set_target_zoom`
- `set_target_zoom_instant`
- `set_target_zoom_authoritative`
- `mark_rendered_zoom`
- `set_visual_layout`
- `reset_zoom_runtime`
- `queue_committed_frame`
- `take_ready_committed_frame`
- `clear_zoom_preview_host_state`
- `set_wheel_render_pending`
- `set_preview_active`
- `set_cancel_pending_render`

### 5.8 UI：`crates/pdf-viewer-ui/src/zoom/zoom_controller.rs`

重点方法：

- `execute_wheel_zoom`
- `step_preview_host`
- `resolve_wheel_zoom`
- `tick_zoom_state`
- `clear_preview_host_with_anchor`
- `settle_zoom_preview_at_target`
- `reset_zoom_preview_host`

Required：每个方法必须记录输入 snapshot、调用 core 的结果、状态变更前后和 outcome；不得只测试返回 bool。

### 5.9 UI：RAF 模块

模块：

- `crates/pdf-viewer-ui/src/zoom/raf_loop.rs`
- `crates/pdf-viewer-ui/src/zoom/raf_committed.rs`
- `crates/pdf-viewer-ui/src/zoom/raf_dispatch.rs`
- `crates/pdf-viewer-ui/src/zoom/raf_dom_cache.rs`
- `crates/pdf-viewer-ui/src/zoom/raf_settle.rs`

重点方法：

- `start_zoom_raf_loop`
- `stop_zoom_raf_loop`
- `is_raf_loop_running`
- `on_wheel_event`
- `ensure_raf_loop_after_wheel`
- `commit_rendered_frame`
- `pop_committed_frame`
- `apply_committed_frame`
- `dispatch_settle_envelope`

Required：

- start 幂等。
- stop 清理完整。
- stale closure 无副作用。
- SetBox 应用前后 geometry 相同。
- active surface 切换可重建。

### 5.10 UI render/present

模块：

- `crates/pdf-viewer-ui/src/render/render_store.rs`
- `crates/pdf-viewer-ui/src/render/commit.rs`
- `crates/pdf-viewer-ui/src/render/workflow.rs`
- `crates/pdf-viewer-ui/src/present/present_store.rs`

重点方法：

- `schedule_render_frame_request`
- `is_render_frame_current`
- `settle_render_frame`
- `abort_render_frame`
- `commit_render_result`
- `allocate_render_frame_token`

Required：验证 token 单调、queued/in-flight 替换、stale reject、accepted commit、page size 不被 stale frame 更新。

### 5.11 TS bridge

模块：

- `src/bridge/zoom/zoom_controller.ts`
- `src/bridge/render/frame_plan.ts`
- `src/bridge/render/render_flow.ts`
- `src/bridge/render/vector_host.ts`
- `src/bridge/render/vector_page_bundle.ts`
- `src/bridge/render/render_wasm_api.ts`

Required：

- TS 只采集输入和调用 facade。
- 每个 `await` 边界检查 `isRenderFrameCurrent`。
- stale frame 不调用 commit。
- immediate mutation 语义与 Rust 判断一致。
- 统一输出 correlation 日志。

---

## 6. 测试驱动方案

### 6.1 测试层级

| 层级 | 目标 | 允许验证的内容 |
|---|---|---|
| Core unit | 纯函数和值对象 | 数值、边界、状态转移、操作序列 |
| UI Rust unit | authority、queue、RAF 可测试部分 | 所有权、幂等、队列、projection |
| TS unit | bridge 外部行为 | facade 调用、await stale gate、日志事件 |
| Cross-layer seam | 一条最小真实链 | wheel → RAF → schedule → stale/commit → SetBox |
| E2E | 真实 Tauri/WebView2 | 打开 PDF、wheel、settle、编辑/保存后的缩放 |

原则：优先测试最高可稳定复现的 seam；不要为每个私有 helper 复制跨层测试。

### 6.2 每个测试必须输出日志

Rust 测试不能只写：

```rust
assert_eq!(actual, expected);
```

必须先输出结构化事件，再断言：

```text
zoom.test.case-start
zoom.test.input
zoom.test.state-before
zoom.test.decision
zoom.test.state-after
zoom.test.assertion
zoom.test.case-end
```

每条日志至少包括：

```text
case_id
module
method
frame_token
input summary
state before
expected
actual
outcome
```

TS 测试使用现有 diagnostics/layout trace collector，断言事件名称、顺序和关键字段；不得只 mock 返回值后结束。

### 6.3 日志实现要求

建议新增统一测试辅助层，而不是每个测试手写字符串：

- Rust：`ZoomTestTrace` 或等价的测试专用 collector。
- TS：`ZoomTestTraceCollector` 或复用 `logPdfLayoutTrace` 的可注入 sink。
- 输出格式：稳定 JSON Lines；一行一个事件。
- 测试失败时打印完整 case trace。
- 测试成功时允许通过环境变量保留完整日志，例如 `ZOOM_TEST_LOG=1`。

测试日志不得依赖系统时间作为唯一排序依据；使用递增 `seq`，时间作为辅助字段。

### 6.4 测试证据文档

每次实现阶段必须更新：

`docs/ZOOM_TEST_EVIDENCE.md`

固定格式：

```markdown
# Zoom Test Evidence

## Run metadata
- Date:
- Commit:
- Environment:
- Command:
- Result:

## Case Z-ANIM-001
- Module:
- Method:
- Contract:
- Input:
- Expected:
- Actual:
- Assertions:
- Log:
  ```jsonl
  {"seq":1,"event":"zoom.test.case-start",...}
  ```
- Result: PASS/FAIL/NOT_RUN
- Follow-up:

## Summary
- Passed:
- Failed:
- Not run:
- Known gaps:
```

没有运行的测试必须写 `NOT_RUN`，不能留空，也不能用“绿色”代替日志证据。

### 6.5 测试矩阵

| ID | 模块 | 方法/路径 | 重点 |
|---|---|---|---|
| Z-STATE-001 | Core state | `HostZoomState` | 三种 zoom 有限、正值、reset |
| Z-ANIM-001 | Core animation | `advance_zoom_animation_state` | 插值、settle、长帧 |
| Z-ANIM-002 | Core animation | `commit_rendered_zoom` | 仅接受值更新 rendered |
| Z-LAYOUT-001 | Core layout | `centered_offset` | 小/等/大 viewport |
| Z-LAYOUT-002 | Core layout | `resolve_layout_fallback` | 非法输入 |
| Z-DECIDE-001 | Core decide | `should_render` | Yes/Soon/Skip |
| Z-TICK-001 | Core tick | `tick_zoom_state_core` | DomOp/AsyncOp 顺序 |
| Z-AUTH-001 | UI authority | `set_target_zoom*` | 唯一写入口与 projection |
| Z-QUEUE-001 | UI queue | `queue/take_committed_frame` | FIFO、替换、reset |
| Z-RAF-001 | UI RAF | start/stop | 幂等与 cleanup |
| Z-RAF-002 | UI RAF | wheel/tick | loop 唤醒和 SetBox |
| Z-RAF-003 | UI RAF | `WHEEL_GESTURE_ACTIVE` 作用域 | 模块私有 + 只读访问器 |
| Z-GESTURE-001 | UI RAF | `stop_zoom_raf_loop` | 停循环释放手势所有权 |
| Z-GESTURE-002 | UI RAF | `commit_rendered_frame` | 手势活跃时入队不应用 |
| Z-GESTURE-003 | UI RAF | `commit_rendered_frame` | 无手势时就地应用 |
| Z-SURFACE-002 | E2E | wheel burst → settle | 手势期可见面连续缩放（settle jump ratio ≈ 1） |
| Z-POSITION-001 | UI RAF + E2E | `apply_committed_frame` / wheel burst → settle | settle 不重居中：wasm unit `settle_frame_keeps_gesture_content_offset`（捕获覆盖前的手势偏移），E2E `\|leftDelta\| ≤ 1 && \|topDelta\| ≤ 1`（Z-004.3） |
| Z-FRAME-001 | UI render | schedule/current/settle | token lifecycle |
| Z-FRAME-002 | TS bridge | `render_flow` | await stale gates |
| Z-SURFACE-001 | UI/TS | surface switch | vector/raster visibility |
| Z-IMM-001 | Cross-layer | immediate mutation | 命名与实现语义一致 |
| Z-E2E-001 | E2E | wheel → settle | 真实链路与日志 |

---

## 7. 文档驱动开发流程

每个模块按以下顺序开发：

1. 在本文更新方法/变量契约。
2. 在测试矩阵新增测试 ID。
3. 先写失败测试和预期日志样例。
4. 实现最小代码。
5. 运行测试并保存日志到 `ZOOM_TEST_EVIDENCE.md`。
6. 更新契约状态：`Required → Implemented`。
7. 运行受影响验证层级。
8. 更新变体文档和 ADR 引用。

禁止：

- 先改算法、最后补文档。
- 只保存测试通过数字，不保存日志。
- 只跑 unit test 就声明跨层契约已验证。
- 用外部项目性能数字替代本项目实测。

---

## 8. 当前 Gap 与实施顺序

### Phase 0：冻结模型与日志 schema

- 明确 `immediateMutationFrame` 是 priority queue 还是 direct apply；本规格推荐 priority queue。
- 明确 anchor 字段的兼容范围。
- 建立 Rust/TS 测试 trace collector。
- 新增 `docs/ZOOM_TEST_EVIDENCE.md` 模板。

### Phase 1：Core 纯计算

按 `state → animation → layout → decide → render → tick` 顺序补齐方法级测试和 JSONL 日志。

### Phase 2：UI authority 与 queue

验证 authority、projection、preview host、committed frame queue、reset/cleanup。

### Phase 3：RAF 与 SetBox

统一几何写入口，验证 start/stop/wheel/tick/settle 和 surface visibility。

### Phase 4：FrameToken 跨层 seam

补齐 schedule → await → stale check → commit/reject → queue → SetBox 的最小跨层测试。

### Phase 5：E2E 与性能证据

执行真实 Tauri E2E，记录 wheel、settle、surface、frame token 和三种 zoom；再建立启动/首帧/RAF 性能基线。

---

## 9. 完成定义

缩放改动只有同时满足以下条件才算完成：

- 文档中的方法和变量范围已更新。
- 相关 Contract ID 有明确 owner 和 failure behavior。
- 每个受影响公开方法有测试 ID。
- 测试包含结构化日志，而不只是 assert。
- 测试说明和结果已写入 `docs/ZOOM_TEST_EVIDENCE.md`。
- 跨层改动至少通过一个 seam test。
- 需要真实 UI 行为时通过 E2E；未运行必须标 `NOT_RUN`。
- 没有 stale frame、双重 zoom authority 或第二几何 writer。
- `git diff --check`、受影响 cargo/TS 测试和必要 E2E 结果均已记录。
