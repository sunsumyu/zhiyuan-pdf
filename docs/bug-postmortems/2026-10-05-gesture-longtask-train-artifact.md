# Postmortem 2026-10-05 — 手势期「~100ms 管线 longtask 列车」证伪：verbose 诊断洪泛伪影

## 结论（先读这个）

ADR-0026 实现循环记录的「既有的 ~100ms/周期管线 longtask 列车（TS 侧、两
路径皆有、疑似读写穿插 layout thrash）——把 reknock 压在 ~10fps，下一修复
循环的靶子」**不存在于产品中**。它是 **E2E harness 强制开启的 verbose 追踪
制造的伪影**：

- **verbose OFF（生产默认）：同一手势主线程 0 长任务**——5/5 次运行，rAF
  满速（175-224 帧 vs verbose 下的 70-99 帧），重 rAF 回调（≥8ms）0-1 个。
- **verbose ON**（`helpers/app.js` 的 `waitForApp` 对每个 E2E 会话强制设
  `__PDF_DIAGNOSTICS_VERBOSE=true` + `__PDF_LAYOUT_TRACE_VERBOSE=true`）：
  同一手势出现高度波动的长任务列车（4-19 个，总 397-1510ms，单最长
  130-151ms）。

列车在 verbose 下必然出现、在生产下从不出现——「生产管线成本」的前提
被证伪。本循环同时修掉了 verbose 洪泛中**可论证浪费**的部分（console
sink 无界 + 无谓 ANSI 格式化），并重标了被伪影「润滑」过的 ADR-0026
p50 契约。

## 取证方法

`tests/e2e/specs/zoom_longtask_attribution_probe.spec.ts`（measurement-only，
已入库）。三个**隔离相位**——每相位 `reloadSession()` + 重新装 fixture，
起点完全相同，唯一变量是诊断配置：

| 相位 | verbose | console sink | 结果（代表性） |
|---|---|---|---|
| A | ON | 直通 | 9-19 LT / 397-1510ms，窗口内 readSnap×33-44 + gBCR/gCS×~170 |
| C | ON | 静音 | 0-8 LT / 0-598ms（不消失！） |
| B | OFF | 直通 | **0 LT / 0ms，5/5 次运行** |

探针仪表（全部页面内注入，无产品改动）：longtask PerformanceObserver、
console.\* 补丁（给每条诊断精确的 performance.now 时间戳——in-page history
只有墙钟）、rAF 回调自耗时、`getBoundingClientRect`/`getComputedStyle`/
`drawImage`/`clearRect` 计数与自耗时。

## 关键中间结论（含两次被推翻的读数）

1. **第一读（错）**：A 窗口塞满 `readSnap`（verboseOnly，每次
   `viewerSession.read()` 都 `JSON.stringify`）+ ~170 gBCR/gCS → 判
   「sink + snapshot 是成本」。C 相位静音 sink 后列车仍在 → **console
   sink 不是全部**。
2. **第一次 C=0 的读数（错）**：早期未隔相位的运行里 C 显示 0 LT——
   事后发现是**相位间 zoom 状态残留**（C 从 ~27.9× 起跑，事件量不同）。
   reload 隔离后 C=7-8 LT。**教训：A/B 必须同起点。**
3. **console 占比（不可证）**：post-fix A 曾测得 19 LT、干净配对运行
   测得 0——同一构建同配置的 run 间方差（本机后台负载）淹没 sink 单相
   差异。**唯一稳定的事实是 B≡0。** 列车剩余构成（~3000 事件/手势的
   分配压力/GC + logPdfLayoutTrace 每调用 5 元素快照 + IPC）**未归因**，
   本循环不追。

## 伪影污染面（已被它带偏的记录，全部更正）

1. **ADR-0026 §Tests 第 2 点**：「既有的 ~100ms 管线 longtask 列车……
   下一修复循环的靶子」→ 证伪，改指本文。
2. **clarity 契约 p50≤1%**：该标定是在「rAF 饥饿采样」下拟合的——
   verbose 洪泛饿死 rAF（探针 70-99 帧 vs 生产 175-224 帧），采样稀疏、
   收敛尾帧主导 → p50 恒 0。限流后 rAF 部分解饿，采样密度最高 ×3
   （n=66），斜坡段周期滞后进入中位数（最差 0.0395）→ 契约红。
   **管线结构性质未变**（渲染落在 knock 时刻的精确 visual，零设计
   拉伸）；重标为 p50≤5%，仍与旧 worker 世界（中位 45-80%）判然分开。
   max≤15%（1-tick 物理界）不受影响（观测 0.05-0.14）。
3. **ADR-0015 的「verbose ON 时 profile 永远无法完成」**：本文给出更
   完整的机理（console sink + 每事件双份格式化 + 分配压力），其「若未来
   重试的前置条件：生产者侧限流」首次落地（作用于 console sink）。

## 修复（最小实现，`src/bridge/shared/diagnostics.ts`）

- **console sink 生产者侧限流**：32ms 窗口内最多 8 条，窗口滚动时输出
  一行抑制摘要；`__PDF_DIAGNOSTICS_HISTORY` 仍逐条全量（探针权威源）；
  **ERROR/WARN 永不限流**（ADR-0013：error 流只留真故障，且必须全量
  可见）；terminal_log IPC 分支不动（ADR-0015 的合批回退边界）。
- **terminalMessage 惰性构建**：ANSI 变体只在该进 IPC 分支时才构建
  （此前对每个事件无条件构建，DEBUG/PROF 事件白付）。
- 红灯契约 `src/__tests__/diagnostics_console_rate_limit.test.ts`（4 例，
  先红 4/4 后绿）：洪泛下 console ≤ 上限且 history 全量、窗口滚动摘要、
  ERROR/WARN 豁免、verbose 关闭时限流同样生效。

## 门禁

core 271/271（无 Rust 改动）、vitest 163/163（159+4）、tsc 0 err、
wasm 重建 + e2e:build、clarity 契约 5 轮绿 + 变异校验红（flag 注入 →
direct=0 → toBeGreaterThanOrEqual 红）、zoom 全套件 **19/19**
（含新探针；首跑 2 失败均定性：tilegrid = tauri-driver ECONNREFUSED
并发假失败、attribution = 60s mocha 超时 → 局部 `this.timeout(300s)`）。
无 Rust 改动，clippy 无对象。

## 运维陷阱（下次必看）

1. **`helpers/app.js` 强制 verbose 意味着所有 E2E 计时都被伪影润滑**。
   任何 E2E 上的性能结论必须先问「verbose 开着吗」；探针
   `zoom_longtask_attribution_probe` 是对照组工具。
2. **同配置 run 间方差（4→19 LT）**：单相差异 <~500ms 不可解读；只有
   「0 vs 非 0」的对照（verbose off/on）跨 5 次运行稳定。
3. **变异校验改已提交文件用 sed 注入 + sed 删除回退**；`git checkout --`
   会把未提交的重标定一起回滚（本次代价：重编辑一轮）。
4. 限流后探针的 console 事件时间线是**采样**的（history 仍全量）——
   归因时以 history/PROF 为准。
