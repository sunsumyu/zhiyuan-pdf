# 缩放模糊问题：成熟方案设计

> 基于 PDF.js、Chrome/PDFium、MuPDF、Poppler 等主流 PDF 渲染器的架构研究。

---

## 一、行业标准做法

### PDF.js（Mozilla，Web 端最成熟的 PDF 渲染器）

**核心策略：CSS preview + drawingDelay 批量渲染 + 双层 canvas**

```
用户滚轮 → CSS transform 即时预览（零延迟）
         → drawingDelay 计时器（默认值，连续滚轮时不断重置）
         → 用户停止滚轮 → timer 到期 → 全量重绘 at 最终 zoom
```

关键机制：
1. **drawingDelay**：连续滚轮事件时不断清除/重设定时器，只在用户停手后触发一次全量重绘。避免中间帧浪费。
2. **minDurationToUpdateCanvas（500ms）**：canvas→DOM 的更新频率上限。渐进渲染期间，temp canvas 到 visible canvas 的拷贝不超过每 500ms 一次。
3. **detail canvas**：当页面在高 zoom 下超过 `maxCanvasPixels` 时，主 canvas 用低分辨率 + CSS 缩放，上面叠加一个视口区域的高分辨率 detail canvas。
4. **渲染取消**：zoom 变化时立即取消正在进行的渲染（`RenderingCancelledException`），避免过期 bitmap 提交。

### Chrome/PDFium

**核心策略：纯光栅化 + CSS preview + 目标 zoom 重绘**

```
bitmap = pageWidth * zoom * DPR × pageHeight * zoom * DPR
```

- 渲染分辨率 = 目标 zoom × devicePixelRatio
- 交互缩放时：CSS `scale()` 即时预览，停手后全量重绘
- 内存保护：bitmap 超 30MB 时递归降采样 50%

### MuPDF / Poppler

**核心策略：DPI 控制 + 全页光栅化**

```
effectiveDPI = 72 * zoom * DPR
transform = fz_scale(DPI / 72, DPI / 72)
```

- PDF 内容是矢量的（路径操作符 + 字体轮廓），在任意分辨率下都能锐利渲染
- 前端负责管理 pixmap 生命周期和显示缩放

---

## 二、我们的架构 vs 行业标准

| 维度 | 我们现在 | PDF.js / Chrome |
|---|---|---|
| **即时反馈** | CSS transform（✓ 同） | CSS transform |
| **中间帧渲染** | 每 16ms 请求渲染，bitmap 不断提交 | drawingDelay 批量渲染，只在停手后渲染一次 |
| **渲染目标** | 每帧都渲染 at visualZoom | 只渲染最终 zoom |
| **渲染取消** | 无（过期 bitmap 由 stale guard 过滤） | 取消进行中的渲染 |
| **双层 canvas** | 有 detail layer 概念但未充分利用 | 成熟的 base + detail 双层 |
| **DPR 处理** | 有 devicePixelRatio | 标准 DPR 缩放 |

### 核心差异

**我们的架构在「每帧都渲染」**——每 16ms 都发起一次完整的 bitmap 渲染。这导致：
1. 大量 CPU/GPU 资源浪费（中间 zoom 的 bitmap 只用一帧就被替换）
2. 渲染队列堆积，延迟增加
3. 过期 bitmap 需要 stale guard 过滤，增加复杂度

**PDF.js 的架构是「只渲染最终 zoom」**——CSS transform 提供即时反馈，用户停手后才触发一次全量重绘。这导致：
1. 零中间帧浪费
2. 渲染延迟最小化（只渲染一次）
3. 无需 stale guard（只有一帧需要提交）

---

## 三、方案设计：CSS Preview + Drawing Delay + 双层 Canvas

### 3.1 总体架构

```
┌─────────────────────────────────────────────────┐
│  Wheel Event                                     │
│  → 更新 targetZoom                              │
│  → CSS transform 即时预览（零延迟）              │
│  → 重置 drawingDelay 定时器                      │
└──────────────────────┬──────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────┐
│  Tick Loop (RAF)                                 │
│  → advance_zoom_animation_state (visualZoom)     │
│  → applyPreviewFrame (CSS transform only)        │
│  → 不发起渲染！                                  │
└──────────────────────┬──────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────┐
│  Drawing Delay Timer 到期                        │
│  → 用户已停止滚轮                                │
│  → 发起一次全量渲染 at targetZoom                │
│  → 渲染完成 → commit → 清除 CSS transform       │
└─────────────────────────────────────────────────┘
```

### 3.2 分阶段实施

---

#### Phase 1：Drawing Delay 机制（核心改动）

**目标**：停止每帧渲染，改为用户停手后渲染一次。

**改动文件**：

| 文件 | 改动 |
|---|---|
| `crates/pdf-viewer-core/src/render/zoom_host.rs` | 新增 `resolve_drawing_delay` |
| `crates/pdf-viewer-core/src/render/zoom_state.rs` | 新增 `drawing_delay_timer` 字段 |
| `crates/pdf-viewer-ui/src/zoom/event.rs` | 修改 `execute_wheel_zoom` 和 `step_preview_host` |
| `src/bridge/zoom/zoom_controller.ts` | 修改 `startSmoothZoomPreview` tick loop |

