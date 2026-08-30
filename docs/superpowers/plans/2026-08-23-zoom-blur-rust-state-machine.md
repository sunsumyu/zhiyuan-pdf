# 缩放模糊修复：Rust 侧状态机架构方案（修正版）

> 所有决策逻辑收进 Rust，TS 只做 DOM 读写 + RAF 驱动。

---

## 一、核心问题：渲染延迟导致 bitmap 滞后于动画

```
T0: visualZoom = 1.0, 发起渲染
T1: visualZoom = 1.05 (动画前进)
T2: visualZoom = 1.10
T3: 渲染完成, lastRenderedZoom = 1.0, visualZoom = 1.15
    → cssScale = 1.15 / 1.0 = 1.15 → 15% 模糊
```

**「停手后渲染一次」是错的** — 整个缩放过程都会模糊。PDF.js 的 drawingDelay 适用于浏览器端（渲染慢），我们是 Tauri + WASM，渲染更快，应该做得更好。

---

## 二、正确策略：渲染目标超前于动画

### 核心思路

渲染时预测 `visualZoom` 在渲染完成时的值，用预测值作为渲染目标。这样当 bitmap 提交时，`cssScale ≈ 1.0`。

```
T0: visualZoom = 1.0, 预测渲染完成时 visualZoom ≈ 1.08
    → 渲染 at 1.08
T3: 渲染完成, lastRenderedZoom = 1.08, visualZoom = 1.15
    → cssScale = 1.15 / 1.08 = 1.064 → 6.4% 模糊（比 15% 好一半）
```

### 预测模型

```rust
fn predict_render_target(
    visual_zoom: f32,
    target_zoom: f32,
    last_render_dt: f32,          // 上次渲染耗时（秒）
    animation_velocity: f32,      // visualZoom 变化速度（zoom/s）
) -> f32 {
    // 预测渲染完成时的 visualZoom
    let predicted_visual = visual_zoom + animation_velocity * last_render_dt;

    // 在 predicted_visual 和 targetZoom 之间取较近的
    // 但不超过 targetZoom（避免 overshoot）
    let lo = visual_zoom.min(target_zoom);
    let hi = visual_zoom.max(target_zoom);
    predicted_visual.clamp(lo, hi)
}
```

### 渲染频率策略

不再「每帧都渲染」也不再「停手后渲染一次」，而是：

| 条件 | 策略 |
|---|---|
| cssScale > 1.10（模糊明显） | 立即渲染 |
| cssScale > 1.03（轻微模糊） | 下一帧渲染 |
| cssScale < 1.03（几乎清晰） | 跳过渲染 |
| 动画 settled | 渲染最终 zoom 一次 |

```rust
fn should_render(
    visual_zoom: f32,
    last_rendered_zoom: f32,
    target_zoom: f32,
    animation_settled: bool,
) -> ShouldRender {
    let css_scale = visual_zoom / last_rendered_zoom.max(0.001);
    let blur = (css_scale - 1.0).abs();

    if animation_settled {
        // 动画结束 — 渲染最终 zoom
        return ShouldRender::Yes { render_zoom: target_zoom, reason: "settle" };
    }

    if blur > 0.10 {
        // 明显模糊 — 立即渲染 at predicted zoom
        ShouldRender::Yes { render_zoom: predicted_zoom, reason: "blur_high" }
    } else if blur > 0.03 {
        // 轻微模糊 — 下一帧渲染
        ShouldRender::Soon { render_zoom: predicted_zoom }
    } else {
        // 几乎清晰 — 跳过
        ShouldRender::Skip
    }
}
```

---

## 三、架构：Rust 状态机 + TS DOM 执行器

### 3.1 分层

```
Rust Core:  tick_zoom_state() — 每帧一个函数调用
            输入: timestampMs, scrollLeft, scrollTop, viewportW/H
            输出: Vec<DomOp> + Vec<AsyncOp>

TS:         RAF 循环 → 调用 Rust → 执行 DOM 操作
            不做任何计算/判断/状态管理
```

### 3.2 Rust 侧输出

```rust
pub enum DomOp {
    SetTransform { translate_x: f32, translate_y: f32, css_scale: f32, origin: String },
    ClearTransform,
    UpdateLayout { display_zoom: f32, render_zoom: f32, host_width: f32,
                   host_height: f32, content_left: f32, content_top: f32 },
    SetScroll { scroll_left: f32, scroll_top: f32 },
}

pub enum AsyncOp {
    RequestRender { reason: String },
    ScheduleNextFrame,
    StopRafLoop,
}
```

### 3.3 TS 侧（简化到 ~150 行）

```typescript
function startSmoothZoomPreview(): void {
    if (wheelZoomRafId !== null) return;

    const tick = (timestampMs: number) => {
        const container = deps.getVectorContainer();
        const scrollEl = deps.getScrollContainer();
        if (!container || !scrollEl) { wheelZoomRafId = null; return; }

        // 1. 读 DOM → 传 Rust
        const output = deps.tickZoomState({
            timestampMs,
            scrollLeft: scrollEl.scrollLeft,
            scrollTop: scrollEl.scrollTop,
            viewportWidth: scrollEl.clientWidth,
            viewportHeight: scrollEl.clientHeight,
        });

        // 2. 执行 Rust 返回的操作
        for (const op of output.domOps) executeDomOp(container, scrollEl, op);
        for (const op of output.asyncOps) executeAsyncOp(op);
    };

    wheelZoomRafId = window.requestAnimationFrame(tick);
}
```

