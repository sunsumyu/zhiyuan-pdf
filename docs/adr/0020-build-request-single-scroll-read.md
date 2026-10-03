# ADR-0020: buildRequest 只读一次 scroll（消除重复 DOM 读）

## Status

Accepted

## Context

### 实测（CDP 手势剖析，2026-10-02）

给 scroll 容器的 `scrollLeft`/`scrollTop` 装计数 getter 后跑一次 12 步
ctrl+wheel 手势：**416 次 scroll DOM 读取**；生产路径（verbose 关）CPU 样本里
`get scrollLeft` 占非 idle 项之首（1.3% ≈ 78ms），与历史记录的 ~62ms 吻合。

### 根因：每次 buildRequest 把 scroll 读两遍

`frame_plan.ts::buildRequest` 每次调用先构造日志对象、再构造返回对象，**两处
都读 `scrollContainer.scrollLeft/scrollTop`**：

```ts
logPdfLayoutTrace('frame.request.build', { ..., scrollLeft: scrollContainer?.scrollLeft || 0, scrollTop: ... });
return { ..., scrollLeft: scrollContainer?.scrollLeft || 0, scrollTop: ... };
```

`buildRequest` 每次渲染 ~6 次（followUp 环更密），故每次渲染 4 次属性读。
注意 ADR-0014 只缓存了 viewport **尺寸**（`clientWidth/Height`）；scroll
**位置**从未被缓存或去重。且日志参数对象在函数体之前求值——即使
`logPdfLayoutTrace` 因非 verbose 提前 return，那两个 scroll 读**已经发生**。

### 修正：重复读不是强制重排的来源（A/B 实测）

用 CDP `Performance.getMetrics` 对同一 14 步手势做了 A/B（读两遍 vs 读一遍）：

| 指标 | 读两遍（还原） | 读一遍（ADR-0020） |
| --- | --- | --- |
| LayoutCount | +207 | +210 |
| LayoutDuration | +79.1ms | +73.7ms |

**两者几乎相同**——同一次调用内的第二次 scroll 读是免费的（两次读之间没有
样式写入，浏览器复用已干净布局）。因此 ~190 次强制重排来自 **`buildRequest`
的调用次数本身**（每次调用前有 presenter 的样式写入使布局变脏），**不是**
调用内的重复读。本 ADR 只消除重复属性读（严格改进、可验证），并**不**声称
降低强制重排——那需要减少 buildRequest 调用或让 scroll 读取避开脏样式窗口
（见技术债）。

## Decision

在 `buildRequest` 顶部把 scroll 位置读入局部变量，日志与返回**共用同一次读取**：

```ts
const scrollLeft = scrollContainer?.scrollLeft || 0;
const scrollTop = scrollContainer?.scrollTop || 0;
logPdfLayoutTrace('frame.request.build', { ..., scrollLeft, scrollTop });
return { ..., scrollLeft, scrollTop };
```

即 `scrollLeft`/`scrollTop` 的 DOM 属性读每次 buildRequest 各 **1 次**（原 2 次）。

### 为什么不引入 scroll 缓存 owner

scroll 位置在缩放期间由 **Rust 直接写**（`raf_loop::on_wheel_event` 锚点滚动、
`raf_committed` 提交帧 `set_scroll_left/top`），跨越 TS 的帧边界；缓存需要
一个能感知这些程序化写入的失效信号，否则读到陈旧值会破坏锚点滚动
（ADR-0008/0016 的正确性依赖 scroll 现值）。该 owner 涉及跨语言失效协议，
风险与收益不匹配——本次只做**无歧义的重复读消除**（严格改进、零行为变更）。
更深的 reads-before-writes（跨调用强制重排）留作后续独立 ADR。

## 红灯契约

`src/__tests__/build_request_scroll_reads.test.ts`（源码契约，沿用 ADR-0014
的 buildRequest 切片模式）：`buildRequest` 函数体内 `.scrollLeft` / `.scrollTop`
**属性访问各出现恰好 1 次**（修复前各 2 次）。

## Consequences

### Positive

1. 每次 buildRequest 的 scroll 属性读减半（手势内 416 → ~208 次）；
2. 日志对象与返回对象同源，杜绝"日志里 scroll 与返回 scroll 不一致"的
   潜在观测歧义。

### Negative / 技术债

1. **未**降低强制重排：A/B 实测（见上）表明重排来自 buildRequest 的**调用
   次数**（每次调用前有 presenter 样式写入使布局变脏），而非调用内重复读。
   要消除它需二选一：(a) 减少 buildRequest 调用（peek/schedule 复用同一
   request 而非各自构建）；(b) 让 scroll 读取避开脏样式窗口——引入 scroll
   单 owner，其值在**样式写入之前**捕获、并在 Rust 程序化写 scroll 时失效。
   两者都需跨 actor 契约，另立 ADR。

## Verification

- 红灯：`build_request_scroll_reads.test.ts` 修复前红 → 修复后绿。
- CDP：同条件手势的 `scroll DOM reads` 计数从 416 → 190（确定性）。
- CDP `Performance.getMetrics` A/B：LayoutCount 读两遍 207 / 读一遍 210
  —— 证实本改动不改变强制重排量（符合预期：只去重属性读）。
- 回归门：vitest 133/133、wasm 21/21、core 269/269、clippy 干净；
  E2E 全量 **12/12 spec 文件（18 测试）全绿**，frame contract max=0.00%。
