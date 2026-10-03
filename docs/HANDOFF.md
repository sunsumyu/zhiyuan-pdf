# Handoff: Zoom Smoothness & Continuity

## Session 2026-10-03 — 缩放后持续模糊（ADR-0023，用户报告"不是矢量化"）

用户截图报告 119% 下文字"不是矢量化"（发虚）。程序化取证链：

1. **文档是矢量**：解析用户 PDF（WPS 导出）——3341 个文本算子、5 种字体、
   5470 条矢量直线；内嵌图像仅 5 个小图标。
2. **当前构建 settle 后是原生清晰**：同文档 119% 实测 R=1.001
   （R = 位图CSS宽/显示CSS宽，1.0 即原生无拉伸）。
3. **但用户截图文字边缘过渡是原生的 ~2×**（同 dpr=1.25 同文档同缩放逐像素
   比对：5.13px vs 2.63px）——截图捕获的是一张被 ~2× 放大的位图。
4. **复现+根因（ADR-0023）**：先缩到 0.66 再快速滚回，settle 后 R=0.812
   **持续不恢复**（base 位图停在初始 1.0 的 744×1052）。根因：mid-gesture 帧
   （preview_active → reuse_active_base_layer 恒真）提交时
   `settle_render_frame_inner` 把 `quantize(visual)` 记成 base 缓存条目——
   该 zoom 的位图从未渲染（幻影条目）；settle 时幻影条目命中 →
   reuse=true → settle 渲染被完全跳过，只做 zoom-state-commit
   （lastRendered 已更新、像素从未重画）。

**修复（ADR-0023）**：(a) remember 只在 `render_base_layer=true`（真渲染过
base 位图）时发生——断毒源；(b) `build_frame_plan_result` 的 base 复用必须
校验 key 在 TS frame cache（`stored_base_frame_keys`）中真实存在——已中毒
状态自愈。缓存条目回归单一事实源：条目 ⇔ 真实存在的位图。

**验证**：3 契约修复前红 → 绿（wasm 24/24）；复现脚本 settle 后
R 0.812 → **1.0**（位图 916×1296 原生新渲）；E2E 全量 12 spec 全绿；
vitest 139/139、core 269/269、clippy 干净。附带修复：缩小时 base 不再
复用远距条目（0.66 时也会原生重渲）。

---

## Session 2026-10-02 续五 — 渲染 abort 降级 DEBUG（ADR-0022）

E2E（打包产物）反复出现 `ERROR [TILELAYE] tile-render-failed ... stale frame`。
但 `stale frame` **不是故障**：手势推进 band/epoch 后 Rust 丢弃旧 band 的在途
tile 请求，`vector_host` 显式捕获并返回 `{aborted:true}`，tile 会在新 band
重新请求——是设计内恢复路径。tile 层的 catch 把所有错误都发
`tile-render-failed`（名字含 failed → ERROR → `console.error` + **每次一条
`terminal_log` IPC**），快速手势下污染错误流。

**修复（ADR-0022）**：abort 判定收敛到**单一所有者**——`vector_page_bundle`
（错误串出生地）导出 `isAbortedRenderRequest(message)`，`vector_host` 的消息
判定改用之（其 `!isFrameCurrent(token)` 分支留在原地）；tile 层 catch 按类
分流：abort → `tile-render-aborted` + `{level:'DEBUG'}`（DEBUG 免 IPC），
真故障仍走 `tile-render-failed` ERROR。

**验证**：源码契约 3/3 修复前红 → 绿；E2E 全量 12 spec（打包产物）全绿，
`ERROR ... stale frame` 出现 **0 次**（修复前每次手势都有），`tile-render-aborted`
以 DEBUG 记录 7 次（分类生效）；vitest 139/139、wasm 21/21、core 269/269、
clippy 干净。

---

## Session 2026-10-02 续四 — 渲染入口单次 request（ADR-0021）+ reflow 债的最终结论

ADR-0020 的 A/B 之后继续定位：`executeActualRender` 里 `peek` 与
`scheduleRender` 背靠背、输入相同，却各自 `buildRequest`（2 次构建/渲染）。
**修复（ADR-0021）**：适配器 `peek`/`scheduleRender` 增加可选 `prebuiltRequest`
参数，渲染入口用已有的 `buildRenderRequest` 构建一次、两处共享；另把
`render-current-page.scheduled` 的两次 `JSON.stringify` 移入 verbose 门控。

**诚实结论（Performance.getMetrics 三组数据：207 / 210 / 210）**：合并后
LayoutCount 无可测变化——这 ~210 次布局是**动画 UI 的正常每帧布局**
（presenter 每帧写 canvas 样式），**不是**病态强制重排。原"~62-79ms reflow
债"即正常动画成本，**关闭，不再追踪**。ADR-0020/0021 的价值是代码卫生
（冗余读/序列化消除、日志与返回同源），保留。

**验证**：源码契约 3/3 修复前红 → 绿；vitest 136/136、wasm 21/21、core
269/269、clippy 干净；E2E 全量 12 spec（打包产物）全绿、frame contract
max=0.00%。

---

## 运维：E2E 必须对打包产物跑（否则 boot 闪烁）

`tests/e2e/wdio.conf.ts` 默认用 `target/debug/pdf-viewer-standalone.exe`。该
路径可能存放两种二进制：**`tauri dev` 模式**（内嵌 `devUrl=http://localhost:5001`，
依赖 dev server 在跑）或 **`npm run e2e:build` 的打包模式**（内嵌 `dist/`，
自包含）。dev 模式下 E2E 会出现 "app HTML never loaded (#pdf-viewer-root
not found)"、以及 boot 未完成时 "wheel did not change target zoom" 等闪烁——
**那是二进制/环境问题，不是产品缺陷**。

