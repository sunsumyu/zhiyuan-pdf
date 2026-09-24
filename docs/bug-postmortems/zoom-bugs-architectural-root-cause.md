# 缩放 Bug 类别的架构性根因 —— 系统性复盘（Postmortem）

> **更新时间**: 2026-09-22
> **范围**: 缩放系统的全部历史故障类别（2026-08 系列修复 + 6 份 ADR + 2026-09-22 surface SetBox 与位置连续性）
> **关联文件**: `crates/pdf-viewer-core/src/render/zoom/`、`crates/pdf-viewer-ui/src/zoom/`、`docs/adr/0001` ~ `0006`
> **性质**: 本文不是单个 bug 的复盘，而是对"缩放 bug 为什么反复发作"的类别级分析

---

## 现象汇总

项目迭代过程中，缩放系统先后出现过以下用户可见症状（commit 记录可考）：

| 症状 | 首次修复 commit |
|------|----------------|
| settle 后位图拉伸不锐化（模糊） | `af9e281` 2026-08-20 |
| 缩放动画期间/之后屏幕出现**两个错位的页面矩形**（幽灵矩形） | ADR-0002 系列重构 |
| 程序化缩放（下拉框/自适应宽度）**先大后小闪烁** | `af9e281`、`192614c` |
| 提交帧期间 flash | `f927b9a`、`10e5cf6`、`d479bc9` |
| 渲染开始时重新设盒导致 flash | `10e5cf6` |
| 双重缩放（double-scaling flash） | `578c058`、`1af4024`（salvage 系列） |
| **48% 下拉框页面放大约 4 倍**（scale(2.083) = 1/0.48） | `32e1ac9` 2026-08-30 |
| 页面左侧/上方漂浮空白瓦片盒 | `32e1ac9`（同 commit 第二缺陷） |
| **松手后页面突然跳到 N 倍**（手势期可见面不缩放，settle 一帧释放） | 2026-09-22 surface SetBox |
| **松手后页面横向滑动**（settle 重新居中，实测 182.5px） | 2026-09-22 位置连续性（本文缺陷六） |

同一类症状（flash、尺寸错误、模糊）反复以不同形式复发，直至对应的**架构级**修复落地才被根治。

---

## 核心结论

**缩放 bug 与系统架构的关系是因果性的，不是相关性的。** 每一类反复发作的缩放 bug 背后都对应一个结构性缺陷；只在原架构上打补丁的修复全部复发或变体复发，结构性重构之后对应故障类别才被消灭。

ADR-0001 对此有明确表述：修复后 *"过期镜像"与"回调未注册"两个故障类别被结构性消灭*；ADR-0002 同样：修复后 *"幽灵矩形"类别被结构性消灭*。

---

## Bug ↔ 架构根因映射

### 缺陷一：双重状态权威（ADR-0001）

- **架构缺陷**: `ZOOM_STATE.target_zoom`（动画/意图）与 `VIEWER_SESSION.current_zoom`（会话快照）各自存储，靠调用方人肉镜像同步。
- **引发的 bug**:
  1. wheel 路径漏调 `set_zoom` 镜像 → settle 渲染用旧 zoom → 位图拉伸不锐化。
  2. settle 采用跨 WASM 可注册回调（`onZoomSettle`），注册发生在模块求值时而 WASM 尚未初始化，异常被 try/catch 吞掉 → 回调槽永远为空 → 最终矢量重绘永不发生。
- **结构性修复**: `ZOOM_STATE` 成为缩放事实的唯一可写存储；settle 改为信封投递（数据停泊 + 固定函数敲门），废除整条回调链。
- **教训**: 凡是"两份状态靠约定同步"的设计，同步必然在某条路径上被遗漏——这不是实现纪律问题，是结构必然。

### 缺陷二：呈现层多写手竞争（ADR-0002）

