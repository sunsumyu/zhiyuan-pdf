# pdf-viewer-ui · render（画布渲染编排）

> 范围：`crates/pdf-viewer-ui/src/render/`（23 个文件，约 2880 行，含子目录 `canvas/` 7 个文件）。
> 上游：`pdf_viewer_core::render`（帧调度纯数据、workflow DTO、layer / progressive / prepared_scene / effective_page_plan / frame_cache、tile_cache / tile_v2 / tile_manager、quality、renderer trait——详见 `docs/modules/core-render.md`）；同 crate 的 `present/`（帧计划组装与呈现状态）、`zoom/`（缩放权威）、`page/page_store`、`editor/`（覆盖层与调试追踪）、`viewer/viewer_controller`。
> 下游：TS 桥 `src/bridge/render/`（render_wasm_api.ts 调 free_api、tile_bridge.ts 调 renderFacade* 瓦片段、vector_worker.ts 调离屏渐进入口）；ui 内部被 `present/present_store`、`zoom/raf_loop`、`viewer/viewer_controller` 反向调用。

## 职责
UI 侧渲染编排：持有帧令牌（FrameToken）的分配/判定/落定状态机与渲染循环信封停泊，把 core 的渲染决策落到 2D 画布（整页、渐进切片、编辑器覆盖层回绘）。对外提供两套 wasm 入口——`renderFacade*` 冻结命名空间与渲染域自由函数——并承载瓦片管理器（TileManager）与渲染质量状态机的 thread_local 宿主。本模块不含渲染策略计算，纯决策均在 core（见上游文档）。

## 文件与方法
（各文件不含内联测试；`canvas/` 单列为末节。）

### `mod.rs` — 渲染编排子模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod` × 16 | canvas、canvas_overlay、commit、五个 core 转发 shim、free_api、host_runtime、progressive_workflow、render_store、tile_cache、tile_host、wasm_facade、workflow |

### `render_store.rs` — 帧令牌状态机：分配/判定/排队与落定（FrameToken 唯一宿主）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::render::scheduler::*` | HostRenderState / RenderFrameEnvelope / RenderFrameTransition 等纯类型转出 |
| thread_local | `RENDER_STATE` | 泛型特化为 serde_json::Value 的帧状态容器 |
| pub | `reset_render_state()` | 清空帧状态为默认 |
| pub | `is_render_frame_current(token)` | token 0 恒假；否则与活动令牌比对 |
| pub | `schedule_render_frame(plan, …)` | 免渲染或与在飞/排队帧同工则跳过；在飞空闲则分配令牌并返信封，否则排队（不动活动令牌） |
| pub | `settle_render_frame(token, from_value)` | 仅匹配在飞令牌可落定；落定后把排队帧提升为在飞 |

