# ADR-0012: 缩放手势刷新只渲染可见视口（viewport-tile 判据）

## Status

Accepted

## Context

### 实测（E2E，2026-09-30/10-01，真实 WebView2）

`zoom_perf_probe` + `zoom_p2_probe`（16 次 ctrl-wheel 突发）+ 录屏逐帧分析
（`jcv7NqPQlt.mp4`，2560×1360@30fps，248 帧）一致显示 P2 卡顿的机制：

1. **每次 wheel 步进触发 ~1.3 次全页 reknock 渲染**（`page-render-duration
   reason=zoom`，单次 32→131ms，随 zoom 增长）。
2. **主线程被阻塞 100–126ms/次**（5ms 心跳定时器间隙 117–139ms、rAF 间隙
   80–203ms），整段手势期间主线程近乎占满 → rAF 饿死 → 动画"突进—卡死"。
3. 阻塞的**不是** Vello 渲染本身（worker 渲染仅 2–6ms），而是全页位图的
   **canvas 重分配 + 拷贝**：`mainStageCanvas` 被重设为
   `page × displayZoom × dpr`（本例最高 3920×5546 ≈ **21.7M 像素**），
   每次 reknock 都重分配并重拷。
4. **瓦片流被饿死**：`tileFacade` 队列 15、ready 0（主线程被占满，瓦片无法
   present），用户只能看到被拉伸的全页位图。
5. 录屏（修复后 build）：活动窗口 5.83s 内 **69% 的帧完全冻结**，27 个离散
   台阶，台阶间 130–400ms 静止。

### 结构性根因（铁律 §5：同一病灶两处表现）

`resolve_render_zoom_result`（`plan_builder.rs`）的视口瓦片判据是**纯内存守卫**：

```
safe_render_zoom = max_canvas_dim / (page_max × dpr)      // 10240/(842×1.25) = 9.73
use_viewport_tile = display_zoom > safe_render_zoom
```

即只有当全页位图会撞上 10240px canvas 上限时才走"只渲染视口"的廉价路径。
于是在常见缩放区间（<9.7×），**每次 reknock 都整页重渲**——而手势期间用户
只能看到视口（≈1.5M 像素），其余 90% 以上的像素是给看不见的区域算的。

这与 >9.7× 时已验证可用的路径（base 复用 + detail 视口瓦片，`css_scale=1`、
清晰、便宜）是**同一机制**，只是判据缺了一个"成本/是否手势刷新"维度。
这是判据不完整，不是缺机制——所以修复是补一个判据，不是新增一条渲染链
（避免铁律 §1 链分叉）。

### 为什么不能用 `preview_settled` 区分

reknock 在**派发时**（`!settled`，visual≠target）发起，但 plan 是**异步构建**
的；到构建时动画往往已收敛（visual≈target），`preview_settled=true`，
故 `render_base_layer=true`（实测日志证实）。所以不能用"是否收敛"判别
"这次渲染是手势中途刷新还是最终渲染"。

**可用且可靠的判据**：派发时捕获的 `request.display_zoom` 与当前
`target_zoom` 是否一致。中途刷新时动画已继续前进，二者必有偏差；
最终渲染二者相等。

## Decision

### 1. `RenderZoomRequest` 增加 `prefer_viewport_tile`

```rust
pub struct RenderZoomRequest {
    ...
    /// 手势中途刷新只需可见视口（ADR-0012）：全页位图的成本是
    /// O(page × zoom²) 的主线程重分配+拷贝，每个 wheel 步一次。
    pub prefer_viewport_tile: bool,
}
```

### 2. 判据并入 `use_viewport_tile`（OR，不改语义）

```rust
let use_viewport_tile = display_zoom > safe_render_zoom + 0.001
    || request.prefer_viewport_tile;
```

### 3. UI 层派生：`display_zoom != target_zoom` ⇒ 手势刷新

`build_frame_plan_result` 在解析 render zoom **之前**读 `zoom_state.target_zoom`：

```rust
let gesture_refresh = (request.display_zoom - target_zoom).abs() > PREVIEW_SETTLED_EPSILON;
// → RenderZoomRequest { prefer_viewport_tile: gesture_refresh, .. }
```

最终渲染（display_zoom == target_zoom）不走此路径，**稳态行为不变**。

### 4. 级联效果（复用既有策略，零新机制）

`use_viewport_tile=true` 后既有策略自然生效：
- `requires_preview_base_refresh` 含 `!use_viewport_tile` ⇒ **base 不再每步重渲**，
  复用当前 base 并被 CSS 缩放（ADR-0004 css_scale 机制已在 >9.7× 验证）；
- `render_detail_layer = use_viewport_tile && !has_reusable_detail_tile`
  ⇒ 只渲染**视口尺寸**的 detail 瓦片（≈1.5M 像素，清晰、无放大补偿）；
- 瓦片流（ADR-0009）拿回主线程时间片，queue/ready 恢复流动。

## Consequences

### 验证（修复后，2026-10-01）

- **单元契约** `present/plan_tests.rs`：RED（2 个手势刷新用例失败）→ GREEN，
  wasm-pack **15/15**；core **265/265**；clippy wasm32 0 warnings；vitest 118/118。
- **E2E 回归**：zoom 套件 **9/9**（帧契约 canvas 漂移 max **0.00%**、
  表面契约 152 帧 unpainted=0 / flickers=0）、文档生命周期 2/2。
- **性能探针**（`zoom_p2_probe`，16 次 wheel 突发）：
  - longtask（>50ms）**22 → 5**，最长 **126ms → 70ms**；
  - rAF 间隙 >40ms 的帧占比降至 ~14%。
- **未解决（诚实记录）**：每个 wheel 步仍有 **~100ms 的主线程阻塞**，且
  **与 zoom 无关**（visual 1.03→5.28 全程 ~100±15ms）——与位图尺寸不相关，
  故不是全页渲染（本 ADR 已消除的路径）所致，而是另一处固定成本
  （疑似：wheel → 虚拟 zoom 布局写入后的 reflow/paint，或瓦片流每步的
  固定开销）。**需 CDP CPU profile 定位**，属下一步（ADR-0013 候选）。

### Positive

1. 手势中途每次渲染的成本从 O(page×zoom²)（≈21.7M px）降到 O(viewport×dpr²)
   （≈1.5M px），**~14×**；中途 reknock 不再把主线程阻塞 100–126ms。
2. 高 zoom 下**可见区域更清晰**：视口瓦片按 display_zoom 原生分辨率渲染，
   不再依赖全页位图的 css_scale 拉伸。
3. 与 >9.7× 路径同一条链，无新增渲染通道（铁律 §1/§2）。

### Negative / 技术债

1. 手势期间**视口之外**的区域显示被 CSS 拉伸的旧 base（低分辨率）——
   与 >9.7× 现状一致，且用户此时看的是视口。
2. base 的实际重渲推迟到 settle（`display_zoom == target_zoom`）或
   cache_zoom 量化步进跨档时，稳态渲染次数不变。
3. P5（缩放中的渐进清晰）仍取决于瓦片流的吞吐，本次不单独立 ADR。
4. **遗留**：每 wheel 步 ~100ms 的 zoom 无关阻塞未定位（见上"未解决"）。