- **架构缺陷**: 动画期间写容器/canvas 几何的路径有 ≥4 个（Rust RAF 每帧 transform、提交帧应用、canvas-present 重排、编辑 overlay 同步），各写手基于对"当前布局 zoom"的不同假设计算，无共享簿记；且 raster/preview 表面是 `pdf-page-container` 的**兄弟节点**，容器的 transform 对它无效，生产代码中它的盒子**从未被更新**。
- **引发的 bug**: 缩放动画期间与 settle 后屏幕上同时出现两个错位的页面矩形（幽灵矩形）——一个是被 transform 的容器，一个是几何陈旧的静止 raster 兄弟元素。
- **结构性修复**: ZoomPresenter 状态机成为动画期唯一几何写手，DOM 写入只能表达为 `SurfaceOp { SetBox / SetTransform / Hide }`；手势开始时切换单一活动面（Hide raster + Show container）。
- **教训**: *对第二个矩形修 transform 公式在数学上再自洽也无效*——它根本不在被变换的子树内（ADR-0002 否决记录）。多写手问题只能用单一写手解决，不能靠算式修正。

### 缺陷三：CSS transform 补偿机制（ADR-0006）

- **架构缺陷**: 容器尺寸 = `page × render_zoom`（滞后值），再用 CSS transform `scale(display_zoom / render_zoom)` 补偿差异。三个 zoom 值 + 一个补偿变换互相耦合，计算分散在 `zoom_css.rs`、`raf_transform.rs`、`pdf_layout_sync.ts` 三处。
- **引发的 bug**: 程序化缩放双重缩放闪烁（容器先以旧尺寸渲染再被 transform 拉伸）；瓦片除以 `cssScale` 期望补偿但补偿已移除后定位错误。
- **结构性修复**: 整体删除 CSS transform zoom（~260 行死代码、3 个模块），容器尺寸直接设为 `page × display_zoom`（SetBox），瓦片坐标直接用 display 空间。
- **教训**: "滞后值 + 补偿变换"是闪烁的温床——中间态必然有一帧被用户看见。直接计算最终值（无补偿）从结构上消灭了中间态。

### 缺陷四：visual_zoom 与 target_zoom 分叉（32e1ac9）

- **架构缺陷**: 程序化缩放路径（下拉框、fit-to-width、几何探测）只写 `target_zoom`，而 visual_zoom 由 RAF 动画推进——但**这些路径没有东西驱动 RAF 动画**。visual 停在旧值，`css_scale = visual / last_rendered` 爆炸为 `target⁻¹`。
- **引发的 bug**: 48% 下拉框选择后容器 `scale(2.083)`（= 1/0.48），页面放大约 4 倍且模糊。
- **修复**: `set_target_zoom_instant()` — 程序化缩放是"意图的瞬间变化"而非手势，visual 直接 snap 到 target；附 TDD 测试固化不变量。
- **教训**: 状态机每加一条新路径，都要回答"这条路径上谁推进动画状态"。语义不同的入口（手势 vs 瞬变）需要不同的写入函数，而不是共用一个"半套"流程。

### 缺陷五：单写手只覆盖了一半几何——表面与容器不对称（2026-09-22）

