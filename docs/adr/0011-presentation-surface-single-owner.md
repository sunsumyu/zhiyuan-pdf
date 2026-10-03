# ADR-0011: 页面表面可见性的单一所有者（PresentationSurfaceOwner）

## Status

Accepted

## Context

### 视频 i5eOPYh5IH（2026-09-30）暴露的新缺陷类

录屏逐帧分析（ffmpeg 抽 187 帧，逐帧原始像素）显示：一次大跨度放大
（display 0.54→2.45）后，**整页表面消失 2 帧（~67ms），再原样恢复**：

| 帧 | 页面白面积 | 页面区域均值 |
|----|-----------|-------------|
| f124 | 76 217 px | ~200（页面可见） |
| **f125–f126** | **900 px** | **26（= `#pdf-scroll-container` 背景色）** |
| f127 | 78 479 px | ~200（恢复） |

差异包围盒 `f124→f125 = [7,4]-[638,335]`（整个视口）；f125/f126 两帧
完全相同（非编解码噪声，是稳定状态）；f123≈f124、f127≈f128（前后静止）。
工具栏/状态栏全程在 → **页面表面（wrapper/container/canvas）被整体隐藏**，
而不是窗口或采集故障。这直接违反 ADR-0009 帧契约的既有断言
“瓦片层从不隐藏”——契约只在“可见态”采样，抓不到瞬时隐藏。

### 结构性根因：可见性有 ≥6 个写者，无单一 owner（铁律 §2）

`grep` 全仓 `.style.display/.visibility/.opacity` 写点，页面表面链
（`#pdf-content-wrapper` → `#pdf-page-container` → `#pdf-vector-main-canvas`
/ `#pdf-tile-layer` / `#pdf-render-target`）的可见性由以下**互不感知**的
入口分别写入：

| 入口 | 写 | 效果 |
|------|----|----|
| `vector_canvas_host.clearVectorCanvasHost` | container display=none / visibility=hidden | 隐藏 |
| `vector_canvas_host.hideVectorCanvasHostForPreview` | container display=none / visibility=hidden | 隐藏 |
| `vector_canvas_host.presentViewportCanvas` | container display=block / visibility=visible | 显示 |
| `pdf_layout_sync.syncLayoutBox` | wrapper display=block | 显示 |
| `pdf_viewer_dom.showDocumentWrapper` / `showEmptyDocumentState` | wrapper display=block/none | 显隐 |
| `page_presenter.commitRasterSurface` | wrapper display=block, raster display=block | 显示 |

这与 ADR-0010 修复的“transform 三写者 + box 空间无 owner”是**同一类缺陷**：
“此刻哪个表面应当可见”没有单一事实源，各入口按各自的局部时序写，合成出
“某一帧整条链都不可见”的合法中间态（例如 preview 路径先
`hideVectorCanvasHostForPreview()` 隐藏容器，再等 `commitRasterSurface`
显示——两步之间若被合成一帧，即为空白）。ADR-0010 收敛了 transform，
可见性这条分叉链尚未收敛。

### 实证（E2E 复现，2026-09-30）

- `zoom_bigburst_probe`（1500×950 窗口，deltaY=±240，35ms 间隔）：主线程
  单帧最长阻塞 **157ms**（放大期间连续 100–150ms 卡顿）。
- `zoom_bigburst_probe`：放大全程 `#pdf-tile-layer` 可见瓦片数 **= 0** 达
  6.5s（仅拉伸的主 canvas 在撑场）；瓦片层计数在 0↔4 间跳变。
- MutationObserver（`zoom_writer_probe`）确认：**静止态下**无隐藏写入；
  空白只在特定时序（preview 换面 / 大 bitmap 重绘窗口）出现，故录屏能捕到、
  常规静态断言捕不到。

## Decision

### 1. PresentationSurfaceOwner —— 页面表面可见性的唯一写者

