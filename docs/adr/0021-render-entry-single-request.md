# ADR-0021: 渲染入口一次构建 request（peek/schedule 共享）

## Status

Accepted

## Context

### 定位（ADR-0020 的 A/B 之后）

ADR-0020 的 A/B 证实：~190 次强制重排来自 `buildRequest` 的**调用次数**
（每次调用前有 presenter 样式写入使布局变脏，`Performance.getMetrics`：
14 步手势 +210 layout / +73.7ms）。

调用点盘点（生产热路径）：

- `render_flow.executeActualRender`：`peek(zoom, reason)` 与
  `scheduleRender(zoom, reason)` **背靠背、输入完全相同**，却各自在适配器
  内部调 `buildRequest` → 每次渲染 2 次 request 构建（2 次强制布局）；
- `renderCurrentPage` 是 reknock/settle 的入口，手势期间每次动画重渲都走它。

另外同类问题：`render-current-page.scheduled` 日志的
`JSON.stringify(plan)` + `JSON.stringify(scheduled)` 在 `logRenderFlow`
（verboseOnly）门控**之前**无条件求值——非 verbose 时每次渲染白算两次序列化。

## Decision

### 1. 适配器接受预构建 request

`peek` / `scheduleRender` 增加可选第三参 `prebuiltRequest`；给了就用它，
不给则照旧内部构建（其余调用点零改动）。类型：

```ts
peek: (displayZoom, renderReason?, prebuiltRequest?: RenderPlanRequest) => RustFramePlan | null;
scheduleRender: (displayZoom, renderReason?, prebuiltRequest?: RenderPlanRequest) => RustRenderFrame | null;
```

### 2. 渲染入口只构建一次

`executeActualRender` 用已有的 `buildRenderRequest`（`pdf_runtime`/
`document_edit_api` 已在用的导出）构建一次，同一次 peek + schedule 共享：

```ts
const request = session.path
    ? deps.framePlanAdapter.buildRenderRequest(effectiveZoom, renderReason)
    : null;
const plan = request ? deps.framePlanAdapter.peek(effectiveZoom, renderReason, request) : null;
const scheduled = request ? deps.framePlanAdapter.scheduleRender(effectiveZoom, renderReason, request) : null;
```

语义不变：peek 与 schedule 收到的字段值与今天两次独立构建完全一致
（两者本就同步背靠背执行，之间无 DOM 写入）；只是 DOM 读从 2 次降为 1 次。

### 3. scheduled 日志按 verbose 门控

`render-current-page.scheduled` 的两次 `JSON.stringify` 移入
`verbosePdfDiagnosticsEnabled()` 守卫内（该函数已导出）。

## 红灯契约

`src/__tests__/render_entry_single_request.test.ts`（源码契约，沿用
ADR-0014/0020 的切片模式）：

1. `executeActualRender` 函数体内 `buildRenderRequest(` 恰好出现 1 次，
   且 `peek(` / `scheduleRender(` 调用带第三参（共享同一 request）；
2. `render-current-page.scheduled` 的日志调用位于
   `verbosePdfDiagnosticsEnabled()` 守卫内。

## Consequences

### Positive

1. 每次渲染的 request 构建从 2 → 1（手势内 ~190 → ~95 次强制布局，
   ≈ 37ms/手势）；
2. 非 verbose 路径不再白算两次 JSON 序列化。

### Negative / 技术债

1. `peek`/`scheduleRender` 签名多一个可选参（其余调用点不受影响）；
2. 剩余 ~95 次/手势的强制布局属于"渲染管线本身要读 scroll"的结构性成本，
   若要继续压需 scroll 单 owner + Rust 写入失效协议（另立 ADR，风险较高）。

## Verification

- 红灯：新契约 3/3 修复前红 → 修复后绿；vitest 136/136、tsc / wasm / core /
  clippy 全绿；E2E 全量 12 spec（打包产物、新渲染入口）全绿，frame contract
  max=0.00%。
- **CDP `Performance.getMetrics` 复测（诚实记录）**：合并后 LayoutCount
  仍为 +210（改动前 207-210，含运行间噪声 ±25ms）——**没有可测的布局收益**。
  结合 ADR-0020 的 A/B（207 vs 210）三组数据一致表明：这 ~210 次布局是
  **动画 UI 的正常每帧布局**（presenter 每帧写 canvas 样式 → 浏览器每帧布局），
  **不是**可归因于 scroll 读取或 request 构建的病态强制重排。
- 因此本 ADR 与 ADR-0020 的实际价值是**代码卫生**（消除冗余属性读/序列化、
  日志与返回同源、调用点更薄），而非性能收益。原"~62-79ms reflow 债"应视为
  正常动画成本，**不再作为优化项追踪**。
