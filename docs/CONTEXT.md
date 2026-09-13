# PDF Viewer — 领域词汇表 (Domain Glossary)

本文档定义 PDF Viewer 项目中的核心领域概念，供 /improve-codebase-architecture 等技能使用。

## 缩放系统 (Zoom System)

### 核心状态

- **Target Zoom** — 用户意图的缩放级别（通过滚轮、下拉框、自适应宽度设置）。
- **Visual Zoom** — 当前呈现给用户的缩放级别（动画期间平滑过渡到 target）。
- **Rendered Zoom** — 已完成渲染的缩放级别（帧提交后更新）。
- **HostZoomState** — 存储上述三个 zoom 值的结构体，是缩放事实的唯一来源（Single Source of Truth）。

### 缩放操作

- **Programmatic Zoom** — 非滚轮触发的缩放下拉框选择、自适应宽度、几何探测等。调用 `set_target_zoom_instant()` 立即同步 visual 和 target。
- **Wheel Zoom** — 鼠标滚轮触发的渐进缩放。调用 `set_target_zoom()` 仅更新 target，由 RAF 动画推进 visual。
- **Settle** — 缩放动画结束，visual 达到 target。触发高质量重渲染。

### 布局计算

- **Anchor (锚点)** — 缩放中心点（页面坐标）。Post-ADR-0007: zoom always centers content — anchor computation is dead code.
- **Viewport Layout** — 容器尺寸和滚动位置：`host_width = max(display_width, viewport_width)`，`content_left = (viewport_width - display_width) * 0.5`（居中）。
- **Display Width/Height** — 页面在当前缩放下的像素尺寸：`page_width × display_zoom`。

## 瓦片渲染 (Tile Rendering)

### 核心概念

- **Tile Manager** — 协调瓦片渲染的顶层模块。持有 TileScheduler（调度）+ TileCache（缓存）+ render_queue（队列）。
- **Tile Scheduler** — 纯调度逻辑：viewport 瓦片计算、增量渲染触发、优先级分配。不持有缓存引用。
- **Tile Cache** — LRU 缓存，存储已渲染的瓦片（TileState: Pending → Rendering → Ready/Failed）。
- **Frame Cache** — 帧级缓存（原 tile_cache_legacy），存储 base layer 和 detail tile 的复用信息。

### 瓦片标识

- **Tile Key** — `{page}|{zoom}|{dpr}|{x}|{y}`，瓦片的唯一标识。
- **Tile Size** — 512×512 逻辑像素（TILE_SIZE = 512.0）。
- **Tile Grid** — 页面在给定缩放下的瓦片网格：`tiles_x = ceil(page_width × zoom / 512)`。

### 调度策略

- **Viewport Tiles** — 可见区域内的瓦片，最高优先级渲染。
- **Near-Viewport Tiles** — 可见区域外 1 个 margin 的瓦片，中等优先级。
- **Far-Viewport Tiles** — 更远的瓦片，最低优先级。
- **Incremental Tiles** — 动画期间每 N 帧渲染一次的瓦片（render_interval = 3）。

## 渲染管线 (Render Pipeline)

### 帧管理

- **Frame Plan** — 渲染计划：display_zoom + host_width/height + scroll_left/top。
- **Frame Token** — 乐观并发控制版本号，用于丢弃过时的渲染请求。
- **Committed Frame** — 已提交的渲染帧，携带几何信息供 DOM 应用。

### 渲染决策

- **Render Decision** — 根据 zoom 变化决定是否需要重渲染、使用什么缩放级别。
- **Flush Decision** — 决定是否需要刷新渲染（如页面切换、缩放变化）。
- **Preview Decision** — 决定是否使用快速预览渲染而非高质量渲染。

### 呈现表面

- **Vector Container** — 主呈现表面（Primary Surface），缩放动画期间始终活跃。
- **Raster Target** — 备用呈现表面（Follower Surface），手势开始时隐藏。
- **SetBox** — 更新容器布局盒的操作（直接设置 width/height/left/top），动画期由 Rust RAF 循环独占执行（ADR-0002 修订 + ADR-0006）。

## 渲染模块 (Render Modules)

### 职责分离

- **effective_page_plan** — 协调器模式：委托 viewport culling、source suppression、path suppression 到子模块。
- **zoom/animation.rs** — 包含 wheel zoom、zoom limits、animation state 推进（建议重命名为 zoom_core）。
- **zoom_layout** — 布局几何：layout fallback、zoom bounds、fit-to-width、mutation frame 判断。

### 渲染流程

1. **Schedule** — 调度渲染请求（FrameToken 分配）。
2. **Execute** — 执行渲染（progressive rendering 或 tile rendering）。
3. **Commit** — 提交渲染结果（CommittedFrame 入队）。
4. **Apply** — 应用到 DOM（通过 RAF loop 或直接 SetBox）。

## 编辑器相关 (Editor)

- **Paragraph Overlay** — 段落覆盖层：编辑器活跃时显示的文本覆盖。
- **Source Suppression** — 源抑制：被覆盖的原始文本不渲染。
- **Path Suppression** — 路径抑制：被覆盖的装饰性路径（如下划线）不渲染。
- **Effective Render Plan** — 最终渲染计划：决定哪些对象、文本、路径实际渲染。
