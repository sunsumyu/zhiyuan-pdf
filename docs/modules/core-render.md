# pdf-viewer-core · render（渲染计划与瓦片调度）

> 范围：`crates/pdf-viewer-core/src/render/`（42 个文件，约 8470 行，含内联测试与子目录 `effective_page_plan/`、`zoom/`）。
> 上游：`crate::models`（VectorPageModel / GlyphPaintPlan / StyledRun / BoundingBox / PageState 等）、`crate::edit`（paragraph_overlay / replacement_region / source_identity / active_target / debug_trace）、`crate::document::page_region_context`（区域快照）、`crate::geometry::bbox_ops`、`crate::typography::font_resolver`、`crate::common`（sanitize / debug）。
> 下游：`pdf-viewer-ui` 约 27 个文件——`render/`（wasm_facade、render_store、free_api、commit、tile_host、tile_cache、progressive_workflow、canvas/renderer 等）、`zoom/`（zoom_store、zoom_controller、raf_loop、zoom_authority）、`present/`（plan_builder、present_store）、`editor/orchestrator/`、`document/mutation_pipeline.rs`。core 内部无其他模块反向依赖 render/。

## 职责
把页面内容模型降维为可执行的渲染计划：视口裁剪与瓦片切分、base/detail 双层缓存键与复用判定、渐进渲染策略、缩放动画状态机与渲染时机决策。全模块为纯数据 + 纯计算（无 DOM、无 WASM 边界、无 thread_local），thread_local 状态容器（ZOOM_STATE / TILE_MANAGER_HOST / RENDER_STATE / QUALITY_SM 等）均驻留 UI crate，由其持有本模块定义的值类型。

## 文件与方法
（各文件内联 `#[cfg(test)] mod tests` 不列入；`effective_page_plan/tests.rs` 仅列覆盖点。）

### `mod.rs` — 模块清单与缩放子系统兼容别名
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use zoom as zoom_host` | 旧路径 `zoom_host` 兼容别名 |
| pub | `pub use zoom::animation as zoom_interaction` | 旧路径 `zoom_interaction` 兼容别名 |
| pub | `pub use zoom::state as zoom_state` | 旧路径 `zoom_state` 兼容别名 |

### `scheduler.rs` — 渲染帧调度纯数据：FrameToken 与帧信封（thread_local 与排程逻辑留在 UI crate）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `HostRenderState<TPlan>` (struct) | 帧状态：next/active/committed/in_flight/queued 五个 token 与在飞/排队帧计划 |
| pub | `RenderFrameEnvelope<TPlan>` (struct) | 帧信封：frame_token + frame_plan，投递给 TS 渲染循环 |
| pub | `RenderFrameTransition<TPlan>` (struct) | 帧转移结果：accepted / settled_frame_plan / next_frame |
| pub | `allocate_render_frame_token(state) -> u32` | 单调递增分配下一帧令牌（wrap 后仍 ≥1） |

### `workflow.rs` — 帧计划信封组装与渐进结果 DTO 转换
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `RenderFrameEnvelope` / `RenderFrameTransition` (type) | 以 FramePlanResult 特化的调度类型别名 |
| pub | `ProgressiveRenderStartResult` / `ProgressiveRenderStepResult` (struct) | 渐进渲染启停结果 DTO（usize 收敛为 u32） |
| pub | `build_render_frame_envelope(token, plan)` | 组装帧信封 |
| pub | `frame_plan_requires_render(plan) -> bool` | base 或 detail 层任一需渲染即为真 |
| pub | `frame_plan_needs_viewport_refresh(plan) -> bool` | 视口瓦片且需细节层时需刷新 |
| pub | `frame_plans_share_render_work(a, b) -> bool` | 比较两帧计划的渲染理由与缓存键是否等价 |
| pub | `progressive_start_result(start)` | 渐进启动结果转 DTO |
| pub | `progressive_step_result(step)` | 渐进步进结果转 DTO |

### `frame_cache.rs` — 视口刷新决策与帧缓存键操作的薄封装
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `resolve_viewport_refresh(state, frame_plan, ts)` | 由帧计划提取参数转调 viewport_refresh 决策 |
| pub | `touch_frame_cache_entry(state, is_detail, key)` | 空键短路后置顶命中键（LRU touch） |
| pub | `store_frame_cache_entry(state, is_detail, key)` | 空键短路后存键并返回被淘汰键 |
| pub | `reset_frame_cache(state)` | 清空全部已存帧缓存键 |

### `viewport_refresh.rs` — 细节层视口刷新的提交抑制与延迟决策
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `VIEWPORT_REFRESH_COMMIT_SUPPRESS_MS` 等 (const) | 提交后抑制 120ms、刷新延迟 56ms |
| pub | `HostViewportRefreshState` (struct) | 刷新抑制截止时间戳 |
| pub | `ViewportRefreshDecision` (struct) | 是否刷新 + 延迟毫秒 |
| pub | `note_viewport_render_commit(state, ts)` | 记录提交时刻，开启抑制窗口 |
| pub | `resolve_viewport_refresh_decision(state, …) -> Decision` | 抑制期内/非细节层不刷新，否则按 56ms 延迟刷新 |

### `plan_builder.rs` — 帧计划核心几何：缩放解析、视口布局、瓦片矩形与缓存键 zoom 量化
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PREVIEW_BASE_REFRESH_RATIO` (const) | 基础层预览刷新比例 0.035 |
| pub | `RenderZoomRequest` / `RenderZoomResult` (struct) | 缩放解析入参与结果（display/render/base_zoom、css_scale、use_viewport_tile） |
| pub | `FramePlanRequest` / `FramePlanResult` (struct) | 帧计划请求与全量结果（缓存键、层策略、瓦片矩形、CommittedLayout 字段） |
| pub | `ViewportLayoutResult` / `ViewportTileResult` (struct) | 宿主/内容居中偏移与瓦片矩形 |
| pub | `clamp_f32(value, min, max)` | NaN/inf 安全的区间截断 |
| pub | `centered_offset(content, viewport)` | 内容小于视口时的居中偏移 |
| pub | `cache_zoom_ratio_delta(a, b)` | 两 zoom 的相对偏差比 |
| pub | `should_prepare_layout(reason) -> bool` | documentMutation 之外的渲染原因才重排布局 |
| pub | `is_stable_document_frame(reason) -> bool` | 判定 editorVisibility/documentMutation 稳定帧 |
| pub | `compute_viewport_layout_result(…)` | 由 display/viewport 尺寸算宿主盒与内容居中偏移 |
| pub | `compute_viewport_tile_result(…)` | 由滚动位置+overscan 算可见瓦片矩形 |
| pub | `resolve_tile_overscan(w, h, zoom)` | 按 zoom 分档的自适应瓦片外扩（220–960px） |
| pub | `compute_visible_content_rect(…)` | 计算可见内容矩形四元组 |
| pub | `resolve_render_zoom_result(request)` | 按 10240px 画布上限解析 render_zoom 与 css_scale，判定是否启用视口瓦片 |

