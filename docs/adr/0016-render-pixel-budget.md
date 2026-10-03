# ADR-0016: 渲染位图像素预算（settle 全页渲染的去楔死）

## Status

Accepted

## Context

### 用户实测（2026-10-01）

ADR-0012~0014 落地后用户仍报告"会卡住"。CDP 剖析（verbose 关，生产路径）
定位：每次手势**结束时的 settle 全页渲染**产生 190–264ms 的一次性冻结
——这正是用户感知的"松开滚轮时卡一下"。

### 根因

`use_viewport_tile` 的判据（ADR-0012 后）是：

```
use_viewport_tile = display_zoom > safe_render_zoom(=10240/(page_max·dpr)) || 手势中途刷新
```

- 手势**中途**刷新走视口瓦片（ADR-0012，已修）；
- 但 **settle 渲染**（display_zoom == target）判据为 false → 每次都
  **整页重渲**：位图 `page × displayZoom × dpr`，5.28× 时 3140×4427 ≈
  **13.9M px**，canvas 重分配 + 全页 drawImage ≈ 190–264ms。

而 >9.7× 的既有稳态早就证明了正确架构：base 按 clamp 缩放（ADR-0004
css_scale 机制）+ detail 瓦片按原生分辨率覆盖视口。判据缺的仍是同一维度：
**成本预算**（ADR-0012 补了"手势中途"，本 ADR 补"位图尺寸"）。

### 预算的取值：相对视口，不是绝对常数

视口外的像素是看不见的细节。base 位图维持在 **2× 视口像素**以内即可
（超出部分由 detail 瓦片按原生分辨率覆盖）。以视口为基准使该预算
自适应屏幕大小/DPR。

## Decision

### 1. `RenderZoomRequest` 增加 `max_render_pixels`

由 `build_frame_plan_result` 从视口推导（TS 无需改动）：

```rust
let viewport_px = viewport_width * viewport_height * dpr * dpr;
let max_render_pixels = (viewport_px * 2.0).max(1_000_000.0);
```

### 2. 预算并入 clamp 与判据

```rust
let budget_zoom = (max_render_pixels / (page_w*page_h*dpr*dpr)).sqrt().max(0.1);
let effective_cap = safe_render_zoom.min(budget_zoom);   // 取更严者
let use_viewport_tile = display_zoom > effective_cap + 0.001 || prefer_viewport_tile;
let render_zoom = if use_viewport_tile { display_zoom } else { display_zoom.min(effective_cap) };
let base_render_zoom = display_zoom.min(effective_cap);
```

`max_render_pixels <= 0` 关闭预算（退回旧行为），便于回滚与对照。

### 3. 级联（全部复用既有机制，零新链路）

- `css_scale`：settle 时 `use_viewport_tile=true` → `css_scale=1.0`（不变）；
  base 的放大由 `resolveCanvasCssBox`（`display × base_render_zoom/display_zoom`）
  与 `CanvasTransformOwner`（ADR-0010）完成——位图 `page×base_render_zoom×dpr`，
  CSS box `page×display_zoom`；
- `show_detail_overlay`（settle 时为 true）：**视口由原生分辨率 detail 瓦片
  覆盖**，可见区域始终清晰；视口外才是放大的 base；
- base 缓存按 `quantize(base_render_zoom, false)`：display 超过 budget 后
  `base_render_zoom ≡ budget_zoom` 恒定 → 量化档位不变 → **settle 不再反复整页重渲**；
- 滚动时 `resolveViewportRefresh` 重渲 detail 瓦片，可见区域持续清晰。

### 4. 阈值（本仓库 fixture：595×842，viewport 1200×800@dpr1.25）

- 视口像素 = 1200×800×1.25² = 1.5M；预算 = 3.0M；
- 页像素/zoom² = 595×842×1.5625 ≈ 0.783M；
- `budget_zoom = sqrt(3.0/0.783) ≈ 1.96×`（旧内存护栏 ≈9.73×）。

即 settle 判据从 9.73× 前移到 ≈1.96×。代价：视口外 base 变模糊（与 >9.73×
现状一致）；收益：5.28× 的 settle 位图从 13.9M px 降到 ≤3M px（base）
+ 视口 detail。

### 5. 红灯契约（先红后绿）

- `crates/pdf-viewer-core/.../plan_builder.rs::budget_tests`（4）：预算关闭时
  5.28× 仍整页（旧行为）；预算 3M 时 5.28× 翻视口瓦片且 base 位图 ≤ 预算；
  预算内 settle 不变；预算只收紧不放松内存护栏。
- `crates/pdf-viewer-ui/src/present/plan_tests.rs`（2 新）：settle 3.0× 翻视口
  瓦片、`render_zoom==3.0`、`base_render_zoom<3.0`、`css_scale==1.0`；
  settle 1.5× 保持整页。


## Consequences

### Positive

1. settle 冻结从 190–264ms 降到像素预算对应的量级（~3.5×+ 减幅），
   且 zoom 超过 budget 后 settle 只渲视口瓦片（base 复用）。
2. 与 >9.7× 完全同一条链路，仅阈值前移（铁律 §1/§2）。
3. 可见区域清晰度不变（detail 瓦片原生分辨率）。

### Negative / 技术债

1. 视口外的 base 是放大后的低分辨率位图——与 >9.7× 现状一致；
   快速滚动时视口边缘可能短暂看到模糊 base，直到 detail 瓦片补齐。
2. 预算系数（2× 视口）是经验值，可后续按实测调优。

## Verification (2026-10-02)

生产路径（verbose 关）CDP 实测，viewport 945×681@dpr1.25：

| 指标 | ADR-0016 前 | ADR-0016 后 |
| --- | --- | --- |
| settle 4.95× 全页位图 | 19.2M px | **2.01M px**（base） |
| settle 主线程冻结 | 190–264ms | **0 longtask** |
| settle `useViewportTile` | false | **true** |
| settle `renderBaseLayer` | true | **false**（reuse base） |
| settle `baseRenderZoom` | 4.95 | **1.603**（= budget_zoom） |
| p99 帧间隔 | — | 42ms |

`resolveFramePlan` 扫描：base 位图从 1.5× 的 1.76M px 起，到 1.9× 封顶
2.01M px 并恒定到 12×——base 缓存档位不再随 display 变化。

回归门：`zoom_gesture_frame_contract`（304 帧 0 漂移/0 错位/0 隐藏）、
`zoom_surface_painted_contract`、`zoom_tile_layer_gesture`、
`zoom_wheel_sudden_jump`、`load_pdf` 全绿；wasm 16/16、core 269/269、
vitest 126/126、clippy 双 crate 干净。