### `workflow.rs` — core 工作流类型转出 + 落定时的缓存与缩放提交副作用
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::render::workflow::*` | 帧信封组装、渐进启停 DTO 等转出 |
| pub | `settle_render_frame_inner(…)` | 经 render_store 落定；被接受则提交已渲染 zoom、记录视口提交时刻、记忆 base 层/细节瓦片或清空细节瓦片 |

### `commit.rs` — 渲染结果提交（落定 + 页面尺寸落地）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `RenderCommitResult` (struct) | 提交结果 DTO：accepted / next_frame / 页面宽高 |
| pub | `commit_render_result(token, zoom, w, h)` | settle 帧；被接受则写回页面尺寸 |

### `host_runtime.rs` — 渲染循环信封停泊与启停（RAF 单驱动的 UI 侧）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `HostRenderLoopState` (struct) | 循环激活标志 + 待发帧信封 |
| thread_local | `RENDER_LOOP_STATE` | 上述状态的 thread_local 容器 |
| pub | `queue_render_loop_frame(frame)` | 停泊信封；循环空闲则取出并置激活 |
| pub | `advance_render_loop_frame(next)` | 优先新帧，其次取停泊帧，否则置循环休眠 |
| pub | `reset_render_loop_runtime()` | 重置循环状态为默认 |

### `progressive_workflow.rs` — 渐进渲染编排：任务构建/步进/取消与整页入口
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `start_progressive_render()` | 由视口 bbox + 覆盖层构建渐进任务并存入 page_store |
| pub | `step_progressive_render(…)` | 劫持画布按预算/上限绘一片，完成即清任务 |
| pub | `cancel_progressive_render()` | 清除渐进任务 |
| pub | `render_page(canvas_id, image_cache)` | 取消任务后经劫持画布整页渲染 |
| pub | `render_page_offscreen(canvas_js, image_cache, dpr)` | 离屏画布版整页渲染 |
| pub | `step_progressive_render_offscreen(…)` | 离屏版渐进步进（与屏上版近似复制） |

### `free_api.rs` — 渲染域自由 wasm 导出（TS 桥经 getWasmApi 发现式调用）
| 可见性 | 方法 | 功能 |
|---|---|---|
| wasm | `resolveCanvasCssBox(request)` | 计算矢量画布元素 CSS 盒 |
| wasm | `resolveFramePlan(request)` | 组装帧计划结果 |
| wasm | `takeFramePlan(request)` | 同 resolveFramePlan（重复别名） |
| wasm | `scheduleRenderFrame(request)` | 组帧计划后排程帧信封 |
| wasm | `commitRenderResult(token, zoom, w, h)` | 转调 commit.rs 提交渲染结果 |
| wasm | `settleRenderFrame(token, zoom)` | 带渲染 zoom 落定帧 |
| wasm | `abortRenderFrame(token)` | 无渲染 zoom 作废帧 |
| wasm | `isRenderFrameCurrent(token)` | 查询帧令牌是否仍有效 |
| wasm | `scheduleRenderFollowUp(zoom, request)` | 读缩放权威判补渲染，必要时 set_zoom 后重排帧 |
| wasm | `queueRenderLoopFrame(frame)` | 转调 host_runtime 停泊信封 |
| wasm | `advanceRenderLoopFrame(frame)` | 转调 host_runtime 推进循环 |
| wasm | `stepZoomFramePlan(request)` | 取缩放预览帧计划 |
| wasm | `resolveViewportRefresh(request)` | 细节层视口刷新决策 |
| wasm | `resolveHostScrollRefresh(request)` | 同 resolveViewportRefresh（重复别名） |
| wasm | `resolveLayoutFallback(request)` | syncHostLayout 缺数据回退布局 |
| wasm | `resolveFitToWidth(vp_w, page_w)` | 适宽缩放计算 |
| wasm | `MIN_ZOOM()` / `MAX_ZOOM()` | 缩放界限常量导出（0.1 / 30.0） |
| wasm | `isImmediateMutationFrame(reason)` | 判定编辑器立即可变帧 |
| wasm | `resolveRenderExecutionPlan(changed, plan)` | 组装 base/detail 两层执行计划 |
| wasm | `resolveLayerExecutionPlan(changed, plan)` | 单层渲染开关决策 |
| wasm | `resolveLayerPresentDecision(use_detail, plan)` | 细节覆盖层显示/保留决策 |
| wasm | `updatePageViewport(zoom, dpr, …)` | 写回 page_store 视口状态 |
| wasm | `renderPage(canvas_id, image_cache)` | 转调 progressive_workflow 整页渲染 |
| wasm | `renderPageOffscreen(canvas_js, image_cache, dpr)` | 离屏整页渲染 |
| wasm | `startProgressiveRender()` | 渐进启动 |
| wasm | `stepProgressiveRender(…)` | 屏上渐进步进 |
| wasm | `stepProgressiveRenderOffscreen(…)` | 离屏渐进步进 |
| wasm | `cancelProgressiveRender()` | 取消渐进任务 |
| wasm | `resolveProgressiveRenderPolicy(request)` | 渐进策略（是否渐进/预算/上限） |
| wasm | `touchFrameCacheEntry(is_tile, key)` | 帧缓存键置顶 |
| wasm | `storeFrameCacheEntry(is_tile, key)` | 存帧缓存键并返回淘汰键 |
| wasm | `resetFrameCache()` | 清空帧缓存键 |

### `wasm_facade.rs` — 冻结 v1 的 `renderFacade*` wasm 命名空间（渐进/提交/缓存/瓦片/质量/stub）
| 可见性 | 方法 | 功能 |
|---|---|---|
| 私有 | `stub(api)` | 返回 `{implemented:false}` 占位结果 |
| wasm | `facade_start_progressive()` | 渐进启动（转调 progressive_workflow） |
| wasm | `facade_step_progressive(…)` | 渐进步进 |
| wasm | `facade_cancel_progressive()` | 取消渐进任务 |
| wasm | `facade_render_page(canvas_id, image_cache)` | 整页渲染 |
| wasm | `facade_commit_result(…)` | 提交渲染结果（经 commit.rs） |
| wasm | `facade_abort_frame(token)` | 无渲染 zoom 作废帧 |
| wasm | `facade_is_frame_current(token)` | 查询帧令牌有效性 |
| wasm | `facade_touch_cache` / `facade_store_cache` / `facade_reset_cache` | 帧缓存键置顶/存储/清空（经 present_store） |
| wasm | `facade_update_viewport(…)` | 更新视口并调度瓦片，返回统计 |
| wasm | `facade_start_tile_animation(target_zoom)` | 启动瓦片动画（标记全部可淘汰） |
| wasm | `facade_update_tile_animation(visual, token)` | 每帧推进动画增量调度 |
| wasm | `facade_end_tile_animation(token)` | 结束动画并排最终高清瓦片 |
| wasm | `facade_next_tile_request()` | 取下一个瓦片渲染请求 |
| wasm | `facade_mark_tile_rendering` / `facade_mark_tile_ready` | 推进瓦片 Rendering/Ready 状态 |
| wasm | `facade_reset_tile(…)` | Rendering 翻回 Pending（作废重排） |
| wasm | `facade_is_tile_ready(…)` | 查询瓦片是否就绪 |
| wasm | `facade_clear_tile_cache(page)` | 清空指定页瓦片 |
| wasm | `facade_tile_stats()` | 瓦片缓存统计 |
| wasm | `facade_start_quality_animation()` | 质量状态机重置为 Low（函数内独立 thread_local） |
| wasm | `facade_update_quality(animating, settled)` | 按动画状态推进质量（另一份独立 thread_local） |
| wasm | `facade_get_quality()` | 读当前质量级（第三份独立 thread_local） |
| wasm | `facade_get_quality_dpi(q)` / `facade_get_quality_budget(q)` | 质量级 → DPI 倍率 / 预算毫秒 |
| wasm | `facade_snapshot_png` / `facade_prewarm_cache` / `facade_set_quality` / `facade_set_debug_overlay` | 四个预留 stub（恒未实现） |

### `tile_host.rs` — 全局唯一 TileManager 宿主（thread_local 必须置于模块作用域）
| 可见性 | 条目 | 功能 |
|---|---|---|
| thread_local | `TILE_MANAGER_HOST` | 共享瓦片管理器实例（文件头注警示函数内声明会裂成多个 static） |
| pub | `with_tile_manager(f)` | 借出共享 TileManager 执行闭包 |

### `tile_cache.rs` — 瓦片类型转出枢纽（legacy base+detail + tile_v2 + tile_manager）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::render::tile_cache::*` | 旧 base+detail 缓存类型全量转出 |
| pub | `pub use core::render::tile_manager::*` | 瓦片管理器（缓存+调度协调）类型转出 |
| pub | `pub use core::render::tile_v2::*` | 瓦片键/状态/LRU 缓存类型转出 |

