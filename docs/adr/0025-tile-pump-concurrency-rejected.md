# ADR-0025: 瓦片泵有界并发 —— **已否决（Rejected）**

## Status

Rejected（2026-10-04 实现后当次回退；保留本文作为记录，防止重蹈）

## Context

HANDOFF 未决 #3 记「瓦片并行渲染 — `pumpRequest` 单飞行，一屏瓦片串行填充；
若滚动仍慢可允许 2-3 张并行」。本次据此推进。

### 取证（2026-10-04 实测）

1. **worker CPU 几乎空闲**：临时探针量单瓦片 `workerMs` **p50=1ms /
   p90=4.4ms**（wasm 渲染本身极廉）。
2. **队列在手势中堆积**：`zoom_p2_probe`（16 次 ctrl-wheel 突发，
   到 ~5.28×）实测**队列峰值 47–55**，仅完成 36，`cache.rendering`
   恒 0–1。
3. **TS 泵是单飞行**：`tile_layer.ts` 的 `if (!inFlight)` 每次只起 1 个
   渲染，`.finally → scheduleTick → rAF` 使**每个瓦片至少等一个 rAF**
   （~16ms/张 → ~60 张/s）。
4. **Rust 对并发无感**（两个 Explore agent 交叉确认）：队列是优先级排序
   `Vec`，`next_render_request` = 弹出最佳有效项，瓦片状态全 per-key；
   唯一串行约束是 TS 的 `if (!inFlight)`，无测试钉住。

**据此假设**：泵是瓶颈 → 有界并发（上限 4）可让串行 worker 背靠背进食，
队列清空速率约 4×，缓解 P2 的档位滞后。

## 实现

`tile_layer.ts`：`inFlight` 单槽 → `inFlightCount + inFlightKeys`（Set），
`pumpRequest` 改同步返回 boolean（丢弃 stale/离页/重复键），`pumpNext`
改 `while (inFlightCount < 4) { nextRequest → pumpRequest }` 循环；
`markRendering` 先于下次 dequeue；重复键守卫（Rust 队列可含同键重复项）。

- 单元红灯契约 `tile_pump_concurrency.test.ts` 6 例：cap=4 填满、
  完成后回填、同键不双渲、单请求不回归、stale 不占槽、mark 顺序
  —— **实现后 6/6 绿**（机制确实按设计工作）。
- 全量门禁绿：tsc 0 / vitest 151+哨兵 / core 269 / wasm 24 / clippy 双目标 0。

## 决定性对照（同代码，只改 `MAX_INFLIGHT_TILES`）

| 指标（`zoom_p2_probe`，16 突发到 5.28×） | cap=1（串行） | cap=4（并发） |
|---|---|---|
| 队列排空窗口（首非零 → 末非零） | **3315ms** | 3630 / 3750 / 4393ms（更慢） |
| 主线程长任务数 | 6 / 9 / 26 | 23 / 18 / 30（更多） |
| rAF 帧数 | 156 / 173 / 172 / 185 | 137 / 135 / 93 / 105 / 116 / 155（更低） |
| **`cache.ready`（真实完成瓦片数）** | **36** | **36（完全相同）** |
| 12 张可见瓦片达成时刻 | 2515ms | 1008 / 3572 / 3916ms |

**`maxReady` 两者完全相同（36）**：真实瓦片工作量一致，并发**零吞吐收益**；
而帧数一致偏低、长任务一致偏多 → 并发**反而增加主线程 jank**（4 个
`drawImage` 在同帧竞争 + 更多跨线程消息）。

## 根因修正（原假设被证伪）

**队列深度 52 不是积压，是手势期持续流式入队的正常 churn**：

- `schedule_incremental_tiles` 每 3 帧在**新档位**重排一批视口瓦片；
- Rust 只在**出队时**清理陈旧/重复项（`tile_manager.rs:100-113`），
  队列长度因此包含尚未出队的陈旧/重复项，是**上界**而非待办数；
- 真实工作 = 每档位约 12 张 × 3 档 ≈ 36 张 —— **正是 `ready=36`**，
  两配置都按时完成。

即：**没有可加速的积压**。60ms 的 reknock 节流是**内容新鲜度**问题
（可见瓦片滞后 visual 1–3 档，P2 残余），**不是**吞吐问题；泵并行
无法改善它，只增加 jank。这与 ADR-0009「手势期串行 ~10ms/张」的
记录一致——串行本就是设计，不是缺陷。

## Decision

1. **回滚** `tile_layer.ts` 的有界并发泵（`git checkout` 还原到
   ADR-0024 的已提交状态）。
2. **删除**随实现新增的 `tile_pump_concurrency.test.ts` 与
   `zoom_tile_queue_drain.spec.ts`（机制契约随实现一并撤）。
3. **关闭** HANDOFF 未决 #3：瓦片并行**经实测否决**，非"待办"。
4. 保留本文记录负结果与修正后的根因，防止未来再次误判队列深度。

## 若未来重试的前置条件

- 必须先证明**存在真实积压**（可加速的待办），而非流式 churn：
  用 `next_render_request` 的**有效请求数**（去重后）而非 `queue_size`
  作为积压指标。
- 若瓶颈确为 worker 串行（单瓦片耗时接近/超过 rAF 间隔），才谈并行；
  本环境单瓦片 1–4ms，远低于 16ms，**不满足**。
- 并发若做，须先在**同代码 A/B**（只改上限）下证明排空窗口与长任务
  不劣化——本次 A/B 显示劣化。

## Consequences

1. 瓦片泵维持单飞行（`inFlight` 单槽）。HANDOFF 未决 #3 关闭。
2. **P2 残余的机制在 2026-10-04 后续取证中修正**（见下）：P2 描述的是
   **detail overlay 补丁**的内容档位滞后（60ms reknock 节流 + 3% 档），
   **不是**瓦片网格——瓦片网格（`pdf-tile-layer` DOM）在手势中本就**冻结**
   （持开页 settle 瓦片、被拉伸），settle 才重排（ADR-0009 设计）。
   详见 Consequences #5。
3. 本次取证确认 `cache.rendering` 在 rAF 采样下几乎恒为 0（1–4ms 渲染
   在两次 ~16ms 采样之间完成），**不可**作为 E2E 并发观测信号；队列
   `queue_size` 因含陈旧/重复项亦非待办数。后续探针须用去重有效数。
4. 无源码净变更（实现已回滚）；仅本 ADR 与 HANDOFF 入库。
5. **P2 取证更正（2026-10-04）**：为量化 P2 写了只读探针
   `tests/e2e/specs/zoom_gesture_tilegrid_probe.spec.ts`，得到决定性事实：
   - 手势全程 `pdf-tile-layer` 只有 **4 张瓦片且 `renderZoom==1`**（开页
     settle 瓦片，`scale(visual/1)` 拉伸到 ~5×）；新档位瓦片 **settle 后**
     才出现（n: 4→5→9→12，档位跳到目标 zoom）。即**瓦片网格手势中冻结**。
   - 手势中 `backCanvas`（detail overlay 补丁）**可见**（`backVis=1`），
     是手势中的新鲜内容源；其内容档位**无法从 DOM 几何反推**（CSS box
     只是视口 rect + ADR-0024 视觉映射，不含渲染档位），需渲染路径插桩。
   - **故"补丁落后 1–3 档"不能映射到瓦片网格**（后者按设计冻结）。P2 的
     准确表述是补丁的内容新鲜度问题；本探针的价值正是防止该误映射。
