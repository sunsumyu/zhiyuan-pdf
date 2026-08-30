# 纸鸢 Zhiyuan — Domain Glossary

## PDF 算符与状态

- **TextState** — 共享文本状态字段（font_size, char_spacing, word_spacing, horizontal_scaling, render_mode, tl）+ 矩阵操作（op_cm, op_bt, op_tm, op_td, op_t_star），嵌入在读路径 GraphicsState 和写路径 PdfTextState 中。定义于 `text_state.rs`。
- **TextMatrixCore** — PDF 文本矩阵三元组（ctm/tm/tlm）及不变量操作（concat_ctm, begin_text, set_text_matrix, translate_text, advance_text, text_render_matrix）。读写路径共享的最底层抽象。
- **GraphicsState** — 读路径状态：TextState + 图形专属字段（line_width, line_cap, line_join, miter_limit, stroke/fill color, alpha, current_font）。
- **PdfTextState** — 写路径状态：TextState + 写入专属字段（font_alias 字节数组）。仅在 pdf_write.rs 内部可见。

## 路径

- **读路径** — 内容流解析（content_parser.rs）：遍历 PDF 算符，构建 RenderObject/StyledRun 向量。使用 GraphicsState 追踪状态。
- **写路径** — 内容流修补（pdf_write.rs）：遍历 PDF 算符，应用文本重排补丁（TextReflowPatch），发射修改后的算符。使用 PdfTextState 追踪状态。

## 字体

- **字体解析链** — pdf_write_font/ 子目录：finder（系统字体查找）→ face（TTF 解析 + 字形子集提取）→ embed（PDF 对象构造）。三模块间通过 SystemFont 数据契约连接。
- **ParsedFont** — 从系统字体或嵌入字体解析得到的字体数据结构，包含字形映射、度量信息、原始字节。

## 缩放渲染

- **缩放权威 (Zoom Authority)** — `ZOOM_STATE`（zoom_store.rs thread_local）：target_zoom / visual_zoom / last_rendered_zoom 等缩放事实的唯一可写存储。所有缩放写入必须经单入口函数；`VIEWER_SESSION.current_zoom` 不是独立存储，而是权威在 viewer session 读快照中的派生投影（字段名保留以兼容 JS 契约，值从 target_zoom 填充）。语义约定：current ≡ target（意图值），已呈现的真实缩放用 last_rendered_zoom 表达。
- **Settle 信封 (Settle Envelope)** — 缩放动画稳定后，Rust RAF 循环构建 FramePlanRequest → schedule_render_frame_request 产出 RenderFrameEnvelope，经 RENDER_LOOP_STATE 停泊、由 Rust 直呼固定全局函数敲门交给 TS 渲染循环的投递机制。取代旧的跨 WASM 可注册回调链（onZoomSettle）——推送帧而非推回调，无注册生命周期。
- **表面操作 (SurfaceOp)** — ADR-0002 定义的唯一几何写操作词汇：`SetBox`（更新容器布局盒）、`SetTransform`（设置 CSS transform）、`SetDisplay`（显示/隐藏表面）。DOM 几何写入只能表达为 SurfaceOp；禁止绕过呈现状态机直接写表面盒子/transform 的新代码路径。
- **呈现表面 (Presentation Surface)** — ADR-0002 中参与几何同步的 DOM 元素。`VectorContainer`（Primary，缩放动画期间始终活跃）和 `RasterTarget`（Follower，手势开始时隐藏）。单活跃面原则：动画期间只有一个表面接受几何写入。
- **已提交布局 (CommittedLayout)** — 渲染管线产出的帧所携带的几何契约：display_zoom + left/top + width/height + scroll_left/scroll_top。提交帧只携带"新布局 zoom + 锚点布局"，呈现状态机统一应用。
- **帧令牌 (FrameToken)** — 渲染管线的乐观并发控制版本号。单调递增，分配于 schedule_render_frame_request。每个 async await 边界检查 `isRenderFrameCurrent(token)`：若另一帧已调度（token 递增），当前帧过期并中止。5+ 检查点形成完整过期检测链。
- **可见表面 (VisibleSurface)** — 渲染管线的呈现层选择：`preview`（快速光栅预览）、`vector`（Vello 矢量渲染）、`detail`（高分辨率细节）、`raster`（回退光栅）。决定当前哪一层向用户可见。

## 瓦片渲染

- **瓦片缓存 (Tile Cache)** — 管理瓦片渲染结果的独立 LRU 缓存，包含 FrameCache 作为底层存储。瓦片缓存键格式：`{page}|{zoom}|{dpr}|{x}|{y}`。LRU 淘汰策略：页面切换时清空，缩放级别变化时标记为可淘汰。
- **瓦片管理器 (Tile Manager)** — 协调瓦片渲染的调度器，负责视口瓦片优先渲染、异步队列管理、缩放动画期间增量渲染。使用 FrameToken 进行乐观并发控制。
- **瓦片键 (Tile Key)** — 瓦片的唯一标识：`{page}|{zoom}|{dpr}|{x}|{y}`，其中 x/y 为瓦片在页面中的逻辑坐标（基于 512×512 逻辑像素网格）。
- **瓦片状态 (Tile State)** — 瓦片的生命周期状态：`Pending`（等待渲染）、`Rendering`（正在渲染）、`Ready`（已渲染完成）、`Failed`（渲染失败）。

## 渐进式质量渲染

- **渲染质量 (Render Quality)** — ADR-0004 定义的三级质量系统：`Low`（动画期间快速渲染，0.75x DPI）、`Medium`（过渡期间平衡质量，1.0x DPI）、`High`（settle 后高清渲染，1.5x DPI）。每级质量控制 DPI 倍率、文本质量、细节级别、每帧最大项目数和渲染预算。
- **质量状态机 (Quality State Machine)** — 管理渲染质量 transitions 的状态机：动画开始时重置为 Low，动画期间逐步升级到 Medium，settle 时跳转到 High。使用帧计数器控制升级时机。
- **质量感知瓦片键 (Quality Tile Key)** — 扩展的瓦片标识，包含质量级别：`{page}|{zoom}|{x}|{y}|{quality}`。低质量瓦片可作为高质量请求的 fallback，但同级质量不视为复用。
