# pdf-viewer-ui · 会话域（document / viewer / page / present / presentation / find / comment / annotation / review / host）

> 范围：`crates/pdf-viewer-ui/src/` 下 10 个会话域目录（41 个文件，约 4700 行，含内联测试）。`editor/`、`zoom/`、`render/` 已有独立文档（ui-editor.md / ui-zoom.md / ui-render.md），不在本文范围。
> 上游：`pdf-viewer-core` 纯数据与决策（`render::viewer_session` / `present_plan` / `plan_builder` / `find_state` / `comment_review_state`、`annotation::annotation_types`、`persistence::review_types`、`models`（PageState / VectorPageModel / GlyphPaintPlan / PersistableRegionPatch））；crate 内权威状态与服务——`ui_state_store`（补丁/撤销历史）、`zoom`（缩放权威 ZOOM_STATE，ADR-0001）、`render`（render_store / tile_cache / frame_cache / workflow / progressive / prepared_scene）、`editor`（session / orchestrator / list_format / debug_trace）、`bridge` + `runtime`（Tauri invoke）、`events`（EventBus）、`viewport_refresh`。
> 下游：TS 前端桥（本文所有 `#[wasm_bindgen]` 导出：DocumentSession / ViewerSession / FindSession / ReviewSession / CommentManager / AnnotationManager / PagePresentationRuntime 及 free_api / controller_facade 平面函数）；crate 内根模块 `application` / `api` / `app_controller` / `commands` 与 `projection_workflow` 也经 `viewer_controller`、`page::context`、`host::command` 等调用这些域。