跑 E2E 前务必先 `npm run e2e:build`（顺带解决 `target/debug/*.exe` 被残留
app 进程占用导致的 `failed to remove file ... 拒绝访问 (os error 5)`：先
`taskkill //F //IM pdf-viewer-standalone.exe` 再 build）。打包产物 E2E
**12/12 spec 全绿**（2026-10-02 实测），比 dev-server 模式稳。

## Session 2026-10-02 续三 — buildRequest 重复 scroll 读（ADR-0020）

CDP 给 scroll 容器装计数 getter 后实测：一次 12 步手势 **416 次 scroll DOM
读**，生产路径 CPU 样本里 `get scrollLeft` 居非 idle 项之首（~78ms）。
根因：`frame_plan.buildRequest`（每次渲染 ~6 次）把 `scrollLeft/scrollTop`
**各读两遍**（日志对象一次、返回对象一次），且日志对象在函数体前求值——
非 verbose 也照读。ADR-0014 只缓存了 viewport 尺寸，scroll 位置从未去重。

**修复**：buildRequest 顶部读入局部变量，日志与返回共用同一次读。
未引入 scroll 缓存 owner——scroll 在缩放期间由 Rust 直接写
（`raf_loop::on_wheel_event` 锚点滚动、`raf_committed`），缓存需跨语言失效
协议，风险不匹配；只做无歧义的重复读消除（严格改进、零行为变更）。

**验证**：源码契约 vitest 修复前红 → 绿；CDP 同条件手势 scroll 读
**416 → 190**；vitest 133/133、tsc 干净；frame contract max=0.00%、
tlg/wheel_sudden_jump/load_pdf/surface_painted 全绿。

**A/B 修正（诚实记录）**：用 `Performance.getMetrics` 对同一 14 步手势做
读两遍 vs 读一遍对比——LayoutCount 207 vs 210、LayoutDuration 79ms vs 74ms，
**几乎相同**。即同调用内的第二次 scroll 读是免费的，~190 次强制重排来自
`buildRequest` 的**调用次数本身**（每次调用前有 presenter 样式写入使布局变脏），
不是重复读。本 ADR 只消除重复属性读，**不**声称降低强制重排。
真正的重排债需减少 buildRequest 调用（合并 peek+scheduleRender，或 scroll
单 owner + Rust 写入失效协议）——另立 ADR。

---

## Session 2026-10-02 续二 — 缩放步长截断（ADR-0019）

`zoom_tile_layer_gesture` ~1/3 闪烁（"wheel did not change target zoom"）。
spec 失败路径 dump 推翻了 auto-fit 竞态假设（诊断显示 auto-fit 全部正确
skipped、初始 target=1.0）：**wheel 生效了，但步长被截断**——单步
deltaY=120 期望 target=0.9013，失败运行实测 0.9601/0.9258/0.9637（每次不同，
全在 0.9013~1.0 之间）。

**根因**：`schedule_render_follow_up` 里 `set_zoom(decision.target_zoom)`。
未结算时决策的 effective_target = **当前 visual**——mid-gesture 跟随触发
（reknock 帧提交后 rendered ≠ visual）就把用户 target 改写成动画半路值，
随后 target 一路"追着 visual 跑"，收敛在时序决定的位置。settle 后触发是
no-op → 时序依赖闪烁。此行先于本会话所有 ADR 就存在；ADR-0017 落地
`request.display_zoom = decision.target_zoom` 后它已完全冗余。

**修复（ADR-0019）**：删除该 `set_zoom`——跟随是渲染侧 actor，只读权威、
绝不回写（铁律 §2 收口）；渲染目标由 request.display_zoom 携带。附带修正：
mid-gesture 跟随帧的 `gesture_refresh`（=|visual−target|>阈值）恢复 TRUE，
走 ADR-0012 视口瓦片路径（此前被覆写后误走整页）。

**验证**：新增 2 契约（mid-gesture/settled 不得写 target_zoom）修复前红 →
绿（wasm 21/21）；`zoom_tile_layer_gesture` **6/6 绿**（修复前 ~1/3 失败）；
frame contract 2/2 且 max=0.00%；core 269/269、vitest 131/131、clippy 干净；
load_pdf/surface_painted/wheel_sudden_jump 全绿。

---

## Session 2026-10-02 续 — canvas 变换一帧滞后（ADR-0018）

ADR-0017 落地后，`zoom_gesture_frame_contract` 仍偶发 1–3 帧（/约330）
4.2–5.3% canvas 宽度漂移。E2E 逐帧 dump 定位：**每帧变换都恰好落后一帧**
（cvW = page×V(i-1)）——不是 tile 队列空闲（ADR-0017 时的初步猜测，推翻），
而是 **Rust 动画时钟与 TS 瓦片 tick 是两个无因果序的 rAF**：tick 每帧先于
Rust advance 运行，变换永远读到上一帧的 visual。平时一帧增量 <1% 无感；
手势反向/收敛帧上一帧就是一整档滚轮步长（≈4.6–5.3%）→ 违例。

**修复（ADR-0018）**：变换同步与动画推进同一 JS turn 内因果有序——
- Rust：`raf_dispatch.rs` 新增 `dispatch_animation_frame()`（固定全局
  `__pdfZoomAnimationFrame`，与 settle 敲门同模式），`raf_loop::tick` 在
  advance 之后立即调用；敲门查找改 `js_sys::global()`（Node 可测）。
- TS：`tile_layer` 抽出 `syncVisualTransforms(zs)`（owner.sync +
  refreshTileTransforms，一次读取、canvas 与瓦片同一 visual），tick 与新
  暴露的 `onZoomAnimationFrame` 共用；`pdf_runtime` 赋值固定全局。

