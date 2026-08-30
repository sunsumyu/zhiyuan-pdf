# 缩放 RAF 循环 Rust 化：消除 WASM 往返 + clone 开销

> 分支 `refactor/architecture-improvements` · 2026-08-23
>
> 基于 improve-codebase-architecture + grilling 流程产出的设计方案。

---

## 一、核心问题

缩放热路径（wheel 事件 + RAF tick）每帧产生 4-5 次 WASM 往返，每次都经过 `serde_wasm_bindgen` 序列化 + `HostZoomState` 深拷贝。在 16ms 帧预算中，这些开销直接导致卡顿。

---

## 二、设计方案

### 2.1 架构变更

```
BEFORE (每帧 4-5 次 WASM 往返):
  TS RAF → tickZoomState() → Rust → DomOps → TS 执行 DOM
  TS Wheel → resolveWheelRequestParams() → WASM
           → handleWheelZoomHost() → WASM
           → readZoomSnapshot() → WASM (clone)
           → resolveCssTransformString() → WASM
           → startSmoothZoomPreview()

AFTER (零 WASM 往返/帧):
  Rust RAF loop (thread_local closure):
    ├─ advance_zoom_animation_state()
    ├─ should_render() → decide render target
    ├─ web-sys DOM: set CSS transform, update scroll
    ├─ poll committed_frame_queue
    └─ request_animation_frame() if !settled

  TS (仅初始化 + render pipeline 回调):
    ├─ bind wheel event → Rust onWheelEvent()
    ├─ render pipeline → Rust commitRenderedFrame()
    └─ 无 RAF 参与
```

### 2.2 关键决策

| # | 决策 | 选择 | 理由 |
|---|------|------|------|
| Q1 | Wheel 管线边界 | 完整接管 | 消除 4-5 次 WASM 调用为 1 次 |
| Q2 | RAF 驱动者 | Rust 驱动 | RAF 循环在 Rust 内部，零跨边界开销 |
| Q3 | 状态访问模式 | 全面迁移 with_zoom_state | 消除 clone，统一访问模式 |
| Q4 | Rust RAF 实现 | wasm_bindgen closure | Rust 调用 requestAnimationFrame |
| Q5 | WASM API 变更 | 新增 + 立即删除旧 | 干净的 API 表面 |
| Q6 | DOM 执行 | web-sys | Rust 直接操作 DOM，无回调 |
| Q7 | 迁移策略 | Big Bang | 无过渡期，状态一致 |

### 2.3 新增 Rust API

```rust
// ─── RAF 循环 ────────────────────────────────────────────────────

/// 启动缩放 RAF 循环。Rust 内部调用 requestAnimationFrame，
/// 每帧推进状态机 + 通过 web-sys 操作 DOM。
pub fn start_zoom_raf_loop();

/// 停止缩放 RAF 循环。
pub fn stop_zoom_raf_loop();

// ─── Wheel 事件 ──────────────────────────────────────────────────

/// 完整 wheel 事件处理。TS 只采集 DOM 原始值传入。
#[derive(Serialize, Deserialize)]
pub struct WheelEventInput {
    pub delta_y: f32,
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
    pub timestamp_ms: f64,
}

#[derive(Serialize, Deserialize)]
pub struct WheelEventOutput {
    pub target_zoom: f32,
    pub visual_zoom: f32,
    pub css_scale: f32,
}

pub fn on_wheel_event(input: WheelEventInput) -> WheelEventOutput;

// ─── Render pipeline 回调 ────────────────────────────────────────

/// 渲染管线提交帧时调用。Rust 内部管理 committed frame 队列。
pub fn commit_rendered_frame(
    display_zoom: f32,
    render_zoom: f32,
    host_width: f32,
    host_height: f32,
    content_left: f32,
    content_top: f32,
    scroll_left: f32,
    scroll_top: f32,
);
```

### 2.4 删除的 WASM 导出