### `present_plan.rs` — 呈现策略：预览是否落定、层复用与渲染开关
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PresentPolicy` (struct) | 十项布尔策略：落定/渲染/复用/细节覆盖层开关 |
| pub | `resolve_present_policy(target, visual, …)` | 由 target/visual zoom 差与可复用层推导全套呈现策略 |
| pub | `preview_is_settled(target, visual) -> bool` | 0.001 容差内视为预览落定 |
| pub | `preview_base_layer_reuse_ratio() -> f32` | 基础层复用比例常量 0.28 |
| pub | `preview_detail_layer_reuse_ratio() -> f32` | 细节层复用比例常量 0.18 |
| pub | `quantize_cache_zoom(zoom, use_tile)` | 按 zoom 分档步长量化缓存 zoom |

### `preview.rs` — 预览呈现平移量计算
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PreviewPresentPlan` (struct) | translate_x/y + css_scale 的预览呈现计划 |
| pub | `resolve_preview_present_plan(current…, next…, css_scale)` | 由前后可见偏移差算预览平移量 |

### `layer.rs` — base/detail 层执行计划：是否渲染、覆盖层去留
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `allow_detail_overlay_retention(frame_plan)` | 视口瓦片且非编辑可见性/文档变更帧才允许保留覆盖层 |
| pub | `LayerExecutionPlan` / `LayerPresentDecision` (struct) | 层执行计划与覆盖层显示/保留决策 |
| pub | `RenderLayerRuntimePlan` / `RenderExecutionPlan` (struct) | 单层运行时计划（缓存键/渲染 zoom）与总执行计划 |
| pub | `resolve_layer_execution_plan(bundle_changed, frame_plan)` | 合并 bundle 变更与帧计划得层渲染开关 |
| pub | `resolve_layer_present_decision(use_detail, frame_plan)` | 决定细节覆盖层显示或保留 |
| pub | `resolve_render_execution_plan(bundle_changed, frame_plan)` | 组装 base/detail 两层完整执行计划 |

### `progressive.rs` — 渐进式渲染预算与任务推进策略
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `BASE/DETAIL_PROGRESSIVE_*` (const) | base 1ms/6 项、detail 2.2ms/10 项预算与一次性阈值 |
| pub | `ProgressiveRenderStart` / `ProgressiveRenderStep` (struct) | 渐进启停与步进进度 |
| pub | `ProgressiveRenderPolicy` (struct) | 是否渐进 + 毫秒预算 + 每帧上限 |
| pub | `ProgressiveVectorRenderTask` (struct) | 渐进任务：条目序列 + 游标 + 视口包围盒 |
| pub | `ProgressiveVectorRenderTask::build(…)` | 由矢量模型+覆盖层构建渐进任务，空则 None |
| pub | `is_complete() / total_items()` | 游标是否走完 / 条目总数 |
| pub | `resolve_progressive_render_policy(use_tile, prefer, total)` | 超过一次性阈值才启用渐进并取对应预算 |
| pub | `ProgressiveRenderPolicyRequest` (struct) | 策略请求 DTO |
| pub | `resolve_progressive_render_policy_request(req)` | DTO 转调核心策略函数 |

