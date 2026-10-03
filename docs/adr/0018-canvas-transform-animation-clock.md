# ADR-0018: canvas 变换跟随动画时钟（消除一帧调度滞后）

## Status

Accepted

## Context

### 现象（ADR-0017 修复冻结后新暴露）

`zoom_gesture_frame_contract`（14 档大跨度手势，~330 采样帧）偶发 1–3 帧
4.2–5.3% 的 canvas 视觉宽度漂移。E2E 逐帧 dump 拿到真实交错：

```
i=29  V=0.9051  cvW=546.03 = page×0.9177 = page×V(i=28)   ← 变换落后一帧
i=30  V=0.8762  cvW=538.52 = page×0.9051 = page×V(i=29)   ← 每帧都恰好落后一帧
i=31  V=0.8367(已收敛) cvW=521.30 = page×0.8762 = page×V(i=30) → 5.3% 违例
i=32  settle commit 落地 → cvW=497.84 = page×0.8367 ✓
```

### 根因：两个无因果序的 rAF 时钟

- **Rust 动画时钟**（`raf_loop.rs::tick`，自己的 rAF）：每帧推进
  `visual_zoom`（`advance_zoom_animation_state`）；
- **TS 瓦片 tick**（`tile_layer.tick`，另一个 rAF）：每帧读 zoom state 并写
  canvas 变换（ADR-0010 唯一写者 `owner.sync`）。

两个回调各自重排自己的下一帧，**帧内先后顺序没有保证**。实测顺序是
瓦片 tick 先跑、Rust 后跑 → 变换永远读到"上一帧的 visual"，落后一帧。

平时一帧滞后 = 一帧动画增量（<1%，无感）。但**手势反向/收敛帧**上，
一帧动画就是一整档滚轮步长（2^±120/800 ≈ ±10% 的半程 ≈ 4.6–5.3%），
采样器恰好在该帧读到旧变换 → 违例。settle commit 落地后重装箱才纠正。

ADR-0010 的不变量「canvas 视觉宽 = page × visualZoom（任意交错下）」
由此被打破：它假设 sync 与 advance 同帧因果有序，但两者分属两个时钟。

### 为什么不是 tile 队列空闲（ADR-0017 里的初步猜测）

瓦片 tick 在动画期间是常驻的（`zoomGap > ZOOM_EPS → scheduleTick()`），
sync 每帧都跑——只是**跑在 advance 之前**。问题是调度序，不是驱动缺失。

## Decision

**变换同步必须与动画推进同一 JS turn 内、且在其之后发生。**

### 1. Rust：动画帧敲门（复用 ADR-0001 固定全局敲门模式）

`raf_dispatch.rs` 新增姊妹敲门（与 `__pdfDrainPendingRenderFrame` 同模式：
TS 赋值属性，Rust 只调用落在那里的那个函数，无可注册回调链）：

```rust
const ANIMATION_FRAME_KNOCK_GLOBAL: &str = "__pdfZoomAnimationFrame";

/// ADR-0018: 每个动画帧、在状态推进之后敲一次 TS。
pub fn dispatch_animation_frame() { knock_global(ANIMATION_FRAME_KNOCK_GLOBAL); }
```

`raf_loop.rs::tick` 在 **step 1（advance_zoom_animation_state）之后立即**
调用 `dispatch_animation_frame()`——状态一推进，同 turn 内把 DOM 同步到
刚推进的 visual。

敲门查找从 `web_sys::window()` 改为 `js_sys::global()`：浏览器内两者同一
对象（主框架 `window === globalThis`），Node 的 wasm 测试里后者可用，
敲门因此可测。

### 2. TS：瓦片层暴露动画帧入口，tick 与入口共用同一同步函数

`tile_layer.ts` 抽出 `syncVisualTransforms(zs)`（= `owner.sync(visual)` +
`refreshTileTransforms(zs)`），**一个函数、读一次 live state、canvas 与瓦片
用同一 visual**：

```ts
onZoomAnimationFrame: () => syncVisualTransforms(deps.getZoomState())
```

