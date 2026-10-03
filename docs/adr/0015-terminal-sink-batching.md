# ADR-0015: 终端日志 sink 合批 —— **已否决（Rejected）**

## Status

Rejected（2026-10-01 实现后当次回退；保留本文作为记录，防止重蹈）

## Context

ADR-0013/0014 之后，CDP profile 中剩余的最大自耗时项是诊断 IPC
（`fetch` ≈ 79ms/手势）：`present`/LAYOUT 通道的 INFO 级事件在缩放手势中
每次渲染产生多条，每条一次 `terminal_log` IPC 往返。

Rust 侧 `terminal_log` 的全部实现是 `eprintln!`（`log_service.rs:155`）——
一条多行消息与多条单行消息在 stderr 上逐字节等价。据此提出合批：
`TerminalSink` 所有者把 32ms 窗口内的消息以 `\n` 连接成一次 IPC，
并设 40 条/批的上限。

## 实现与验证过程中发现的问题

实现（含 40 条/批上限）后：

1. **E2E 契约集体超时**：此前 12s 的 `zoom_gesture_frame_contract` /
   `zoom_surface_painted_contract` 变为 60s mocha 超时（总时长 5m+，
   是 webdriver 命令在饥饿的主线程上反复重试）。
2. **可稳定复现、且与 E2E harness 无关**：CDP 直接剖析（verbose OFF）
   健康（p50=20ms）；**verbose ON 时 profile 永远无法完成**（页面楔死）。
   E2E harness 恰好在 `waitForApp` 里打开 verbose 追踪。
3. 回退合批（保留 PROF 免 IPC、视口所有者等全部其它改动）后，
   E2E 立即恢复正常时长（13.5s / 11.2s / 8.4s，4/4 通过）。

### 根因分析

verbose 追踪下手势产生**大量**诊断事件（`logPdfLayoutTrace` 全量快照路径
每次渲染 ~30 次 × 每次 5 元素快照）。合批把"逐条小 IPC"改为
"每 32ms 一次大批"：

- 队列在两个 flush 之间无界增长（主线程越忙，排队越多）；
- 每次 flush 在**主线程**上做巨型 `join('\n')` + 跨 wasm/IPC 边界的
  大字符串序列化；
- 40 条/批的上限不足以抵消队列增长速率——事件产生速率远高于
  40 条/32ms 的消费速率时，上限只丢弃尾部，主线程仍被
  "持续满批的 join + 序列化"占据。

即：**在事件产生速率无上界的通道上做主线程侧合批，会把日志开销从
O(1)/条 放大为 O(队列长度)/窗口，形成正反馈楔死**。E2E（verbose 常开）
是第一个踩中的环境；生产 verbose 关闭时不触发，但这正是
"测试环境才暴露"的链分叉（铁律 §5）。

## Decision

1. **回退** `TerminalSink` 合批；`emitPdfDiagnostic` 终端分支恢复逐条
   `targetInvokeV3('terminal_log')`。
2. **保留**同期的 PROF 免 IPC（探针通道，in-page history 为权威源）——
   它只减少事件数，不改变每条的处理方式，无此风险。
3. 代码中留注释指向本 ADR。

## 若未来重试的前置条件

- 合批必须在**生产者侧限流**（verbose 洪泛时先降采样事件，再谈合批）；
- 或把 join/序列化移出主线程（worker 内攒批）；
- 或按通道分级：PROF/DEBUG 永不进终端 sink（已做），INFO 仅
  human-visible 节点进 sink（需要事件白名单，而不是通道级规则）。

## Consequences

1. `fetch`（诊断 IPC）维持 ~79ms/手势的已知成本（占 busy 总量 ~16%）。
2. PROF 免 IPC 的收益保留（`fetch` 79→31ms 的部分来源）。
3. 契约测试 `terminal_sink.test.ts` 随实现一并删除；ADR 记录防止重蹈。