**Rust 侧**：

```rust
// zoom_host.rs

/// Drawing delay configuration.
/// 连续滚轮事件时不断重置，用户停手后 timer 到期触发渲染。
pub const DRAWING_DELAY_MS: u32 = 150;  // 150ms — 比 PDF.js 更激进

pub struct DrawingDelayState {
    pub timer_started_at_ms: f64,
    pub delay_ms: u32,
    pub render_scheduled: bool,
}

pub fn resolve_drawing_delay_decision(
    state: &DrawingDelayState,
    current_timestamp_ms: f64,
    target_zoom: f32,
    last_rendered_zoom: f32,
) -> DrawingDelayDecision {
    if state.render_scheduled {
        // 已经调度了渲染，等完成
        return DrawingDelayDecision { should_render: false, should_reset_timer: false };
    }

    let elapsed = current_timestamp_ms - state.timer_started_at_ms;
    if elapsed >= state.delay_ms as f64 {
        // Timer 到期 — 用户已停手，触发渲染
        DrawingDelayDecision {
            should_render: needs_render(target_zoom, last_rendered_zoom),
            should_reset_timer: false,
        }
    } else {
        // Timer 未到期 — 继续等待
        DrawingDelayDecision { should_render: false, should_reset_timer: false }
    }
}
```

**TS 侧**：

```typescript
// zoom_controller.ts — tick loop 改为纯 CSS preview

function startSmoothZoomPreview(): void {
    if (wheelZoomRafId !== null) return;

    let drawingDelayTimer: number | null = null;

    const tick = (timestampMs: number) => {
        const snapshot = deps.readZoomSnapshot();
        const previewHostResult = deps.stepPreviewHost(
            snapshot.targetZoom, timestampMs,
        );
        if (!previewHostResult?.preview) {
            // 动画结束
            wheelZoomRafId = null;
            return;
        }

        // 只做 CSS preview，不发起渲染
        applyPreviewFrame(previewHostResult.preview);

        if (!previewHostResult.decision?.continuePreview) {
            // 动画 settled — 启动 drawingDelay timer
            wheelZoomRafId = null;
            drawingDelayTimer = window.setTimeout(() => {
                drawingDelayTimer = null;
                deps.requestRender('zoom');  // 只渲染一次
            }, DRAWING_DELAY_MS);
            return;
        }

        // 重置 drawingDelay timer
        if (drawingDelayTimer !== null) {
            window.clearTimeout(drawingDelayTimer);
            drawingDelayTimer = null;
        }

        wheelZoomRafId = window.requestAnimationFrame(tick);
    };

    wheelZoomRafId = window.requestAnimationFrame(tick);
}
```

**效果**：
- 连续滚轮时：只做 CSS transform，不渲染
- 停手后 150ms：触发一次全量渲染 at targetZoom
- 渲染结果 cssScale = 1.0，无模糊

---

#### Phase 2：渲染取消机制

**目标**：新渲染请求时取消旧的进行中渲染。

**改动文件**：

| 文件 | 改动 |
|---|---|
| `crates/pdf-viewer-ui/src/render/host_runtime.rs` | 新增 `cancel_pending_render` |
| `src/bridge/render/render_wasm_api.ts` | 新增 WASM 导出 |

**核心逻辑**：

```rust
// host_runtime.rs
pub fn cancel_pending_render() {
    // 标记当前渲染为已取消
    // 下次 commit_render_result 时检查取消标记，丢弃结果
}
```

```typescript
// zoom_controller.ts — wheel event handler
function bindWheelZoom(): void {
    scrollContainer.addEventListener('wheel', (event) => {
        // 取消进行中的渲染
        deps.cancelPendingRender();

        // 正常的 zoom 逻辑...
    });
}
```

**效果**：过期 bitmap 不会提交，消除 stale guard 的需要。

---

#### Phase 3：Detail Canvas 双层渲染

**目标**：视口区域始终清晰，全页用低分辨率。

**改动文件**：

| 文件 | 改动 |
|---|---|
| `crates/pdf-viewer-core/src/render/present_plan.rs` | 修改 `resolve_present_policy` |
| `crates/pdf-viewer-core/src/render/zoom_host.rs` | 新增 `resolve_detail_render_target` |
| `src/bridge/render/vector_canvas_host.ts` | 修改 canvas 层级管理 |

**核心逻辑**：

```
Base Layer: 低分辨率全页（zoom * 0.5），CSS scale 到正确尺寸
Detail Layer: 视口区域高分辨率（zoom * 1.0）
```

```rust
// present_plan.rs
pub fn resolve_present_policy(...) -> PresentPolicy {
    let preview_active = !preview_is_settled(target_zoom, visual_zoom);

    // Preview 期间：detail layer 始终渲染（视口区域清晰）
    let render_detail_layer = use_viewport_tile && !has_reusable_detail_tile;

    // Base layer：只在 settled 或 zoom 变化大时渲染
    let render_base_layer = !has_reusable_base_layer && (
        preview_settled || zoom_ratio_delta(base_render_zoom, last_rendered_zoom) > 0.15
    );

    // 关键：preview 期间允许 detail layer 渲染
    let allow_render_during_preview = preview_active && render_detail_layer;

    PresentPolicy { ... }
}
```