### `prepared_scene.rs` — 页面场景空间索引：96pt 网格桶 + 视口快速裁剪
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PreparedPageScene` (struct) | z 序对象索引、bbox 表、桶索引、段落活跃对象 id 表 |
| pub | `build(vector_model, paint_plan) -> Option<Self>` | 全空场景返回 None，否则建索引 |
| pub | `visible_vector_indices(viewport) -> Vec<usize>` | 桶命中候选内再按 bbox 相交过滤，保持 z 序 |
| pub | `active_text_object_ids(paragraph_id)` | 查段落关联的活跃文本对象 id 集合 |
| 私有 | `vector_object_bbox(object)` | text 合并 run 框 / path 顶点框 / image 位置框 |
| 私有 | `resolve_bucket_keys(bbox)` | bbox 覆盖的 96pt 桶坐标列表 |

### `viewport_culling.rs` — 视口相交判定与 run/path 包围盒计算
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `resolve_page_viewport_bbox(state, w, h)` | display 视口按 zoom 换算为页面坐标 bbox（防非法值） |
| pub | `glyph_run_intersects_viewport(run, viewport)` | 字形 run 框与视口相交 |
| pub | `paragraph_intersects_viewport(paragraph, viewport)` | 段落框与视口相交 |
| pub | `region_intersects_viewport(region, viewport)` | 区域框与视口相交 |
| pub | `vector_object_intersects_viewport(obj, viewport)` | 按 text/path/image 分派相交判定 |
| pub | `styled_run_bbox(run)` | 由原点+字号+宽度构造 run 包围盒 |
| pub | `path_object_bbox(path) -> Option` | 遍历顶点求包围盒，退化时 None |
| 私有 | `text/path/image_object_intersects_viewport` | 三类对象各自的相交判定 |

### `source_suppression.rs` — 源文本抑制：被段落覆盖层替换的原始文本不再绘制
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `SuppressedVectorTextRuns` (struct) | 被抑制的对象 id 集 + run 下标集 |
| pub | `is_empty() / extend(other)` | 空判定与合并 |
| pub | `suppressed_count_for_text_object(text)` | 统计该对象被抑制的 run 数 |
| pub | `suppresses_run(index, run)` | 按下标或对象 id 判定 run 是否被抑制 |
| pub | `run_text_is_list_marker_only(text)` | 判定纯列表标记符号 run |
| pub | `text_run_spatially_matches_replacement_region(run, region)` | 空间重叠阈值判定（标记 run 永不匹配） |
| pub | `glyph_run_spatially_matches_replacement_region(run, region)` | 字形 run 版空间匹配 |
| pub | `text_object_matches_overlay_source_text(obj, src, region)` | 归一化文本包含 + 空间双重匹配（已标 dead_code） |
| pub | `glyph_paragraph_matches_overlay_source_text(paragraph, overlay)` | 字形段落文本归一化匹配覆盖层源文本 |
| pub | `matching_text_run_refs(obj, active_ids, region)` | 汇总命中覆盖层的 run 引用集合 |
| pub | `text_object_should_be_suppressed(obj, active_ids)` | 对象级活跃 id 命中即整对象抑制 |
| 私有 | `bbox_overlap_width/height`、`normalize_source_match_text` | 重叠宽高计算与去空白文本归一化 |

### `path_suppression.rs` — 装饰性细路径/图像抑制：正文替换时擦除行内横线装饰
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `should_suppress(obj, index, markers, region, bbox)` | 非 marker 占用且符合细横条特征时返回抑制摘要串 |
| 私有 | `image_object_bbox(image)` | 图像位置包围盒 |
| 私有 | `allowed_path_height(object)` | 按填充/描边宽度放宽的允许高度 |
| 私有 | `source_row_decoration_matches(…)` | 行带重叠、尺寸、壳层重叠多条件判定 |
| 私有 | `source_row_decoration_summary(…)` | 生成调试用几何摘要串 |
| 私有 | `bbox_overlap_width/height`、`row_overlap_height` | 重叠度量计算 |

### `paint_plan.rs` — 布局推断结果到字形绘制计划的转换（当前无外部消费者）
| 可见性 | 方法 | 功能 |
|---|---|---|
| 私有 | `paint_mode_from_render_mode(mode)` | render_mode 1/2/其余 → Stroke/FillStroke/Fill |
| 私有 | `resolve_run_font(run)` | 由 run 样式合成 FontHints 解析字体 |
| 私有 | `build_paint_run(page, region, paragraph, run)` | LayoutRun 转 GlyphPaintRun |
| 私有 | `build_editor_session(paragraph)` | 规范化字体名并联合 run 框成编辑会话 |
| 私有 | `is_decorative_text(text)` | 判定纯装饰符号文本 |
| 私有 | `build_control_style(paragraph)` | 选非装饰 run 生成编辑器控件样式 |
| pub | `build_field_editor_params(request)` | 由 run 快照构造字段编辑器参数 |
| pub | `build_glyph_paint_plan(layout)` | 布局结果整体转字形绘制计划（段落/区域/会话/样式） |

### `snapshot_paint_plan.rs` — 区域快照回绘：把段落/字段组快照合成为字形绘制 run（当前无外部消费者）
| 可见性 | 方法 | 功能 |
|---|---|---|
| 私有 | `to_paint_mode(render_mode)` | render_mode → PaintMode |
| pub | `build_resolved_font_face(name, bold, italic, hints)` | 缺省时合成 hints 再解析字体 |
| 私有 | `build_run_bbox(…)` | 由基线、字号、字符原点构造 run 框 |
| 私有 | `build_snapshot_paint_run(…)` | StyleRunSnapshot 转 GlyphPaintRun |
| pub | `resolve_run_layout(line_left, cursor, run, measure)` | 字符原点换算绝对坐标并回退测量宽度 |
| pub | `build_paragraph_snapshot_paint_runs(snapshot, page, measure)` | 标记 run + 正文 run + 合成 run 全量回绘 |
| pub | `build_field_group_snapshot_paint_runs(snapshot, baseline, page)` | 字段组键/值两列回绘 |

### `tile_cache.rs` — 兼容转发 shim（新代码请用 tile_v2 / tile_manager）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use super::tile_cache_legacy::*` | 全量转发旧缓存模块，维持消费方路径不变 |

