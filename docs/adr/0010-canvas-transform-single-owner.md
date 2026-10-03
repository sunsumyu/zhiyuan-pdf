# ADR-0010: Canvas 几何单一所有者（boxZoom 事实源 + 单写者）

## Status

Accepted

## Context

### 四轮症状端修补的教训（违反 architecture-principles §5）

2026-09-29/30 四轮录屏驱动的修复（平移项 → presentScale → 统一公式 →
settle 窗口）全部在症状端打补丁，每轮引入或暴露下一个缺陷。§5 链分叉诊断
信号——"修一处后另一处出新 bug"——逐条命中。对照铁律复盘，结构性根因有二：

1. **canvas transform 三个写者**：Rust `apply_canvas_visual_scale`（RAF
   每帧）、TS `presentViewportCanvas`（'none'）、TS `commitVectorRenderResult`
   （presentScale 回调）——互相覆盖、竞态（violates ADR-0002 presentation
   single writer 与铁律 §2 单一所有者）。
2. **"canvas box 处于哪个缩放空间"这一关键事实无 owner**：三个写者各自
   猜测 box 等于 lastRendered / displayZoom / renderZoom 之一。异步时序中
   三者互不相等——位图复用/跳渲染的帧 commit（更新 lastRendered）但不
   present（box 停旧档）——猜测必然间歇性出错（violates 铁律 §2、§5）。

### 实证（zoom_frame_probe 14 档大跨度连滚，2026-09-30）

- 快速缩放段 canvas 与瓦片视觉共同偏离 ZOOM_STATE.visual 6-12%
  （f39: box=0.658 档、lastRendered=0.700、visual=0.700 → 视觉 0.658）
- tile tick 依据 ZOOM_STATE 的假 settled（visual==target 但 box 滞后）
  睡眠 → 瓦片位置/缩放冻结在 drawTile 时刻
- settle 渲染窗口（displayZoom=收敛中值、RAF 停止、presentScale 竞态）
  → settle 后永久 ~2% 缩放偏差

## Decision

### 1. CanvasTransformOwner —— transform 唯一写者 + boxZoom 唯一事实源

```
CanvasTransformOwner（TS，tile_layer 同族模块）
  状态: boxZoom —— canvas CSS box 的实际档位，全系统唯一事实源
  接口:
    presentFrame(displayZoom)   → boxZoom = displayZoom（唯一更新点）
    sync(visualZoom)            → transform = scale(visualZoom / boxZoom)
    reset()                     → boxZoom = 1（文档/页面重置）
  不变量（任何时序下成立，由帧契约逐帧断言）:
    canvas 视觉宽 = pageWidth × visualZoom
```

### 2. Rust 退出 transform 写入

`apply_canvas_visual_scale` 删除（Rust RAF 只推进 ZOOM_STATE 动画状态——
它已有的职责）。写者收敛为 TS tile tick，满足 ADR-0002。

### 3. tile tick 常驻 sync（值比较短路）

tick 每帧调用 `sync(visualZoom)`；visual 与 boxZoom 均未变时跳过 DOM 写。
睡眠判定不再依赖 ZOOM_STATE 的 settled（P3 根除）：只要 boxZoom ≠ visual
就持续补偿，settle 渲染窗口（P4）自动覆盖。

### 4. 坐标映射归属（铁律 §2/§3 收敛项，本次一并记录）

瓦片呈现公式 `left = rect.left × (visual/Zr)` 属于坐标转换，原则 owner 是
`core::coordinate_transform`。本次不迁移（避免大范围改动），记录为技术债：
tile_geometry.ts + 帧契约的网格数学待收敛到统一 mapper。

## Consequences

### Positive

1. 单写者 + boxZoom SSOT：P1（失同步）、P2（空-presents 错补偿）、
   P3（假 settled 睡眠）、P4（settle 窗口偏差）同解——非法状态不可表示。
2. 恢复 ADR-0002 单写者与铁律 §2。
3. Rust/TS 边界更干净：Rust 只管动画状态，DOM 几何归 TS。

### Negative

1. transform 更新从 Rust RAF（可能 60fps）移到 TS tile tick——帧率取决于
   主线程空闲度（perf 探针：手势期 ~30fps）。补偿公式每帧全量重算，
   残差 ≤ 一帧动画量（<2%），帧契约容差内。
2. `scale(visual/X)` 计算仍存在两处（canvas owner、瓦片 refresh）——
   语义一致（同为 box→visual 补偿），收敛到 coordinate mapper 为技术债。

## Tests（TDD 红灯先行）

1. 单元：`canvas_transform_owner.test.ts`——乱序 present/sync 时序表驱动，
   断言不变量（实现不存在 → 红灯）。7 例，含"presentFrame 同帧重算
   transform"（防闪烁）与"boxZoom 而非 lastRendered 作除数"（原缺陷）。
   变异校验：把 `visual/boxZoom` 改成 `visual` 后 2 例转红，证明测试有牙。
2. E2E：`zoom_gesture_frame_contract.spec.ts` 扩展为大跨度 14 档手势
   （复现用户 100%→30%→329% 场景）——当前代码上红灯（探针实证 6-12% 漂移）。
3. 绿灯后全套回归：zoom 套件、vitest、clippy wasm32。

## Outcome（2026-09-30 实测）

| 指标（canvas 视觉宽 vs page×visual） | 修复前 | 修复后 |
|---|---|---|
| 超 4% 帧数 | 51 / 379 | 0 / 363 |
| max 漂移 | 18.8% | 0.01% |
| p50 / p90 | 1.5% / 1.5% | 0.00% / 0.00% |

帧契约（瓦片网格 ±0.02、canvas 宽 ±4%、层不隐藏）全绿；zoom E2E 套件
（tile_layer_gesture / wheel_raf / surface_transition / sudden_jump /
dom_behavior）全绿；vitest 111 全绿；clippy wasm32 无告警；core 265 测试全绿。

## References

- ADR-0002（presentation single writer）、ADR-0003（tile 架构；Phase 4
  base-layer 移除未完成——双层并存是本 ADR 收敛的直接背景）
- ADR-0004（canvas compositor scale 授权）、ADR-0006（SetBox）、
  ADR-0009（手势瓦片流式）
- architecture-principles.md §1/§2/§5