**验证**：源码契约 vitest 5/5（修复前 5/5 红）；E2E contract 修复后
**13/14 绿且绿运行 max=0.00%（漂移消失，原 p99 2.8–3.8%/max 5.3%）**；
wasm 19/19、core 269/269、vitest 131/131、clippy/tsc 干净、其余 E2E 门全绿。
已知无关闪烁：`zoom_tile_layer_gesture` "wheel did not change target zoom"
（~1/3，本会话任何 ADR 之前即存在，boot 时序问题，待单独排查）。

---

## Session 2026-10-02 — 缩放后硬冻结（ADR-0017）

用户报"还是缩放后就卡住不动了"——不是卡顿，是**硬冻结**：渲染主线程被占满，
CDP 连 `Profiler.enable` 都超时（宏任务饿死），进程 446s CPU / 1.98GB RSS。

**根因**（CDP 零 shim 复现 + `followup.decide` 打点）：跟随渲染环不收敛——

```
followup.decide rendered=1.1249064 target=1.142407 visual=1.142407 schedule=true decideTarget=1.142407 reqZoom=1.1249064
```

逐帧完全相同、永不收敛。`schedule_render_follow_up` 已算出 `decision.target_zoom`
并 `set_zoom`，却仍用调用方传入的 request（`buildRequest(renderedDisplayZoom)`
——**已渲染的旧 zoom**）调度。于是新帧渲染旧 zoom → `commit_rendered_zoom`
再写旧 zoom → `needs_render(target, lastRendered)` 恒真 → ~200 帧/s 忙环
（detail 瓦片缓存命中、无像素工作，纯调度环）。ADR-0016 的廉价 settle 路径
把这一直存在的契约缺陷暴露成忙环（ADR-0016 是触发条件，不是根因）。

**修复（ADR-0017）**：`request.display_zoom = decision.target_zoom`——执行采用
决策（单一所有者），核心纯函数与 TS 均不动。另加 `runRenderLoop` 迭代上限
（120，`render-loop.runaway-abort`）作纵深防御：未来的不收敛退化为丢帧而非冻结。

**验证**：
- 红灯：`render/follow_up_tests.rs` 3 契约（修复前 2 红）→ 绿，wasm 19/19；
- CDP 复现：修复前 2s 内 232 帧 wedge；修复后 12 次手势 0 wedge（峰值 28 帧，
  RSS 稳定 ~878MB）；
- E2E A/B：`zoom_gesture_frame_contract` 修复前 **3/3 次 5m16s 超时挂死**
  （冻结在 E2E 直接复现）→ 修复后 ~13.5s 完成；
- 回归门：core 269/269、wasm 19/19、vitest 126/126、clippy 双 crate 干净、
  `load_pdf`/`zoom_tile_layer_gesture`/`zoom_wheel_sudden_jump`/
  `zoom_surface_painted_contract` 全绿。

**新暴露的残留（→ 已由 ADR-0018 修复，见上）**：冻结消失后，frame contract 偶发
捕获 1–3 帧（/约330）4.2–5.1% canvas 宽度漂移。修复前该窗口是无限的（冻结本身）；
修复后收敛到 1–3 帧。后续方向：变换 sync 跟随动画帧/settle 收敛点驱动，不耦合
tile 队列（详见 ADR-0017 Verification）。

---

## Session 2026-10-01/02 — 手势卡顿收口（ADR-0012~0016）

用户录屏复盘后仍报"会卡住"。逐层 CDP 剖析 + 逐帧像素分析，连修四类根因、
否决一次、再补一类：

| ADR | 根因 | 修复 | 实测 |
|---|---|---|---|
| 0012 | 手势中途每次滚轮都整页重渲（reknock 判据只看内存护栏 9.73×） | `prefer_viewport_tile`：手势中途刷新走视口瓦片 | longtask 22→5，ltMax 126→70ms |
| 0013 | 热路径无条件日志（`read()` 每次 `JSON.stringify`、每次渲染 `[PAGE-SIZE]`） | 删除/门控 | `__wbg_log` 507→26ms，帧成本 33-35→22-24ms |
| 0014 | 每帧多次 `getBoundingClientRect` 强制同步重排；`readSnap` 热路径 stringify | ViewportGeometry 单 owner（帧内缓存 + 显式失效）+ 惰性 stringify + PROF 免 IPC | gBCR 100→19ms，busy 575→491ms，p50 帧间隔 20ms |
| ~~0015~~ | ~~terminal sink 批量写~~ | **否决**：verbose 洪流下无界队列 → 主线程 `join` + IPC 序列化楔死，E2E 60s 超时 | 已回滚 + 复盘 |
| 0016 | **settle 全页渲染尖峰**（松开滚轮时 190–264ms 冻结）：判据仍以 10240 内存护栏为准，5.28× 时分配 13.9M px 位图 | `max_render_pixels`（2× 视口像素）并入 clamp/判据：base 位图封顶、视口由原生分辨率 detail 瓦片覆盖 | settle 位图 19.2M→2.01M px，**0 longtask** |

**ADR-0016 关键**：判据从"内存护栏（10240px，≈9.73×）"前移到"成本预算
（2× 视口像素，≈1.6×）"。`base_render_zoom = min(safe_render_zoom, budget_zoom)`
恒定 → base 缓存档位不再随 display 变化 → settle 不再反复整页重渲。完全
复用 >9.7× 的既有链路（css_scale + detail 瓦片），零新链路（铁律 §1/§2）。

