# ADR-0017: 缩放跟随渲染的收敛（follow-up 必须按决定的目标重建请求）

## Status

Accepted

## Context

### 用户实测（2026-10-02）

用户报告「还是缩放后就卡住不动了」。这次不是卡顿（stutter），是**硬冻结**：
渲染主线程被占满，CDP 连 `Profiler.enable` 都超时（宏任务被饿死），
`Page.captureScreenshot` / `Runtime.evaluate` 一并超时；进程 446s CPU / 1.98GB RSS。

### 根因（CDP 实测，零 shim 复现）

连续 ctrl+wheel 后进入**不收敛的跟随渲染环**。逐帧实测（`followup.decide` 打点）：

```
followup.decide rendered=1.1249064 target=1.142407 visual=1.142407 schedule=true decideTarget=1.142407 reqZoom=1.1249064
followup.decide rendered=1.1249064 target=1.142407 visual=1.142407 schedule=true decideTarget=1.142407 reqZoom=1.1249064
...（逐帧完全相同，永不收敛，~200 帧/s）
```

循环链（每次 ~2ms，无像素工作——detail 瓦片缓存命中，纯调度环）：

```
strategyVectorRender
  → commitRenderResult(frameToken, renderPlan.renderZoom)   // 再次把 lastRenderedZoom 写成旧值
  → nextFrameFromTransition
  → scheduleRenderFollowUp(renderPlan.displayZoom)          // 用「旧」display_zoom 重建请求
  → Rust resolve_render_follow_up_decision(rendered=旧, target=新, visual=新)
      → schedule_latest_target = needs_render(1.142407, 1.1249064) = true   // 恒真
  → schedule_render_frame_request(request)                  // request.display_zoom = 旧值 1.1249064
  → 新帧 displayZoom=1.1249064 → 渲染 → commit 又写 1.1249064 → …
```

### 缺陷边界

`schedule_render_follow_up`（`crates/pdf-viewer-ui/src/render/free_api.rs`）**已经算出
正确的 `decision.target_zoom` 并 `set_zoom`**，却仍用**调用方传入的 `request`** 调度：

```rust
// frame_plan.ts::scheduleRenderFollowUp
renderApi.scheduleRenderFollowUp(renderedDisplayZoom, buildRequest(renderedDisplayZoom, 'zoom'))
```

该 request 的 `display_zoom` = **已渲染的旧 zoom**。于是调度出的帧渲染旧 zoom，
`commit_rendered_zoom` 再次记录旧 zoom，而 `needs_render(target, lastRendered)` 恒真 → 无限环。

这是「决策与执行各持一半状态」的单点契约缺陷：决定目标 zoom 的
`resolve_render_follow_up_decision` 与构建请求的 `buildRequest` 分属两侧，
跟随帧只采用了前者的"是否调度"、没采用其 `target_zoom`（铁律 §2 单一所有者）。

### 与 ADR-0016 的关系

ADR-0016 让 settle 走视口瓦片路径（`render_base_layer=false` + detail 瓦片缓存命中），
使每次跟随渲染变得**极廉价**（~2ms，无像素工作）。廉价化把一个原先被
O(page×zoom²) 位图成本掩盖的**调度不收敛**暴露成每秒 200 帧的忙环——
即 ADR-0016 是触发条件，不是根因；根因是本 ADR 的契约缺陷。

## Decision

跟随帧的请求必须**由决定的目标重建**，而不是沿用调用方的旧请求：

```rust
pub fn schedule_render_follow_up(rendered_display_zoom: f32, request_js: JsValue) -> JsValue {
    let mut request: FramePlanRequest = from_value(request_js).unwrap_or_default();
    let zoom_state = read_zoom_state();
    let decision = resolve_render_follow_up_decision(
        rendered_display_zoom,
        zoom_state.target_zoom,
        zoom_state.visual_zoom,
    );
    if !decision.schedule_latest_target {
        return JsValue::NULL;
    }
    set_zoom(decision.target_zoom);
    request.display_zoom = decision.target_zoom; // ← 收敛关键：执行采用决策
    match schedule_render_frame_request(&request) { ... }
}
```

`FramePlanRequest.display_zoom` 是唯一的 zoom 权威输入；请求其余字段
（page/viewport/scroll/dpr）与 zoom 无关，沿用调用方即可。TS 侧无需改动
（`buildRequest` 仍传 `renderedDisplayZoom`，被 Rust 覆盖）——零新链路、零桥改动。

### 为什么不在 `resolve_render_follow_up_decision` 里修

该函数是**纯决策**（返回 `target_zoom`），语义正确；缺陷在执行侧未采用该决策。
把请求构建塞进纯函数会让它承担副作用职责，违反单一所有者。

