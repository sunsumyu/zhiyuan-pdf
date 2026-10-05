# ADR-0026: 主线程直渲手势视口补丁（手势过程中清晰）

## Status

Accepted（2026-10-05，grill-with-docs 设计会话裁决；实现走红灯契约循环，
见 Tests——实现另起会话）

**Amends ADR-0009**：修订其权衡的前半句——"手势中允许模糊（换 60fps 跟手）"
由"手势中精确 visual 每帧直渲"取代。ADR-0009 的统一呈现公式、瓦片流式、
统一几何不变量（q → q × visualZoom）全部保留。

## Context

### 新需求（2026-10-04 用户提出）

**"手势过程中也要清晰"**——改变设计契约的前半（ADR-0009 的核心权衡）。
后半（settle 必然清晰，ADR-0023/0024 守护）不变。此前 P2 被定性为设计
行为（HANDOFF #1），本 ADR 落地后该签名消解。

### 探针裁决（zoom_reknock_cost_probe.spec.ts，零插桩，3 轮复现）

- 视口 reknock 渲染周期 p50=70.9–90.6ms，其中 **worker 渲染仅 3.1–3.5ms
  （~4%）**，wasm 段 1ms、blit 0.2ms——**~95% 耗在 worker 往返排队**
  （单 worker 共享 + 主线程消息延迟）。持续完成率 8.2–9.5/s。
  全页渲（对照）roundMs 140–213ms。
- 候选 A（worker 逐帧重渲）需 4.4–5.6× 提速才能进 16ms 帧预算——**不可行**。
- **A′（本 ADR）**：主线程直渲视口补丁，周期 ≈ 3.5ms（wasm 本体）+
  0.2ms（blit）+ ~1ms（wasm 准备段）≈ **5ms < 16ms ✓**。

### 机制先例已在代码里

`vector_host.ts` 的 `isOverlayRender` 分支（编辑 overlay 渲染）就是完整的
主线程直渲：同步 wasm `renderPageOffscreen`（`progressive_workflow.rs`，
任意线程可用，页面上下文由既有 `updatePageViewport` 调用点保证）+ blit
进 render buffer。A′ = 把手势 reknock 的 detail 层渲染换到同一机制，
**机制零新增**。

### 两个决定问题性质的事实

1. **补丁渲染 zoom 本来就是精确 visual**。knock 链
   `__pdfDrainPendingRenderFrame → renderCurrentPage('zoom', visualZoom)`
   以精确 visual 构请求（C1：render tracks visual）；`quantize_cache_zoom`
   只作用于缓存键。P2 实测的 15–27 档内容滞后**全部来自传输延迟**，不是
   量化设计。→ 同步直渲后"精确 visual 每帧渲"是 plan 语义的自然延伸。
2. **决策权威已全在 core**。blur 门（`PREVIEW_REKNOCK_BLUR_THRESHOLD=0.02`
   / `PREVIEW_REKNOCK_INTERVAL_MS=60`，`zoom_render.rs`）、present policy
   （`resolve_present_policy`）、detail 复用比（0.18）、量化步——全部是
   Rust 常量 + 契约测试。A′ 不需要把任何决策逻辑搬进 TS。

### 候选对照

| 候选 | 结论 |
|---|---|
| A：worker 逐帧重渲 | 探针证伪（周期需 4.4–5.6× 提速） |
| A 变体：专属 detail worker | 消 worker 侧排队但主线程消息延迟共享，周期下限不确定，次于 A′ |
| **A′：主线程直渲（本 ADR）** | 周期 ~5ms，先例已存在，采纳 |
| B：vello/WebGPU 逐帧（未决 #4） | 根本解，另行立项，不阻塞 |
| C：收紧量化档 | ADR-0025 已证加重负载增加 jank，否决 |

### 预缓存为何不进本 ADR（Deferred）

设计会话专项评估过"缩放的提前预缓存"（2026-10-05 用户问询）：

1. **补丁层结构性无效**：(a) 能 present 的只有当前 visual 的像素——预渲
   未来 zoom 无法提前上屏，提前上屏就是 CSS 拉伸（要消掉的东西）；
   (b) 缩放轨迹单调穿过各 zoom 值，每个值只渲一次，缓存零命中率；
   (c) 预取区域取决于光标锚点，手势开始前不可知。