**验证**（verbose 关，生产路径）：
- 单测：core 269/269（+4 预算契约）、wasm 16/16（+2 预算契约）、vitest 126/126、clippy 双 crate 干净；
- E2E：`zoom_gesture_frame_contract` 304 帧 0 漂移/0 错位/0 隐藏、`zoom_surface_painted_contract`、`zoom_tile_layer_gesture`、`zoom_wheel_sudden_jump`、`load_pdf` 全绿；
- CDP：settle 4.95× 时 `useViewportTile=true`、`renderBaseLayer=false`、`baseRenderZoom=1.603`、longtask=0、p99 帧间隔 42ms。

**遗留技术债**：`buildRequest` 的 `scrollLeft/scrollTop` 读（~62ms，需
reads-before-writes 重构）；present/LAYOUT INFO IPC（~79ms，重试前需满足
ADR-0015 复盘列出的前置条件）。

---

## Session 2026-09-30 续三 — CanvasTransformOwner（ADR-0010，结构性收敛）

**转折**：续二的 `scale(visual/lastRendered)` 统一公式虽让 canvas 与瓦片
互相一致，但两者**共同**偏离 ZOOM_STATE.visual 6-12%——因为它们都在用
"猜测"的分母（lastRendered / renderZoom）反推 canvas box 所处的缩放空间，
而 commit-without-present（位图复用/跳渲染帧）会让 lastRendered 前进而 box
停在旧档。四轮症状端修补（平移项 → presentScale → 统一公式 → settle 窗口）
命中 architecture-principles §5 的链分叉信号。

**根因**：canvas transform 有三个写者（Rust RAF / presentViewportCanvas /
presentScale），且"box 处于哪个缩放空间"无 owner。

**修复（ADR-0010）**：
- 新增 `src/bridge/render/canvas_transform_owner.ts`——transform **唯一写者**
  + boxZoom **唯一事实源**：`presentFrame(displayZoom)` 记录 box 空间并同帧
  重算 transform（防闪烁）；`sync(visualZoom)` 每 tick 重算
  `scale(visual/boxZoom)`；`reset()`。
- presenter（`vector_canvas_host.ts`）与 commit（`vector_host.ts`）只**告知**
  owner 新 box 空间（displayZoom），不再写 transform；tile tick 每帧
  `sync()`。
- 删除 Rust `apply_canvas_visual_scale`（`raf_loop.rs`）与 dom cache 的
  main_canvas 字段——Rust 只管动画状态，DOM 几何归 TS（ADR-0002 单写者）。

**实测（14 档大跨度手势，canvas 视觉宽 vs page×visual）**：

| | 修复前 | 修复后 |
|---|---|---|
| 超 4% 帧数 | 51 / 379 | **0 / 363** |
| max 漂移 | 18.8% | **0.01%** |

**验证**：帧契约全绿；zoom E2E 套件全绿；vitest 111/111；clippy wasm32
0 warnings；core 265/265。TDD：单测红灯先行（含变异校验），E2E 14 档红灯
先行。

**遗留**（ADR-0010 §4 记录的技术债）：
- 坐标映射 `left = rect.left × (visual/Zr)` 属 `core::coordinate_transform`
  职责，尚未迁移（tile_geometry.ts + 帧契约网格数学待收敛）
- backCanvas（detail overlay）无 scale 跟踪（同续二）
- vello-wasm + WebGPU prototype（ADR-0009 引用段）

---

## Session 2026-09-30 续二 — 帧契约守卫 + settle 窗口统一公式

**新契约**：`tests/e2e/specs/zoom_gesture_frame_contract.spec.ts`——手势全程
rAF 逐帧采样（337 帧），断言：①每张可见瓦片 kx/ky 在 512 网格 ±0.02 内；
②mainCanvas 视觉宽 = page×visual ±4%；③瓦片层从不隐藏。把平移项、
present 跳变两类靠录屏发现的 bug 锁进 CI。

**契约立刻抓到第三个缺陷**（presentScale 修复后仍有残余）：反向 settle 后、
settle 渲染完成前的 ~100ms 窗口内 `apply_canvas_visual_scale` 的 settled
分支写 `transform='none'`（假设 presenter box 已匹配 target），而
lastRendered=0.949≠target=1.0 → canvas 显示 page×0.949 vs 瓦片 page×1.0
（5% 漂移，z=[1.000,0.999,0.949] 实证）。

**修复**（`raf_loop.rs apply_canvas_visual_scale`）：消除 settled 特例，
统一 `scale(visual/lastRendered)`——settle 帧落地后 lastRendered==target →
scale=1 与 'none' 等价；窗口内则持续补偿。至此 canvas transform 无分支：
恒等于 `scale(visual/lastRendered)`（RAF 每帧）/ `scale(visual/Z_present)`
（present 帧），瓦片恒等于 `baseRect × scale(visual/Zr)`——**三层表面
（canvas、backCanvas 除外、瓦片）共享同一几何公式**。

**验证**：zoom 套件 **8/8 全绿**（含新契约）；vitest **104/104**（wasm 已
重建，哨兵通过）；clippy wasm32 0 warnings；wasm-pack test 12/12。

**遗留**：
- presentScale 分母已修正为 `renderPlan.displayZoom`（canvas box 的实际
  空间；maxCanvasDim clamp 场景 ≠ renderZoom）
- backCanvas（detail overlay）无 scale 跟踪：zoom 帧决策不走 detail（已
  验证 hide）；未来 plan 改走 detail 需同步补 scale
- vello-wasm + WebGPU prototype（GPU 矢量逐帧，ADR-0009 引用段）

---

## Session 2026-09-30 续 — canvas present 帧跳变修复（视频 12:19 归因）