- **架构缺陷**: ADR-0006 删除 CSS transform zoom 后，几何的唯一写手变成了"直接写 DOM 盒"，但写手只被约束到**容器**（`pdf-page-container`）。渲染表面（`#pdf-vector-main-canvas`）虽然也是几何字段（`canvas/page width|height` 在 Z-004 的字段清单里），却没有被纳入手势路径的写手。契约写了"单一写手"，但没有写"单一写手必须覆盖全部可见面"。
- **引发的 bug**: ctrl+滚轮连续缩放时，手势期间容器变大而用户真正看到的 canvas 保持上一次提交的 CSS 盒 → **视觉上完全不缩放**；松手 settle 那一帧 `apply_committed_frame` 一次性把 canvas 写成 `page × display_zoom`，整段手势累积的缩放量在一帧内释放。用户主观描述为"缩放后松开滚轮，页面突然放大/缩小到 N 倍"，并判断为最严重问题。
- **误诊记录**: 该现象在文档里曾以三条推测根因记录（wheel `delta_y` 未 clamp、`max_zoom = 30.0` 过宽、`visualZoom` 被误读为极端值）。E2E 实测证伪：wheel 路径从未产出极端 zoom，`targetZoom` 全程等于几何倍率。**推测根因全部指向"数值错误"，真实根因是"几何写入面缺失"**——这正是本复盘开篇"先问结构，再修公式"的反面教材。
- **结构性修复**:
  1. 表面盒写入收敛为唯一 helper `raf_dom_cache::set_surface_box(w, h)`，手势与 settle 两条路径共用；`DomCache` 缓存 `#pdf-vector-main-canvas`。手势盒与 settle 盒同值 → `settle jump ratio = 1`（E2E 实测）。
  2. 顺带收窄了上一轮为"几何跳动"引入的 `WHEEL_GESTURE_ACTIVE` 作用域：从共享的 `zoom_store.rs` 迁到 `raf_loop.rs` 模块私有 `thread_local!`，只暴露 `begin_wheel_gesture`（私有）/`end_wheel_gesture`（`pub(super)`，仅 `stop_zoom_raf_loop`）/`is_wheel_gesture_active`（`pub(super)`，只读）。共享容器里的会话标志会让无关模块互相影响，属于同一类"权威边界不清"。
  3. 契约补齐：Z-004.1（手势 SetBox 必须覆盖渲染表面）、Z-004.2（所有权最小作用域）、C-002 增加 I1 视觉尺寸连续与 I4 所有权不得超出 RAF 会话。
- **教训**: **"单一写手"契约必须按可见面枚举，而不是按模块枚举**。ADR-0006 是一次正确的结构简化，但它在删除 transform 的同时把"缩放如何被看见"从一条隐含路径变成了一条显式路径——凡是被删除机制顺带承担过的职责，都要在新机制里显式落位，否则它会以"某个面不更新"的形式在 settle 时刻集中暴露。另外：**"松手后跳变"这类症状天然指向时序，但根因可以是几何面的缺失**；判断依据不能是症状出现的时间位置，只能是"手势期可见面的盒是否等于 settle 盒"。

### 缺陷六：同一几何量的两个生产者——手势锚点 vs 帧居中（2026-09-22）