### `tile_cache_legacy.rs` — base/detail 双层缓存键、复用匹配与帧缓存键 LRU（虽名 legacy 仍为主力）
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `DETAIL_TILE_REUSE_MARGIN` 等 (const) | 覆盖边距 96px、最近层/键容量与预览/落定复用比例 |
| pub | `BaseLayerCacheEntry` / `DetailTileCacheEntry` (struct) | 基础层与细节瓦片缓存条目（键 + 缓存 zoom + 场景键 [+ 瓦片矩形]） |
| pub | `HostPresentState` (struct) | 活跃层 + 最近基础层(4)/最近细节瓦片(8) 呈现状态 |
| pub | `HostFrameCacheState` (struct) | 已存 base(4)/detail(12) 帧缓存键列表 |
| pub | `FrameCacheStoreResult` (struct) | 存键结果：被淘汰键列表 |
| pub | `build_base_cache_key(path, page, scene, zoom, dpr)` | 拼基础层缓存键 |
| pub | `build_detail_cache_key(…tile_rect)` | 拼细节瓦片缓存键（含瓦片矩形） |
| pub | `find_reusable_base_layer(state, scene, zoom, settled)` | 精确命中优先，否则在允许比例内取最近 zoom 匹配层 |
| pub | `reusable_base_layer_is_displayed(reusable, active)` | 仅当复用命中就是屏上活跃层才视为已显示 |
| pub | `find_reusable_detail_tile(state, …, settled)` | 同上，另要求瓦片矩形含边距地覆盖视口 |
| pub | `remember_base_layer(state, layer)` | 设为活跃层并压入最近列表 |
| pub | `remember_detail_tile(state, tile)` | 设为活跃瓦片并压入最近列表 |
| pub | `clear_detail_tiles(state)` | 清空活跃与最近细节瓦片 |
| pub | `touch_frame_cache_key(state, is_detail, key)` | 命中键置顶 |
| pub | `store_frame_cache_key(state, is_detail, key)` | 去重置顶并按容量淘汰，返回淘汰键 |
| pub | `clear_frame_cache_keys(state)` | 清空全部帧缓存键并返回 |
| 私有 | `detail_tile_covers_viewport(_geometry)`、`cache_zoom_matches`、`cache_zoom_ratio_delta` | 覆盖判定与 zoom 匹配/偏差 |
| 私有 | `best_matching_base_layer` / `best_matching_detail_tile` | 在允许比例内取 zoom 偏差最小的候选 |
| 私有 | `push_recent_base_layer` / `push_recent_detail_tile` | 去重置顶并截断容量 |

### `tile_v2.rs` — 瓦片渲染系统 v2：瓦片键、瓦片状态与 LRU 瓦片缓存（ADR-0003）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `TILE_SIZE` (const) | 瓦片逻辑尺寸 512×512 |
| 私有 | `MAX_TILE_CACHE_SIZE` (const) | 缓存上限 64 |
| pub | `TileKey` (struct) | 瓦片键 `{page}|{zoom}|{dpr}|{x}|{y}`；手写 Hash/Eq（f32 转 bits） |
| pub | `TileKey::new(page, zoom, dpr, x, y)` | 构造瓦片键 |
| pub | `TileKey::to_string_key() -> String` | 序列化为字符串键 |
| pub | `TileState` (enum) | Pending / Rendering / Ready / Failed |
| pub | `Tile` (struct) | 瓦片元数据：键、状态、逻辑/像素矩形、last_used |
| pub | `Tile::new(key, logical_rect, dpr)` | 初始 Pending，按 dpr 换算像素矩形 |
| pub | `mark_rendering / mark_ready / mark_failed` | 状态流转 |
| pub | `touch(timestamp)` | 更新最后使用时间戳 |
| pub | `TileRect` (struct) | 逻辑或像素坐标矩形 |
| pub | `TileCache` (struct) | HashMap + access_order 的 LRU 缓存 |
| pub | `new() / with_capacity(max_size)` | 默认容量 / 自定义容量构造 |
| pub | `insert(tile)` | 更新或插入，满时 LRU 淘汰 |
| pub | `get / get_mut(key)` | 取瓦片并 touch LRU |
| pub | `peek(key) / tile_state(key)` | 只读查询，不触碰 LRU（热路径） |
| pub | `contains(key)` | 存在性判定 |
| pub | `remove(key)` | 移除瓦片 |
| pub | `clear_page(page)` | 清空指定页全部瓦片 |
| pub | `mark_all_eligible_for_eviction()` | 缩放变化时全部时间戳归零便于淘汰 |
| pub | `get_pending_tiles(page)` | 该页全部 Pending 瓦片（当前无外部调用者） |
| pub | `get_viewport_tiles(page, zoom, dpr, x, y, w, h)` | 同页同 zoom/dpr 且矩形与视口相交的瓦片 |
| pub | `calculate_tile_grid(w, h, zoom, dpr)` | 按页尺寸算瓦片网格键（硬编码 page=0，无外部调用者） |
| pub | `stats() -> CacheStats` | 各状态计数与容量统计 |
| 私有 | `touch_tile / evict_lru` | LRU 置顶与最旧淘汰 |
| 私有 | `Tile::rect_intersects_viewport(…)` | 瓦片矩形与视口相交判定 |
| pub | `CacheStats` (struct) | total/pending/rendering/ready/failed/max_size |

### `tile_scheduler.rs` — 瓦片调度器：视口/近邻优先级与动画增量调度（ADR-0005 拆分的纯调度侧）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `TilePriority` (enum) | Viewport=0 / NearViewport=1 / FarViewport=2 |
| pub | `TileRenderRequest` (struct) | 瓦片键 + 优先级 + frame_token |
| pub | `ViewportState` (struct) | 位置尺寸 + 页/zoom/dpr |
| pub | `AnimationState` (struct) | 动画中标志、visual/target zoom、渲染间隔与帧计数 |
| pub | `TileScheduler` (struct) | 持有视口、动画与当前帧令牌 |
| pub | `new()` | 初始 zoom 1.0、间隔 3 帧 |
| pub | `update_viewport(page, zoom, dpr, …, token)` | 更新视口状态与当前帧令牌 |
| pub | `start_animation(target_zoom)` | 记录动画起点（visual 取当前视口 zoom） |
| pub | `update_animation(visual_zoom, token)` | 每帧推进动画计数与令牌 |
| pub | `end_animation(token)` | 结束动画 |
| pub | `current_frame_token / is_animating` | 令牌与动画中查询 |
| pub | `animation_state / viewport_state` | 状态只读访问 |
| pub | `schedule_viewport_tiles()` | 视口瓦片 Viewport 级 + 外扩一圈 NearViewport 级 |
| pub | `schedule_incremental_tiles()` | 动画期间仅按当前 visual zoom 调度视口瓦片 |
| pub | `should_render_incremental()` | 动画中且帧计数到间隔才触发增量 |