**用户反馈**：平移项修复后的新录屏仍有错位——但模式变了：不再是瓦片列间
断裂（第一段手势 f_1.2 完全干净 ✓ 平移修复生效），而是 zoom-in reknock
present 时刻的**整表面错位帧**（右缘碎片 f_3.3、~10px 重影 f_4.0、内嵌帧
两份完整页面叠加）。

### 根因（present 与 RAF 的合成帧窗口）

reknock commit present 时：canvas box 重设为 `page × Z_new`（display 空间）
+ `transform='none'`；下一个 zoom RAF tick 才写 `scale(visual/Z_new)`。
两回调之间被合成的那一帧里，canvas 显示 `page × Z_new` 而瓦片在
`page × visual`——错位比例 `(visual/Z_new − 1)`。zoom-in 中 reknock 先于
visual 收敛到达时 Z_new > visual → canvas 超宽 → 右缘伸出页面外（碎片）+
与瓦片重影。30fps 录屏稳定捕捉；E2E 静态断言抓不到（settle 后恢复）。

### 修复

- `vector_host.ts`：`VectorCommitOptions.presentScale?: () => number`；
  `commitVectorRenderResult` 在 present 循环后写
  `mainCanvas.transform = scale(visual/renderZoom)`——present 合成帧即与
  瓦片同几何（`page × visual`），RAF tick 下一帧写同式（visual 前进 <1 帧）
- `render_flow.ts`：deps 增 `getVisualZoom`，presentScale =
  `getVisualZoom() / renderPlan.renderZoom`
- `pdf_runtime.ts`：`getVisualZoom: () => readZoomState().visualZoom`
- settle 帧 present：visual ≈ renderZoom → scale ≈ 1，与原 'none' 等价

### 验证

- E2E zoom 套件 **7/7 全绿**；vitest 103/104（wasm 哨兵，预期）

### 遗留

- 手势中期 E2E 断言盲区：合成帧级错位只能靠逐帧采样（zoom_frame_probe）
  或录屏捕捉。若再报错位，先跑 probe 抓 present 时刻表面几何。
- backCanvas（detail overlay）无 scale 跟踪：zoom reknock 的 detail 帧
  （useViewportTile=true）present 后同样静态。当前 zoom 帧决策不走 detail
  （已验证 hide），若未来 plan 改走 detail 需同步补 scale。

---

## Session 2026-09-30 早 — ADR-0009 呈现公式平移项修复

**用户反馈**：新录屏，手势期严重多重曝光——第二列内容断裂重叠（"FL)"、
"(Nacos, S|entineI)"）、zoom-out 时"两份页面"。先排查了"是否没编译"
（时间线 11:15 构建 → 11:18 录屏 + 行为指纹确认跑的是新代码），根因是
ADR-0009 呈现公式的几何 bug。

### 根因（平移项缺失）

呈现公式 `left = rect.left` + `scale(visual/Zr)` 只对 x=0/y=0 瓦片成立。
数学要求：`left = rect.left × s`（s = visual/Zr）。漏掉平移项时，所有
x>0/y>0 的瓦片错位 `512 × (s − 1)` px——zoom-in 内容重叠断裂、zoom-out
露出底下 canvas 成"两份页面"；`refreshTileTransforms` 只更新 scale 不同步
位置，收敛中错位量持续变化 = 闪烁。

### 修复（`src/bridge/render/tile_layer.ts`）

- `drawTile`：`left = rect.left × s`、`top = rect.top × s`（width/height
  保持 render 空间值，scale 负责缩放）
- `ActiveTile` 增 `baseLeft/baseTop`；`refreshTileTransforms` 每帧重算
  left/top/transform 三者
- E2E 契约升级：**网格对齐断言** `left/(512×a)`、`top/(512×a)` 必须是
  ±0.02 内的整数——这才是能抓到本 bug 的检验（scale 幅度不是对齐判据，
  旧档兜底瓦片 |a−1| 可达全 zoom 差但像素级对齐，如 a=0.9013 @ kx=1.000）

### 验证

- E2E zoom 套件 **7/7 全绿**；vitest 103/104（wasm 哨兵，预期）
- `zoom_tile_layer_gesture.spec.ts`：正向 settle 后 ≥4 张当前档 + 全部
  在网格上；反向 settle 同样全部在网格上

---

## Session 2026-09-29（上会话）— ADR-0009 手势期瓦片流式渲染

**用户反馈**："顺滑了，但是还是一闪一闪的，像素有错位"；随后明确要求从架构
全局找原因、符合开闭原则、参考其它框架。

### 一、架构根因（本次真正的发现）

1. **ADR-0004 的 zero-stretch 路径"已建未接线"**：Rust `TileScheduler` 完整
   实现了手势期增量渲染（`update_animation` → 每 3 帧
   `schedule_incremental_tiles` → 按 visualZoom 出请求，含单测），但 TS 侧
   `tileFacade.updateAnimation` **零调用**——`tick` 在 gap > 2% 时直接
   return。手势期可见内容退化为主 canvas 的 CSS 拉伸位图（ADR-0003 用户
   原话否决的方案），瓦片层整体隐藏到 settle——两表面坐标系不一致 =
   用户持续报告的"一闪一闪、像素错位"。
2. **历史修正**：当年 native vello 渲染 O(100ms) 的主因是 headless 链路
   （GPU→CPU 回读 + Tauri IPC），不是 vello 本身（`fdde982` 删除的
   vello_renderer.rs 证据：copy_texture_to_buffer + map_async(Read) +
   ImageBuffer + IPC）。vello-wasm + WebGPU（WebView2 已支持）恰好消除这两
   项，列为后续 prototype 候选（不阻塞本 ADR）。