| 导出名 | 原因 |
|--------|------|
| `resolveWheelRequestParams` | 合并入 `on_wheel_event` |
| `handleWheelZoomHost` | 合并入 `on_wheel_event` |
| `resolveWheelRenderDecision` | Rust RAF 循环内部决策 |
| `resolvePreviewTickDecision` | Rust RAF 循环内部决策 |
| `resolveZoomCommitDecision` | Rust RAF 循环内部决策 |
| `resolveFlushDecision` | Rust RAF 循环内部决策 |
| `resolveCssTransform` | Rust RAF 循环内部决策 |
| `resolveCssTransformString` | Rust RAF 循环内部决策 |
| `resolveSettledTransform` | Rust RAF 循环内部决策 |
| `readZoomSnapshot` | Rust 内部直接读 state |
| `readZoomState` | 改为 `with_zoom_state` 闭包 |
| `getZoomState` (deprecated) | 同上 |
| `setWheelRenderPending` | Rust 内部状态 |
| `getWheelRenderPending` | Rust 内部状态 |
| `queueCommittedFrame` | Rust 内部队列 |
| `takeReadyCommittedFrame` | Rust 内部队列 |
| `takeCancelPendingRender` | Rust 内部状态 |
| `cancelDrawingDelay` | Rust RAF 循环内部管理 |
| `clearPendingAnchor` | Rust RAF 循环内部管理 |
| `clearPreviewPresent` | Rust RAF 循环内部管理 |
| `setTargetZoom` | Rust RAF 循环内部管理 |
| `markRenderedZoom` | Rust RAF 循环内部管理 |
| `stepPreviewHost` | Rust RAF 循环内部管理 |

**保留的 WASM 导出：**
| 导出名 | 原因 |
|--------|------|
| `resetZoomState` | 初始化需要 |
| `syncHostLayout` | 布局计算（非缩放专用） |
| `resolveFramePlan` / `takeFramePlan` | 渲染管线使用（非缩放专用） |
| `scheduleRenderFrame` / `commitRenderResult` / `settleRenderFrame` | 渲染管线 |
| `resolveFitToWidth` | 非缩放专用 |
| `resolveLayoutFallback` | 布局回退 |
| `resolveCanvasCssBox` | Canvas 尺寸计算 |
| `isImmediateMutationFrame` | 渲染管线 |
| `renderPage` / `renderPageOffscreen` / progressive 系列 | 渲染管线 |
| `MIN_ZOOM` / `MAX_ZOOM` | 常量 |

### 2.5 web-sys DOM 操作

Rust RAF 闭包内直接操作 DOM：

```rust
use web_sys::{Window, Document, Element, HtmlElement, CssStyleDeclaration};

fn apply_css_transform(container: &Element, css_scale: f32, translate_x: f32, translate_y: f32) {
    let style = container.style();
    if css_scale == 1.0 && translate_x == 0.0 && translate_y == 0.0 {
        style.set_property("transform", "").unwrap();
    } else {
        let transform = resolve_css_transform_string(translate_x, translateY, css_scale);
        style.set_property("transform", &transform).unwrap();
    }
    style.set_property("transform-origin", "0 0").unwrap();
}

fn update_scroll(scroll_container: &Element, scroll_left: f32, scroll_top: f32) {
    let el: &HtmlElement = scroll_container.dyn_ref().unwrap();
    el.set_scroll_left(scroll_left as i32);
    el.set_scroll_top(scroll_top as i32);
}

fn update_layout(container: &Element, left: f32, top: f32, width: f32, height: f32) {
    let style = container.style();
    style.set_property("left", &format!("{}px", left)).unwrap();
    style.set_property("top", &format!("{}px", top)).unwrap();
    style.set_property("width", &format!("{}px", width)).unwrap();
    style.set_property("height", &format!("{}px", height)).unwrap();
}
```

### 2.6 RAF 循环实现

