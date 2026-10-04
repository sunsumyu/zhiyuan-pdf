# ADR-0024: DetailOverlayOwner —— backCanvas（视口补丁）纳入统一几何

## Status

Accepted

## Context

### 症状（2026-10-03 录屏，docs/bug-postmortems/2026-10-03-backcanvas-stale-patch.md）

快速缩小手势（100%→42%）后页面静止，画面上仍有一块 ≈1.5× 目标档的白底
渲染碎片伸出页面右/下缘，静止 0.5–1.4s 才消失；手势进行中全程可见两份
不同档位内容叠加（"双重曝光"）。逐帧取证排除了主 canvas 与瓦片层——
元凶是 **detail overlay（backCanvas，`#pdf-vector-detail-canvas`，z-index 2）
上残留的视口瓦片补丁**。

### 根因链

1. 手势中每个 reknock 帧只渲视口补丁并 present 到 backCanvas
   （`resolve_present_policy`：preview 期间 `use_viewport_tile=true` +
   `show_detail_overlay=true`）；reknock 节流 60ms + 串行 worker → 可见
   补丁落后 visual 1–3 档。
2. **backCanvas 是唯一不做逐帧几何补偿的呈现表面**：ADR-0009 统一公式
   `scale(visual/renderZoom)` 只覆盖主 canvas（CanvasTransformOwner）与
   瓦片（refreshTileTransforms）；backCanvas 的 CSS box 冻结在提交时刻，
   `presentViewportCanvasFromSource` 每次裸写 base 空间 left/top。
3. **手势结束无人隐藏它**：`hideDetail` 唯一活跃触发点是"下一次 base
   present 顺带隐藏"；settle 走 skip-render / zoom-state-commit 时无任何
   present → 补丁滞留（存在永久变体：settle 落在合法复用 base 分支时
   `frame_plan_requires_render=false`，永不调度新帧）。

三份 HANDOFF 记录过该债（2026-09-30 续/续三、ADR-0010 遗留项），第三次
被录像实锤后才升级为修复。

### 备选与取舍

- **方向 2（手势结束主动 hideDetail）**：一刀切隐藏会破坏合法的
  settle 复用路径——over-budget 页面 settle 时 `retainDetailOverlay`
  依赖补丁继续可见；隐藏/重现的切换点也难以定义（谁判断"手势结束"）。
- **方向 3（skip-render settle 强制 hide/present）**：只在单一路径打补丁，
  present 与 box 仍无补偿，手势进行中的 P2（落后 1–3 档）不解决。
- **方向 1（本 ADR）：backCanvas 纳入统一几何**。与 ADR-0010 对主 canvas
  的处理同构：盒子留在 base 空间，逐帧用 `s = visual/boxZoom` 重写
  映射。一次解决 P1（静止对齐）、P2（手势中对齐）与 P3（根因）。

## Decision

### 1. DetailOverlayOwner —— backCanvas 视觉映射的唯一写者

`src/bridge/render/detail_overlay_owner.ts`。记录补丁的 base 事实
（rect + recordZoom），每帧重写视觉映射：

```
s = visualZoom / zoom
left = rect.left × s,  top = rect.top × s
transform = scale(s)   （origin 0 0）
width/height 留在 base 空间（scale 负责映射）
```

与 ADR-0009 瓦片公式、ADR-0010 主 canvas 公式完全一致——**页面坐标 q
在三个表面上都落在 q × visualZoom**，旧档位补丁与新档位底图/瓦片天然
对齐，无需任何隐藏/切换。API：

- `present(rect, zoom, visual)`：present 当帧原子记录 + 写映射
  （ADR-0018 教训：同帧重算，严禁让合成器看到"新盒 + 旧变换"）；
- `sync(visual)`：逐帧重写；未跟踪时 no-op；
- `visibility 守卫`：`visibility:hidden` / `opacity:0` 时跳过写（隐藏
  的表面不得被陈旧跟踪复活）；
- `reset()`：丢弃跟踪（hide 路径 / teardown）；
- 元素键单例 `getDetailOverlayOwner(canvas)`（同 CanvasTransformOwner
  模式）；`resetDetailOverlayOwner(canvas)` 连单例一起丢弃。

### 2. 写者表（单写者铁律收敛项）

| backCanvas 几何 | 写者 |
|---|---|
| 位图像素 + raw CSS box（base 空间） | presenter（`presentViewportCanvasFromSource`，现状不变） |
| left/top/transform（视觉空间映射） | **DetailOverlayOwner（唯一）**，同 JS turn 覆盖 raw box |

### 3. 接线点（三个，全部只经 owner）

1. **present**：`commitVectorRenderResult`（vector_host.ts）——detail
   present 落地时取 pending 的 viewport rect + 帧的 displayZoom +
   `getVisualZoom()`，`present(rect, displayZoom, visual)`。
2. **sync**：`syncVisualTransforms`（tile_layer.ts）——主 canvas sync 之后
   同 tick 补 detail sync；`getDetailCanvas` 注入 TileLayerDeps
   （pdf_runtime.ts 按元素 id 解析）。两个驱动（动画时钟 knock + tile
   tick）自动共享。