### `tile_manager.rs` — 瓦片管理器：缓存 + 调度 + 渲染队列协调与 FrameToken 并发控制（ADR-0005 协调侧）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `TileManager` (struct) | 组合 TileCache（pub 字段）、TileScheduler 与渲染队列 |
| pub | `new()` | 空缓存 + 空调度器构造 |
| pub | `update_viewport(…)` | 更新调度器并调度视口瓦片 |
| pub | `start_animation(target_zoom)` | 启动动画并标记全部瓦片可淘汰 |
| pub | `update_animation(visual_zoom, token)` | 到间隔时调度增量瓦片 |
| pub | `end_animation(token)` | 结束动画并调度最终高清瓦片 |
| pub | `next_render_request() -> Option<Request>` | 按优先级取队首，跳过过期令牌与 Ready/Rendering 重复 |
| pub | `mark_rendering / mark_ready / mark_failed(key)` | 推进对应瓦片状态 |
| pub | `reset_stale_rendering(key) -> bool` | 渲染作废时把 Rendering 翻回 Pending 以便重排 |
| pub | `is_tile_ready(key)` | 只读判定瓦片就绪 |
| pub | `get_ready_viewport_tiles()` | 当前视口内全部 Ready 瓦片 |
| pub | `clear_page(page)` | 清页缓存 |
| pub | `stats() -> TileManagerStats` | 缓存统计 + 队列长度 + 令牌 + 动画标志 |
| 私有 | `schedule_viewport_tiles / schedule_incremental_tiles` | 转调调度器并逐个入队 |
| 私有 | `schedule_tile(key, priority)` | Pending/Failed/缺席瓦片才入队（防饿死与重复） |
| pub | `TileManagerStats` (struct) | 统计 DTO |

### `quality.rs` — 渐进质量系统：Low/Medium/High 质量级、质量状态机与质量感知瓦片键（ADR-0004）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `RenderQuality` (enum) | Low=0 / Medium=1(默认) / High=2，可序比较 |
| pub | `dpi_multiplier()` | 0.75 / 1.0 / 1.5 |
| pub | `text_quality()` | 0.5 / 0.8 / 1.0 |
| pub | `detail_level()` | 0.3 / 0.7 / 1.0 |
| pub | `max_items_per_frame()` | 100 / 50 / 30 |
| pub | `budget_ms()` | 2.0 / 4.0 / 8.0 |
| pub | `QualityStateMachine` (struct) | 当前/目标质量 + 帧计数 + 升级阈值(5 帧) |
| pub | `new()` | 初始 Low、目标 High |
| pub | `start_animation()` | 重置为 Low 开始动画 |
| pub | `update(is_animating, settled)` | 落定跳 High；动画满阈值升 Medium |
| pub | `current() / set_target(t) / reset() / upgrade()` | 查询、改目标、重置、手动逐级升级 |
| pub | `QualityRenderRequest` (struct) | 带质量的渲染请求（页/zoom/dpr/质量/令牌/视口） |
| pub | `effective_dpi / budget_ms / max_items` | dpr×质量倍率等派生值 |
| pub | `ViewportBounds` (struct) | 瓦片选取用视口矩形 |
| pub | `QualityTileKey` (struct) | 扩展质量的质量感知瓦片键（当前无外部消费者） |
| pub | `cache_key()` | 序列化 `{page}|{zoom}|{x}|{y}|{quality:?}` |
| pub | `can_reuse_for(target)` | 仅低质量可作高质量 fallback（同级不算复用） |

### `renderer.rs` — 画布渲染后端抽象 trait 与绘图指令 DTO
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `DrawCommand` (enum) | Text / Rect / Line 三种绘图指令（serde tag="type"） |
| pub | `PdfRenderer` (trait) | `render(commands)` 全量/增量绘制、`clear()`、`name()` |

### `viewer_session.rs` — 宿主查看器会话快照 DTO
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `HostViewerSession` (struct) | path/current_page/page_count/current_zoom/页面尺寸/document_revision |
| pub | `Default for HostViewerSession` | A4 默认页面、zoom 1.0 |