### 五个 core 转发 shim（逻辑已迁 core，仅维持旧导入路径）
| 文件 | 可见性 | 条目 | 功能 |
|---|---|---|---|
| `effective_page_plan.rs` | pub | `pub use core::render::effective_page_plan::*` | 有效渲染计划（覆盖层抑制+视口裁剪）转出 |
| `frame_cache.rs` | pub | `pub use core::render::frame_cache::*` | 帧缓存键薄封装函数转出（状态绑定在 present_store） |
| `layer.rs` | pub | `pub use core::render::layer::*` | base/detail 层执行计划转出 |
| `prepared_scene.rs` | pub | `pub use core::render::prepared_scene::*` | 页面场景空间索引转出 |
| `progressive.rs` | pub | `pub use core::render::progressive::*` | 渐进任务与策略类型转出 |

### `canvas_overlay.rs` — 编辑器覆盖层在页面画布上的回绘
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `path_bbox_summary(path)` | 求路径段包围盒宽高 |
| 私有 | `summarize_overlay_render_plan(plan)` | 生成前 4 行/每行 8 run 的调试摘要串 |
| 私有 | `count_overlay_underline_runs(plan)` | 统计带下划线样式的 run 数 |
| 私有 | `draw_editor_marker_page(…)` | marker 绘制缝隙（已标 dead_code，未接线） |
| 私有 | `draw_graphic_markers(…)` | 重绘图形 marker 的源矢量对象 |
| pub(crate) | `draw_active_editor_shell_overlay_page(…)` | replaces_source 转持久绘制；否则仅记插入符调试（不落画布） |
| pub(crate) | `draw_persisted_paragraph_overlay_page(…)` | 白填替换区遮蔽底图 → 重绘图形 marker → 按持久渲染计划逐 run 绘制 |

