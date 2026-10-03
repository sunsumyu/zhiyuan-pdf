# ADR-0009: 手势期瓦片流式渲染（接通 ADR-0004 的 zero-stretch 路径）

## Status

Accepted

## Context

用户需求（ADR-0003 原话）："不要用css这种方式，css这种拉伸方式不可能有高分辨率"；
ADR-0004 修订版承诺了零中间拉伸的路径："512×512 viewport tiles render in O(10ms)
each, making per-frame re-render at visual_zoom feasible. … This is the follow-up
work."

现状（commit 5266a48 引入）违背了这条路线：手势期主表面是主 canvas 的 CSS
`scale(visual/lastRendered)` 拉伸位图（用户明确拒绝的模糊拉伸），瓦片泵在
`gap > 2%` 时完全不工作；瓦片层在手势开始时被整体 `display:none`，settle 时
恢复——两层表面坐标系不一致，正是用户持续报告的"一闪一闪、像素错位"。

### 排查过的替代方案（行业对比，2026-09-29 会话）

| 方案 | 中间清晰度 | 结论 |
|---|---|---|
| PDF.js / Chrome：CSS 预览 + 停手重绘 | 模糊（CSS 插值） | 违背用户需求，否决 |
| SumatraPDF：档位跳变 + 立即重绘 | 锐利但不连续 | 违背平滑需求，否决 |
| Mapbox GL：GPU 矢量逐帧 | 锐利且连续 | 金标准；vello-wasm+WebGPU 为其本项目形态 |
| **本 ADR：瓦片流式逐帧（CPU 光栅）** | 锐利且连续 | 既有管线内可行，采纳 |

### 历史：vello 头less 渲染失败的真正原因（fdde982 删除的 vello_renderer.rs）

native wgpu headless 渲染链 = GPU 渲染 → `copy_texture_to_buffer` →
`map_async(Read)` CPU 回读 → ImageBuffer → Tauri IPC → WebView 显示。
O(100ms) 大头在回读 + IPC，不在 vello 光栅化。vello-wasm + WebGPU（WebView2
已支持）恰好消除这两项，是后续 prototype 候选（另行立项），不阻塞本 ADR。

## Decision

接通已有的 Rust 增量调度（`TileScheduler::update_animation` →
`schedule_incremental_tiles`，每 3 帧按 visualZoom 出请求），并以**统一呈现
公式**取代瓦片层隐藏开关：

### 1. 单一事实源 `tileZoomIntent`

```
tileZoomIntent(zs) = animating ? quantize(visualZoom, 0.03) : targetZoom
```

所有瓦片请求的有效性判断（pumpRequest / drawTile 的 stale 检查）统一引用它。
手势期新增行为 = 改这一个函数的返回值语义，不再向各判断点散布手势分支。

### 2. 统一呈现公式（取代隐藏开关）

每张瓦片呈现时（无条件）：

```
canvas.style.transform = scale(visualZoom / tile.renderZoom)
```

- settle 瓦片：renderZoom ≈ visual → scale ≈ 1，恒等无感。
- 手势增量瓦片：renderZoom = 量化 visualZoom → scale ≤ ~3%，微补偿，
  无可见模糊。
- **旧 zoom 瓦片在新手势中继续有效且对齐**（scale = visual/旧zoom），
  直到被 LRU 挤掉——`clearDom` 于手势开始不再必要，`setTilesHidden`
  整个删除。

两层表面（主 canvas 拉伸 + 瓦片）从此共用同一几何公式：页面点 p 都落在
`p × visualZoom`。错位的结构性根源消除。模糊的主 canvas 逐渐被清晰瓦片
覆盖（Mapbox 式渐进），settle 不再有"模糊→清晰"的突变。

### 3. 手势期泵接线

- `tick` far-from-settle 分支：`beginGesture` 后调用
  `tileFacade.updateAnimation(tileZoomIntent(zs), epoch)` 唤醒 Rust 增量
  调度，随后与 settle 路径共用同一段泵逻辑（提取 `pumpNext`）。
- frame token 策略：zoom 量化档变化时才 `nextEpoch()`——Rust
  `next_render_request` 会丢弃 token 过期的请求，逐帧 bump token 会让
  增量请求永远渲染不出来。
- visualZoom 在 TS 侧量化（0.03 步长）后喂给 Rust：未量化的 f32 会产生
  每帧不同的 cache key，缓存永远不命中。

## Consequences

### Positive

1. 手势期可见内容 = 原生分辨率瓦片（微 scale 补偿 ≤3%），中间模糊消除。
2. 双表面统一几何公式，像素错位的结构性根源消除。
3. 删除 `tilesHiddenForGesture` / `setTilesHidden` 状态机——手势行为从
   "修改隐藏开关"变为"扩展 intent 函数"，符合开闭原则。
4. 复用 Rust 侧已实现并有测试的增量调度，零 Rust 改动（本 ADR 范围内）。

### Negative

1. 手势期瓦片渲染负载增加（每 3% zoom 档一批视口瓦片，串行 ~10ms/张）。
2. 量化步长（0.03）与 Rust `render_interval`（3 帧）是两个需要共同调参的
   旋钮，极端快手势下可能出现瓦片批积压（frame token 过滤会丢弃过期批，
   表现为该档瓦片缺失，由主 canvas 拉伸兜底）。

### Mitigations

1. LRU（12 张）自动淘汰旧 zoom 档瓦片。
2. 主 canvas CSS 拉伸保留为未覆盖区域的兜底表面（同一几何公式，无错位）。
3. E2E 探针验证：手势中期截屏页面实宽应 ≈ 595 × visualZoom（内容跟随缩放），
   且无 <300px 的冻结旧瓦片残影（2026-09-27 帧分析的验收口径）。

## Tests

- `tile_layer` vitest：intent 量化、呈现 scale 公式、手势期泵调用序列。
- 既有 zoom E2E 套件（7 spec）全绿。
- `zoom_frame_probe`：手势中期表面采样无 multi-surface 错位帧。

## References

- ADR-0003（tile 架构）、ADR-0004（revised，zero-stretch 路径承诺）
- ADR-0006（SetBox）、ADR-0008（光标锚定）
- `crates/pdf-viewer-core/src/render/tile_scheduler.rs`（增量调度，已实现）
- 行业对比研究：2026-09-29 会话（PDF.js / SumatraPDF / Mapbox GL）