### `facade_types.rs` — 视口请求参数 DTO
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ViewportLayoutRequest` (struct) | display/viewport 尺寸四元组 |
| pub | `ViewportTileRequest` (struct) | 上述 + 滚动、内容偏移与 overscan |

### `find_state.rs` — 查找会话快照 DTO
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `HostFindScope` (enum) | Page / Document 查找范围 |
| pub | `HostFindSession` (struct) | 关键词、范围、活动命中、总数与命中页列表 |
| pub | `HostFindNavigationResult` (struct) | 导航结果：有命中、活动下标/页、是否回绕 |

### `comment_review_state.rs` — 评论审阅会话快照 DTO
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `HostCommentReviewScope` (enum) | Page / Document 审阅范围 |
| pub | `HostCommentReviewSession` (struct) | 面板开关、范围、查询串、选中评论 id |

## effective_page_plan/ — 有效渲染计划（覆盖层抑制 + 视口裁剪 + 调试追踪）

### `effective_page_plan/mod.rs` — 子模块清单与计划条目类型
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EffectiveVectorRenderEntry` (enum) | Object{下标+被抑制 runs} 或 ParagraphOverlay |
| pub | `GlyphParagraphRef` (struct) | 字形段落引用（区域/段落下标 + 被抑制 run 集合） |
| pub | `EffectiveGlyphRenderEntry` (enum) | Paragraph(GlyphParagraphRef) 或 ParagraphOverlay |
| 私有 | `PreparedOverlay` (struct) | 预置覆盖层：替换区域、抑制集合、行抑制框、插入标志与全部调试计数 |
| pub | re-export `SuppressedVectorTextRuns` | 自 source_suppression 转出 |
| pub | re-export `build_effective_vector_render_plan` | 自 vector_plan 转出 |
| pub | re-export `build_effective_glyph_render_plan` | 自 glyph_plan 转出 |

### `effective_page_plan/overlay_predicates.rs` — 覆盖层判定与预置（全部 pub(super)）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(super) | `overlay_paragraph_object_ids(overlay)` | 收集覆盖层目标的源对象 id 集 |
| pub(super) | `overlay_paragraph_object_indices(overlay)` | 目标源下标 + 显式 source 下标并集 |
| pub(super) | `overlay_renders_last(overlay)` | 编辑壳/持久画布所有者的覆盖层最后绘制 |
| pub(super) | `overlay_suppresses_text_source(overlay)` | replaces_source 时抑制源文本 |
| pub(super) | `overlay_suppresses_row_paths(overlay)` | 编辑壳/持久画布所有者抑制行路径 |
| pub(super) | `overlay_intersects_viewport(overlay, bbox, page_w)` | 按视口剔除框判定与视口相交 |
| pub(super) | `prepare_overlays(overlays, bbox, page_w)` | 视口过滤 + 组装 PreparedOverlay 列表 |
| pub(super) | `insert_overlay_if_needed(o, entries)` | 非最后绘制者按序插入覆盖层条目 |

### `effective_page_plan/object_summary.rs` — 对象 bbox 与调试摘要（全部 pub(super)）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(super) | `vector_object_bbox(object)` | 三类对象合并包围盒 |
| pub(super) | `vector_object_summary(object, index)` | 生成 idx/type/id/颜色/bbox 摘要串 |
| pub(super) | `record_overlay_object_summary(overlay, s)` | 摘要滚动记入 overlay 的 3 个槽位 |

### `effective_page_plan/trace.rs` — 覆盖层调试事件追踪（全部 pub(super)）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(super) | `trace_overlay_identity(po, vi, vm)` | 记录每个覆盖层身份与可见文本对象事件 |
| pub(super) | `trace_overlay_summary(o)` | 记录 overlay-min / overlay-compact / overlay-path-summary 三级摘要事件 |

### `effective_page_plan/vector_plan.rs` — 矢量有效渲染计划构建主流程
| 可见性 | 方法 | 功能 |
|---|---|---|
| 私有 | `resolve_visible_indices(model, scene, bbox)` | 有空间索引用索引，否则线性过滤相交 |
| 私有 | `build_entries_without_overlays(vi, vm)` | 无覆盖层时剔除隐形文本（render_mode==3）直接产出 |
| 私有 | `TextSuppressionOutcome` (enum) | RunLevel / NonMarkerRuns / NoMatch 三态 |
| 私有 | `decide_text_suppression(obj, index, overlay)` | id/z_index/数组下标命中分级决定抑制方式 |
| 私有 | `apply_text_suppression(outcome, obj, overlay, runs)` | 落实 run 级或对象级抑制并累计计数 |
| 私有 | `check_path_suppression(obj, index, overlay)` | 记录相交计数并按细横条规则抑制路径 |
| 私有 | `process_visible_objects(vi, vm, overlays)` | 逐对象逐覆盖层应用抑制，产出条目序列 |
| pub | `build_effective_vector_render_plan(model, scene, bbox, overlays)` | 主入口：可见过滤→抑制→覆盖层按序插入→追踪 |

### `effective_page_plan/glyph_plan.rs` — 字形回退渲染计划构建
| 可见性 | 方法 | 功能 |
|---|---|---|
| 私有 | `process_glyph_paragraph(region, paragraph, …)` | 段落级四路匹配收集被抑制 run，安排覆盖层与段落引用顺序 |
| pub | `build_effective_glyph_render_plan(plan, bbox, overlays)` | 区域/段落两级视口过滤后逐段处理 |

### `effective_page_plan/tests.rs` — 覆盖点（不逐方法列出）
覆盖：零高路径抑制、分节线/近邻线保留、下伸部路径抑制、无 id 文本抑制、空间文本/字形抑制、匹配文本保留、纯路径抑制、覆盖层抑制字形/路径、覆盖层最后绘制、右瓦片保留、列表标记保留、z_index 顺序处理。

## zoom/ — 缩放子系统（合并后的缩放宿主，兼容别名 zoom_host / zoom_interaction / zoom_state）