## canvas/ — 2D 画布渲染器（CanvasRenderer 实现，拆自单体 canvas.rs）

### `canvas/mod.rs` — 子模块清单与共享类型
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `CanvasRenderer` (struct) | 2D 渲染器：ctx/canvas/dpr/画布高度 + 劫持/透明表面标志 |
| pub(crate) | `CoordinateMode` (enum) | PageSpace / EditorLocal 坐标模式（当前行为等价） |
| pub(crate) | `use draw::draw_text_run_core` | 维持旧导入路径转出 |
| pub | `use draw::{render_run_standalone, TextMetricsSnapshot}` | 独立 wasm 文本入口与度量快照转出 |

### `canvas/surface.rs` — CanvasRenderer 生命周期：构造、尺寸同步与页面表面变换
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `new_overlay(canvas)` | 以 alpha 上下文构造编辑器覆盖层渲染器 |
| pub | `new_hijacked(target_id)` | 按 id 抓取既有页面画布直接绘制 |
| pub | `new_offscreen(canvas_js, dpr)` | 由 JS 画布句柄构造离屏渲染器 |
| pub | `sync_size(w, h, zoom)` | 劫持模式跳过；否则重设画布/CSS 尺寸并乘 dpr×zoom 变换 |
| pub | `measure_text_metrics(…)` | 设字体测文本宽度，空文本以 "Hg" 兜底 |
| pub | `clear_dirty_rect(x, y, w, h)` | 透明面 clear，否则白填脏矩形（外扩 0.5px） |
| pub(crate) | `prepare_page_surface(state, …)` | 复位变换 → 白底全画布 → 设 zoom×dpr 视口偏移变换 |
| pub(crate) | `apply_page_transform(state, …)` | 仅设页面缩放与视口偏移变换（不清屏） |

### `canvas/draw.rs` — 底层文本绘制核心与独立 wasm 导出
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `TextMetricsSnapshot` (struct) | 文本度量快照（ascent/descent 未被消费） |
| wasm | `render_run_standalone(…)` | 独立 wasm 入口：页面坐标绘制单个文本 run |
| pub(crate) | `draw_text_run_core(…)` | 共享文本绘制：render_mode 3 跳过、像素吸附、逐字形原点或 scale_x、fill/stroke 与下划线 |

### `canvas/renderer.rs` — 原语指令绘制与 PdfRenderer trait 实现
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `draw_text_run(…)` | 转调 draw_text_run_core（EditorLocal 模式） |
| 私有 | `draw_text_command(…)` | 设字体颜色后 fill_text |
| 私有 | `draw_rect_command(…)` | 填充或描边矩形；横条形状（≥120×≤6）记调试事件 |
| 私有 | `draw_line_command(…)` | 两点描线；横线形状记调试事件 |
| pub | `render(commands)`（impl PdfRenderer） | 分派 Text/Rect/Line 指令逐条绘制 |
| pub | `clear()`（impl PdfRenderer） | 劫持模式跳过；白底清屏并恢复 dpr 变换 |
| pub | `name()`（impl PdfRenderer） | 返回后端名 "Canvas2D" |

### `canvas/vector.rs` — 矢量对象绘制：路径/图像/文本分派与抑制
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `draw_vector_object(…)` | 按对象类型分派三个绘制器 |
| 私有 | `draw_path_object(…)` | move/line/close 构路径，先填充后描边；与活动编辑壳相交的横条路径记调试 |
| 私有 | `draw_image_object(…)` | 从图像提供方 Map 取 HtmlImageElement 或 ImageBitmap 按位绘制 |
| 私有 | `draw_text_object(…)` | 逐 run 跳过隐形与被抑制者，解析字体后绘制 |