3. **行业对比**（带来源）：PDF.js/Chrome = CSS 预览 + 停手重绘（中间必糊，
   否决）；SumatraPDF = 档位跳变 + 立即重绘（不连续，否决）；Mapbox GL =
   GPU 矢量逐帧（金标准）。本项目可行路径 = **瓦片流式逐帧（CPU 光栅，
   既有管线）**，长期 = vello-wasm/WebGPU prototype。

### 二、实施（`src/bridge/render/tile_layer.ts`，ADR-0009）

- **单一事实源 `tileZoomIntent`**：手势期 = 量化 visualZoom（步长 0.03），
  settle 期 = targetZoom。所有请求有效性判断统一引用，新增手势行为不再向
  各判断点散布分支（开闭原则落点）。
- **统一呈现公式**：每张瓦片无条件
  `transform = scale(visualZoom / renderZoom)`（origin 0 0）。页面点 p 在
  瓦片与 canvas 上都落在 p × visualZoom——新旧档瓦片天然共存对齐，
  **删除了整个 hide/clear 开关**（clearDom 于手势开始不再调用）。
- **手势期泵接线**：far 分支调 `updateAnimation(intent, epoch)` 唤醒 Rust
  增量调度 + `pumpNext`（与 settle 路径共用）。frame token 策略：仅在 zoom
  量化档前进时 bump（逐帧 bump 会让 Rust 把队列请求全判 stale，永远渲染
  不出）。
- **刷新覆盖收敛区间**：`refreshTileTransforms` 移到 tick 公共路径
  （每 tick 幂等刷新 ≤12 张 compositor-only transform）；near-settle 区间
  泵保持常驻（`zoomGap > ZOOM_EPS` 时 scheduleTick），否则旧档瓦片
  transform 冻结在 far→near 边界（实测 0.9215 vs 终值 0.9013）。
- **保留旧档兜底瓦片**：settle clearDom 判据加"距 target ≤ 1 档不清"——
  旧档瓦片按公式对齐，作为 target 档渲染期间的视觉兜底，LRU 自然淘汰。
- **埋点**：`gesture-stream.pump-discard` / `gesture-stream.draw-discard`
  （INFO，请求被有效性检查丢弃时触发，后续排查用）。

### 三、验证

- vitest 103/104（唯一失败 = wasm 新鲜度哨兵，本次零 Rust 改动，预期）
- E2E zoom 套件 **7/7 全绿**（dom_behavior / surface_transition /
  wheel_raf_behavior / wheel_sudden_jump / tile_layer_gesture /
  frame_probe / perf_probe）
- `zoom_tile_layer_gesture.spec.ts` 按新契约改写：手势期层从不隐藏 +
  settle 后 ≥4 张当前档 aligned 瓦片 + 反向手势层保持可见（缓存命中时
  Rust 不发请求，DOM 维持旧档瓦片，视觉由 settle canvas 保证——契约注明）
- 过程中的诊断方法沉淀：browser.execute 内 console.log 不进 wdio 日志，
  探针数据必须作为 execute 返回值带出

### 四、遗留（下一会话）

1. **vello-wasm + WebGPU prototype**：最小页面在 WebView2 渲染一页真实
   PDF，测每帧毫秒数，决定是否立项 GPU 矢量逐帧（ADR-0009 引用段）。
2. **反向手势缓存命中不重铺瓦片**（既有缺口，本次契约化接受）：反向回到
   Rust cache 已 Ready 的 zoom 时无渲染请求，瓦片层维持旧档（视觉正确）。
   如需"瓦片层始终当前档"，需加 present-from-cache 路径。
3. 手势期瓦片渲染负载（每 3% 档一批视口瓦片、worker 串行 ~10ms/张）在
   极快手势下可能积压——frame token 会丢弃过期批，主 canvas 拉伸兜底。
   若实测仍有空档，考虑 2-3 张并行渲染（HANDOFF 未决 #3）。
4. `GESTURE_TILE_ZOOM_STEP`(0.03) × Rust `render_interval`(3 帧) 两个旋钮
   可按手感联调。

---

## Session 2026-09-28（上会话）— 双重曝光收口 + 手势期主线程饥饿治理

**用户反馈**：新录屏，缩放时仍有点抖——画面里新旧两份页面渲染同屏错位
（双重曝光），快速连滚时每段都出现。

### 一、双重曝光根因与修复（`src/bridge/render/tile_layer.ts`）

探针实锤：上会话的瓦片手势隐藏有效，但留了两个暴露窗口——
① near-settle（zoomGap≤2%）泵瓦片时取消整层隐藏，用户继续滚动后旧 target
瓦片要等下一次 rAF tick 才重新隐藏；② in-flight 瓦片在手势重新拉开后完成
呈现，`drawTile` 无条件取消整层隐藏。

修复：
- `notifyZoomGesture`（wheel handler 同步调用）在 gap > eps 且 !animStarted
  时立即 `beginGesture()`（clearDom + 整层隐藏 + startAnimation），不等
  rAF tick；tick far 分支复用同一函数
- `drawTile` 取消整层隐藏加 near-settle 守卫（`|visual−target| ≤ eps`）

验证：探针 `multi-surface gesture frames` 2 → 0（×2 轮）。

### 二、手势期主线程饥饿（perf 探针 + 分层归因）

新增 `tests/e2e/specs/zoom_perf_probe.spec.ts`（无断言仪器）：wheel 同步
耗时、心跳（setInterval 5ms，区分主线程忙 vs 合成器忙）、rAF 间隔、
`__PDF_DIAGNOSTICS_HISTORY` 切片。

**重要**：跑探针时**别开 `__PDF_DIAGNOSTICS_VERBOSE`**——诊断洪水本身会
污染测量（probe 默认已关）。history 上限已从 1000 调到 5000，避免 verbose
模式下 wheel 事件计时被挤出。