- **架构缺陷**: `content_left/content_top` 有两个独立来源：手势路径经 `animation.rs::compute_anchor_content_offset` 写入 `ZOOM_STATE.visual_layout.content_left/top`（保持鼠标下点不动的锚点偏移），settle 路径经 `plan_builder.rs::compute_viewport_layout_result` 生成 `frame.content_left/top`（始终居中）。`apply_committed_frame` 在 settle 时同时写 `width/height/left/top`，其中 `width/height` 只有一个生产者（frame = committed render），`left/top` 却有两个。手势释放后 settle 直接消费 `frame.content_left/top`，覆盖手势写入的锚点值。
- **二次缺陷（读取顺序）**: 第一次"修复"只是在 DOM 写入点改成读 `ZOOM_STATE.visual_layout`，但**同一函数内更早的 settled 分支已经把 `visual_layout` 覆盖成 frame 的居中值**。于是"先覆盖、后读取"，读到的仍是居中值——修复无效（实测仍红 `leftDelta = -182.5`）。真正的修复是把捕获点前移到 settled 分支**之前**：`let gesture_offsets = ...` 先取值，再用同一组局部变量写状态和写 DOM。
- **引发的 bug**: ctrl+滚轮缩放后松手，页面横向滑动 182.5px（实测 `container.left` 从 339.75 → 157.25），而宽度保持不变（1110.30）。用户主观描述为"页面突然横向跳动"。
- **误诊记录**: 第一遍修复只测了 `width` 连续性（settle jump ratio ≈ 1），断言通过即认为修复完成。正是 `left/top` 未被断言覆盖，才让这个缺陷藏了一轮。**断言盲区让同类缺陷逃脱验证**。第二遍修复又踩了顺序陷阱：改成读 `visual_layout` 后仍红，说明"读对源"与"在对的时间读"是两件事，必须用同一二进制/规格对做红→绿对照才能证伪。
- **结构性修复**: `apply_committed_frame` 在 settled 分支覆盖 `visual_layout` **之前**捕获 `gesture_offsets`，再用捕获值同时写状态与写 DOM；仅当捕获为 `None` 时回退到 `frame.content_left/top`。`width/height` 仍从 frame 读取。同时把 E2E 断言从"只测 scale 连续"扩展到"scale + 位置双连续"（`|containerLeftDeltaSettle| ≤ 1 && |containerTopDeltaSettle| ≤ 1`），并加 wasm unit `settle_frame_keeps_gesture_content_offset`。
- **教训**:
  1. **同一几何量不得有两个生产者同时写入可见 DOM**。如果手势期和 settle 期对同一字段有不同的计算，必须在 settle 时明确选择一个权威源，而不是让两个值竞争。
  2. **断言必须覆盖全部几何维度**——只测宽度不测位置，等于让一半的几何处于无验证状态。本次缺陷之所以漏掉，正是因为第一遍修复的断言只覆盖了 `width`。
  3. **单一权威不只是"读哪个源"，还包括"在哪个时刻读"**。同一函数内先覆盖再读取，等价于没修。凡是要从将被复写的状态里取值，必须把取值点前移到复写之前并显式命名（`gesture_offsets`），让顺序成为代码可读的一部分。
  4. 这与缺陷五（表面与容器不对称）同构：**"单一写手"契约不仅要覆盖全部可见面，还要覆盖每个可见面的全部几何字段**。`width/height/left/top` 是同一个 SetBox 的四个字段，它们的权威来源必须一致或显式分离。

---

## 为什么缩放域的 bug 特别容易长在架构上

缩放系统的固有复杂度放大了任何结构缺陷：

1. **三个 zoom 值天然要漂移**（target 意图 / visual 呈现 / rendered 已渲染）。权威设计一旦出错（双权威），状态分裂是必然的。
2. **跨 WASM 边界**。Rust 决策 + TS 执行，任何"回调注册"式协作都有初始化时序陷阱（ADR-0001 事故二）。
3. **双表面**。vector 容器与 raster 层是 DOM 兄弟节点，几何一致性没有单一写手就必然出现幽灵矩形（ADR-0002）。
4. **RAF 异步循环**。多写手在各帧交错写入，任何中间态不一致都会被用户看见（flash 家族）。

这四个特性解释了为什么**别的模块打补丁能好，缩放系统打补丁必复发**——异步 + 多状态 + 多表面的组合让每个"局部修复"都在另一条路径上留下同样的结构洞。

---

## 补丁 vs 结构性修复：复发模式

修复历史的可验证规律：

| 修复类型 | 例子 | 结局 |
|----------|------|------|
| 修公式 | 对双矩形修 transform 公式（历次尝试） | 无效——录屏证伪，矩形不在变换子树内 |
| 修时序 | reorder CSS transform before syncLayoutBox（`192614c`） | 缓解单个 flash 变体，flash 家族继续复发 |
| **改权威** | 删除镜像存储、信封投递（ADR-0001） | "过期镜像/回调丢失"类别消灭 |
| **改写手** | ZoomPresenter 单一写手（ADR-0002） | "幽灵矩形"类别消灭 |
| **删机制** | 移除 CSS transform（ADR-0006） | "双重缩放闪烁"类别消灭 |