### `zoom.rs` (mod) — 缩放子模块清单与全量 re-export
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use animation/state/zoom_decide/zoom_layout/zoom_render/zoom_tick::*` | 便于外部以 `zoom::HostZoomState` 等路径访问 |

### `zoom/state.rs` — 缩放状态数据结构（值类型，thread_local 容器在 UI crate zoom_store.rs）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `VisualLayoutState` (struct) | display_zoom + content_left/top 的视觉布局快照 |
| pub | `PendingCommittedFrame` (struct) | 待提交帧的几何契约字段 |
| pub | `PreviewHostState` (struct) | 预览激活、wheel 渲染挂起、待提交帧、取消挂起渲染标志 |
| pub | `DrawingDelayState` (struct) | settle 后延迟绘制计时器状态 |
| pub | `HostZoomState` (struct) | 缩放权威数据结构：target/visual/last_rendered zoom + 布局 + 预览 + 延迟 |
| pub | `Default for HostZoomState` | 三 zoom 均为 1.0 |
| pub | `ZoomAnimationStep` (struct) | 单帧动画步：visual_zoom + settled |

### `zoom/animation.rs` — 缩放动画插值、wheel 请求解析与预览帧构建
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ZOOM_SETTLED_THRESHOLD` (const) | 落定容差 0.0008（与 UI 提交路径共用） |
| pub | `WheelZoomRequest` / `WheelZoomResult` (struct) | wheel 请求与结果（target_zoom + 锚点内容偏移 + transform origin） |
| pub | `ZoomLimitsRequest` / `ZoomLimitsResult` (struct) | 安全最大 zoom 上限解析 |
| pub | `ZoomPreviewFrame` (struct) | 预览帧：settled、visual_zoom、基准 zoom、呈现平移与帧计划 |
| pub | `clamp_zoom(value, min, max)` | NaN 回退 1.0 的缩放截断 |
| pub | `clamp_f32 / clamp_unit / centered_offset` | 区间截断与居中偏移（与 plan_builder 重复实现） |
| pub | re-export `sanitize_positive / sanitize_non_negative` | 转出 common 清洗函数 |
| pub | `resolve_wheel_zoom_request(req, layout)` | 解析 wheel 增量为目标 zoom，保持光标下页面点不动 |
| pub | `resolve_zoom_limits_result(req)` | 由画布上限/页面/dpr 求安全 max zoom |
| pub | `advance_zoom_animation_state(state, ts)` | 指数逼近推进 visual_zoom，落定吸附并输出步 |
| pub | `commit_rendered_zoom(state, zoom)` | 提交已渲染 zoom：更新 last_rendered 并复位动画时钟 |
| pub | `build_zoom_preview_frame(req, state, build)` | 推进动画→按 visual zoom 构建帧计划→算预览平移 |
| 私有 | `AnchorRequest` / `compute_anchor_content_offset(…)` | 锚点不变量：新内容偏移使光标下坐标不动（无布局则居中） |

### `zoom/zoom_decide.rs` — wheel 渲染时机与提交/冲刷陈旧帧守卫（纯决策）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `WheelRenderDecisionRequest/Decision` (struct) | wheel 渲染决策入参/出参（立即/延迟落定/跳过 + 空闲延时） |
| pub | `PreviewTickDecisionRequest/Decision` (struct) | 预览 tick 决策：续预览、冲刷提交帧、立即渲染 |
| pub | `RenderFollowUpDecision` (struct) | 渲染后是否补渲染最新目标及目标值 |
| 私有 | `needs_render / wheel_render_idle_ms / preview_is_active` | 差值判定、空闲延时(16/48/64ms)、预览激活判定 |
| pub | `resolve_preview_render_zoom(visual, …)` | 恒返回 visual_zoom：位图精确跟随所见（无拉伸模糊） |
| pub | `resolve_wheel_render_decision(req)` | 综合落定/预览/允许渲染得 wheel 渲染决策 |
| pub | `resolve_preview_tick_decision(req)` | 落定则冲刷+补渲染挂起帧，否则续预览 |
| pub | `resolve_render_follow_up_decision(…)` | 预览期以 visual、落定后以 target 判定是否补渲染 |
| 私有 | `COMMIT_STALE_RATIO` / `FLUSH_STALE_RATIO` (const) | 提交陈旧阈值 0.10、冲刷陈旧阈值 0.15 |
| pub | `ZoomCommitDecisionRequest/Decision` (struct) | 提交帧决策：apply_now / skip_stale / preview_settled |
| pub | `resolve_zoom_commit_decision(req)` | 未落定排队，落定后按 10% 比例判陈旧 |
| pub | `ZoomFlushDecisionRequest/Decision` (struct) | 冲刷帧决策：apply / skip_stale |
| pub | `resolve_flush_decision(req)` | 按 15% 比例判冲刷帧陈旧 |

### `zoom/zoom_layout.rs` — 布局回退、适宽缩放、画布 CSS 盒与渲染原因分类
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `MIN_ZOOM` / `MAX_ZOOM` (const) | 0.1 / 30.0 缩放界限 |
| pub | `LayoutFallback` / `LayoutFallbackRequest` (struct) | syncHostLayout 缺数据时的完整回退布局（css_scale 恒 1，ADR-0006） |
| pub | `resolve_layout_fallback(request)` | 由页面尺寸×display_zoom 推全部尺寸 |
| pub | `FitToWidthResult` (struct) | fit_zoom + should_fit |
| pub | `resolve_fit_to_width(vp_w, page_w)` | 页宽超出视口才计算适宽 zoom 并截断 |
| pub | `CanvasCssBoxRequest` / `CanvasCssBox` (struct) | f64 入参保证与 TS 浮点运算逐位一致 |
| pub | `resolve_canvas_css_box(request)` | 矢量画布元素盒：display 盒按 base_render_zoom 重缩放 |
| pub | `is_immediate_mutation_frame(reason)` | editorVisibility/documentMutation 为立即变更帧 |