分层结论（逐层排除）：
1. **wheel 同步处理 2-15ms**（`wheel-event-timing` PROF）、无持续阻塞。
2. **worker 渲染仅 2-11ms**（`reknock-phase-timing` PROF，worker 上报
   renderMs；`recvDelayMs` 经 time-origin 差校正后波动 <0.5ms → 投递即时）。
   roundMs（post→回包）45-222ms 全部是**主线程自己的任务链推迟了回包处理**。
3. **`logPdfLayoutTrace` 每次调用无条件 `readPdfLayoutSnapshot()`（5 元素
   gBCR+getComputedStyle = 强制同步 reflow）**——禁用状态也照付，~30 次/
   迭代。已修：先查 verbose/keyNode 再快照。**settle 后 rAF p90 213→21ms**
   （这个修复也惠及翻页等所有渲染路径）。
4. **plan 构建**：每步 peek/schedule/follow-up 各 2-8ms（`plan-build-timing`
   PROF），多次/迭代累积 ~30-50ms。
5. 剩余手势期 p90 ~135ms：plan 构建 + 渲染循环固有成本，需要 Rust 侧优化
   （plan 去重/提速、减少每步 peek/schedule 调用次数）。

埋点为永久保留：`reknock-phase-timing`（wasmMs/roundMs/workerMs/
recvDelayMs/blitMs）每次 reknock 一条 PROF 事件；`plan-build-timing`
（label/ms）每次 plan 构建超 2ms 一条；`wheel-event-timing`（wasmMs）每次
wheel 事件一条。

**前后对比**（perf 探针，同一 fixture）：
- 手势期 rAF 中位 **140ms → 20-34ms**（~7fps → ~30fps）、p90 ~200ms → ~135ms
- settle 后中位/p90 **86/213ms → 20/20ms**（质变）
- 双重曝光 multi-surface gesture frames **2 → 0**

### 三、实验后放弃：锚点 visual 基准

postmortem 上篇建议的"锚点 old_zoom 用 visual_zoom"已实装又经二分移除——
它在 `zoom_wheel_sudden_jump` 引入 ~1.6px settle 漂移（门限 1.0px，~50%
复现）。要做对需要把锚点 scroll 放进 ZOOM_STATE 由 settle 帧消费同一份值，
勿直接重试单纯换基准。

### 验证（最终）

zoom 套件 7/7（含 perf probe）；vitest 103/104（1 个 wasm 新鲜度哨兵，属
预期）；tsc/clippy wasm32/fmt 清。

### 遗留（下一会话）

1. **Rust plan 构建去重/提速**（`build_frame_plan_result` 每步 2-8ms × 多次
   /迭代，累积 ~30-50ms）。先查 `[PAGE-SIZE] log::info!` 去向，再看 peek/
   schedule/follow-up 是否可合并为一次调用。
2. 锚点 visual 基准的彻底方案（scroll 进 ZOOM_STATE）。
3. 瓦片并行渲染（HANDOFF 未决 #3）。

---

## Session 2026-09-27 深夜（上会话）— 瓦片遮蔽根因修复

**产物**：`docs/bug-postmortems/zoom-frame-analysis-2026-09-27.md`（帧分析 + 探针归因全记录）。

**根因（探针实锤，HEAD 活 bug）**：手势期用户看到的"页面"是瓦片层残留的旧
zoom 瓦片（z-index 3），它盖住平滑缩放的主 canvas（z-index 1）且不跟随缩放
→ 标签动、内容冻结、锚点平移表现为"内容滑动"；settle 清瓦片时 canvas 才
突然可见（跳变）；反向放大时旧低倍瓦片即 f_015 迷你页残影。

**修复**（`src/bridge/render/tile_layer.ts`，未提交）：
- 手势启动（gap > NEAR_SETTLE_EPS 且 !animStarted）：`clearDom()` 旧瓦片 +
  `setTilesHidden(true)` 隐藏整层——canvas 已提供连续视觉，旧瓦片"防空白"
  前提不成立
- 恢复两条路：① 新 zoom 首块瓦片 `drawTile` 成功呈现时；② 完全 settle 时
  兜底（反向回到 Rust 缓存仍 Ready 的 zoom 时不会有新渲染请求，①永远不触发）
- `clear()` / `!path` 重置路径同步恢复

**验证**：
- vitest 103/104（唯一失败 `zoom_wasm_binary` 是 wasm mtime<1h 新鲜度哨兵，
  重建 wasm 即绿）
- 新增 `tests/e2e/specs/zoom_tile_layer_gesture.spec.ts`：正/反向手势断言
  瓦片层 hide→restore 全链路 ✓
- 既有回归全绿（单跑）：zoom_dom_behavior / zoom_surface_transition /
  zoom_wheel_raf_behavior / zoom_wheel_sudden_jump / tile_layer / hello ✓
- 探针复跑（`tests/e2e/specs/zoom_frame_probe.spec.ts`）：手势中期截屏页面
  实宽 ≈595×visual（内容跟随缩放，修复前是 595px 冻结瓦片）；反向截屏无残影

**新增工程坑（两个新环境坑，均已在本文档顶部约束区补过一次）**：
1. **`target/debug` 二进制被 `tauri dev`/裸 `cargo build` 覆盖成 dev 模式**：
   webview 请求 `devUrl`(localhost:5000) → ERR_CONNECTION_REFUSED → E2E 报
   "app HTML never loaded"。特征：窗口能开、DevTools 标题
   `chrome-error://chromewebdata/`。修复：`npm run e2e:build`。
