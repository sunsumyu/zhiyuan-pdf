# ADR-0019: 跟随渲染不得写 zoom 权威状态（缩放步长截断）

## Status

Accepted

## Context

### 现象

`zoom_tile_layer_gesture` E2E ~1/3 运行失败："wheel did not change target zoom"。
取证（spec 失败路径 dump，2026-10-02）推翻了最初怀疑（auto-fit 竞态——诊断显示
auto-fit 全部正确 skipped，初始 target=1.0）：**wheel 生效了，但步长被截断**——

```
单步 ctrl+wheel（deltaY=120，期望 target 1.0 → 2^(-120/800)=0.9013）：
失败运行实测 target = 0.9601 / 0.9258 / 0.9637 / 0.9504（每次不同，全在 0.9013~1.0 之间）
```

### 根因：渲染跟随写了 zoom 权威状态

`schedule_render_follow_up`（`render/free_api.rs`）在调度前执行
`set_zoom(decision.target_zoom)`。而 `resolve_render_follow_up_decision`
的语义是：**未结算时 effective_target = visual_zoom（动画当前位置）**。
于是 mid-gesture 的时序：

1. wheel 把权威 target 写为 0.9013，动画开始向它收敛；
2. reknock 帧在 visual=0.96 处渲染并提交；提交后 `nextFrameFromTransition`
   触发跟随，此刻 visual 已继续前移（如 0.955）；
3. `needs_render(visual=0.955, rendered=0.96) ≥ 0.001` → 跟随触发，
   `set_zoom(0.955)` → **权威 target 被改写为动画半路值**；
4. 跟随帧渲 0.955、提交、再跟随……target 一路"追着 visual 跑"，
   最终收敛在 0.9013~1.0 之间某个时序决定的位置——**用户的缩放步长被截断**。

settle 之后触发则无害（effective_target = target，写等于没写）——所以是
时序依赖的闪烁，而非必现。此缺陷自该行引入起就存在（先于本会话所有 ADR）；
ADR-0017 落地 `request.display_zoom = decision.target_zoom` 后，这行
`set_zoom` 已完全冗余，只剩危害。

### 为什么这是链分叉（铁律 §2）

zoom 权威（target_zoom）的合法写者是用户手势路径（wheel → on_wheel_event）、
UI 选择、reset。**渲染跟随是渲染侧 actor，它读取权威、渲染决策目标，
绝不应该回写权威。** `set_zoom` 还会广播 VIEWER_ZOOM_CHANGE——把一次
渲染内部决策冒充成用户意图通知 UI。

## Decision

删除 `schedule_render_follow_up` 中的 `set_zoom(decision.target_zoom)`。

- 跟随帧渲染的目标已由 ADR-0017 的 `request.display_zoom = decision.target_zoom`
  携带——与 zoom 权威无关；
- settle 路径：删除前后行为完全一致（原本就是 no-op）；
- mid-gesture 路径：权威 target 不再被截断；跟随帧此时
  `gesture_refresh = |visual - target| > 阈值` 恢复为 TRUE → 走 ADR-0012
  视口瓦片路径（此前被覆写后反而误走整页路径）。

## 红灯契约

`render/follow_up_tests.rs` 新增：

- `follow_up_never_writes_the_zoom_authority_mid_gesture`：
  target=1.5 / visual=1.30 / rendered=1.10 → 跟随返回 1.30 的帧（ADR-0017 契约
  不回退），且 **`target_zoom` 仍为 1.5**。修复前红（被写成 1.30）。
- `follow_up_never_writes_the_zoom_authority_settled`：settled 场景 target 不变。

## Consequences

### Positive

1. 缩放步长确定性恢复：单步 wheel 恒定 2^(-deltaY/800)，`zoom_tile_layer_gesture`
   闪烁消除。
2. 渲染侧对 zoom 状态只读（铁律 §2 收口）；UI 不再收到渲染伪造的
   zoom-change 事件。
3. mid-gesture 跟随帧回到 ADR-0012 视口瓦片路径（顺带修掉一个被掩盖的
   整页重渲）。

### Negative / 技术债

1. `set_zoom` 的 VIEWER_ZOOM_CHANGE 不再在跟随路径广播——若有 UI 依赖
   渲染跟随来刷新 zoom 显示，需改由 wheel 路径的 syncZoomSelect 负责
   （现状即如此，无依赖）。

## Verification

- 红灯：新增 2 契约修复前红 → 修复后绿；wasm/core/vitest/clippy 全绿。
- E2E：`zoom_tile_layer_gesture` 连续多次运行不再出现
  "wheel did not change target zoom"；`zoom_gesture_frame_contract` 保持
  max=0.00% 漂移。
