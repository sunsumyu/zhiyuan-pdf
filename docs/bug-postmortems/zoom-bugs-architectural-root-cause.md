# 缩放 Bug 类别的架构性根因 —— 系统性复盘（Postmortem）

> **更新时间**: 2026-09-11
> **范围**: 缩放系统的全部历史故障类别（2026-08 系列修复 + 6 份 ADR）
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

### 缺陷四：visual_zoom 与 target_zoom 分叉（32e1ac9，最新）

- **架构缺陷**: 程序化缩放路径（下拉框、fit-to-width、几何探测）只写 `target_zoom`，而 visual_zoom 由 RAF 动画推进——但**这些路径没有东西驱动 RAF 动画**。visual 停在旧值，`css_scale = visual / last_rendered` 爆炸为 `target⁻¹`。
- **引发的 bug**: 48% 下拉框选择后容器 `scale(2.083)`（= 1/0.48），页面放大约 4 倍且模糊。
- **修复**: `set_target_zoom_instant()` — 程序化缩放是"意图的瞬间变化"而非手势，visual 直接 snap 到 target；附 TDD 测试固化不变量。
- **教训**: 状态机每加一条新路径，都要回答"这条路径上谁推进动画状态"。语义不同的入口（手势 vs 瞬变）需要不同的写入函数，而不是共用一个"半套"流程。

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
2. **补丁只允许用于止血**。凡是被修过两次的缩放 bug，第三次必须做结构归因，禁止继续打补丁。
3. **新故障类别 = 新 ADR**。本项目 6 份 ADR 中 4 份（0001/0002/0006 + 0003 瓦片）直接源于缩放故障的架构归因，这个纪律应延续。
4. **不变量测试固化结构**。每次结构性修复都应附不变量测试（如 I1 视觉尺寸连续、I2 锚点连续、`i4_instant_zoom_snaps_visual`），让结构缺陷在回归时立即暴露，而不是等录屏。

---

## 参考

- `docs/adr/0001-zoom-single-authority-no-wasm-callbacks.md`
- `docs/adr/0002-zoom-presentation-single-writer.md`
- `docs/adr/0006-css-transform-zoom-removal.md`
- `docs/CONTEXT.md` — 缩放系统术语（Target/Visual/Rendered Zoom）
- 关键 commit: `32e1ac9`、`64ea187`、`af9e281`、`6fb721f`、`c16b709`、`578c058`