2. **瓦片层隐式预取已存在**：手势期泵以 `tileZoomIntent = quantize(visual)`
   流式渲染（ADR-0009），visual 收敛于 target，泵末段产出即 final 档；
   base cache LRU 保留旧档（zoom 回退复用，ADR-0023 条目绑定真位图）。
   显式 look-ahead（intent → quantize(target)）的可见收益在 A′ 后趋零
   （补丁每帧盖住视口，settle 视口已新鲜），而成本恰是 ADR-0025 实测的
   jank 源（手势期更多瓦片完成 → 更多主线程 blit/present → rAF 下降），
   且 A′ 帧预算已预支 ~5ms/帧。
3. **滚动/平移方向预取**（Mapbox 式）：滚动新鲜度问题未探针，另行立项。

重估前置条件：A′ 落地后若"补丁未 retain 路径"的 settle 仍有可见尾巴，
才重开瓦片 look-ahead。

## Decision

### 1. A′ = 传输换执行上下文，不是新链

手势期 reknock 的 detail 层视口补丁渲染从 worker 往返换为主线程同步直渲，
复用 `isOverlayRender` 分支机制（同步 wasm 渲染 → blit → 既有 present）。

**直渲判据（语义）**：`renderReason === 'zoom'` 且 preview 未 settle 且
`use_viewport_tile` 的 detail 层帧。接线时从既有 plan 信号取；若 plan 未
暴露所需信号（如 previewSettled），以最小桥接补齐——**判据不得在 TS 重新
发明**（决策权威在 core 的 plan）。

**同步性是成立前提**：直渲路径不得引入 await。一个 await 就把中止窗口
打开回 worker 时代（ADR-0022 的 5+ 检查点正是为异步窗设计的）。

### 2. 精确 visual 渲染；缓存读绕过、写保留

- 直渲帧按精确 visualZoom 渲染 → 补丁 scale ≡ 1（DetailOverlayOwner 只补
  跳帧瞬时差），结构性零设计拉伸。
- **缓存读绕过**（`isOverlayRender` 同款）：不因量化档命中而跳过新鲜
  渲染——复用旧内容 = 主动选择模糊，与需求相抵。
- **缓存写保留**：present 后照旧 `storeFrameCacheEntry`。今天存的就是
  精确 zoom 位图 + 量化键，settle 量化复用语义不变。

### 3. 节奏：core 常量重调（决策权威不动）

- `PREVIEW_REKNOCK_INTERVAL_MS` 60 → **16**：允许逐帧 eligibility。取 16
  而非 0 的原因——120Hz 屏上 RAF 每 8.3ms 一跳，16ms 把 reknock 上限钉在
  60Hz。
- `PREVIEW_REKNOCK_BLUR_THRESHOLD` **0.02 保持**：语义 = 可容忍拉伸。
  精确 visual 直渲下它自然成为节奏门——zoom 没动够 2% 就不渲，慢手势
  自动降频；稳态拉伸在 0 → 2%+单帧 delta 内振荡。
- `render_in_flight` 抑制**语义保留**：同步直渲把 in-flight 窗口收敛到
  同一 JS turn 内，下一帧 RAF 时已清——门不再构成节流约束，但 settle
  worker 路径仍依赖它。
- 预览期 detail 复用判据（0.18 复用比等）若抑制逐帧新鲜度，属本次常量
  重调范围，具体值由契约 + A/B 实测定。

### 4. 预算守卫 + 回滚开关

- **跳帧守卫**：单帧直渲成本超阈值（初值 ~10ms，探针校准）→ 跳过该帧
  reknock。补丁停留在上一帧内容由 DetailOverlayOwner 继续几何对齐——
  几何永不错（ADR-0024 契约），内容新鲜度退化恰好回今天的 P2 行为
  （用户已确认非缺陷）。无状态、自然背压、永不比现状差。
- **回滚开关**：env/flag 一键回退整条 worker 路径（ADR-0016
  `max_render_pixels<=0` 式先例），成本一行分支。

### 5. scratch canvas 复用（GC 纪律）

先例代码每渲 `new OffscreenCanvas`（`vector_host.ts:891`）——编辑 overlay
低频无妨；60Hz × ~6MB（1200×800@1.25dpr 视口位图）≈ 360MB/s 分配率是
GC 风暴。**实现取优（2026-10-05）**：`CanvasRenderer::new_offscreen` 接受
任何带 `getContext('2d')` 的对象（worker 即传 OffscreenCanvas）——直渲
直接写入既有 detail stage buffer（`getRenderBufferCanvas` 的复用缓冲），
零新增分配、零 blit，优于本节原定的"复用 scratch"下限。