---

## 四、Drawing Delay（仅用于 settle 后）

Drawing delay 只在动画 settled 后使用 — 避免 settled 后立即渲染（可能 targetZoom 还在微调）。

```rust
const SETTLE_DRAWING_DELAY_MS: u32 = 80;  // settled 后等 80ms 再渲染

// tick_zoom_state 内部逻辑：
if animation_settled && !drawing_delay_active {
    drawing_delay_active = true;
    drawing_delay_start = timestamp_ms;
    async_ops.push(AsyncOp::StartTimer { delay_ms: SETTLE_DRAWING_DELAY_MS });
}

if drawing_delay_active && (timestamp_ms - drawing_delay_start) >= SETTLE_DRAWING_DELAY_MS {
    drawing_delay_active = false;
    async_ops.push(AsyncOp::RequestRender { reason: "settle" });
}
```

---

## 五、文件改动清单

### Rust Core (`pdf-viewer-core`)

| 文件 | 改动 |
|---|---|
| `render/zoom_host.rs` | 新增 `tick_zoom_state`, `should_render`, `predict_render_target`, `DomOp`, `AsyncOp`, `ZoomTickInput/Output` |
| `render/zoom_state.rs` | 新增 `DrawingDelayState`, `AnimationVelocity`, 扩展 `HostZoomState` |
| `render/zoom_interaction.rs` | 新增 `compute_preview_translate`, 暴露 `animation_velocity` |

### Rust UI/WASM (`pdf-viewer-ui`)

| 文件 | 改动 |
|---|---|
| `render/free_api.rs` | 导出 `tickZoomState`, `onWheelEvent` |
| `zoom/event.rs` | 重构为新 state machine 接口 |

### TS

| 文件 | 改动 |
|---|---|
| `zoom/zoom_controller.ts` | 从 741 行 → ~150 行。只保留 RAF + DOM 执行 |
| `render/render_wasm_api.ts` | 新增 bridge 类型 |
| `render/frame_plan.ts` | 新增 adapter |
| `viewer/pdf_runtime.ts` | 更新 deps |

---

## 六、关键设计决策

### 6.1 为什么不「停手后渲染一次」

| 方案 | 体验 | 问题 |
|---|---|---|
| 每帧渲染 at visualZoom | 模糊（bitmap 滞后） | 资源浪费 + 延迟导致模糊 |
| 停手后渲染一次 | 整个缩放过程模糊 | **体验差** |
| **渲染 at predicted zoom** | **模糊大幅降低** | 需要预测模型 |

### 6.2 预测模型的精度

- 动画速度可通过历史 dt 推算（`velocity = (visualZoom_new - visualZoom_old) / dt`）
- 渲染延迟可通过历史渲染时间统计
- 预测不准时：overshoot 比 undershoot 好（高分辨率 bitmap CSS 缩小 = 锐利）

### 6.3 渲染频率自适应

不再固定 16ms，而是根据模糊程度动态调整：
- cssScale > 1.10 → 立即渲染（高频）
- cssScale 1.03~1.10 → 下一帧渲染（中频）
- cssScale < 1.03 → 跳过（低频）
- settled → 渲染一次最终 zoom

---

## 七、预期效果

| 指标 | 改动前 | 改动后 |
|---|---|---|
| TS zoom_controller.ts | 741 行 | ~150 行 |
| 模糊度（cssScale 偏差） | 5-20% | 1-6% |
| 渲染次数 | 每帧都渲染 | 按需渲染（模糊时才渲染） |
| CPU 占用 | 高（每帧渲染） | 中（按需渲染） |
| Drawing delay | 无 | 仅 settled 后 80ms |
| Rust 决策函数 | 8 个独立函数 | 1 个 `tick_zoom_state` |

---

## 八、实施路线图

### Phase 1: 状态机骨架 + 预测渲染（~400 行 Rust + ~150 行 TS）

1. 新增 `DomOp`, `AsyncOp`, `ZoomTickInput/Output`
2. 实现 `tick_zoom_state` 核心函数
3. 实现 `should_render` + `predict_render_target`
4. WASM 导出
5. TS 侧重写为纯执行器
6. 验证：缩放模糊明显降低

### Phase 2: 渲染取消 + settle drawing delay（~100 行 Rust + ~30 行 TS）

1. 渲染取消机制
2. Settle drawing delay (80ms)
3. 验证：过期 bitmap 不提交，settle 后干净

### Phase 3: 完整迁移（~200 行 Rust + ~100 行 TS）

1. 迁移 commit/flush/anchor 决策到 Rust
2. 清理 TS 废弃代码
3. 验证：所有 zoom 行为正确