### `canvas/page.rs` — 整页渲染编排与渐进时间切片
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `render_vector_slice(…)` | 按预算/上限推进渐进任务：对象与覆盖层逐条绘制，返回处理数 |
| pub | `render_page(…)` | 整页渲染：视口 bbox → 准备表面 → 有效矢量计划（或字形回退计划）逐条绘制 |

### `canvas/debug.rs` — 仅命中活动编辑壳的画布调试日志
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `active_shell_bbox_for_debug()` | 取活动编辑器替换区域的文本清除框 |
| pub(crate) | `debug_bbox_intersects_active_shell(bbox)` | 判定包围盒是否触及编辑壳 |
| pub(crate) | `debug_log_canvas_method(…)` | 相交才记录画布绘制调试事件（避免逐绘制开销） |

## 相关文档
- `docs/modules/core-render.md` — 上游 core 渲染计划与瓦片调度（本模块消费的全部纯决策）
- `docs/adr/0003-tile-based-rendering.md` — 瓦片渲染架构（tile_host / renderFacade* 瓦片段的依据）
- `docs/adr/0004-always-vector-rendering.md` — 始终高清矢量渲染（质量状态机、css_scale 语义）
- `docs/adr/0005-tile-manager-scheduler-split.md` — TileManager 拆分为调度器 + 协调器
- `docs/CONTEXT.md` — FrameToken / VisibleSurface / 瓦片管理器 / 渲染质量等术语
- `architecture-refactor-plan.md` — `*_store` / `*_api` 命名公约与文件重命名映射

## 疑点
- 命名与公约不符：`wasm_facade.rs` 按重构计划应改名 `wasm_api/render_api.rs`（"facade" 属废弃命名），`host_runtime.rs` 应改 `platform_bridge.rs`，两者均未执行；`free_api.rs` 同为 WASM 边界却游离于 `*_api.rs` 公约之外（其头注自述原 `wasm_api/render_api.rs` 删除后重建于此），于是渲染域存在两套并行 wasm 入口（`renderFacade*` 与自由函数名），渐进四件套在两边完全重复暴露。
- 潜在 bug：`wasm_facade.rs` 三个质量函数各自在函数体内声明 `QUALITY_SM` thread_local，形成三个互不相通的 static——start 的重置永远不会影响 update/get 的实例；这正是该文件瓦片段注释与 `tile_host.rs` 头注明确警告的反模式。且整套 `renderFacade*Quality*`（含 dpi/budget 与四个 stub）在 TS 侧零调用，属冻结接口中的死表面。
- 重复实现：`free_api.rs` 的 `takeFramePlan` 与 `resolveFramePlan`、`resolveHostScrollRefresh` 与 `resolveViewportRefresh` 逐字节重复（纯别名）；`progressive_workflow.rs` 的 `step_progressive_render(_offscreen)` 与 `render_page(_offscreen)` 仅构造渲染器一行不同，主体近似复制。
- 死代码/失效区分：`canvas_overlay.rs` 的 `draw_editor_marker_page` 自带 `#[allow(dead_code)]`（注释称 retained seam, not yet wired）；`canvas/mod.rs` 的 `CoordinateMode` 两变体在 `draw_text_run_core` 中 y_scale 均为 1.0，PageSpace/EditorLocal 当前无行为差异；`TextMetricsSnapshot` 的 `_ascent/_descent` 无消费者。
- 与 ADR 的出入：ADR-0003 宣布以纯瓦片渲染取代 base+detail，但本模块 `workflow.rs` 落定路径仍在记忆 base/detail 层缓存键，瓦片管线（tile_host + renderFacade*Tile，仅被 tile_bridge.ts 调用）与实际整页绘制 `canvas/page.rs::render_page` 互不衔接——mark_ready 仅翻状态，无瓦片上画布的路径，两套体系并存；ADR-0004 的渲染质量状态机仅在未接线的 `renderFacade*Quality*` 中出现，`facade_set_quality` 仍是 stub。
- 状态访问越层：`render_store` 与 `present/present_store` 职责交叠——帧排程核心在 render_store，而 wasm 入口的缓存/settle 一律转经 present_store；`zoom/raf_loop.rs` 与 `present_store.rs` 直接 `.with(RENDER_STATE)` 触碰内部字段，绕过函数接口，与重构计划"禁止外部直接 .with()"的方向相悖。