### `zoom/zoom_render.rs` — 渲染时机引擎：模糊阈值、预测目标与动画中重渲染敲门（ADR-0002 reknock）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ShouldRender` (enum) | Yes / Soon / Skip 三档渲染决策 |
| 私有 | `BLUR_HIGH/LOW_THRESHOLD` (const) | 模糊 >0.10 立即渲染、>0.03 下帧渲染 |
| pub | `PREVIEW_REKNOCK_BLUR_THRESHOLD` (const) | 动画中敲门模糊阈值 0.02 |
| pub | `PREVIEW_REKNOCK_INTERVAL_MS` (const) | 敲门最小间隔 60ms |
| pub | `PreviewReknockRequest` (struct) | 模糊量、已过时长、是否有渲染在飞 |
| pub | `should_reknock_preview_render(req)` | 三条件齐备才在动画中敲门 TS 渲染循环 |
| pub | `predict_render_target(visual, target, velocity, ms)` | 按速度预测渲染完成时 zoom，夹在 visual 与 target 之间 |
| pub | `should_render(visual, rendered, target, settled, v, ms)` | 落定即渲染；否则按模糊档位选 Yes/Soon/Skip |

### `zoom/zoom_tick.rs` — 缩放状态机每帧编排：tick 核心（ADR-0002 RAF 单驱动）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `DomOp` (enum) | TS 同步执行的 DOM 操作：UpdateLayout / SetScroll / SetWrapperSize |
| pub | `AsyncOp` (enum) | TS 调度的异步操作：RequestRender / ScheduleNextFrame / StopRafLoop / 起/停绘制延迟 |
| pub | `ZoomTickInput` / `ZoomTickOutput` (struct) | 每帧输入（时间戳/滚动/视口）与输出（zoom、settled、操作序列） |
| 私有 | `DRAWING_DELAY_MS` (const) | tick 侧绘制延迟 30ms（UI 侧另有 50ms 版本） |
| pub | `tick_zoom_state_core(state, input)` | 推进动画→决定渲染时机（含绘制延迟）→决定 RAF 续停 |

### `zoom/decision.rs` — 决策子模块兼容转发 hub
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use super::{zoom_decide, zoom_layout, zoom_render, zoom_tick}::*` | 维持 `zoom::decision::*` 旧导入路径 |

## 相关文档
- `docs/adr/0003-tile-based-rendering.md` — 瓦片渲染架构（tile_v2 / tile_scheduler / tile_manager 的依据）
- `docs/adr/0004-always-vector-rendering.md` — 始终高清矢量渲染与 css_scale 语义（quality.rs、plan_builder 的依据）
- `docs/adr/0005-tile-manager-scheduler-split.md` — TileManager 拆分为调度器 + 协调器
- `docs/adr/0002-zoom-presentation-single-writer.md`、`docs/adr/0006-css-transform-zoom-removal.md`、`docs/adr/0007-resolve-anchor-semantics.md` — 缩放权威、SurfaceOp 与锚点语义
- `docs/CONTEXT.md` — Zoom Authority / Settle Envelope / FrameToken / 瓦片缓存 / 渲染质量等术语
- `docs/architecture-overview.md`、`docs/page-presentation-runtime-architecture.md` — 管线全貌与呈现运行时
- `docs/modules/core-document-persistence.md` — 同格式兄弟模块文档

## 疑点
- 命名与公约不符：`scheduler.rs` 实为帧令牌/信封纯数据（按公约更像 `*_types`），真正的调度器叫 `tile_scheduler.rs`；`workflow.rs` 无流程编排，只是 DTO 组装/转换；`facade_types.rs` 并非 WASM 边界（真边界在 UI crate 的 wasm_facade.rs），只是视口请求 DTO；`renderer.rs` 的 `PdfRenderer` 名字暗示整页渲染器，实际仅支持 text/rect/line 三种指令。
- 疑似死代码：`paint_plan.rs` 与 `snapshot_paint_plan.rs` 全部 pub 函数在 workspace 内（core 之外）零引用；`quality.rs` 的 `QualityTileKey` / `QualityRenderRequest` / `ViewportBounds`、`tile_v2.rs` 的 `get_pending_tiles`、`tile_scheduler.rs` 的 `FarViewport` 优先级（调度器从不产出该级）无消费者；`source_suppression.rs` 的 `text_object_matches_overlay_source_text` 已自带 `#[allow(dead_code)]` 标记。
- 潜在 bug：`tile_v2.rs` `calculate_tile_grid` 生成的 `TileKey` 硬编码 `page=0`（函数签名无 page 参数），若启用会与实际页号不符。
- 重复实现：`clamp_f32` / `centered_offset` 在 `plan_builder.rs` 与 `zoom/animation.rs` 各一份；`cache_zoom_ratio_delta` 在 `plan_builder.rs` 与 `tile_cache_legacy.rs` 各一份；`vector_object_bbox` / image bbox 在 `prepared_scene.rs`、`viewport_culling.rs`、`path_suppression.rs`、`effective_page_plan/object_summary.rs` 四处同型实现。
- 常量漂移风险：settle 绘制延迟在 `zoom/zoom_tick.rs`（`DRAWING_DELAY_MS=30`）与 UI crate raf_loop（注释所指 `SETTLE_DRAWING_DELAY_MS=50`，`zoom_render.rs` 头注已声明归 UI 侧）两处并存，语义相同数值不同。
- 与 CONTEXT.md 术语的偏差：CONTEXT.md"质量感知瓦片键"（`{page}|{zoom}|{x}|{y}|{quality}`）对应的 `QualityTileKey` 实际未被使用；`tile_cache_legacy` 的 base/detail 缓存键（`{path}|{page}|{scene}|base|{zoom}|{dpr}` 等）是 CONTEXT.md 未覆盖的另一套键格式；`tile_v2::TileKey` 键序为 `{page}|{zoom}|{dpr}|{x}|{y}`（与词条一致）但 cache_key 序列化省略了 dpr 之外的 `QualityTileKey` 格式细节。