## 职责
- **document/**：文档会话——打开/关闭/旋转/撤销重做管线、补丁持久化、评论与批注的宿主侧 invoke 封装及面板/覆盖层数据组装。
- **viewer/**：ViewerSession——当前文档绑定、页码、页面尺寸与文档修订号（缩放为派生投影），并作为全运行时重置枢纽。
- **page/**：单页渲染上下文 thread_local（PageState / PreparedScene / 渐进任务）及与 viewer/zoom 的同步。
- **present/**：帧计划装配（复用 core 纯决策）与 PRESENT/FRAME_CACHE/VIEWPORT_REFRESH 三个 thread_local 运行时、帧入队与提交。
- **presentation/**：翻页编排——翻页意图状态机、fast-flip 检测、相邻页预取决策、渲染队列动作决策。
- **find/**：查找工具栏控制器状态（结果、替换请求）、查找会话快照（host 侧）与 WASM 绑定。
- **comment/**：CommentManager WASM 边界——评论审阅面板/覆盖层加载与评论增删改。
- **annotation/**：AnnotationManager WASM 边界——PDF `/Annot` 规范层 list/delete。
- **review/**：ReviewSession WASM 边界（变更 accept/reject）与评论审阅会话快照存储。
- **host/**：平台桥接——打开/重置文档会话命令、翻页导航、宿主布局同步与滚动刷新决策。

## document（文档会话）

### `document/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod comment / document_api / document_types / free_api / host_pipeline / io / mutation_pipeline / patch_persistence` | 声明 8 个子模块（纯清单） |

### `document/document_api.rs` — DocumentSession：文档级操作的结构化 WASM API（P1）
| 可见性 | 方法 | 功能 |
|---|---|---|
| wasm | `new()` (constructor) | 创建零尺寸 DocumentSession 句柄 |
| wasm | `open(request)` | 异步按 path/bytes/URL 打开 PDF 并建立会话 |
| wasm | `close(w, h)` | 关闭文档并重置宿主会话为默认页尺寸 |
| wasm | `undo()` / `redo()` | 文档级撤销 / 重做一步 |
| wasm | `rotate(delta)` | 异步将当前页旋转 delta×90 度 |
| wasm | `hasUnsavedChanges()` | 是否存在待持久化的可见补丁 |
| wasm | `patchCount()` | 返回内存中可持久化补丁数量 |
| wasm | `canUndo()` / `canRedo()` | 查询撤销 / 重做可用性 |
| wasm | `requestRefresh(source, frameRequest)` | 记录变更修订并调度渲染帧 |
| wasm | `applyPatch(patch)` | 将区域补丁应用到内存文档 |
| wasm | `buildRegionPatch(...)` | 构造区域文本补丁（转调 editor_controller） |
| wasm | `applyRegionReplacements(...)` | 批量应用区域文本替换（转调 editor 事务管线） |

### `document/document_types.rs` — DocumentResponse 的 JsValue 序列化辅助
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::document::document_types::{DocumentError, DocumentResponse}` | 复用 core 的响应 DTO |
| 私有 | `response_to_js(resp)` | DocumentResponse 序列化为 JsValue |
| pub | `ok_response(data)` | 构造带载荷的成功响应 |
| pub | `ok_empty()` | 构造无载荷成功响应（标注 dead_code） |
| pub | `err_response(error)` | 构造错误响应 |

### `document/comment.rs` — 评论/批注宿主管线：Tauri invoke 封装 + 审阅面板与覆盖层组装
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::annotation::*`（约 22 个 DTO） | 复用 core 评论/批注请求与结果类型 |
| pub | `PdfCommentReviewDisplay` (type) | 会话+审阅+面板+覆盖层的组合显示模型 |
| 私有 | `PathPageArgs` / `PathRequestArgs<T>` (struct) | invoke 参数信封（camelCase） |
| pub | `list_page_comments(path, page)` | 异步 smart_invoke("read_comments") 读取页评论 |
| pub | `list_page_annotation_targets(path, page)` | 异步读取页面批注目标 |
| pub | `review_document_comments(path, request)` | 异步按页/文档范围+关键字检索评论 |
| pub | `load_comment_review(path, page)` | 读会话后加载审阅面板+覆盖层 |
| pub | `load_comment_overlay(path, page)` | 加载当前页评论覆盖层标记 |
| pub | `load_comment_target_overlay(path, page)` | 加载"添加批注"目标覆盖层标记 |
| pub | `set_comment_review_panel_open_and_load(...)` | 设置面板开合并重载审阅显示 |
| pub | `toggle_comment_review_panel_and_load(...)` | 切换面板开合并重载 |
| pub | `set_comment_review_scope_and_load(...)` | 切换页/文档范围并重载 |
| pub | `set_comment_review_query_and_load(...)` | 设置过滤关键字并重载 |
| pub | `select_comment_review_and_load(...)` | 设置选中评论并重载 |
| 私有 | `load_comment_review_from_session(...)` | 按会话范围组装审阅结果与覆盖层 |
| 私有 | `build_comment_review_panel(...)` | 组装面板：meta 文本/页摘要 chip/评论卡片 |
| 私有 | `build_comment_review_card_actions()` | 生成卡片动作 Jump/Edit/Delete |
| 私有 | `build_comment_overlay_display(...)` | 评论列表转覆盖层标记（含选中态） |
| 私有 | `build_comment_target_overlay_display(...)` | 批注目标转覆盖层标记 |
| 私有 | `build_percent_frame(...)` | 盒矩形转百分比定位框并保底最小百分比 |
| pub | `add_region_comment(path, request)` | 异步 smart_invoke("apply_comment") 添加区域评论 |
| pub | `delete_page_annotation(path, request)` | 异步删除页面批注 |
| pub | `update_page_comment(path, request)` | 异步更新页面评论 |

（无 thread_local；会话状态存于 `review::review_store::COMMENT_REVIEW_SESSION`。）

### `document/free_api.rs` — document 域遗留平面 WASM 导出（TS 桥兼容层）
| 可见性 | 条目 | 功能 |
|---|---|---|
| wasm | `undoDocumentPipeline` / `redoDocumentPipeline` | 平面导出文档撤销/重做 |
| wasm | `openDocumentPipeline` | 平面导出异步打开文档管线 |
| wasm | `pickDocumentPipeline` | 平面导出文件选择并打开管线 |
| wasm | `rotateDocumentPipeline` | 平面导出旋转当前页管线 |
| wasm | `closeDocumentPipeline` | 平面导出关闭文档管线 |
| wasm | `readViewerSession` | 平面导出读取 viewer 会话快照 |
| wasm | `getViewerSession` | 已废弃别名（转调 readViewerSession） |
| wasm | `setViewerDocument` | 平面导出绑定 viewer 文档 |

### `document/host_pipeline.rs` — 打开/关闭/旋转/撤销重做的文档管线编排
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `OpenDocumentPipelineRequest` / `...Result` (struct) | 打开文档管线 DTO（path/初始 zoom/默认页尺寸等） |
| pub | `CloseDocumentPipelineResult` (struct) | 关闭结果 DTO（closed/页码/zoom） |
| pub | `PickDocumentPipelineRequest` (struct) | 文件选择管线 DTO |
| pub | `RotateDocumentPipelineResult` / `DocumentMutationPipelineResult` (struct) | 旋转/撤销重做的 changed 结果 DTO |
| 私有 | `open_session_from_file_result(...)` | 打开成功后建立宿主文档会话并汇总结果 |
| pub | `open_document_pipeline(request)` | 异步 open_pdf_file 后绑定会话 |
| pub | `pick_document_pipeline(request)` | 异步弹文件选择后接续 open 管线 |
| pub | `close_document_pipeline(w, h)` | 重置宿主会话并包装关闭结果 |
| pub | `rotate_document_pipeline(delta)` | 异步旋转当前页并包装结果 |
| pub | `undo_document_pipeline()` / `redo_document_pipeline()` | 转调 ui_state_store 撤销/重做并包装 changed |

### `document/io.rs` — 与 Tauri 宿主的 PDF 文件级 I/O
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `OpenPdfFileResult` / `RotateCurrentPageResult` (struct) | 打开/旋转结果 DTO |
| pub | `open_pdf_file(path)` | 经 target_invoke("open_pdf") 打开并返回页数 |
| pub | `pick_pdf_file()` | 经 target_invoke("pick_file") 返回所选文件路径 |
| pub | `rotate_current_page(delta)` | 经 target_invoke("save_pdf") 写入当前页旋转 |

### `document/mutation_pipeline.rs` — 文档变更后的刷新管线
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `DocumentRefreshPipelineResult` (struct) | 刷新结果：修订号 + 可选渲染帧信封 |
| pub | `request_document_refresh(reason, request)` | 修订号+1、重置渲染运行时并调度渲染帧 |

### `document/patch_persistence.rs` — 区域补丁的内存应用与宿主持久化
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `apply_document_patch_direct(patch)` | Rust 直连应用补丁并入撤销历史（记 trace） |
| pub | `apply_document_patch(patch_js)` | 解析 JsValue 补丁应用（失败记错误 trace） |
| pub | `has_persistable_patches()` | 是否存在可持久化补丁 |
| pub | `collect_persistable_patches_js()` | 收集补丁并序列化为 JsValue |
| pub | `clear_persistable_patches(clear_history)` | 清空补丁队列（可选连撤销历史） |
| pub | `save_persistable_patches(path, page)` | 合并编号修订补丁后经 host 保存并清空队列 |

## viewer（查看器会话）

### `viewer/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod free_api / viewer_api / viewer_controller / viewer_store` | 声明 4 个子模块 |

### `viewer/viewer_api.rs` — ViewerSession：viewer 状态的结构化 WASM API
| 可见性 | 方法 | 功能 |
|---|---|---|
| wasm | `new()` (constructor) | 创建零尺寸 ViewerSession 句柄 |
| wasm | `read()` | 读取会话快照（path/页数/zoom/页尺寸，带 console 日志） |
| wasm | `setDocument(path, count, zoom)` | 绑定新打开文档进会话 |
| wasm | `reset()` | 重置会话为空默认态 |
| wasm | `setCurrentPage(page)` | 设置当前页码 |
| wasm | `setCurrentZoom(zoom)` | 经缩放权威单入口写当前缩放 |
| wasm | `setPageDimensions(w, h)` | 更新活动页尺寸（供缩放/命中测试） |
| wasm | `readState()` | 读会话状态（NoDocument/DocumentOpen） |
| wasm | `getState()` | 已废弃别名（转调 readState） |
| wasm | `setState(updater)` | Nutrient 风格函数式原子更新（仅可变字段生效） |

### `viewer/viewer_controller.rs` — viewer 域控制器：会话写入口与全运行时重置枢纽
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ViewerRuntimeResetOptions` (struct) | 重置选项（缓存/刷新/编辑器模式/补丁） |
| pub | `reset_viewer_runtime(options)` | 按选项重置渲染/展示/渐进/缩放/编辑器/查找/评论 |
| pub | `reset_session()` | 重置会话存储并以全量选项重置运行时 |
| pub | `reset_zoom_view(initial_zoom)` | 重置缩放运行时并重置运行时（保留补丁） |
| pub | `note_document_mutation(reason)` | 修订号+1 并重置渲染/展示/渐进/缩放预览 |
| pub | `read_session()` | 读取会话快照（zoom 为权威派生投影） |
| pub | `set_document(path, count, zoom)` | 绑定文档并重置运行时（清补丁、保编辑器模式） |
| pub | `set_page(page)` | 设当前页、重置刷新并广播 VIEWER_PAGE_CHANGE 事件 |
| pub | `set_zoom(zoom)` | 经缩放权威单入口写缩放并广播 ZOOM_CHANGE 事件 |
| pub | `set_page_size(w, h)` | 记录页面尺寸（带日志） |

### `viewer/viewer_store.rs` — VIEWER_SESSION thread_local 会话存储
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `thread_local VIEWER_SESSION` | HostViewerSession 会话存储（RefCell） |
| pub | `pub use core::render::viewer_session::*` | 复用 core 会话数据结构（HostViewerSession 等） |
| pub | `ViewerSessionState` (enum) | NoDocument / DocumentOpen 派生枚举 |
| pub | `ViewerSessionState::as_str()` | 状态转字符串字面量 |
| pub | `read_viewer_state()` | 由 path 是否存在派生会话状态 |
| pub | `reset_viewer_session()` | 会话重置为默认值 |
| pub | `set_viewer_document(path, count, zoom)` | 绑定文档（zoom 转写缩放权威，不落存储） |
| pub | `set_current_page(page)` | 写当前页码 |
| pub | `set_zoom_and_page_dimensions(zoom, w, h)` | zoom 写权威、页尺寸写存储（下限 1.0） |
| pub | `read_viewer_session()` | 快照；current_zoom 从 ZOOM_STATE.target_zoom 派生 |
| pub | `set_page_dimensions(w, h)` | 写页面尺寸（下限 1.0） |
| pub | `bump_document_revision()` | 文档修订号单调 +1 并返回 |
| pub | `current_document_revision()` | 读取当前修订号 |

### `viewer/free_api.rs` — viewer 域遗留平面 WASM 导出
| 可见性 | 条目 | 功能 |
|---|---|---|
| wasm | `initPageContext(...)` | 解析并解压调色板后初始化页面上下文 |
| wasm | `setCurrentPage(page)` | 平面导出设置当前页（转调 set_page） |
| wasm | `dumpEditorDebugTrace(filter)` | 将编辑器调试 trace 按 filter 打到 console |

## page（页面上下文）

### `page/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod context / page_store` | 声明 2 个子模块 |

### `page/page_store.rs` — 单页渲染状态 thread_local（PAGE_STATE / PREPARED_SCENE / 渐进任务）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `thread_local PAGE_STATE` | 当前页 PageState（zoom/dpr/视口/模型/绘制计划） |
| pub | `thread_local PREPARED_SCENE` | 预渲染场景 PreparedPageScene（可空） |
| pub | `thread_local PROGRESSIVE_RENDER_TASK` | 渐进矢量渲染任务（可空） |
| pub | `HostPageState` (type) | RefCell&lt;PageState&gt; 类型别名 |
| pub | `with_page_state(f)` | 只读借用页面状态执行闭包 |
| pub | `with_page_and_scene(f)` | 同时借页面状态与场景执行闭包 |
| pub | `with_progressive_task_mut(f)` | 可变借用渐进任务执行闭包 |
| pub | `set_progressive_task(task)` | 覆写或清除渐进渲染任务 |
| pub | `init_page_context(model, plan, ...)` | 构建 PreparedScene、写页面状态并返回页尺寸 |
| pub | `update_page_viewport(zoom, dpr, ...)` | 更新 zoom/dpr/视口（缺省推页尺寸）并返回页尺寸 |
| pub | `reset_progressive_render_task()` | 清除渐进渲染任务 |

### `page/context.rs` — 页面初始化/视口更新工作流（与 viewer/zoom 同步）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `init_page_context_from_models(...)` | 初始化页面上下文并同步 zoom/尺寸到 viewer 会话 |
| pub | `update_page_viewport_workflow(...)` | 更新视口并把 zoom 经权威单入口写 ZOOM_STATE |

## present（帧计划与展示状态）

### `present/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod plan / plan_builder / present_store` | 声明 3 个子模块 |

### `present/plan.rs` — core 展示策略纯函数 re-export
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::render::present_plan::*` | preview_is_settled / quantize_cache_zoom / resolve_present_policy 等 |

### `present/plan_builder.rs` — 宿主侧帧计划装配（粘合 zoom/session/瓦片缓存）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::render::plan_builder::*` | FramePlanRequest/Result 与 core 纯决策函数 |
| pub | `build_frame_plan_result(request, zoom, session, present, key)` | 解析缩放/布局/可见矩形/缓存复用，生成完整帧计划 |

（核心逻辑：stable_document_frame 强制整页重渲；预览未落定时按 PREVIEW_BASE_REFRESH_RATIO 决定是否刷新 base 层；可复用 detail tile / base layer 决定 effective cache zoom 与缓存键。）

### `present/present_store.rs` — PRESENT/FRAME_CACHE/VIEWPORT_REFRESH thread_local 与帧调度
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `thread_local PRESENT_STATE` | HostPresentState（活动 base 层/复用决策状态） |
| pub | `thread_local FRAME_CACHE_STATE` | HostFrameCacheState（base/detail 帧缓存键 LRU） |
| pub | `thread_local VIEWPORT_REFRESH_STATE` | HostViewportRefreshState（刷新抑制窗口） |
| pub | `with_present_state(f)` | 只读借用展示状态执行闭包 |
| pub | `build_frame_plan_result(request)` | 先读 session 再进 ZOOM_STATE 防双借，装配帧计划 |
| pub | `resolve_viewport_refresh(request)` | 构帧计划后判定细节层视口刷新决策 |
| pub | `touch_frame_cache_entry(is_detail, key)` | LRU touch 命中键，返回是否命中 |
| pub | `store_frame_cache_entry(is_detail, key)` | 存键并返回被淘汰键 |
| pub | `reset_frame_cache()` | 清空帧缓存键 |
| pub | `reset_present_runtime(cache, refresh)` | 重置展示状态（可选连带缓存/刷新窗口） |
| pub | `schedule_render_frame_request(request)` | 构帧计划、逐出 editorVisibility 过期在飞帧后入队信封 |
| pub | `commit_render_frame(token, zoom)` | 以渲染缩放提交帧令牌，返回是否接受 |
| pub | `settle_render_frame(token, zoom?)` | 结算帧令牌，返回帧转移结果 |
| pub | `pub use render::render_store::is_render_frame_current` | 帧令牌时效判定 re-export |

## presentation（翻页编排）

### `presentation/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod page_turn / presentation_api / render_queue` | 声明 3 个子模块 |

### `presentation/page_turn.rs` — 翻页状态机：意图接受、可见标记、资产准入与预取决策
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `PREVIEW_PREFETCH_*` / `VECTOR_PREFETCH_*` (const) | normal/fast-flip 下的预取窗口（3/8、2/1、2/0 页） |
| 私有 | `FAST_FLIP_THRESHOLD_MS` (const) | 翻页间隔 <100ms 进入高速模式 |
| pub | `PageTurnPhase` (enum) | Idle/Turning/Preview/Vector/Detail/RasterVisible 六相 |
| pub | `PageTurnSnapshot` (struct) | 翻页快照（最新 turn id/页码/方向/相位/fast_flip 等） |
| pub | `PageTurnDecision` (struct) | 翻页决策（接受/拒绝理由+快照） |
| pub | `PageVisibleDecision` (struct) | 页面可见决策（表面类型+可否预取） |
| pub | `PageAssetAdmission` (struct) | 资产准入结果（角色/资产类型/优先级） |
| pub | `PagePrefetchTarget` / `PagePrefetchDecision` (struct) | 预取目标与整体预取决策 |
| 私有 | `thread_local PAGE_TURN_STATE` | RefCell&lt;PageTurnSnapshot&gt; 翻页状态存储 |
| pub | `read_page_turn_snapshot()` | 读取翻页快照 |
| pub | `reset_page_turn_state()` | 重置翻页状态 |
| pub | `request_page_turn(target, reason, now_ms)` | 校验（无文档/越界/同页）后接受翻页并检测 fast-flip |
| pub | `is_latest_page_turn(id, page)` | 判断给定翻页是否仍是最新意图 |
| pub | `mark_page_visible(page, surface)` | 标记最新页某表面可见并广播 PAGE_TURN_VISIBLE |
| pub | `can_prefetch(page)` | 当前相位与可见页是否允许预取 |
| pub | `admit_page_asset(page, role, kind)` | 准入 current/prefetch 资产并计算优先级 |
| pub | `decide_adjacent_prefetch(anchor, count)` | 生成相邻页预取跑道（顺向 preview/vector + 逆向 preview） |
| 私有 | `reject / reject_prefetch` | 构造拒绝决策并广播 PAGE_TURN_REJECT |
| 私有 | `normalize_reason / normalize_surface / normalize_role / normalize_asset_kind` | 入参白名单归一化（未知归 "unknown"） |
| 私有 | `visible_phase / phase_allows_prefetch` | 表面→相位映射与相位→可预取判定 |
| 私有 | `current_asset_priority / prefetch_priority` | 资产类型优先级（100~50）与方向一致加成（30/10） |
| 私有 | `prefetch_window_for_asset` | 按资产类型与 fast-flip 取预取窗口 |
| 私有 | `push_prefetch_runway / push_prefetch_candidate` | 填充顺/逆向预取候选（越界剔除） |
| 私有 | `resolve_direction` | 目标页相对当前页的方向（±1/0） |
| 私有 | `emit_decision / emit_visible` | wasm32 下经 EventBus 广播决策 |
| 测试 | `rejects_stale_page` 等 7 例 | 覆盖过期拒绝、方向偏好、fast-flip 启停与节流 |

### `presentation/presentation_api.rs` — PagePresentationRuntime：翻页编排的结构化 WASM API
| 可见性 | 方法 | 功能 |
|---|---|---|
| wasm | `new()` (constructor) | 创建零尺寸 PagePresentationRuntime 句柄 |
| wasm | `requestPageTurn(target, reason, nowMs)` | 请求翻页（返回决策 DTO） |
| wasm | `readPageTurn()` | 读取翻页快照 |
| wasm | `isLatestPageTurn(id, page)` | 判断是否最新翻页意图 |
| wasm | `markPageVisible(page, surface)` | 标记页面表面可见 |
| wasm | `canPrefetch(page)` | 查询可否预取 |
| wasm | `admitPageAsset(page, role, kind)` | 资产准入查询 |
| wasm | `decideAdjacentPrefetch(anchor, count)` | 相邻页预取决策 |
| wasm | `resolveRenderQueueAction(...)` | 渲染队列动作决策查询 |
| wasm | `reset()` | 重置翻页状态 |

### `presentation/render_queue.rs` — 渲染队列动作决策（提交抑制/派发/替换）
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `COMMIT_SUPPRESS_MS` / `SCROLL_DEBOUNCE_MS` (const) | 提交后抑制 120ms、滚动去抖 56ms |
| pub | `RenderQueueAction` (struct) | 动作结果（action/suppress/排队效果/拒绝理由） |
| pub | `resolve_render_queue_action(source, executing, now, last)` | 空闲派发、提交后抑制 scroll、执行中按导航/非导航替换排队 |
| 私有 | `normalize_render_source(source)` | 来源白名单归一化 |
| 测试 | `suppresses_scroll...` 等 4 例 | 覆盖抑制、空闲派发与两类替换 |

## find（查找）

### `find/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod controller_facade / find_api / find_store / host_find_store` | 声明 4 个子模块 |

### `find/find_api.rs` — FindSession：查找工具栏的结构化 WASM API（P2）
| 可见性 | 方法 | 功能 |
|---|---|---|
| wasm | `new()` (constructor) | 创建零尺寸 FindSession 句柄 |
| wasm | `open(page, count, path)` | 打开查找工具栏并同步页码/文档上下文 |
| wasm | `close()` | 关闭工具栏并清空结果与宿主会话 |
| wasm | `toggle(page, count, path)` | 切换工具栏开合 |
| wasm | `clear()` | 清空当前搜索结果（保留开合状态） |
| wasm | `setCurrentPage(page)` | 通知控制器页码变更 |
| wasm | `readState()` | 读派生状态（Closed/Open/Searching/Active） |
| wasm | `getState()` | 已废弃别名（转调 readState） |

### `find/find_store.rs` — FindController 状态、结果 DTO 与替换请求构造
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `SearchMatch` / `SearchBox` / `SearchResult` (struct) | 匹配项/盒矩形/结果集 DTO |
| pub | `FindScope` (enum) | Page / Document 搜索范围 |
| pub | `FindSessionState` (enum) | Closed/Open/Searching/Active 派生态 |
| pub | `FindSessionState::as_str()` | 状态转字符串字面量 |
| 私有 | `FindSessionState::derive(...)` | 由 is_open+query+匹配数按需派生状态 |
| pub | `read_find_state()` | 读取派生查找会话状态 |
| pub | `FindControllerState` / `FindStateUpdate` / `CurrentPageMatch` (struct) | 控制器快照/更新结果（含当前页匹配与跳页提示） |
| pub | `ReplaceRequest` (struct) | 单条替换请求 DTO（区域 id/kind/原文本等） |
| pub | `FindToolbarState` (struct) | 工具栏展示态（计数文本/可替换性） |
| 私有 | `thread_local CONTROLLER` + `FindControllerInner` | 控制器内部状态（is_open/结果/页码/path） |
| pub | `open_find(page, count, path)` | 打开工具栏并记录上下文 |
| pub | `close_find()` | 关闭工具栏、清结果并清宿主查找会话 |
| pub | `toggle_find(page, count, path)` | 按开合状态分发 open/close |
| pub | `set_search_result(result, scope, page)` | 存搜索结果并同步宿主查找会话 |
| pub | `clear_search()` | 清空结果与宿主查找会话 |
| pub | `move_active(step)` | 活动匹配步进（环回），必要时给出跳页目标 |
| pub | `set_current_page(page)` | 更新当前页码 |
| pub | `read_toolbar_state()` | 组装工具栏展示态 |
| pub | `build_replace_requests(replacement, all, scope)` | 按范围/可编辑类型构造替换请求列表 |
| 私有 | `is_editable_kind(kind)` | 仅 paragraph/list-item 区域可替换 |
| 私有 | `build_state_update / build_current_page_matches / build_toolbar_state` | 组装更新结果/当前页匹配/工具栏态 |

### `find/controller_facade.rs` — findController* 平面 WASM 导出（TS 迁移期兼容层）
| 可见性 | 条目 | 功能 |
|---|---|---|
| wasm | `findControllerOpen / Close / Toggle` | 平面导出开合控制（与 FindSession 同构） |
| wasm | `findControllerSetResult(result, scope, page)` | 平面导出写入搜索结果 |
| wasm | `findControllerClear` | 平面导出清空结果 |
| wasm | `findControllerMoveActive(step)` | 平面导出活动匹配步进 |
| wasm | `findControllerSetCurrentPage(page)` | 平面导出设置当前页 |
| wasm | `findControllerGetToolbarState` | 平面导出读取工具栏态 |
| wasm | `findControllerGetReplaceRequests(...)` | 平面导出构造替换请求列表 |

### `find/host_find_store.rs` — 宿主侧查找会话快照（供 viewer 驱动 UI）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::render::find_state::*` | HostFindSession / HostFindScope / 导航结果 DTO |
| pub | `thread_local FIND_SESSION` | HostFindSession 快照存储（RefCell） |
| pub | `clear_find_session()` | 重置查找会话快照 |
| pub | `read_find_session()` | 读取查找会话快照 |
| pub | `set_find_session(query, scope, pages, preferred)` | 写入会话并解析初始活动索引 |
| pub | `move_find_match(step)` | 活动匹配环回步进并计算是否跨圈 |
| 私有 | `resolve_initial_active_index(...)` | 按偏好页定位初始索引（缺省 0） |
| 私有 | `wrapped_between(...)` | 按步进方向判断是否跨圈 |

## comment（评论 WASM 边界）

### `comment/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod comment_api` | 声明唯一子模块 |

### `comment/comment_api.rs` — CommentManager：评论审阅 UX 的结构化 WASM API（P2）
| 可见性 | 条目 | 功能 |
|---|---|---|
| wasm | `new()` (constructor) | 创建零尺寸 CommentManager 句柄 |
| wasm | `clearReviewSession()` | 清空评论审阅会话快照 |
| wasm | `readReviewSession()` | 读取评论审阅会话快照 |
| wasm | `loadReview(path, page)` | 异步加载审阅面板+覆盖层 |
| wasm | `loadOverlay(path, page)` | 异步加载当前页评论覆盖层 |
| wasm | `loadTargetOverlay(path, page)` | 异步加载批注目标覆盖层 |
| wasm | `setPanelOpenAndLoad(...)` | 设置面板开合并加载 |
| wasm | `togglePanelAndLoad(...)` | 切换面板开合并加载 |
| wasm | `setScopeAndLoad(...)` | 设置页/文档范围并加载 |
| wasm | `setQueryAndLoad(...)` | 设置过滤关键字并加载 |
| wasm | `selectAndLoad(...)` | 选中评论并加载 |
| wasm | `addRegionComment(path, request)` | 异步添加区域评论 |
| wasm | `deleteAnnotation(path, request)` | 异步删除批注 |
| wasm | `updateComment(path, request)` | 异步更新评论 |
| 私有 | `parse_scope(scope)` | "document"/其余 → HostCommentReviewScope |

## annotation（批注 WASM 边界）

### `annotation/mod.rs` — 域说明与模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod annotation_api` | 声明唯一子模块（文档注释界定与 comment 域的分层） |

### `annotation/annotation_api.rs` — AnnotationManager：PDF `/Annot` 规范层结构化 WASM API（P3）
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `response_to_js(resp)` | AnnotationResponse 序列化为 JsValue |
| pub | `ok_response(data)` / `ok_empty()` / `err_response(error)` | 构造成功/空成功/错误响应（后两者标注 dead_code） |
| 私有 | `parse_annotation_kind(raw)` | 子类型字符串白名单 → AnnotationKind |
| 私有 | `target_to_annotation(t)` | 批注目标(w,h) 转 /Annot 模型(right,bottom) |
| wasm | `new()` (constructor) | 创建零尺寸 AnnotationManager 句柄 |
| wasm | `list(path, page)` | 列出页面全部批注（结构化响应） |
| wasm | `delete(path, page, id)` | 按 id 删除页面批注 |

## review（审阅）

### `review/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod review_api / review_store` | 声明 2 个子模块 |

### `review/review_api.rs` — ReviewSession：变更 accept/reject 的结构化 WASM API（P2）
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `read_review_feed()` | 汇集待审补丁并组装 ReviewFeedResult |
| pub | `ReviewSessionState` (enum) | Idle / HasPending 派生态 |
| pub | `ReviewSessionState::as_str()` | 状态转字符串字面量 |
| 私有 | `ReviewSessionState::derive(count)` | 由待审数量派生状态 |
| pub | `read_review_state()` | 读取审阅会话派生状态 |
| wasm | `new()` (constructor) | 创建零尺寸 ReviewSession 句柄 |
| wasm | `readFeed()` | 读取待审变更 feed（含修订号） |
| wasm | `accept(patchKey)` / `reject(patchKey)` | 接受 / 拒绝（还原）单个变更 |
| wasm | `acceptAll()` / `rejectAll()` | 全部接受 / 全部拒绝 |
| wasm | `readState()` | 读会话状态（Idle/HasPending） |
| wasm | `getState()` | 已废弃别名（转调 readState） |

### `review/review_store.rs` — COMMENT_REVIEW_SESSION thread_local（评论审阅会话快照）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use core::render::comment_review_state::*` | HostCommentReviewSession / Scope 等 DTO |
| pub | `thread_local COMMENT_REVIEW_SESSION` | 评论审阅会话快照存储（RefCell） |
| pub | `clear_comment_review_session()` | 重置为默认快照 |
| pub | `read_comment_review_session()` | 读取快照 |
| pub | `set_comment_review_panel_open(open)` | 写面板开合 |
| pub | `toggle_comment_review_panel()` | 翻转面板开合 |
| pub | `set_comment_review_scope(scope)` | 写搜索范围（页/文档） |
| pub | `set_comment_review_query(query)` | 写过滤关键字 |
| pub | `select_comment_review_comment(id?)` | 写选中评论 id |
| 私有 | `replace_comment_review_session(next)` | 整体替换快照并返回 |
| 私有 | `update_comment_review_session(update)` | 读-改-写快照的通用闭包封装 |

## host（平台桥接）

### `host/mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod command / layout / scroll` | 声明 3 个子模块 |

### `host/command.rs` — 宿主文档会话命令：打开/重置/翻页/缩放选择
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `OpenDocumentSessionRequest` / `HostActionResult` (struct) | 会话请求与统一动作结果 DTO |
| pub | `open_document_session(request)` | 绑定文档、设默认页尺寸并重置缩放运行时 |
| pub | `reset_host_document_session(w, h)` | 全量重置会话并恢复默认页尺寸/缩放 1.0 |
| pub | `navigate_prev_page()` | 上一页（无文档或首页则不变更） |
| pub | `navigate_next_page()` | 下一页（无文档或末页则不变更） |
| pub | `apply_zoom_selection(zoom)` | 程序化缩放即时跳转（visual 直接吸附 target） |

### `host/layout.rs` — 宿主布局同步：DOM/展示/宿主尺寸与内容偏移
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `HostLayoutOverride` (struct) | 调用方可覆盖的宿主布局（宽高+内容偏移） |
| pub | `SyncHostLayoutRequest` / `SyncHostLayoutResult` (struct) | 布局同步请求/结果 DTO |
| pub | `sync_host_layout(request)` | 清洗输入、按 display_zoom 推 DOM 尺寸（css_scale 恒 1）并写视觉布局 |
| 测试 | `committed_state_has_identity_css_scale` 等 5 例 | 覆盖 css_scale 恒等、无变换过冲、缺省与非法输入 |

### `host/scroll.rs` — 滚动触发的视口刷新决策入口
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `resolve_host_scroll_refresh(request)` | 无文档或缩放未落定时跳过，否则转视口刷新决策 |

## 疑点
- **host_ 前缀未按计划改名**：`docs/framework-refactor-completion-plan.md` §3.2/§8 要求 `host/` 目录改 `platform/`、`host_runtime.rs` 改 `platform_bridge.rs`、`host_find_store.rs` 并入 `find_store.rs`，且全局无 host_ 前缀——均未执行；`document/host_pipeline.rs`、`document/io.rs` 等同属 host_ 语境残留（计划未显式列出）。
- **controller_facade 与 find_api 重复**：`find/controller_facade.rs` 的 `findController*` 平面导出与 `FindSession` 完全同构，属迁移期双绑定，TS 切换后应删除（同 `document/free_api.rs`、`viewer/free_api.rs` 兼容层）。
- **跨域依赖与命名混淆**：`document/comment.rs`（宿主 invoke 封装）与 `comment/` 域同名易混；`comment/comment_api.rs` 反向依赖 `review/review_store.rs` 的会话状态（文件自述为域自包含而迁出 viewer，但 comment→review 耦合仍在）；`viewer/free_api.rs` 里的 `initPageContext`（page 域职责）与 `dumpEditorDebugTrace`（editor 调试工具）跨域泄漏。
- **死代码**：`document_types.rs` 与 `annotation_api.rs` 的 `ok_empty()` 均 `#[allow(dead_code)]`；`ViewerSession::read()` 内嵌 console 日志疑似调试残留；`#[deprecated]` 的 `getViewerSession` / `getState`（Viewer/Find/Review 三处）违反计划"无 #[deprecated] 方法"验收项。
- **thread_local 数量超标**：本范围内 11 个 thread_local（VIEWER_SESSION、PAGE_STATE、PREPARED_SCENE、PROGRESSIVE_RENDER_TASK、PRESENT_STATE、FRAME_CACHE_STATE、VIEWPORT_REFRESH_STATE、PAGE_TURN_STATE、CONTROLLER、FIND_SESSION、COMMENT_REVIEW_SESSION），与计划"域状态 ≤ 2（AppContext + EventBus）"相距甚远；`viewer_controller` 同时充当全局重置枢纽（直接 reset editor/find/review/zoom/render 各域）。
- **疑似重复/近似命名**：`present/`（帧计划状态）与 `presentation/`（翻页编排）目录名高度相似易混；`present/plan.rs` 与 `plan_builder.rs` 各持一层 core re-export；`page/context.rs::init_page_context_from_models` 与 `page_store::init_page_context` 同名近义；`io.rs::rotate_current_page` 直接经 `save_pdf` 写旋转，绕过补丁/撤销历史（与 `patch_persistence` 管线并行）。