时序类补丁（reorder、remove redundant call）在 git 历史上出现了 5+ 次（`f927b9a`、`d479bc9`、`10e5cf6`、`192614c`……），这正是**结构性问题以变体形式复发**的证据：每次都修掉了"这一处"的中间态，但产生中间态的机制还在。

---

## 当前架构的遗留风险

主干架构经 6 轮 ADR 加固后是健康的（单一权威、单一写手、无 CSS transform、决策可脱离浏览器测试）。仍有两处已知架构级残留：

### 1. `resolveCanvasCssBox` 公式 TS 侧重复内联（ADR-0002 明确遗留）

- 位置：`src/bridge/render/vector_canvas_host.ts:259-266`
- 问题：同一几何公式在 Rust 与 TS 各存一份，违反单一来源。这正是历史上"cssScale 计算分裂"bug（`c16b709` 修复对象）的同款土壤。
- 状态：ADR 标注"另立任务"，**尚未收敛**。

### 2. `canvas.rs` 的 unwrap 群（rust-project-standards 审计发现）

- 位置：`crates/pdf-viewer-ui/src/render/canvas.rs`（1145 行，渲染主路径）
- 问题：30+ 处 `.unwrap()`（`window().unwrap()`、DOM 查询链 `.unwrap().unwrap()`）。zoom 路径上任何 DOM 查询返回 `None` 即 panic → 白屏。
- 风险：与 ADR-0001 事故二同构——"异常被吞/直接崩溃"类故障在跨边界代码里是结构性高危。

---

## 方法论结论（给下一个缩放 bug）

1. **先问结构，再修公式**。出现缩放异常时，按顺序排查：
   - 是否出现了**第二份状态**（某个新变量在镜像 zoom 信息）？
   - 是否出现了**第二个写手**（RAF 循环之外有代码在写容器/canvas 几何）？
   - 是否出现了**第二处计算**（同一公式在 TS/Rust 各算一遍）？
   - 新路径的**动画谁推进**（程序化入口是否 snap 了 visual）？
   - 唯一写手是否**覆盖了全部可见面**（容器 + 渲染表面），还是只覆盖一半？（缺陷五）
   - 同一可见面的**全部几何字段**（`width/height/left/top`）是否各有唯一权威来源？是否有一个字段存在两个生产者（手势锚点 vs 帧居中）？（缺陷六）
   - 会话级标志是否放在**共享状态容器**里，导致无关模块互相影响？（缺陷五）
2. **补丁只允许用于止血**。凡是被修过两次的缩放 bug，第三次必须做结构归因，禁止继续打补丁。
3. **新故障类别 = 新 ADR**。本项目 6 份 ADR 中 4 份（0001/0002/0006 + 0003 瓦片）直接源于缩放故障的架构归因，这个纪律应延续。
4. **不变量测试固化结构**。每次结构性修复都应附不变量测试（如 I1 视觉尺寸连续、I2 锚点连续、`i4_instant_zoom_snaps_visual`、`Z-GESTURE-001/002/003`），让结构缺陷在回归时立即暴露，而不是等录屏。
5. **按可见面计数，而不是按症状计时**。症状"发生在 settle 那一刻"不等于根因在 settle 时序；先量"手势期的可见面盒 == settle 盒？"，再谈时序（缺陷五的误诊记录）。
6. **断言覆盖全部几何维度**。只断言 `width` 会漏掉 `left/top`（缺陷六的误诊记录）。任何"几何连续性"断言必须按字段逐一覆盖，不能用一个维度代表整组几何。

---

## 参考

- `docs/adr/0001-zoom-single-authority-no-wasm-callbacks.md`
- `docs/adr/0002-zoom-presentation-single-writer.md`
- `docs/adr/0006-css-transform-zoom-removal.md`
- `docs/CONTEXT.md` — 缩放系统术语（Target/Visual/Rendered Zoom）
- 关键 commit: `32e1ac9`、`64ea187`、`af9e281`、`6fb721f`、`c16b709`、`578c058`