2. **TEMP 盘满（TEMP=D:\tmp，D 盘 99%）→ E2E 全灭**：msedgedriver 会话
   profile（scoped_dir*）写不进 → "failed to write prefs file"/session not
   created；ts-node 转译临时文件写不进 → mocha "A dynamic import callback was
   not specified"。修复：清 `/d/tmp/scoped_dir*` 与 `*.tmp`（本次释放 4.8G）。
   症状像 harness 坏了，实际是盘满。F 盘也只剩 ~2.3G（target/ 所在），后续
   cargo 构建可能告急。
3. wdio 的多个 `--spec` 参数互相覆盖（只跑最后一个）；跑多 spec 用 glob，
   失败时按 HANDOFF 惯例**逐个单跑**判定。

---

## Session Context（上一会话：zoom 平滑性修复）
- Branch: `refactor/architecture-improvements`
- 起自 5f679b1（anchor offset 首个尝试）；本会话产出 3 个提交（见下）
- 本会话由两轮用户视频反馈驱动：①"缩放不顺滑" ②"流畅一些了，还有其它问题"

## Commits This Session

| Commit | 内容 |
|---|---|
| `5266a48` | settle 收敛 + canvas 平滑缩放 + 手势标志 + committed 队列 latest-wins（问题一~七） |
| `5055589` | 光标锚点缩放（ADR-0008）+ 瓦片 near-settle 提前渲染 |
| `ac88fd2` | 滚动瓦片节流放宽（120→16ms、移动阈值 24→4px） |
| `f4f66cc` | 锚点 × 溢出：scroll 补偿 + flex→block 布局 |

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

### 十一：锚点 × 溢出滚动（commit f4f66cc）

锚点公式把 `content_left` clamp 到 `[0, display−viewport]`，当页面溢出
视口时 clamping 吃掉的偏移会让光标页面点漂移。本提交补上 `scroll_left/top`
补偿：wheel 事件在写 `content_left` 的同时，把 clamping 的剩余写入
`scrollLeft/scrollTop`。

同步发现：`#pdf-scroll-container` 的 `display:flex;justify-content:center`
会把溢出的内容对称挤出，左侧部分因 `scrollLeft` 不能为负而永远不可达。
改为 `display:block` —— 小页面的居中仍由容器的显式 `left` 偏移负责
（`compute_viewport_layout_result` 居中，写入 `visual_layout`），溢出时
偏移落在 `[0, display-viewport]` 内，`scrollLeft` 可达。

`anchor_layout` 辅助函数同时返回 `(content_left, scroll_left)`；
`WheelZoomResult` 新增 `anchor_scroll_left/top`；`raf_loop::on_wheel_event`
直接写入 scroller DOM。

## Test Results (final, after f4f66cc)

| 套件 | 结果 |
|---|---|
| `cargo test -p pdf-viewer-core` | **265 passed**（含 6 个锚点单测） |
| `npx wasm-pack test --node crates/pdf-viewer-ui` | **12 passed** |
| `npx vitest run src/__tests__/` | **17 files / 104 tests passed** |
| E2E zoom 套件（4 spec）× 2 轮 | **4/4 × 2 全绿**；`settle jump ratio ≈ 1.006`，`position delta {0,0}` |
| E2E 其余 6 spec | **全过**（tile_layer/load_pdf/page_presentation/editor_bugs/diag_doubled_page/hello） |
| clippy native + wasm32 | **0 warnings** |

## 改动文件（本轮）

- `crates/pdf-viewer-core/src/render/zoom/animation.rs` — `anchor_layout`
  (content_left + scroll_left 联合计算)、`WheelZoomResult` 新增
  `anchor_scroll_left/top`、1 个新单测
- `crates/pdf-viewer-ui/src/zoom/raf_loop.rs` — `on_wheel_event` 写
  `anchor_scroll_left/top` 到 scroller DOM
- `src/index.css` — `#pdf-scroll-container` 从 `display:flex;
  justify-content:center` 改为 `display:block`

## 未决事项 / 后续建议

1. **#4 手势中滚动失灵 — 假设已被探针否证**。探针（缩放动画中设 `scrollTop=300`）
   显示滚动位置从头到尾保持 300 不被覆盖，收敛也正常。原描述缺乏证据；
   若现象真实存在，更可能是主线程/合成器性能问题而非逻辑问题，需重新取证
   （录视频逐帧对齐时间戳）。
2. **resize 期间锚点重置** — `syncHostLayout` 仍写居中 offset，缩放中
   resize 会把锚点重置到居中（ADR-0008 Negative 已记录）。
3. **瓦片并行渲染** — `pumpRequest` 仍是单飞行（`inFlight` 一次一张），
   一屏瓦片串行填充。若滚动仍觉慢，可允许 2-3 张并行。

## 重要工程约束（踩过的坑）

- **改 Rust 后必须重建两个产物再跑 E2E**：`npm run wasm:pdf-viewer-ui`
  + `npm run e2e:build`。pkg 与 target/debug 都不在 git 内，否则 E2E 跑的
  是旧二进制（本会话据此被误导过一次）。
- `zoom_wasm_binary.test.ts` 断言 wasm 文件 mtime < 1 小时，改 Rust 后
  不重建会失败——属预期。
- `cargo fmt` 会顺带改动两个无关文件（`layout_engine.rs`、
  `editor_api/mod.rs` 的 import 排序），提交前需 `git checkout --` 还原。
- **E2E 并发 4 worker，偶发假失败**：同一 spec 重跑即过（本会话
  `zoom_wheel_raf_behavior` 遇到一次）。判定失败前先单独重跑该 spec。
- 写 E2E 探针时把输出落到仓库内的文件（如 `e2e_probe.log`），
  `/tmp` 在 Git Bash 下不可靠；用完记得删。

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
