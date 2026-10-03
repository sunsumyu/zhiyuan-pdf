# 缩放不顺滑 — 录屏帧分析 + 运行时探针诊断（2026-09-27）

会话：sess_399c006c（分析缩放问题）+ 本次续分析（帧分析 → 运行时探针归因）。

## 素材

- 录屏帧：`/c/temp/zoom_frames/f_001..025.jpg`（4fps，25 帧）。动作：ctrl+滚轮
  100%→26%（约 1.5s）→ 停顿 → 放大回 105%。光标约 (1163, 497)。
- 运行时探针：`tests/e2e/specs/zoom_frame_probe.spec.ts`（WebdriverIO + tauri-driver
  驱动真实应用），页面内 rAF 采样器逐帧 dump ZOOM_STATE + 各表面几何/可见性，
  另有三张手势中期截屏。fixture：`tests/e2e/fixtures/multipage.pdf`。

## 结论（TL;DR）

**缩放手势期间，用户看到的"页面"是瓦片层残留的旧 zoom 瓦片，不是平滑缩放的
canvas。** 旧瓦片（z-index 3）盖住主 canvas（z-index 1），而瓦片在手势期既不
重渲染也不清除、也不跟随缩放——于是：标签每档 wheel 都更新、容器按锚点公式
平移（内容"滑动"），但可见内容纹丝不动；settle 后瓦片清除、canvas 才突然
可见（"跳变"）；反向缩放时旧低倍瓦片就是左上角的迷你页残影。

这一根因在当前 HEAD（f4f66cc，全新构建）经探针确认仍然存在——**不是旧构建
产物问题，是活 bug**。

## 帧时间线（视频）

| 帧 | 标签 zoom | 内容实测宽(px) | 内容 scale(基准 581@100%) | 页面左上角 | 阶段 |
|---|---|---|---|---|---|
| f_001 | 100% | 581 | 1.00 | (827, 219) | 初始 settled |
| f_003 | 77% | 583 | 1.00 | (884, 307) | 手势中 |
| f_005 | 71% | 581 | 1.00 | (905, 334) | 手势中 |
| f_007 | 26% | 582 | 1.00 | (1035, 519) | 手势中（最后一次 wheel） |
| f_009 | 26% | 416 | 0.72 | (1035, 519) | 松手后开始收缩 |
| f_011 | 26% | 178 | 0.31 | (1021, 500) | 收敛中 |
| f_013 | 26% | 175 | 0.30 | (1023, 500) | 收敛停滞 |
| f_015 | 61% | 双影 | — | — | 反向放大：旧 26-30% 小页面残影 + 新渲染并存 |
| f_017 | 105% | ~433 + 小页残影 | 0.71 | — | 放大中 canvas 追赶 |
| f_019–f_025 | 105% | 602 | 1.03 | (808, 195) | settled 稳定 |

## 根因链（探针证据）

探针数据（HEAD 构建，zoom out 1.0→0.233→in→1.0）：

1. **主 canvas 在手势期确实在平滑缩放**（每帧 box 变化 + `transform: matrix`
   连续，595→502→…→143px；反向时 matrix scale 1.015→1.457 连续上升）——
   `apply_canvas_visual_scale`（`raf_loop.rs:286`）按设计工作。
2. **但 4 张 1.0-zoom 旧瓦片全程可见**（可见瓦片签名从 ms=1 到结束恒为
   `512,83,512,330,83 / n=4`，恰为 100% 页面的 2×2 网格），位置钉在容器原点，
   不缩放、不清除、不被顶替（手势期瓦片泵在 near-settle 前不渲染）。
3. **瓦片宿主 `#pdf-tile-layer` z-index 3 > backCanvas 2 > mainCanvas 1**
   （`vector_canvas_host.ts:176-178`、`tile_layer.ts ensureHost`）→ 可见内容
   = 冻结的旧瓦片。
4. **截屏验证**：标签 54% 的手势中期截屏中页面实测 595 CSS px 宽 = 100% 尺寸
   （冻结瓦片），同帧探针数据 canvas 实际 502px——canvas 被完全遮蔽。
5. 手势期容器 left/top 每档 wheel 按锚点公式平移（`on_wheel_event` 写 DOM），
   内容不动 → 视觉即"内容朝光标方向滑动"（视频 f_002–f_007 的 (827,219)→
   (1035,519) 漂移）。锚点公式用 `prior.display_zoom`（上一档 target）而可见
   内容还在 1.0 scale，误差逐档累计（26% 理想 top≈425 vs 实际 519）。