```rust
// zoom_host.rs — detail render target
pub fn resolve_detail_render_target(
    visual_zoom: f32,
    target_zoom: f32,
    last_rendered_zoom: f32,
) -> DetailRenderTarget {
    if (target_zoom - last_rendered_zoom).abs() / last_rendered_zoom.max(0.01) < 0.30 {
        // zoom 变化小（< 30%）：detail layer 在 visualZoom 处渲染
        DetailRenderTarget { render_zoom: visual_zoom, priority: High }
    } else {
        // zoom 变化大：detail layer 在 targetZoom 处渲染（跳过中间帧）
        DetailRenderTarget { render_zoom: target_zoom, priority: Medium }
    }
}
```

**效果**：
- 视口区域始终有高分辨率 bitmap
- 全页低分辨率 + CSS 缩放提供基础清晰度
- 用户感知模糊大幅降低

---

#### Phase 4：渲染频率自适应

**目标**：根据缩放速度动态调整渲染策略。

**改动文件**：

| 文件 | 改动 |
|---|---|
| `crates/pdf-viewer-core/src/render/zoom_host.rs` | 新增 `resolve_adaptive_render_strategy` |

```rust
pub struct AdaptiveRenderStrategy {
    /// 渲染模式：Batched（停手后渲染一次）| Continuous（每帧渲染）
    pub mode: RenderMode,
    /// 渲染目标 zoom
    pub render_zoom: f32,
    /// 是否取消旧渲染
    pub cancel_previous: bool,
}

pub fn resolve_adaptive_render_strategy(
    zoom_velocity: f32,           // 缩放速度（zoom/s）
    zoom_diff: f32,               // |target - visual|
    last_render_interval_ms: f32, // 上次渲染间隔
) -> AdaptiveRenderStrategy {
    if zoom_velocity.abs() > 5.0 {
        // 快速缩放：batched 模式，停手后渲染
        AdaptiveRenderStrategy {
            mode: RenderMode::Batched,
            render_zoom: 0.0,  // 等 settled
            cancel_previous: true,
        }
    } else if zoom_diff < 0.1 {
        // 接近目标：continuous 模式，精细渲染
        AdaptiveRenderStrategy {
            mode: RenderMode::Continuous,
            render_zoom: 0.0,  // visualZoom
            cancel_previous: false,
        }
    } else {
        // 中等速度：混合模式
        AdaptiveRenderStrategy {
            mode: RenderMode::Continuous,
            render_zoom: 0.0,  // visualZoom
            cancel_previous: true,
        }
    }
}
```

---

## 四、实施路线图

| Phase | 改动量 | 预期效果 | 优先级 |
|---|---|---|---|
| Phase 1: Drawing Delay | 中（~200 行） | 消除中间帧渲染浪费，settle 后一次清晰 | P0 |
| Phase 2: 渲染取消 | 小（~50 行） | 消除 stale bitmap 提交 | P1 |
| Phase 3: Detail Canvas | 大（~400 行） | 视口区域始终清晰 | P2 |
| Phase 4: 自适应策略 | 中（~100 行） | 根据缩放速度动态优化 | P3 |

**建议先做 Phase 1**——这是投入产出比最高的改动，直接对齐 PDF.js 的核心策略。

---

## 五、与现有架构的兼容性

| 现有组件 | Phase 1 后的状态 |
|---|---|
| `resolve_wheel_render_decision` | 保留，但 tick loop 不再每帧调用 |
| `resolve_preview_render_zoom` | 保留，settled 后渲染使用 |
| `applyPreviewFrame` | 保留，纯 CSS preview |
| `applyCommittedFrame` | 保留，settled 后提交 |
| `resolve_settled_transform` | 保留，settle 过渡使用 |
| `resolve_css_transform_string` | 保留，CSS transform 字符串 |
| `wheel_render_idle_ms` | 废弃（drawingDelay 替代） |
| `resolve_preview_tick_decision` | 简化（不再决定是否渲染） |

---

## 六、风险评估

| 风险 | 影响 | 缓解 |
|---|---|---|
| Drawing delay 导致 settle 后有 150ms 模糊 | 低（用户已停手） | 可调参数，50-200ms |
| 渲染取消导致 bitmap 缺失 | 中 | Fallback 到 CSS preview |
| Detail canvas 增加内存 | 中 | 限制 detail canvas 尺寸 |
| 自适应策略增加复杂度 | 中 | 先不做 Phase 4 |

---

## 七、总结

行业标准的核心洞察：**CSS transform 是零成本的即时预览，bitmap 渲染是昂贵的最终提交。两者不应在同一帧竞争。**

我们的架构需要从「每帧渲染 + stale guard 过滤」转向「CSS preview + drawing delay + 一次提交」。这与 PDF.js、Chrome、MuPDF 的核心策略一致。
