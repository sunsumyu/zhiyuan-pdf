# ADR-0013: 热路径禁止无条件日志（消除缩放手势的日志洪泛）

## Status

Accepted

## Context

### CPU profile 定位（CDP，2026-10-01）

ADR-0012 落地后，`zoom_p2_probe` 仍显示**每个 wheel 步 ~100ms 的主线程阻塞，
且与 zoom 无关**（visual 1.03→5.28 全程 100±15ms）。为定位它，用
`tools/cdp-profile.mjs`（WebView2 `--remote-debugging-port` + `Profiler` 采样
200µs）对同一手势做了 CPU profile：

| 自耗时（含） | 函数 | 说明 |
|---|---|---|
| **506.8ms** | `__wbg_log`（Rust→JS console 桥） | 手势期间 **2456 条 console 消息** |
| 120.9ms | `sendIpcMessage`→`fetch` | TS 诊断 INFO 级事件的 Tauri IPC |
| 97.8ms | `getBoundingClientRect` | 强制 reflow（syncLayoutBox/tile tick） |
| 12.6ms | `drawImage` | 渲染本身 |

**渲染只占 12.6ms；日志占 506ms。** 阻塞的"每步 ~100ms"主要由日志构成。

### 洪泛源头

按 console 前缀统计（16 次 wheel，~4.4s）：

```
console messages during gesture: 2456
  2245  [WASM-ViewerSession]   ← ViewerSession::read() 的遗留调试日志
   111  [CHAIN]
   100  (none)
```

`viewer_api.rs` 的 `ViewerSession::read()`（**全应用最热的读路径**——render flow、
tile layer、zoom controller 每帧都读会话快照）在**每次调用**时无条件
`console::log_1`：

```rust
pub fn read(&self) -> JsValue {
    let session = viewer_store::read_viewer_session();
    web_sys::console::log_1(&JsValue::from_str(&format!(          // ← 每帧 ~9 次
        "[WASM-ViewerSession] read() is called. path={:?}, page_count={}",
        session.path, session.page_count)));
    to_value(&session).unwrap_or(JsValue::NULL)
}
```

另有 `present/plan_builder.rs` 的 `log::info!("[PAGE-SIZE] …")` 在**每次
plan 构建**时无条件触发（peek/schedule/followUp 每次 render 各构建一次）。

### 为什么这是结构性问题（铁律 §2）

热路径上的日志写入没有 owner、没有开关：`chain_trace` 有运行时开关
（`CHAIN_ENABLED`，默认 ON），`emitPdfDiagnostic` 有 `verboseOnly` 门，
而这两处**绕过了所有门**，直接打 console。任何一次会话读/plan 构建的
频率变化都会放大它——"修一处后另一处出新 bug"的典型温床。

## Decision

### 1. 热路径零无条件日志

- `ViewerSession::read()`：**删除** `console::log_1`（遗留调试输出，
  "read() is called" 无任何信息量）。
- `present/plan_builder.rs`：**删除** `[PAGE-SIZE]` 的每次构建日志
  （页面尺寸在打开/换页时变化，需要时由文档事件侧打点）。

### 2. 契约锁进 CI（沿用本仓库的源码契约测试范式）

`src/__tests__/hot_path_logging.test.ts` 以源码契约断言：
`viewer_api.rs` 的 `read()` 无 `log_1`、ui `plan_builder.rs` 无无条件
`log::info!`。防止同类日志再次进入热路径（OCP：新日志点必须经过
`chain_trace` 门或 `emitPdfDiagnostic` 的 `verboseOnly` 门）。

## Consequences

### 验证（修复后，2026-10-01，CDP 复测同一手势）

| 自耗时（含） | 修复前 | 修复后 |
|---|---|---|
| `__wbg_log`（Rust→JS console） | **506.8ms** | **26.5ms**（−95%） |
| `sendIpcMessage`（诊断 IPC） | 120.9ms | 85.3ms |
| `getBoundingClientRect`（强制 reflow） | 97.8ms | 100.4ms（未变，独立技术债） |
| `drawImage`（渲染本身） | 12.6ms | 13.3ms |

- 手势期间 console 消息 **2456 → ~150**（`[WASM-ViewerSession]` 2245 → 0）。
- rAF 典型帧成本 **33–35ms → 22–24ms**；最大阻塞 221 → 154ms（剩余峰值是
  最高 zoom 的最终 settle 全页渲染，一次性）。
- 回归：wasm-pack **15/15**、vitest **121/121**（含 3 个新契约）、zoom E2E
  7/7（帧契约漂移 max 0.00%、表面契约 0 空白 0 闪烁）、clippy 0 warnings。

### Positive

1. 消除手势期间 ~2245 次 wasm→JS console 跨界调用（506ms 自耗时），
   典型帧成本从 33–35ms 降到 22–24ms。
2. 诊断通道回归设计意图：`chain_trace` 门控编辑链路，`emitPdfDiagnostic`
   门控渲染诊断，热路径零直写。
3. 契约测试防回归，新日志必须走门。

### Negative / 技术债

1. `getBoundingClientRect` ~100ms 的强制 reflow 是布局同步读取的固有成本
   （syncLayoutBox/tile tick 需要视口几何），需独立 ADR（缓存/批量化）。
2. `sendIpcMessage` 85ms 来自 INFO 级 PROF 事件的 IPC（每次 render 一条），
   可批量化，另立技术债。
3. `ViewerSession::read()` 若确需观测，应通过 `chain_trace` 门（默认可关）
   或独立探针，而非常开日志。