```
PresentationSurfaceOwner（TS，render/ 同族模块）
  状态: active: 'vector' | 'raster' | 'none'   ← 唯一事实源
  接口（全部经 owner，外部不得直接写 style）:
    showVector()            → 先显示容器/主 canvas，再隐藏 raster（原子换面）
    showRaster()            → 先显示 raster + wrapper，再隐藏容器（原子换面）
    hideAll()               → 文档关闭/重置；仅此路径允许 none
    showDetail()/hideDetail()
  不变量（由帧契约逐帧断言）:
    I1 文档打开期间 active ≠ 'none'（页面表面永不整体不可见）
    I2 任何相邻两帧之间不允许 painted→unpainted→painted（无闪烁）
    I3 换面是“先显后隐”：新表面在同一 JS turn 内先可见，再隐旧表面
```

### 2. 所有 hide/show 写点收敛到 owner

`clearVectorCanvasHost` / `hideVectorCanvasHostForPreview` /
`presentViewportCanvas` / `commitRasterSurface` / `syncLayoutBox` /
`showEmptyDocumentState` 的可见性写入全部改为调用 owner 的方法；删除各自
散落的 `style.display/visibility` 直写。`clearVectorHost`（文档重置）走
`hideAll()`；preview 换面走 `showRaster()`（先显 raster 再隐容器）。

### 3. 空白不可表示

`hideVectorCanvasHostForPreview` 与 `commitRasterSurface` 之间的“先隐后显”
窗口消除：换面原子化（先显目标，再隐源），任何时刻至少一个表面可见。

## Consequences

### Positive

1. 可见性单写者 + active 事实源：视频的“整页空白 2 帧”这一类
   （P1）在结构上不可表示——没有“两个表面都不可见”的中间态。
2. 与 ADR-0010 同一范式（单写者 + SSOT），铁律 §2 一致收敛。
3. 帧契约新增“表面逐帧 painted”断言，把这一类缺陷锁进 CI。

### 验证（修复后，2026-09-30）

- **单元契约** `presentation_surface_owner.test.ts`：7/7 通过（含换面原子性、
  无 none 态、`showDocument` 重断言）。
- **E2E 契约** `zoom_surface_painted_contract.spec.ts`（真实 WebView2）：
  - 非空泛守卫：人为隐藏 container → 采样器判定 unpainted ✓（证明检测器
    能失败，不是盲采样）。
  - 不变量扫描：1500×950 窗口、0.36→2.20 大跨度扫掠，**90 帧全部 painted，
    unpainted=0，flickers=0**。
- **回归**：zoom E2E 8/8、文档生命周期 E2E 3/3、vitest 118/118；
  帧契约 canvas 漂移 max=0.01%（阈值 4%），无 LAYER-HIDDEN。
- **Rust 侧**：`start_zoom_raf_loop` 的 raster-hide/container-show 非原子对
  已删除；`raf_dom_cache` 不再解析 raster（可见性不归 Rust 管），
  `cargo clippy` 干净。

### Negative / 技术债

1. owner 覆盖 wrapper/container/mainCanvas/raster/backCanvas 五者的**可见性**；
   wrapper 的**尺寸/定位**仍由 `syncLayoutBox` 负责（尺寸 ≠ 可见性），
   `syncLayoutBox` 通过 `showDocument()` 只做“重断言可见”，不再直写 display。
2. 视频 P1 的**精确触发时序**未能在此次 E2E 复现（大窗口 + 大 delta 快滚
   下主线程阻塞 157ms，空白只在更极端的时序出现）。故帧契约的 painted
   断言是**不变量守卫**（可能在当前代码上直接通过），P1 的“红灯”由
   **单元契约**（owner 的换面原子性 + 无 none 态）承担。
3. 主线程 100–157ms 阻塞（P2 卡顿）属渲染调度问题（reknock 渲染仍在主
   线程 prep），不在本 ADR 范围；记录为独立技术债。
