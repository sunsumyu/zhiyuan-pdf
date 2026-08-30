# ADR-0002: 缩放呈现单一写手；表面同步状态机

日期：2026-08-28　状态：已接受
前置：ADR-0001（ZOOM_STATE 单一权威）

## 背景

ADR-0001 统一了**缩放状态**的权威，但**呈现几何**（谁写 DOM 盒子/transform/scroll）仍是多写手。线上症状（录屏证据）：缩放动画期间与 settle 后屏幕上同时出现**两个错位的页面矩形**；缩放模糊与顺滑度问题伴随存在。

结构性调查（全量写入方盘点）确认了三个架构缺陷：

1. **双表面，第二表面不归动画管**。raster/preview 呈现层 `pdf-render-target` 是 `pdf-page-container` 的**兄弟节点**（`index.html:152-153`），容器的 CSS transform 对它无效。缩放动画期间**没有任何代码**隐藏或跟随它（写入方盘点 B1）。它的 CSS 盒子在生产代码中**从未被更新**（唯一写入方 `syncLayoutBox` 无生产调用方，B3）——一旦它可见（preview-first 呈现、vector 渲染失败回退），它就是一个静止的、几何陈旧的幽灵矩形。
2. **多写手竞争**。动画期间写容器/canvas 几何的路径有：Rust RAF（每帧 transform）、提交帧应用（容器盒子+scroll+transform，可能中途入队）、canvas-present 重排（提交时 canvas 盒子）、编辑 overlay 同步。各写手基于对不同"当前布局 zoom"的假设计算，无共享簿记。
3. **基数簿记曾经断裂**。`last_rendered_zoom`（容器几何所处 zoom 的唯一记录）在提交几何时曾不更新，导致 RAF scale（旧基数）与提交几何（新 zoom）分裂——同一页面两个矩形按不同基数缩放。

## 决定

**引入呈现状态机（ZoomPresenter），动画阶段它是几何的唯一逻辑写手**（State 模式 + 单一写手原则 + ports/adapters 分层）：

```
pdf-viewer-core/render/zoom/presentation.rs   ← 纯决策（可本地单测，无 web-sys）
    PresentationSurface { role: Primary | Follower, ... }
    SurfaceOp { SetBox / SetTransform / Hide }  ← 唯一的几何写操作词汇
    begin_gesture / tick / apply_committed / settle  → Vec<SurfaceOp>
pdf-viewer-ui/zoom/raf_loop.rs                ← 薄适配器：执行 SurfaceOp（web-sys）
```

核心规则：

1. **手势开始切换单一活动面**。vector 容器永远注册（Primary）；若 raster 层可见则立即 `Hide(raster) + Show(container)` —— container 保留着上次 settle 的位图（同页内容，display:none 不清除 canvas），切换无缝。**禁止 Follower 跟随方案**：raster 是 `width:100%` 铺满 wrapper 的静态流元素，wrapper 尺寸随 container 布局盒变化，任何"跟随"都会与 transform 缩放脱节（本次录屏双矩形的直接成因），结构性不可救。
2. **单一几何、单一表面**。动画期间 presenter 是唯一几何写手：container 布局盒 + 围绕 cursor 锚定的 `translate3d+scale` transform + scroll。settle 前渲染管线产出的帧一律经 presenter 应用。
3. **提交帧契约**。提交帧只携带"新布局 zoom + 锚点布局"，presenter 统一应用：更新布局盒、更新 `last_rendered_zoom` 簿记、按新基数 `s = visual / layout_zoom` 重算锚定 transform。不变量（单测固化）：
   - I1 视觉尺寸连续：`layout_prev × s_prev == layout_new × s_new == page × visual`
   - I2 锚点连续：锚点页面坐标在提交前后始终位于 cursor 视口坐标
   - I3 提交只改盒子与基数，**不产生视觉跳变**（I1+I2 的推论）
4. **settle 契约**。最终帧（layout_zoom == target）应用后：清空 transform、调度一次性清理，几何写手交还正常渲染管线（Idle 阶段管线照旧）。raster 层的再显示仍由 preview-first 呈现路径负责（`commitRasterSurface`），与手势期互斥。
5. **词汇收敛**。DOM 几何写入只能表达为 `SurfaceOp`；禁止绕过 presenter 直接写表面盒子/transform 的新代码路径（评审红线）。

## 后果

- "幽灵矩形"类别被结构性消灭：动画期间全屏只有一个活动面；raster 层要么隐藏（手势期）、要么独占（preview-first，此时无手势）。
- 缩放模糊与顺滑度获得统一解法空间：动画中提交更清晰位图只需换 canvas 内容（presenter 已保证几何一致），`css_scale` 始终有限且锚定连续。
- 复杂度下降：几何写入从 ≥4 个分散写手收敛为 1 个决策组件 + 1 个执行适配器；决策可脱离浏览器测试。
- 遗留：`resolveCanvasCssBox` 公式在 TS 侧重复内联（`vector_canvas_host.ts:259-266`），后续应收敛到 WASM 单一来源（另立任务）。

## 否决的替代方案

- **仅修 transform 公式**（历次尝试）：第二个矩形是 wrapper 内 `width:100%` 的 raster 兄弟元素，不在被变换的子树内，数学再自洽也无效——录屏已证伪。
- **Follower 跟随/盒子归一**：raster `width:100%` 参照 wrapper，而 wrapper 尺寸随 container 布局盒跳动；跟随节奏永远追不上布局跳动，脱节是结构性的。
- **对 raster 独立推导锚定 transform**：需读 wrapper 偏移与陈旧盒子，复杂且脆；切面方案一行 op 解决。
- **白屏边缘场景的取舍**：首次导航后、settle 前立即滚轮时 container 可能暂无位图（闪一帧空白，settle 渲染即补上）；概率远低于双矩形出现率，且可后续用"位图存在性检查"优化。
