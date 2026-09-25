# Handoff: Zoom Smoothness & Continuity

## Session Context
- Branch: `refactor/architecture-improvements`
- 起自 5f679b1（anchor offset 首个尝试）；本会话产出 3 个提交（见下）
- 本会话由两轮用户视频反馈驱动：①"缩放不顺滑" ②"流畅一些了，还有其它问题"

## Commits This Session

| Commit | 内容 |
|---|---|
| `5266a48` | settle 收敛 + canvas 平滑缩放 + 手势标志 + committed 队列 latest-wins（问题一~七） |
| `5055589` | 光标锚点缩放（ADR-0008）+ 瓦片 near-settle 提前渲染 |
| `ac88fd2` | 滚动瓦片节流放宽（120→16ms、移动阈值 24→4px） |

## 问题与修复

### 一~七（commit 5266a48）

**一：wasm 编译不过（阻塞）** — `5f679b1` 的 raf_loop.rs 引用了 ADR-0007
已删除的 `anchor_content_left/top` 字段。先还原为居中布局解除阻塞。

**二：收敛失败（raf_behavior）** — `frame_plans_share_render_work` 用量化
cache key 判重，reknock 帧与 settle 帧量化后同 key → settle 渲染被跳过 →
`lastRenderedZoom` 永不收敛。修复：判重增加 `render_zoom` 比较（ε=0.001）。

**三：手势期 canvas 不缩放** — 上会话为规避收敛失败禁了手势期 re-knock。
收敛修好后恢复（`!settled` 取代 `!settled && !in_gesture`）。

**四：reknock 帧乱序提交** — FIFO committed 队列手势期积压，RAF 停止后
最旧帧被 apply，容器几何回跳 81px。修复：队列改 latest-wins（容量 1）。

**五：settle 帧被图层复用拦截** — settle zoom 与已提交位图差 1.5% < 2%
复用阈值 → `requires_render=false` → settle 渲染根本不调度。修复：
`render_flow.ts` 在 `scheduleRender` 返回 null 且 zoom 已 settled 时，
直接把 plan 经 `commitRenderedFrame` 提交，收敛几何与状态。

**六：契约测试失败** — `zoom_raf_contract.test.ts` 期望 `WHEEL_GESTURE_ACTIVE`
标志（上会话写了测试没实现）。按契约实现：raf_loop 私有 thread_local +
`pub(super) is_wheel_gesture_active()`，commit 路径据此决定 queue/apply。

**七：canvas 位图跳步缩放（视频①"不顺滑"主因）** — 位图只在 reknock 呈现
时 re-box，帧间内容完全不缩放。修复：RAF tick 每帧写
`canvas.style.transform = scale(visual/lastRendered)`（compositor-only），
re-knock 呈现时与 re-box 原子重置，settle 清除。
`zoom_anti_flash.test.ts` 契约收窄为"禁止 transform 布局容器"。

### 八：光标锚点缩放（commit 5055589，ADR-0008）

视频②"缩放起点内容滑动"。根因：`on_wheel_event` 把容器瞬移到居中布局，
canvas 内容从新原点缩放。ADR-0007 删掉的锚点代码未恢复。

- `animation.rs`：新增 `anchor_content_offset`，公式
  `new_left = cursor − (cursor − old_left)/old_zoom × new_zoom`，
  clamp 到 `[0, display−viewport]`；page<viewport 或无 prior layout 回退居中
- `WheelZoomResult` 加 `anchor_content_left/top`
- `raf_loop.rs`：`on_wheel_event` 写锚点值进 `visual_layout` 和 DOM
- `present/plan_builder.rs`：`display_zoom` 匹配时取 `visual_layout` 偏移，
  保证 settle 提交帧几何与 wheel 事件一致
- 5 个新单测；ADR-0008；ADR-0007 标 Superseded；CONTEXT.md 恢复公式

### 九：settle 后瓦片逐块弹出（commit 5055589）

瓦片层 `isAnimating` 早退：动画期间不渲染，settle 后从 clearDom 的空白
开始串行填充。修复：`NEAR_SETTLE_EPS=0.02`，gap ≤ 2% 就开始调度/pump；
`clearDom` 的 zoom 不匹配分支加 settled 守卫，旧瓦片留在屏上被 LRU 逐块顶替。

### 十：滚动顿挫（commit ac88fd2）

- `SCROLL_THROTTLE_MS` 120→16（一个 RAF 帧）：原来 10 个滚动事件丢 8 个
- `VIEWPORT_MOVE_EPS` 24→4：小幅触摸板滚动也触发重排

## Test Results (final, after ac88fd2)

| 套件 | 结果 |
|---|---|
| `cargo test -p pdf-viewer-core` | **264 passed**（含 5 个新锚点单测） |
| `npx wasm-pack test --node crates/pdf-viewer-ui` | **12 passed** |
| `npx vitest run src/__tests__/` | **17 files / 104 tests passed** |
| E2E zoom 套件（4 spec）× 多轮 | **4/4 全绿**；`settle jump ratio ≈ 1`，`position delta {0,0}` |
| E2E 其余 6 spec | **全过**（tile_layer/load_pdf/page_presentation/editor_bugs/diag_doubled_page/hello） |
| clippy native + wasm32 | **0 warnings** |

## 未决事项 / 后续建议

1. **#4 手势中滚动失灵** — `apply_committed_frame` 手势期跳过所有几何写入，
   缩放动画进行中滚动不生效。需要给滚动输入单独开一条不抢容器所有权的
   路径，与 ADR-0002 有轻微摩擦。**未做，需先取证**（滚轮事件是否到达、
   卡在哪一层）。
2. **锚点 × 滚动滚动条** — 当前只写 `content_left`，放大到页面溢出视口时
   clamp 到 0，真正的锚点需要写 `scrollLeft`。属于 #4 的范畴。
3. **resize 期间锚点重置** — `syncHostLayout` 仍写居中 offset，缩放中
   resize 会把锚点重置到居中（ADR-0008 Negative 已记录）。
4. **瓦片并行渲染** — `pumpRequest` 仍是单飞行（`inFlight` 一次一张），
   一屏瓦片串行填充。若 #1 后仍觉慢，可允许 2-3 张并行。

## 重要工程约束（踩过的坑）

- **改 Rust 后必须重建两个产物再跑 E2E**：`npm run wasm:pdf-viewer-ui`
  + `npm run e2e:build`。pkg 与 target/debug 都不在 git 内，否则 E2E 跑的
  是旧二进制（本会话据此被误导过一次）。
- `zoom_wasm_binary.test.ts` 断言 wasm 文件 mtime < 1 小时，改 Rust 后
  不重建会失败——属预期。
- `cargo fmt` 会顺带改动两个无关文件（`layout_engine.rs`、
  `editor_api/mod.rs` 的 import 排序），提交前需 `git checkout --` 还原。

## Commands

```bash
cargo test -p pdf-viewer-core
npx wasm-pack test --node crates/pdf-viewer-ui
npx vitest run src/__tests__/
npm run wasm:pdf-viewer-ui && npm run e2e:build   # Rust 改动后必须
npm run e2e -- --spec "tests/e2e/specs/zoom_*.spec.ts"
```

## ADR 变更
- `docs/adr/0007-resolve-anchor-semantics.md` — 标 Superseded
- `docs/adr/0008-restore-cursor-anchored-zoom.md` — 新增（本会话）
