# ADR-0022: 渲染 abort 类错误降级为 DEBUG（单一判定源）

## Status

Accepted

## Context

### 现象

E2E（打包产物）输出里反复出现：

```
ERROR [TILELAYE] tile-render-failed page=0 | x=0 | y=0 | error=Error: stale frame
```

但 `stale frame` **不是故障**——`vector_host.ts` 对它显式捕获并返回
`{ aborted: true }`（设计内恢复路径）：手势推进 band/epoch 后，Rust 丢弃
旧 band 的在途 tile 请求，新 band 会重新请求。tile 层随后 `resetTile`
让该 tile 可被重新调度。

### 代价：错误流污染 + 每次 abort 一条 terminal IPC

`tile_layer.ts` 的 catch 把**所有**错误都发到
`emitPdfDiagnostic('tile-layer', 'tile-render-failed', ...)`。该事件名含
`failed` → `inferLevel` 判为 **ERROR** → `console.error` **且每次一条
`terminal_log` IPC 到 Rust**（`diagnostics.ts`：DEBUG/TRACE 免 IPC）。
快速手势下 band 变化频繁，ERROR 噪音会把真正可行动的故障淹没——
这正是 ADR-0013「热路径日志纪律」要防的观测劣化。

### 根因：abort 判定没有单一所有者

"哪些渲染错误属于设计内 abort" 的知识散落两处：
- `vector_host.ts`：`errMsg === 'stale frame' || includes('stale page asset request') || !isFrameCurrent(token)`；
- `vector_page_bundle.ts`：`throw new Error('stale frame')` 的抛出方。

tile 层的 catch 没有这份知识，把恢复路径当故障上报。

## Decision

### 1. 单一判定源

在 `vector_page_bundle.ts`（abort 错误串的出生地）导出谓词：

```ts
export function isAbortedRenderRequest(message: string): boolean {
    return message === 'stale frame' || message.includes('stale page asset request');
}
```

`vector_host.ts` 的消息判定改用该谓词（其 `!isFrameCurrent(token)` 分支
是调用点特有知识，保留在原地）。

### 2. tile 层按类分流

```ts
if (isAbortedRenderRequest(message)) {
    emitPdfDiagnostic('tile-layer', 'tile-render-aborted', {...}, { level: 'DEBUG' });
} else {
    emitPdfDiagnostic('tile-layer', 'tile-render-failed', {...});   // 仍为 ERROR
}
```

- `tile-render-aborted` + `{ level: 'DEBUG' }`：`inferLevel` 尊重显式 level
  （`options.level` 优先）；DEBUG 走页内历史、**免 terminal IPC**；
- 真故障（wasm/canvas/worker 失败）保持 ERROR 不变。

## 红灯契约

`src/__tests__/tile_render_error_classification.test.ts`（源码契约）：

1. `vector_page_bundle.ts` 导出 `isAbortedRenderRequest`，且其判定覆盖
   `stale frame` 与 `stale page asset request`；
2. `vector_host.ts` 的消息判定使用该谓词（不再内联字符串比较）；
3. `tile_layer.ts` 的 catch：abort 类走 `tile-render-aborted` +
   `{ level: 'DEBUG' }`；`tile-render-failed` 仍存在（真故障路径）。

## Consequences

### Positive

1. 错误流恢复"ERROR = 可行动故障"的语义；手势期不再被 abort 刷屏；
2. 每次 abort 免一条 `terminal_log` IPC（DEBUG 免 IPC，与 ADR-0013 纪律一致）；
3. abort 判定单一所有者，后续新增 abort 错误串只改一处。

### Negative / 技术债

1. 若未来出现新的 abort 类错误串而未加入谓词，会被误报 ERROR——但那正是
   应该被人看到并归类的情况（fail-loud）。

## Verification

- 红灯：新契约修复前红 → 修复后绿；vitest / tsc / wasm / core / clippy 全绿。
- E2E：全量 12 spec（打包产物）全绿，且输出中不再出现
  `ERROR ... tile-render-failed ... stale frame`。
