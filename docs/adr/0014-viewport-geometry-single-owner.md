# ADR-0014: 视口几何的单一所有者（帧内合并同步读取）

## Status

Accepted

## Context

### CPU profile（CDP，2026-10-01，ADR-0013 之后）

`getBoundingClientRect` 自耗时 **~100ms/手势**，是渲染（`drawImage` 13ms）的
近 8 倍，也是剩余阻塞的第二大项。按调用栈归因：

| inclusive | 调用栈 |
|---|---|
| **47.9ms** | `getBoundingClientRect` ← `buildRequest` ← `peek`/`schedule`/`followUp`（**每次渲染 ~6 次**） |
| 27.6ms | ← `E`（viewport 相关读） |
| 14.6ms | ← `resolveHostScrollRefresh` ← scroll 监听 |

### 结构性根因（铁律 §2）

视口几何（scroll container 的宽高/dpr）没有 owner：`frame_plan.ts` 的
`buildRequest`、`pdf_layout_sync.ts` 的 `syncLayoutBox`、`tile_layer` 的
两处、`zoom_controller` 各自 `getBoundingClientRect()`。同一帧内这些读会重复
发生（一次渲染 6 次），而**每次同步几何读都会在样式脏时强制 reflow**。

关键在于：这些读取**几乎总是同一个值**——scroll container 的尺寸只在
窗口 resize（或侧栏切换）时变化，缩放本身不改它。读的是"当前布局结果"，
却每次重新触发布局计算。

这与 ADR-0010（transform 三写者）、ADR-0011（可见性六写者）同类：
**多个读者无缓存、无 owner，重复付出同步布局成本**。

## Decision

### 1. `ViewportGeometry` 单一所有者（`src/bridge/viewer/viewport_geometry.ts`）

```ts
export type ViewportGeometry = { width: number; height: number; dpr: number };

createViewportGeometry({ getScrollContainer }): {
    read(): ViewportGeometry;   // 缓存命中则零布局读
    invalidate(): void;         // 标脏，下次 read 重新测量
}
```

- `read()` 首次测量后缓存；`clientWidth/clientHeight` 为主，`rect` 仅作 0 兜底。
- `invalidate()` 由**帧边界**驱动：窗口 `resize` 监听 + tile 层 rAF tick 首行。
  一帧内 N 次读 → **1 次测量**。

### 2. 所有调用点收敛

- `frame_plan.ts::buildRequest` → `getViewportGeometry().read()`
- `pdf_layout_sync.ts::syncLayoutBox` → 同上
- 其余 `getBoundingClientRect` 保持（不在热路径的：editor、probe 等）

### 3. 正确性

- resize 监听保证窗口变化后必重新测量；
- tile 层每帧 `invalidate()` 保证侧栏切换等同帧内的布局变化被吸收；
- 测量值在一帧内恒定，与"渲染读同一帧几何"的语义一致。

## Consequences

### 验证（修复后，2026-10-01，CDP 同手势三轮对比）

| 自耗时（含） | ADR-0013 后 | + 视口所有者 | + PROF 免 IPC |
|---|---|---|---|
| `getBoundingClientRect` | 100.4ms | 21.4ms | **19.4ms** |
| `fetch`（诊断 IPC） | 69.8ms | 76.4ms | **78.9ms**¹ |
| `buildRequest`（`r`） | — | 76.4ms | **62.3ms** |
| busy 总量（非 idle） | 575ms | 581ms | **491ms** |

¹ `fetch` 余量来自 `present`/LAYOUT 通道的 INFO 级事件，仍走终端 sink；
`r` 余量来自 `scrollLeft/scrollTop` 读取（脏布局时的固有 reflow）。

- rAF 帧间隙 **>40ms 的帧从 25 个降到 1 个**（190ms，为最高 zoom 的最终
  settle 全页渲染，一次性），**p50 = 20ms**。
- 回归：wasm-pack **15/15**、vitest **126/126**、zoom E2E **8/8**
  （帧契约漂移 max 0.00%、表面契约 0 空白 0 闪烁）、clippy 0 warnings。
- 配套：`ViewerSession::read` 的 `readSnap` 诊断原为 `verboseOnly` 但
  `JSON.stringify` 被急切求值（最热读路径上的纯浪费），已改为仅在 verbose
  开启时序列化。

### Positive

1. 每次渲染的几何读从 ~6 次降到 ≤1 次/帧，`getBoundingClientRect`
   100→19ms；`readSnap` 急切 stringify 移除。
2. PROF 探针不再逐条 IPC（设计意图本就是"in-page history 为权威探针源"）。
3. 与 ADR-0010/0011 同一范式（单所有者 + 缓存 + 显式失效），铁律 §2 收敛。
4. 契约测试锁定"帧内单次测量"与"热路径经 owner 读取"，防回归。

### Negative / 技术债

1. 失效是**显式**的：新增会改变 scroll container 尺寸的 UI（如新侧栏）必须
   调用 `invalidateViewportGeometry()`（侧栏切换已接线；resize 已监听）。
2. `present`/LAYOUT 通道的 INFO 事件仍逐条 IPC（~79ms/手势），可批量化。
3. `buildRequest` 的 `scrollLeft/scrollTop` 读取（~62ms/手势）是脏布局下的
   固有 reflow，根治需"读写分批"（reads-before-writes）重构，另立 ADR。