```rust
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

thread_local! {
    static RAF_HANDLE: RefCell<Option<i32>> = RefCell::new(None);
    static RAF_CLOSURE: RefCell<Option<Closure<dyn FnMut(f64)>>> = RefCell::new(None);
}

pub fn start_zoom_raf_loop() {
    RAF_HANDLE.with(|handle| {
        if handle.borrow().is_some() { return; }

        let window = web_sys::window().unwrap();

        // Create and store the closure
        let closure = Closure::once_into_js(move |timestamp_ms: f64| {
            tick_and_schedule(timestamp_ms);
        });

        let handle_val = window.request_animation_frame(closure.as_ref().unchecked_ref()).unwrap();
        *handle.borrow_mut() = Some(handle_val);
        *RAF_CLOSURE.borrow_mut() = Some(closure.into());
    });
}

fn tick_and_schedule(timestamp_ms: f64) {
    ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();

        // 1. Advance animation
        let step = advance_zoom_animation_state(&mut state, Some(timestamp_ms));

        // 2. Apply CSS transform via web-sys
        if let Some(window) = web_sys::window() {
            if let Some(doc) = window.document() {
                if let Some(container) = doc.get_element_by_id("vector-canvas-container") {
                    apply_css_transform(&container, step.css_scale, 0.0, 0.0);
                }
            }
        }

        // 3. Check committed frame queue
        // ... poll and apply

        // 4. Decide whether to continue
        if !step.settled {
            // Schedule next frame
            let window = web_sys::window().unwrap();
            let closure = Closure::once_into_js(move |ts: f64| {
                tick_and_schedule(ts);
            });
            let handle = window.request_animation_frame(closure.as_ref().unchecked_ref()).unwrap();
            RAF_HANDLE.with(|h| *h.borrow_mut() = Some(handle));
            RAF_CLOSURE.with(|c| *c.borrow_mut() = Some(closure.into()));
        } else {
            // Settled — start drawing delay, then render final
            RAF_HANDLE.with(|h| *h.borrow_mut() = None);
            RAF_CLOSURE.with(|c| *c.borrow_mut() = None);
        }
    });
}
```

### 2.7 TS 侧变更

`zoom_controller.ts` 从 795 行缩减到 ~150 行：

```typescript
// zoom_controller.ts — 新的简化版

export function createZoomController(deps: ZoomControllerDeps): ZoomController {
    let wheelBound = false;

    function bindWheelZoom(): void {
        if (wheelBound) return;
        const scrollContainer = deps.getScrollContainer();
        if (!scrollContainer) { setTimeout(bindWheelZoom, 250); return; }

        scrollContainer.addEventListener('wheel', (event: WheelEvent) => {
            if (!(event.ctrlKey || event.metaKey) || !deps.getCurrentPath()) return;
            event.preventDefault();
            event.stopPropagation();

            const rect = scrollContainer.getBoundingClientRect();
            // Rust 接管全部逻辑
            deps.onWheelEvent({
                deltaY: event.deltaY,
                viewportX: event.clientX - rect.left,
                viewportY: event.clientY - rect.top,
                viewportWidth: scrollContainer.clientWidth,
                viewportHeight: scrollContainer.clientHeight,
                pageWidth: deps.getCurrentPageWidth(),
                pageHeight: deps.getCurrentPageHeight(),
                scrollLeft: scrollContainer.scrollLeft,
                scrollTop: scrollContainer.scrollTop,
                timestampMs: performance.now(),
            });
        }, { passive: false });

        wheelBound = true;
    }

    function commitRenderedFrame(frame: RustAnchorFramePlan): void {
        // 直接调用 Rust，不再有复杂的决策逻辑
        deps.commitRenderedFrame(frame);
    }

    return { bindWheelZoom, commitRenderedFrame, ... };
}
```

---

## 三、文件改动清单

### Rust Core (`pdf-viewer-core`)