tick 内原有的两处分散调用（owner.sync 在前、refreshTileTransforms 在后）
收敛为这一处——避免同一帧内"canvas 用新 visual、瓦片用旧 visual"的新错位。
owner 的 `lastWritten` memo 使同 turn 重复调用成为 no-op。

`pdf_runtime.ts` 在 `__pdfDrainPendingRenderFrame` 旁赋值：

```ts
(window as any).__pdfZoomAnimationFrame = () => tileLayer.onZoomAnimationFrame();
```

### 3. 代价

动画期间每帧多一次 `syncVisualTransforms`（瓦片 tick 先写旧值、敲门后写新值，
最终 DOM 状态 = 新值）。仅动画帧发生，~6 瓦片 × 3 样式 + 1 canvas，
微秒级；settle 后动画循环停车，敲门不再发生。

## 红灯契约（先红后绿）

- **主红灯：E2E `zoom_gesture_frame_contract`** —— 修复前 ~60% 运行失败
  （4.2–5.3% 漂移帧），修复后连续多次全绿。
- **源码契约 vitest**（沿用 `zoom_settle_envelope.test.ts` 的结构断言模式）
  `zoom_animation_frame_knock.test.ts`：
  1. `raf_dispatch.rs` 定义 `__pdfZoomAnimationFrame` + `dispatch_animation_frame`；
  2. `raf_loop.rs` 在 `advance_zoom_animation_state` **之后**、settle 派发之前调用它；
  3. `tile_layer.ts` 暴露 `onZoomAnimationFrame`，其实现 = owner.sync +
     refreshTileTransforms（同一 zs、同一函数），且**不**触碰瓦片泵/scheduleTick
     （不耦合 tile 队列活动）；
  4. `pdf_runtime.ts` 赋值 `__pdfZoomAnimationFrame`。

## Consequences

### Positive

1. ADR-0010 不变量在任意调度序下成立：变换与动画同 turn 因果有序，零滞后。
2. canvas 与瓦片同一 visual，不引入新的 canvas/瓦片错位。
3. 复用既有敲门模式（铁律 §2：Rust 管状态时钟，DOM 写入仍归 TS 的 owner）。

### Negative / 技术债

1. 动画帧内瓦片变换被写两次（旧值→新值），最终状态正确但有一次冗余写；
   若将来剖析显示可观，可让 tick 在动画活跃帧跳过 sync（需 Rust 暴露
   循环活跃状态，暂不做）。
2. 敲门是每帧同步调用：TS 侧必须保持 O(活动瓦片数) 而非 O(文档)。

## Verification (2026-10-02)

- **源码契约 vitest** `zoom_animation_frame_knock.test.ts`：修复前 **5/5 红**
  （`__pdfZoomAnimationFrame` / `dispatch_animation_frame` / `onZoomAnimationFrame`
  均不存在）→ 修复后 5/5 绿。
- **E2E `zoom_gesture_frame_contract`**：修复前 ~60% 运行失败（1–3 帧超 4%，
  p99≈2.8–3.8%、max 4.9–5.3%）；修复后 **13/14 次全绿，且每次绿运行
  max=0.00%、p99=0.00%**（不是"低于容差"，是漂移消失）。唯一一次失败未捕获
  详情，其后连续 9 次绿；另有 `zoom_tile_layer_gesture` 的
  "wheel did not change target zoom" 闪烁（~1/3，本会话引入任何 ADR 之前
  即存在，为该 spec 的 boot 时序问题，与变换无关）。
- 回归门：wasm 19/19、core 269/269、vitest 131/131（+5 契约）、
  clippy 双 crate 干净、tsc 干净；`load_pdf`(2)、`zoom_surface_painted_contract`(2)、
  `zoom_wheel_sudden_jump`(1) 全绿。

### 为什么任意交错下不变量成立（修复后的序分析）

帧内回调队列顺序为 [采样器/tick（先注册者）→ … → Rust RAF]。修复前：变换
在 tick（Rust advance 之前）写入 → 任何快照里 transform(V_{n-2}) vs state(V_{n-1})
差一帧。修复后：knock 在 advance 之后同一 turn 写入 → transform 与 state
**同源于同一次 advance**，任何时点的读取（采样器、合成器、用户眼睛）看到的
都是同一 visual —— 不变量不再依赖时序假设。