6. **settle 后**：瓦片层在 settled 且 zoom 变化时 `clearDom` → 旧瓦片突然消失，
   一直被遮蔽的 canvas 突然可见（视频 f_009 起的"松手后才收缩"+ 位图跳步）。
7. **反向缩放双影**：zoom-out settle 后新 low-zoom 瓦片就位；zoom-in 手势开始
   → 这些低倍瓦片又成为新的"旧瓦片"（z3 顶层、容器原点、不缩放），与新渲染
   同屏 → f_015/f_017 的迷你页残影。HEAD 同样复现（缩放中 161 帧可见
   w<300 的旧瓦片）。

### 视频缺陷 C（收敛停滞 @0.30）的定位

视频 f_011→f_013 卡在 0.30 对应 HANDOFF 问题二"收敛失败"（量化 cache key 跳过
settle 渲染），已由 `5266a48` 修复；本次探针 run 1 在完整 settle 窗口下收敛到
0.233 无停滞。run 2 中"卡 0.4"是探针自身时序伪影（自动化负载把 wheel 节流到
~400ms/档、settle 窗口被挤掉 + 采样器 1 位小数舍入），非产品缺陷。

### 排除项

- `main.ts` zoom 按钮 handler：无未定义变量 bug（zoom-out 按钮 0.25 下限与
  MIN_ZOOM=0.1 不一致，小问题）。
- scale(2.083) 爆炸：`apply_zoom_selection` 已 instant snap，维持排除。
- canvas transform 被高频重置 / backCanvas 可见性：探针未见异常。

## 修复方向

核心：**手势期让瓦片层退出可见层**。候选（按侵入度排序）：

1. `tile_layer.ts`：`zoomGap > NEAR_SETTLE_EPS` 进入动画分支时，把
   `#pdf-tile-layer` 整层 `display:none`（或对瓦片宿主施加与 canvas 相同的
   `scale(visual/lastRendered)`），near-settle 恢复显示并照常泵新瓦片。
   canvas 已提供连续视觉（证据 1），"旧瓦片防空白"的前提已不成立。
2. 反向手势（deltaY 变号）时立即清除低倍瓦片（针对残影的最小补丁，不治本）。
3. 瓦片几何跟随手势缩放（给每个 tile canvas 乘 visual/lastRendered）——最重，
   不推荐。

另：手势期锚点公式在瓦片层隐藏后即与像素现实一致（canvas scale =
visual/lastRendered 仍不等于 target，锚点仍有残差；可让 `on_wheel_event` 的
锚点用 `visual_zoom` 做 old_zoom 基准，残差由 RAF 收敛吸收）。

## 工程环境备注（本次踩坑）

- **E2E harness 失效根因**：`target/debug/pdf-viewer-standalone.exe` 被
  `tauri dev` / 裸 `cargo build` 覆盖成 dev 模式二进制（启动后 webview 请求
  `devUrl` http://localhost:5000 → ERR_CONNECTION_REFUSED → chrome-error 页，
  E2E 报 "app HTML never loaded"）。特征：窗口能开、DevTools 标题是
  `chrome-error://chromewebdata/`。**修复：`npm run e2e:build` 重建**（约 2 分钟）。
  与 HANDOFF "改 Rust 后必须重建两个产物" 是同一类坑，但这次是"构建方式错了"
  而非"没重建"。
- WebView2 配置目录 `%LOCALAPPDATA%/com.zhiyuan.app/EBWebView` 曾改名备份为
  `EBWebView.bak-0927`（排查中间步骤，非最终根因；确认无碍后可删）。
- 新增 `tests/e2e/specs/zoom_frame_probe.spec.ts`（手势逐帧探针，无断言）与
  `tests/e2e/specs/harness_diag.spec.ts`（harness 诊断，dump 窗口句柄页面态）。
  探针输出 `e2e_probe.log`（已删）。

---

# 续篇（2026-09-28）：瓦片修复后的残余抖动 — 暴露窗口收口

## 素材

- 用户新录屏（sess_5f3bace1，video-03299bfe）：简历页 ctrl+滚轮多段缩放
  （100→71→54→118→54→39→57→106%），每段快速连滚。双重曝光依旧：
  两份页面渲染（一份锐利偏白、一份模糊偏灰）同屏错位。
- 探针复跑 ×3（`zoom_frame_probe.spec.ts`，简单 fixture + 逐帧表面采样）。

## 探针结论（修复前，HEAD + 上会话未提交瓦片修复）

1. **上会话的瓦片修复本身有效**：手势期瓦片层正确隐藏（`tileVisFrames≈0`）、
   backCanvas（z2）全程隐藏（`bkVisibleFrames=0`）、raster 隐藏。