### 6. 红线落点（逐条）

| 红线 | 落点 | 结论 |
|---|---|---|
| 单一渲染链（死门禁 1） | 像素仍由 wasm CanvasRenderer 绘制（core → paint plan → Rust canvas → DOM canvas）；主线程/worker 只是执行上下文 | 不破坏；本 ADR 明文：**执行上下文不是渲染链的组成部分** |
| 单写者（死门禁 2） | 补丁像素 + raw box 仍走 presenter，视觉映射仍 DetailOverlayOwner（ADR-0024） | 零新写者；直渲不得新增 backCanvas 样式写点 |
| ADR-0019 | 直渲只读 ZOOM_STATE；last_rendered_zoom 推进走既有 commit 路径 | 不破坏 |
| ADR-0016 像素预算 | 补丁 = 视口尺寸（~1.5M px @ fixture），天然 ≤ 预算；base 不受影响 | 不破坏；更大视口/DPR 由跳帧守卫兜底 |
| 热路径洁净（死门禁 5 / ADR-0013） | 60Hz 下 `reknock-phase-timing` PROF 与 `logRenderChain` 每帧发射 | **风险点**：直渲帧的诊断发射必须门控/降频（采样或 DEBUG），观测需求由契约定采样率 |
| ADR-0022 中止语义 | 同步路径消掉 await 中止窗，abort 检查点收缩为前置检查 | 简化非破坏；不新增 abort 类别 |
| ADR-0025 教训 | "加重渲染负载 → 更多 jank" 否决过候选 C | 正面回应：A′ 负载 = 每帧有界 ~5ms + 跳帧守卫，与无界并发不同 |
| 死门禁 6 | Rust 常量改动 → `npm run wasm:pdf-viewer-ui` + `npm run e2e:build` 后才跑 E2E | 流程照旧 |

### 7. 范围排除

- settle 全页/整页路径不动（worker 往返 + 量化复用 + 绘制延迟照旧）；
- 滚动 `resolveViewportRefresh` 不动（未探针，后续候选）；
- 编辑 overlay 直渲路径不动（已是主线程，不受影响）；
- 瓦片泵（ADR-0009/0025）不动。

## Consequences

### Positive

1. 手势中视口内容 = 原生分辨率、精确 visual、每帧新鲜——"手势过程中也
   清晰"达成；P2 的 15–27 档滞后签名消解（p95 拉伸 ≤3%）。
2. settle 时刻补丁已新鲜（最后一帧 visual≈target），settle 视觉无缝；
   底下 base/瓦片工作不可见。
3. worker 从 reknock 补丁渲染中解放 → 手势期瓦片泵获得更多 worker 空间。
4. 中止窗收缩（ADR-0022 检查点减少）、渲染决策链零分叉（决策仍在 core）。

### Negative

1. 手势期主线程新增 ~5ms/帧同步负载（60Hz 下 ~30% 帧预算）——由跳帧
   守卫与 blur 节奏门双重封顶；慢机器上表现为回退到今日 P2 行为而非 jank。
2. 60Hz 诊断发射需采样门控——`reknock-phase-timing` 的全量时序数据在
   直渲帧上不再免费（ADR-0013 约束与观测需求的折中）。
3. 判据若需 plan 新信号（如 previewSettled 到 TS），是 Rust→TS 桥面一处
   小扩展（需契约）。
4. 双执行上下文并存——未来维护者需理解"执行上下文 ≠ 渲染链"（本 ADR
   §6 明文防误读）。

## Tests（TDD 红灯先行；实现另起会话，本 ADR 只定契约）

### 1. Rust core 契约（先红后绿）

- `zoom_render.rs` reknock 契约 5 例改参：INTERVAL 16 语义
  （elapsed<16 抑制 / ≥16 且 blur≥0.02 触发 / in-flight 抑制保持）。
- 若判据需 plan 新信号：`plan_builder` 契约先行。

### 2. vitest 契约

- 直渲路由：符合判据的 detail 帧走直渲分支（不 postMessage worker）；
- 精确 zoom：直渲帧以 visualZoom 渲（非量化档）；
- 缓存读绕过 / 写保留：量化命中不跳渲；present 后照旧 store；
- scratch 复用：连续直渲不新建 OffscreenCanvas；尺寸变化重建；
  teardown 释放；