3. **reset**：`hideDetailCanvas`、`presentViewportCanvas` 的 hideDetail
   分支、`clearVectorCanvasHost`（teardown）——隐藏即丢跟踪。

### 4. 不改的部分

- `deferVisiblePresent` 恒 true ⇒ `applyViewportCanvasFrame` 内的裸 box
  分支是死代码，保持不动（present 只接在真实 present 路径上）。
- P2 的 reknock 节流（60ms/2%）不调——对齐修复后，落后 1–3 档的补丁
  内容仍是旧档（设计内，settle 后被替换），但几何上不再错位。

## Consequences

### Positive

- P1 永久变体封死：settle skip-render 后补丁仍被逐帧对齐到
  `q × visualZoom`，不再溢出页面框（静止态"白色假页面"消失）。
- P2 缓解：手势中补丁与底图/瓦片同 tick 对齐，双重曝光只剩"内容档位
  差"（设计内），不再有几何错位。
- 三个 zoom 驱动表面（主 canvas / 瓦片 / backCanvas）收敛到同一条
  ADR-0009 公式，后续任何"补一层表面"都有模板可抄。

### Negative

- backCanvas 的 left/top 现在有两个相邻写点（presenter raw box → owner
  视觉映射覆盖），同 JS turn 内原子；若未来有人在 present 之后又写
  raw box，会复现冻结。ADR-0022 的帧级取证方法可抓。
- 每帧多一次 O(1) 的 style 写（有值比较短路，静止态零写入）。

## Tests（TDD 红灯先行）

`src/__tests__/detail_overlay_owner.test.ts`，7 例（实现不存在 → 红灯
"failed to resolve import"），绿灯后全量回归：

1. present 原子写视觉映射（0.65 档补丁、visual 已到 0.42——录屏中
   "补丁伸出页面"那一帧的不存在性断言）；
2. 乱序 present/tick 时序表驱动：任意交错下
   `left = rect.left × (visual/zoom)`，端到端
   `cssX = pagePoint × visual`；
3. 静止态永久变体消解：present(0.65) + 三次 sync(0.42)（模拟 settle
   skip-render 后永无 present）仍对齐；
4. present 前 sync no-op（不写无支撑的 transform）；
5. reset 后 sync 不写；
6. 隐藏表面跳过写 + 重现后沿用上次映射（retain 路径）；
7. 元素键单例 + reset 丢弃单例。

**E2E 帧级契约（2026-10-04 补齐，postmortem P4）**：
`tests/e2e/specs/zoom_detail_overlay_geometry.spec.ts`。像素无关谓词：
补丁可见的每一帧，`backCanvas.getBoundingClientRect()` 必须落在
主 canvas 视觉矩形内（容差 8px = floor/ceil 取整余量；修复前违规量级
是数百 px）。两例：

1. 检测器非空转：人为把表面推出页面右/下缘 → 必须被标记（同 JS turn
   还原，不合成中间态）；
2. 不变量：快速缩小 ≥3 档（12×deltaY120@60ms）+ 静止 2s，rAF 逐帧采样
   ——手势 sanity（finalTarget<0.8、跨度>1.25×）+ 路径非空转（补丁
   可见帧 ≥1）+ **全部可见帧含于页面** + 静止窗采样 ≥30 帧且全净
   （P1 永久变体的帧级封死）。

**变异校验（测试有牙）**：把 `s = visualZoom / baseZoom` 暂改为 `s = 1`
（补丁冻结 base 空间）→ E2E 红："2/43 visible frames had the patch
outside the displayed page"（首帧 ms=179，rects 记录补丁右/下缘超出
页面 7-9px）——正是录屏缺陷的几何特征；还原后 2/2 绿。
zoom 全套件 15 spec：并发轮 1 个假失败（4-worker 已知类别），15/15
单跑全绿（blank/p2/writer/frame/sudden_jump 逐个验证）。

**2026-10-04 契约加固**：初版把 rAF 采样帧数当非空转门槛
（`frames<60` / rest 窗 `<30`），负载下 rAF 帧率下降（长任务占主线程）
→ 误报 "sampler captured too few frames: 42-48"，而**几何不变量本身
从未失败**。改为语义门槛（`frames<15` / rest 窗 `<8`，仅防采样器死亡；
真正的非空转靠"手势确实缩小 + 补丁确实可见"两条语义断言），与姊妹契约
`zoom_gesture_frame_contract` 一致（后者不用帧数门槛）。加固后连跑 7 次
全绿；变异（`s=1`）仍被捕获（"3/43 帧越界"）——证明降门槛未削弱测试牙。

## References

- docs/bug-postmortems/2026-10-03-backcanvas-stale-patch.md（取证与
  修复方向讨论）
- ADR-0009（统一 present 公式）、ADR-0010（CanvasTransformOwner，本
  ADR 的结构模板）、ADR-0011（hideDetail 可见性归属）、ADR-0018
  （同帧原子重算）、ADR-0022（帧级取证）
- AGENTS.md 死门禁 2（单写者铁律）