### 为什么不去掉 `nextFrameFromTransition` 的 followUp 分支

该分支承载「手势期间连续重渲」（bitmap 追踪 visual zoom，C1 契约），是必要机制；
问题只是它必须收敛到 target。

## 红灯契约（先红后绿）

`crates/pdf-viewer-ui/src/render/follow_up_tests.rs`（`wasm_bindgen_test`）：

1. `follow_up_rebuilds_request_at_decided_target`：模拟实测不收敛场景
   （`target=visual=1.142407`，`rendered=1.1249064`，`request.display_zoom=1.1249064`），
   断言返回帧 `frame_plan.display_zoom == 1.142407`（且 ≠ 旧值）。**修复前失败**。
2. `follow_up_converges_when_target_equals_rendered`：settled 后
   （target=visual=rendered）返回 `NULL`，不产生多余帧。
3. `follow_up_tracks_visual_zoom_during_gesture`：手势中（target≠visual）
   跟随帧渲染 `visual_zoom`（C1 契约不回退）。

## Consequences

### Positive

1. 跟随环收敛：每帧向 target 前进，`needs_render` 至多 1 帧后变 false。
2. 硬冻结消失（不再有无限调度环）。
3. 修复位于单一所有者（free_api 的 follow-up 执行点）；核心纯函数与 TS 均不动。

### Negative / 技术债

1. 需确认 `request` 其余字段在新 zoom 下仍有效——page/viewport/scroll/dpr 与
   zoom 无关，安全。
2. ~~保留 `set_zoom(decision.target_zoom)` 以最小化行为变更~~ —— **已被
   ADR-0019 推翻并删除**：mid-gesture 它把权威 target 覆写成动画半路的
   visual，截断用户缩放步长（~1/3 E2E 闪烁的根因）。跟随帧渲染目标由本 ADR
   的 `request.display_zoom` 携带即足，zoom 权威只允许用户手势路径写。

## Verification

- 修复前：`follow_up_rebuilds_request_at_decided_target` 红（`display_zoom=1.1249064`）。
- 修复后：3/3 绿；CDP 复现脚本连续 12 次手势不再 wedge（峰值渲染 232→28/2s，
  RSS 1.98GB→稳定 ~878MB，无 runaway 增长）。
- E2E A/B（`zoom_gesture_frame_contract`，本机 WebView2 151）：
  - **修复前：3/3 次 5m16s 超时挂死**（冻结直接在 E2E 复现，spec 无法完成）；
  - 修复后：~13.5s 完成；4 次中 3 次全绿（p50/p90=0%，p99≈2.8%），
    1 次命中 1/348 帧 5.11%。

### 修复后新暴露的残留（非本 ADR 引入，已定位，待后续 ADR）

冻结消失后，`zoom_gesture_frame_contract` 偶发捕获 1–3 帧（/约330帧）4.2–5.1%
的 canvas 宽度漂移。定位（z 三元组为证）：

```
CANVAS-WIDTH-JUMP cvW=434.2 expected≈415.4 z=[0.698, 0.698, 0.730]
                  → box = page×lastRenderedZoom(0.730)，而 visual 已收敛 0.698
```

这是 ADR-0010 的"每帧驱动"耦合问题：主 canvas 变换的唯一每帧写者是
tile tick（`tile_layer.tick → owner.sync(visualZoom)`），而 tick 只在
tile 有工作时被调度（`pumpNext: inFlight || queue_size>0`）。手势两拍之间
动画已收敛（visual==target）但 settle 帧尚未 commit 时，box 仍停在上一次
commit 的 lastRendered 空间、scale=1，采样器读到 `page×lastRendered`。
settle commit 落地后 `onRenderCommitted → notifyViewportChanged → tick → sync`
即收敛——窗口 1–3 帧。

- **修复前该窗口是无限的**（follow-up 永不 commit → box 永远停在 lastRendered，
  即冻结本身）；修复后收敛到 1–3 帧。此残留不是本 ADR 的回归
  （A/B：关掉本修复 + 循环上限护栏 → 4/4 绿但每 run 仅采样 115–140 帧、
  30–51s；修复后每 run 321–363 帧、13s，逐帧漂移率反而更低）。
- 后续方向：~~变换 sync 跟随动画帧/settle 收敛点驱动，不耦合 tile 队列~~
  **已在 ADR-0018 落地**——且根因与本节初步猜测不同：不是 tile 队列空闲，
  而是变换写入与动画推进分属两个无因果序的 rAF（每帧恒定落后一帧）。
  修复后该漂移归零（max=0.00%），见 ADR-0018 Verification。