2. **暴露窗口在 near-settle**：用户滚动有短暂停顿时 zoomGap ≤ 2%，
   near-settle 泵开始呈现 target 瓦片并**取消隐藏整层**；用户继续滚动后，
   旧 target 瓦片要等下一次 rAF tick 才被重新隐藏。而手势期 rAF 被饿到
   **~7fps**（采样器 356 帧/11.25s，手势段每 150ms 档只采到 1-2 帧，
   settle 后 51fps）——窗口实测长达 ~140ms/档。锐利的旧缩放瓦片
   （白、清晰）叠在被 transform 连续缩放的 canvas（灰、模糊）上
   = 录屏中的双重曝光。修复前探针 `multi-surface gesture frames = 2`。
3. 次要：手势起始那帧仍有 1 帧旧瓦片闪现（隐藏发生在 rAF tick，晚于
   wheel 事件一帧）。

## 修复（本次，`src/bridge/render/tile_layer.ts`）

- **同步手势隐藏**：`notifyZoomGesture`（zoom controller 在 wheel handler 里
  于 `onWheelEvent` 之后同步调用）现在在 gap > NEAR_SETTLE_EPS 且
  `!animStarted` 时**立即** `beginGesture()`（clearDom + 整层隐藏 +
  startAnimation），不再等 rAF tick。tick 的 far 分支复用同一 `beginGesture`。
- **呈现恢复加 near-settle 守卫**：`drawTile` 取消整层隐藏的条件从
  "有 target 瓦片呈现" 收紧为 "呈现时 `|visualZoom − targetZoom| ≤
  NEAR_SETTLE_EPS`"。堵住"in-flight 瓦片在手势重新拉开后完成呈现、把
  旧 target 瓦片重新放回可见层"的第二个窗口。

## 验证

- 探针（修复后 ×2）：`multi-surface gesture frames = 0`，手势期瓦片全程
  隐藏，settle 后正常回归；`zoom_*.spec.ts` 6/6 全绿；vitest 104/104。
- `zoom_wheel_sudden_jump.spec.ts` 采样增强（补 scrollLeft/Top、scroller
  rect、container style left/top 落盘），后续排查复用。

## 实验：锚点 visual 基准（已实装又移除，勿盲目重试）

postmortem 上篇"修复方向"建议锚点 old_zoom 用 `visual_zoom`。本次实装
（raf_loop.rs 合成 `VisualLayoutState{display_zoom: visual_zoom, ...}` 作为
anchor 基准）后 `zoom_wheel_sudden_jump` 出现 **1.6px settle 漂移**（门限
1.0px），且 ~50% 复现。二分验证（还原该改动 → 3/3 全绿 delta{0,0}）确认
因果。机制：连发 burst 内所有 wheel 都读到同一个滞后的 visual_zoom，
锚点写出的 scroll 与 settle 帧从 DOM 重读的 scroll（浏览器对布局变化做
scroll anchoring 归一化，产生分数值 165.6）打架，settle 应用 164。

**结论**：要做对这个优化，锚点 scroll 必须进 ZOOM_STATE 由 settle 帧消费
同一份值（像 `anchor_content_offset` 一样），而不是 settle 时从 DOM 重读；
单纯换基准不行。当前锚点残差（快速连滚时内容逐档滑动，≤一档 zoom 差、
~200ms 内被 RAF 收敛吸收）维持现状。

## 遗留（下一步）

1. **手势期每步一次的 p90 卡顿（~180ms）**：perf 探针 + `reknock-phase-timing`
   PROF 埋点已归因——worker 渲染仅 2-11ms、投递即时，主线程被**链式渲染
   循环里的 wasm plan 构建（peek/schedule/follow-up，~8ms/次 × 多次/迭代）**
   填满，worker 回包处理被推迟（roundMs 膨胀是症状）。下一会话：plan 构建
   去重/提速，先查 `build_frame_plan_result` 的 `[PAGE-SIZE] log::info!` 去向。
   已修的伴生问题：`logPdfLayoutTrace` 曾在禁用状态也做 5 元素强制布局
   快照（~30 次/迭代，30-60ms），现为先判后照快照——settle p90 213→21ms。
2. 锚点 visual 基准的彻底方案（见上节）。
3. 瓦片并行渲染（HANDOFF 未决 #3）。
4. 诊断洪水：verbose 模式下每事件 console+IPC，会污染 perf 测量——探针
   （zoom_perf_probe.spec.ts）实测时保持 verbose 关闭；DEBUG/TRACE 已不再
   发 terminal_log IPC。