- 跳帧守卫：上帧成本超阈值 → 本帧跳过且几何 sync 照旧；
- 诊断门控：直渲帧 PROF 发射被采样/门控；
- 单写者回归：present/sync/reset 仍只经 DetailOverlayOwner（既有 7 例
  不破）。

### 3. E2E 契约（验收阈值）

`zoom_gesture_tilegrid_probe` 零插桩配对诊断改造为
`zoom_gesture_clarity_contract.spec.ts`。**实现后标定修正（2026-10-05
实现循环）**：原定"手势帧补丁拉伸 p95 ≤ 3%、max ≤ 6%"基于 visual 逐帧
连续缓动的假设。实现后帧级取证发现两个更高的天花板：

1. **wheel tick 量子（物理）**：`on_wheel_event` 立即应用 target 布局
   （virtual zoom），visual 逐 tick 跳变（本 fixture 120 deltaY ≈
   10.8%/tick）——任何渲染管线都无法快过 tick 本身，tick 窗口内补丁
   必有一个 tick 的滞后。
2. ~~既有的 ~100ms/周期管线 longtask 列车（TS 侧，非直渲引入）~~
   **【2026-10-05 证伪，见 postmortem
   `docs/bug-postmortems/2026-10-05-gesture-longtask-train-artifact.md`】**
   该列车不存在于产品中：verbose OFF（生产默认）下同一手势 **0 长任务**
   （5/5 次运行）。列车是 E2E harness 强制 verbose 追踪的诊断洪泛伪影
   （verbose ON 时 4-19 个长任务、run 间波动大；console sink 只是次要
   分量）。A′ 落地当日随修：console sink 生产者侧限流（ADR-0015 前置
   条件，4 契约）+ p50 标定重钉（下条）。

两者之上 p95≤3% 不可达，契约改钉物理边界：

- **p50 拉伸 ≤ 5%**——渲染落在 knock 时刻的精确 visual（零设计拉伸），
  残余测量值为斜坡段周期滞后。原定 ≤1% 是 verbose 洪泛饿死 rAF 下的
  稀疏采样伪影（采样 n≈20-40、收敛尾帧主导 → 恒 0；限流解饿后采样密度
  ×3、斜坡滞后进入中位，最差实测 0.0395——postmortem 2026-10-05）；
  该界仍与旧 worker 世界（中位 45-80%）判然分开；
- **max ≤ 15%**——滞后 ≤ 1 tick（10.8% + 余量）；worker 旧世界为
  4–8 tick（15–27 档 = 45–80%）；
- **直渲事件 ≥ 3**——路径非空转（`ts.layer.gesture-direct-render`）；
- settle 段存在双峰时序（settle 渲染落地时机/复用命中随负载摆动），
  单轮阈值契约天然 flaky——归既有 ADR-0023/0024 契约管辖，本文不再
  重复断言；
- **变异校验（测试有牙）**：flag `__pdfGestureDirectRenderDisabled=true`
  注入后 `direct ≥ 3` 必红——flag 注入轮实证 2 次（direct=0 → 红）。

A/B 对照（同构建、ratio 指标）：直渲 gesture p95 0.049–0.064 /
max 0.111–0.131 vs worker+16ms 0.057–0.117 / 0.118–0.176——16ms reknock
间隔使 worker 路径新鲜度同步受益，ratio 维度两者均 ≈1 tick；A′ 的独立
收益 = worker 卸载（瓦片泵让路）+ 每帧直渲余量 + 每渲染零位图分配/零拷贝。
（原「longtask 修复后解锁」从句随第 2 点证伪删除。）

## References

- 探针：`tests/e2e/specs/zoom_reknock_cost_probe.spec.ts`（A′ 成本裁决）
- postmortem 2026-10-05（longtask 列车证伪 + p50 重钉 + console 限流）：
  `docs/bug-postmortems/2026-10-05-gesture-longtask-train-artifact.md`；
  归因探针 `tests/e2e/specs/zoom_longtask_attribution_probe.spec.ts`
- ADR-0009（被修订的权衡与统一公式）、0013（热路径）、0015（终端 sink
  合批否决——本 ADR 落地其「生产者侧限流」前置条件于 console sink）、
  0016（预算/回滚开关先例）、0018（同帧原子）、0019（zoom 权威只读）、
  0022（中止分类）、0024（DetailOverlayOwner）、0025（负载-jank 教训）
- HANDOFF 未决 #0（需求与探针数据）、#1（P2 定性，A′ 落地后由本 ADR
  契约接管手势中新鲜度）