| 文件 | 改动 |
|------|------|
| `render/zoom/state.rs` | 新增 `RAFHandle` 状态、`CommittedFrameQueue` |
| `render/zoom/decision.rs` | 新增 `on_wheel_event()`, `start_zoom_raf_loop()`, `stop_zoom_raf_loop()` |
| `render/zoom/animation.rs` | RAF 闭包内的 tick 函数，web-sys DOM 操作 |

### Rust UI/WASM (`pdf-viewer-ui`)

| 文件 | 改动 |
|------|------|
| `zoom/zoom_controller.rs` | 新增 WASM 导出 `onWheelEvent`, `startZoomRafLoop`, `commitRenderedFrame` |
| `zoom/zoom_store.rs` | `read_zoom_state()` 改为 `with_zoom_state` 闭包 |
| `zoom/free_api.rs` | 删除 20+ 旧导出，新增 3 个新导出 |
| `render/free_api.rs` | 删除被合并的决策函数导出 |

### TS

| 文件 | 改动 |
|------|------|
| `zoom/zoom_controller.ts` | 795 行 → ~150 行。只保留 init + RAF 注册 |
| `render/render_wasm_api.ts` | 删除旧 API 类型，新增 3 个新类型 |
| `render/frame_plan.ts` | 删除旧 adapter 方法 |
| `viewer/pdf_runtime.ts` | 更新 zoom controller deps |
| `__tests__/zoom_anti_flash.test.ts` | 重写为测试新架构 |

### Cargo.toml

| 文件 | 改动 |
|------|------|
| `crates/pdf-viewer-core/Cargo.toml` | 新增 `web-sys` 依赖（features: Window, Document, Element, HtmlElement, CssStyleDeclaration） |

---

## 四、实施步骤

### Step 1: Rust RAF 循环骨架 (~200 行)
1. `Cargo.toml` 添加 `web-sys` features
2. `render/zoom/state.rs` 新增 RAF 状态
3. `render/zoom/decision.rs` 实现 `start_zoom_raf_loop`, `stop_zoom_raf_loop`
4. 实现 RAF 闭包 + web-sys DOM 操作

### Step 2: on_wheel_event (~150 行)
1. `render/zoom/decision.rs` 实现 `on_wheel_event`
2. 合并参数计算 + 状态更新 + CSS 决策
3. 返回 `WheelEventOutput` 给 TS（仅用于 syncZoomSelect 等）

### Step 3: WASM 导出 + 旧导出删除 (~100 行)
1. `zoom/free_api.rs` 新增 3 个导出
2. 删除 20+ 旧导出
3. `render/free_api.rs` 删除被合并的导出

### Step 4: TS 重写 (~-600 行)
1. `zoom_controller.ts` 重写为 ~150 行
2. `render_wasm_api.ts` 更新类型
3. `frame_plan.ts` 删除旧 adapter
4. `pdf_runtime.ts` 更新 deps

### Step 5: 测试更新 (~+40 行)
1. `zoom_anti_flash.test.ts` 重写
2. 新增 Rust 单测：`on_wheel_event`, RAF 循环状态机

---

## 五、风险与缓解

| 风险 | 缓解 |
|------|------|
| web-sys DOM API 不够用 | 用 `dyn_ref::<HtmlElement>()` 获取高层 API，必要时用 `js_sys` 调用 JS 函数 |
| RAF closure 内存泄漏 | `Closure::once_into_js` + `RAF_CLOSURE` thread_local 管理生命周期 |
| 缩放期间其他 DOM 操作（editor overlay 等）冲突 | RAF 闭包内只操作 zoom 相关元素，不触碰其他 DOM |
| render pipeline 的 committed frame 时序 | 用 `CommittedFrameQueue` + RAF 内 poll，避免竞态 |
| 回归：缩放行为变化 | 保留现有 Rust 单测，新增 RAF 循环行为测试 |
