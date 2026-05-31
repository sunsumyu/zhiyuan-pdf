# Sovereignty PDF Viewer - 功能到方法全面映射与重构方案

> 审计日期: 2026-05-03  
> 总方法数: 2,029 (排除 noise 后)  
> 功能域: 28 个  
> 孤儿方法: 633 个 (需确认)

## 功能域分布总览

| 功能域 | 映射方法数 | Kernel | WASM UI | Tauri Host | Frontend | Utils | 说明 |
|--------|-----------|--------|---------|------------|----------|-------|------|
| F1 文档生命周期 | 60 | 0 | 0 | 50 | 10 | 0 | 打开/关闭/保存/旋转/元数据 |
| F2 页面导航 | 37 | 0 | 0 | 0 | 37 | 0 | 翻页/滚动/页面跳转 |
| F3 缩放控制 | 85 | 0 | 85 | 0 | 0 | 0 | 缩放级别/动画/限制 |
| F4 渲染管线 | 343 | 0 | 343 | 0 | 0 | 0 | Canvas/帧计划/渐进渲染/缓存 |
| F5 矢量提取 | 25 | 25 | 0 | 0 | 0 | 0 | PDF内容提取/布局推断 |
| F6 文本编辑-核心 | 75 | 0 | 75 | 0 | 0 | 0 | 编辑器激活/光标/输入 |
| F7 文本编辑-格式化 | 58 | 0 | 58 | 0 | 0 | 0 | 粗体/斜体/颜色/字体 |
| F8 文本编辑-草稿布局 | 44 | 44 | 0 | 0 | 0 | 0 | 草稿文本/样式保持 |
| F9 文本编辑-替换补丁 | 48 | 0 | 48 | 0 | 0 | 0 | 文本替换/持久化 |
| F10 搜索替换 | 28 | 0 | 0 | 28 | 0 | 0 | 文档搜索/批量替换 |
| F11 批注-高亮 | 16 | 0 | 0 | 16 | 0 | 0 | 文本高亮 |
| F12 批注-评论 | 60 | 0 | 0 | 60 | 0 | 0 | 评论/审阅 |
| F13 批注-标注目标 | 28 | 0 | 0 | 28 | 0 | 0 | 批注区域管理 |
| F14 AI辅助 | 28 | 0 | 0 | 0 | 28 | 0 | AI建议/差异预览 |
| F15 字体排版 | 199 | 199 | 0 | 0 | 0 | 0 | 字体解析/匹配/布局 |
| F16 PDF读写内核 | 21 | 21 | 0 | 0 | 0 | 0 | PDF解析/文本提取 |
| F17 布局分析 | 17 | 17 | 0 | 0 | 0 | 0 | 段落/列表/语义 |
| F18 坐标几何 | 65 | 65 | 0 | 0 | 0 | 0 | 坐标变换/投影 |
| F19 BBox视口裁剪 | 60 | 60 | 0 | 0 | 0 | 0 | 包围盒/视口计算 |
| F20 历史撤销重做 | 36 | 36 | 0 | 0 | 0 | 0 | 撤销/重做/命令 |
| F21 状态会话管理 | 58 | 0 | 58 | 0 | 0 | 0 | 应用状态/会话 |
| F22 WASM导出 | 29 | 0 | 29 | 0 | 0 | 0 | WASM绑定 |
| F23 DOM与UI | 28 | 0 | 0 | 0 | 28 | 0 | UI操作/事件 |
| F24 事件输入 | 8 | 0 | 0 | 0 | 8 | 0 | 键盘/鼠标/滚轮 |
| F25 调试诊断 | 33 | 33 | 0 | 0 | 0 | 0 | 日志/追踪 |
| F26 插件基础设施 | 9 | 0 | 0 | 0 | 9 | 0 | 插件加载/事件总线 |
| F27 算法基础 | 26 | 26 | 0 | 0 | 0 | 0 | 图算法/LCA |
| F28 工具辅助 | 26 | 0 | 0 | 0 | 26 | 0 | 字符串/数值处理 |
| **孤儿方法** | **633** | **177** | **0** | **0** | **405** | **51** | **需确认归属** |

---

## 孤儿方法清单（不属于任何明确功能）

以下 633 个方法无法被归类到上述 28 个功能域中，需要逐一确认其功能归属或删除：

### Kernel 层 (177 个)

**crates\pdf-viewer-core\src\algorithms\graph.rs**
- `add_edge` - 图算法：添加边
- `are_neighbors` - 图算法：检查相邻
- `build_adjacency` - 图算法：构建邻接表
- `find_connected_component` - 图算法：找连通分量
- `find_path` - 图算法：找路径

**crates\pdf-viewer-core\src\algorithms\lca.rs**
- `add_child` - LCA算法：添加子节点
- `find_lca` - LCA算法：找最近公共祖先

**crates\pdf-viewer-core\src\analysis\analyzer.rs**
- `create_semantic_region` - 语义分析：创建语义区域
- `detect_layout_pattern` - 布局检测
- `resolve_regions` - 区域解析

**crates\pdf-viewer-core\src\document\list_item_region_builder.rs**
- `chars_count` - 字符计数
- `resolve_body_left` - 解析左边距

**crates\pdf-viewer-core\src\document\page_region_context.rs**
- `chars_count` - 字符计数
- `get_object_display_text` - 获取显示文本
- `get_run_visible_glyph_width` - 获取字形宽度
- `split_key_value_text` - 分割键值文本

**crates\pdf-viewer-core\src\geometry\coordinate_transform.rs**
- `point` - 点构造
- `x_from_pdf` - PDF坐标转换

**crates\pdf-viewer-core\src\geometry\layout_engine.rs**
- `finish_line` - 完成行布局
- `is_forced_line_break_run` - 判断强制换行
- `is_no_end` - 判断无结束
- `is_no_start` - 判断无开始
- `layout_paragraph` - 段落布局
- `mock_run` - 模拟运行
- `test_cjk_no_start_rule` - 测试CJK规则
- `test_justified_alignment` - 测试对齐

**crates\pdf-viewer-core\src\lib.rs**
- `get_core_version` - 获取核心版本

**crates\pdf-viewer-core\src\models.rs**
- `default_horizontal_scaling` - 默认水平缩放
- `default_scale` - 默认缩放
- `default_scale_x` - 默认X缩放
- `flip_y` - Y轴翻转
- `from_styled` - 从样式创建

**crates\pdf-viewer-core\src\persistence\engine.rs**
- `get_reflow_key` - 获取重排键

**crates\pdf-viewer-core\src\persistence\history_manager.rs**
- `clear` - 清除历史
- `push` - 推入历史

**crates\pdf-viewer-core\src\persistence\models.rs**
- `default_scale_x_model` - 默认X缩放模型

**crates\pdf-viewer-core\src\persistence\state_manager.rs**
- `should_prefetch_page` - 是否预取页面

**crates\pdf-viewer-core\src\render\paint_plan.rs**
- `build_control_style` - 构建控制样式
- `build_field_editor_params` - 构建字段编辑参数
- `is_decorative_text` - 判断装饰文本

**crates\pdf-viewer-core\src\render\renderer.rs**
- `clear` - 清除渲染器
- `name` - 获取名称

**crates\pdf-viewer-core\src\render\snapshot_paint_plan.rs**
- `resolve_run_layout` - 解析运行布局

**crates\pdf-viewer-core\src\text\editable_segments.rs**
- `build_contiguous_segments_in_range` - 构建连续段
- `build_field_groups` - 构建字段组
- `create_editable_segment` - 创建可编辑段
- `detect_field_label_anchors` - 检测字段标签锚点
- `get_run_style_signature` - 获取运行样式签名
- `get_run_visible_glyph_width` - 获取字形宽度
- `get_segment_patch_key` - 获取段补丁键
- `is_colon_token` - 判断冒号标记
- `looks_like_short_field_token` - 判断短字段标记

**crates\pdf-viewer-core\src\text\glyph_layout.rs**
- `compute_run_aware_caret_left` - 计算光标左侧
- `estimated_gap_source_advance` - 估计间隙推进
- `extract_decorative_prefix` - 提取装饰前缀
- `glyph_left` - 字形左侧
- `glyph_right` - 字形右侧
- `glyph_visual_width` - 字形视觉宽度
- `has_suspicious_run_geometry` - 检测可疑几何
- `infer_run_advance` - 推断运行推进
- `is_ascii_word_start` - 判断ASCII单词开始
- `is_cjk_unified` - 判断CJK统一
- `measure_glyph_run` - 测量字形运行
- `run_aware_caret_left` - 运行感知光标左侧
- `run_aware_caret_right` - 运行感知光标右侧
- `should_insert_gap` - 是否插入间隙
- `visual_width` - 视觉宽度

**crates\pdf-viewer-core\src\text\list_semantics.rs**
- `extract_numbering_prefix` - 提取编号前缀
- `format_numbering_marker` - 格式化编号标记
- `is_decorative_glyph` - 判断装饰字形
- `parse_numbering_value` - 解析编号值

**crates\pdf-viewer-core\src\text\semantic_axiom.rs**
- `infer_role` - 推断角色

**crates\pdf-viewer-core\src\text\style_preservation.rs**
- `is_decorative_run_text` - 判断装饰运行文本
- `line_selection_range` - 行选择范围
- `make_style_run` - 创建样式运行

**crates\pdf-viewer-core\src\typography\font_resolver.rs**
- `classify_symbol_family` - 分类符号字体族
- `split_family_and_style` - 分离字体族和样式
- `strip_subset_prefix` - 去除子集前缀

**crates\pdf-viewer-core\src\typography\matcher.rs**
- `build_match_request` - 构建匹配请求
- `build_match_request_with_descriptor` - 带描述符构建匹配
- `choose_best_match` - 选择最佳匹配
- `choose_top_matches` - 选择顶级匹配
- `descriptor_postscript_match_boosts_candidate` - PostScript匹配提升
- `exact_family_match_beats_unrelated_candidate` - 精确族匹配胜出
- `extract_style_name` - 提取样式名
- `push_reason` - 推入原因
- `score_system_font_candidate` - 系统字体评分
- `split_family_name` - 分离字体族名

### Frontend 层 (405 个)

**src\bridge\diagnostics.ts**
- `createDiagnosticsContainer` - 创建诊断容器
- `createDiagnosticItem` - 创建诊断项
- `createDiagnosticList` - 创建诊断列表
- `createDiagnosticPanel` - 创建诊断面板
- `formatDiagnosticMessage` - 格式化诊断消息
- `formatDiagnosticTime` - 格式化诊断时间
- `getDiagnosticIcon` - 获取诊断图标
- `getDiagnosticLevel` - 获取诊断级别
- `showDiagnosticPanel` - 显示诊断面板
- `updateDiagnosticPanel` - 更新诊断面板

**src\bridge\document_edit_api.ts**
- `buildRenderRequest` - 构建渲染请求
- `clearPersistablePatches` - 清除持久化补丁
- `commitEdits` - 提交编辑
- `createDocumentEditApi` - 创建文档编辑API
- `getCurrentPath` - 获取当前路径
- `getCurrentZoom` - 获取当前缩放
- `invalidateRenderCache` - 无效化渲染缓存
- `saveEdits` - 保存编辑
- `syncViewerState` - 同步查看器状态

**src\bridge\editor_host.ts**
- `activateEditorFromPoint` - 从点激活编辑器
- `applyFormatAction` - 应用格式化动作
- `buildCaret` - 构建光标
- `clear` - 清除
- `closeActiveEditor` - 关闭活动编辑器
- `commitActiveEditor` - 提交活动编辑器
- `createEditorHost` - 创建编辑器主机
- `deleteForward` - 向前删除
- `handleActiveEditorInput` - 处理活动编辑器输入
- `hasPendingEdits` - 是否有待定编辑
- `isTextEditEnabled` - 是否启用文本编辑
- `moveCaretTo` - 移动光标到
- `openRegionEditor` - 打开区域编辑器
- `saveEdits` - 保存编辑
- `setTextEditEnabled` - 设置文本编辑启用
- `syncTargets` - 同步目标

**src\bridge\editor_host_diagnostics.ts**
- `createEditorHostDiagnostics` - 创建编辑器诊断
- `readDiagnostics` - 读取诊断
- `updateDiagnostics` - 更新诊断

**src\bridge\editor_host_view.ts**
- `ensureEditorHostView` - 确保编辑器视图
- `hideEditorShell` - 隐藏编辑器外壳
- `hideInteractionTargets` - 隐藏交互目标
- `positionEditorShell` - 定位编辑器外壳
- `readHostReferenceBox` - 读取主机参考框
- `renderInteractionTargets` - 渲染交互目标
- `snapshotHostOverlays` - 快照主机覆盖
- `suspendHostOverlays` - 暂停主机覆盖

**src\bridge\frame_plan.ts**
- `createFramePlanAdapter` - 创建帧计划适配器
- `buildRenderRequest` - 构建渲染请求
- `getCurrentPageHeight` - 获取当前页面高度
- `getCurrentPageWidth` - 获取当前页面宽度
- `getDynamicMaxZoom` - 获取动态最大缩放
- `getMaxCanvasDim` - 获取最大画布尺寸
- `getScrollContainer` - 获取滚动容器
- `getWasmApi` - 获取WASM API

**src\bridge\layout_trace.ts**
- `logPdfLayoutTrace` - 记录PDF布局追踪
- `logPdfRenderTrace` - 记录PDF渲染追踪
- `logPdfZoomTrace` - 记录PDF缩放追踪

**src\bridge\pdf_annotation_controller.ts**
- `createPdfAnnotationController` - 创建PDF批注控制器
- `refresh` - 刷新
- `updateAnnotation` - 更新批注

**src\bridge\pdf_comment_contracts.ts**
- `CommentScope` - 评论范围
- `CommentStatus` - 评论状态
- `CommentType` - 评论类型
- `PdfComment` - PDF评论
- `PdfCommentCreateRequest` - PDF评论创建请求
- `PdfCommentUpdateRequest` - PDF评论更新请求

**src\bridge\pdf_comment_controller.ts**
- `addComment` - 添加评论
- `clearCommentSession` - 清除评论会话
- `createPdfCommentController` - 创建PDF评论控制器
- `deleteComment` - 删除评论
- `getCommentReview` - 获取评论审阅
- `loadComments` - 加载评论
- `refresh` - 刷新
- `replaceComment` - 替换评论
- `setCommentReview` - 设置评论审阅
- `updateComment` - 更新评论

**src\bridge\pdf_comment_dom.ts**
- `buildCommentOverlay` - 构建评论覆盖
- `buildCommentOverlayView` - 构建评论覆盖视图
- `buildCommentReviewPanel` - 构建评论审阅面板
- `buildCommentReviewPanelView` - 构建评论审阅面板视图
- `clearCommentOverlay` - 清除评论覆盖
- `clearCommentReviewPanel` - 清除评论审阅面板
- `renderComment` - 渲染评论
- `renderCommentReview` - 渲染评论审阅
- `showCommentOverlay` - 显示评论覆盖
- `showCommentReviewPanel` - 显示评论审阅面板
- `updateCommentOverlay` - 更新评论覆盖
- `updateCommentReviewPanel` - 更新评论审阅面板

**src\bridge\pdf_comment_host_actions.ts**
- `buildHostAction` - 构建主机动作
- `buildHostActionList` - 构建主机动作列表
- `executeHostAction` - 执行主机动作

**src\bridge\pdf_comment_overlay_view.ts**
- `buildCommentOverlay` - 构建评论覆盖
- `buildCommentOverlayView` - 构建评论覆盖视图
- `clearCommentOverlay` - 清除评论覆盖
- `renderComment` - 渲染评论
- `showCommentOverlay` - 显示评论覆盖
- `updateCommentOverlay` - 更新评论覆盖

**src\bridge\pdf_comment_review_view.ts**
- `buildCommentReviewPanel` - 构建评论审阅面板
- `buildCommentReviewPanelView` - 构建评论审阅面板视图
- `clearCommentReviewPanel` - 清除评论审阅面板
- `renderCommentReview` - 渲染评论审阅
- `showCommentReviewPanel` - 显示评论审阅面板
- `updateCommentReviewPanel` - 更新评论审阅面板

**src\bridge\pdf_comment_wasm_bridge.ts**
- `buildCommentFromWasm` - 从WASM构建评论
- `buildWasmCommentCreateRequest` - 构建WASM评论创建请求
- `buildWasmCommentUpdateRequest` - 构建WASM评论更新请求

**src\bridge\pdf_find_controller.ts**
- `clearFindSession` - 清除查找会话
- `createPdfFindController` - 创建PDF查找控制器
- `findNext` - 查找下一个
- `findPrevious` - 查找上一个
- `getFindSession` - 获取查找会话
- `goToPage` - 跳转到页
- `moveMatch` - 移动匹配
- `openRegionEditor` - 打开区域编辑器
- `setFindSession` - 设置查找会话
- `updateFindScope` - 更新查找范围

**src\bridge\pdf_runtime.ts**
- `bindTileRefreshOnScroll` - 绑定滚动刷新瓦片
- `bindWheelZoom` - 绑定滚轮缩放
- `clampZoom` - 限制缩放
- `createPdfViewerRuntime` - 创建PDF查看器运行时
- `defaultPageHeight` - 默认页面高度
- `defaultPageWidth` - 默认页面宽度
- `ensureWasmInitialized` - 确保WASM初始化
- `getWasmApi` - 获取WASM API
- `handlePdfViewerKeydown` - 处理PDF查看器按键
- `openTextPdfFlow` - 打开文本PDF流程
- `readTargetZoom` - 读取目标缩放
- `renderCurrentPage` - 渲染当前页面
- `resetPdfViewerState` - 重置PDF查看器状态
- `syncTextEditButton` - 同步文本编辑按钮
- `syncZoomSelect` - 同步缩放选择
- `viewerSession` - 查看器会话

**src\bridge\pdf_window_api.ts**
- `bindTextEditToolbarButton` - 绑定文本编辑工具栏按钮
- `buildRenderRequest` - 构建渲染请求
- `createMessageBubble` - 创建消息气泡
- `createSuggestionCard` - 创建建议卡片
- `executePdfCommands` - 执行PDF命令
- `getNodes` - 获取节点
- `handlePdfViewerKeydown` - 处理PDF查看器按键
- `initialize` - 初始化
- `onCancelV3` - 取消V3
- `onCloseV3` - 关闭V3
- `onCommitV3` - 提交V3
- `onDebugV3` - 调试V3
- `onInputV3` - 输入V3
- `onOpenV3` - 打开V3
- `openPdfFile` - 打开PDF文件
- `registerPdfWindowApi` - 注册PDF窗口API
- `renderCurrentPage` - 渲染当前页面
- `resetPdfViewerState` - 重置PDF查看器状态
- `syncTextEditButton` - 同步文本编辑按钮
- `syncZoomSelect` - 同步缩放选择
- `toggle` - 切换
- `undoSavedEdit` - 撤销保存的编辑
- `waitForAnimation` - 等待动画
- `watchTextEditToolbarButton` - 监视文本编辑工具栏按钮

**src\bridge\render_flow.ts**
- `createRenderFlow` - 创建渲染流程
- `renderScheduledFrame` - 渲染计划帧

**src\bridge\vector_canvas_host.ts**
- `clearVectorHost` - 清除矢量主机
- `createVectorCanvasHost` - 创建矢量画布主机
- `ensureCanvasSize` - 确保画布尺寸
- `getCanvas` - 获取画布
- `getCanvasContext` - 获取画布上下文
- `getVectorContainer` - 获取矢量容器
- `resizeCanvas` - 调整画布大小
- `syncCanvasSize` - 同步画布大小

**src\bridge\frame_cache.ts**
- `clearFrameCache` - 清除帧缓存
- `createFrameCache` - 创建帧缓存
- `findTile` - 查找瓦片
- `invalidateVectorRenderCache` - 无效化矢量渲染缓存
- `rememberTile` - 记住瓦片
- `resetFrameCache` - 重置帧缓存
- `storeFrameCacheEntry` - 存储帧缓存条目
- `touchFrameCacheEntry` - 触摸帧缓存条目

**src\bridge\vector_host.ts**
- `createVectorHost` - 创建矢量主机
- `getVectorContainer` - 获取矢量容器
- `hideVectorHost` - 隐藏矢量主机
- `showVectorHost` - 显示矢量主机

**src\bridge\page_bundle.ts**
- `createPageBundle` - 创建页面束
- `getPageBundle` - 获取页面束
- `releasePageBundle` - 释放页面束

**src\bridge\geometry_probe.ts**
- `createViewerGeometryProbe` - 创建查看器几何探测器
- `measureDomToPageScale` - 测量DOM到页面缩放
- `resolveClientPointToPagePoint` - 解析客户端点到页面点
- `resolveProjection` - 解析投影

**src\bridge\viewer_session.ts**
- `createViewerSession` - 创建查看器会话
- `read` - 读取
- `reset` - 重置
- `setCurrentPage` - 设置当前页面
- `setDocument` - 设置文档
- `setPageDimensions` - 设置页面尺寸

**src\bridge\wasm_loader.ts**
- `loadWasm` - 加载WASM
- `loadWasmWithProgress` - 带进度加载WASM

**src\bridge\zoom_controller.ts**
- `createZoomController` - 创建缩放控制器
- `readZoomState` - 读取缩放状态
- `sanitizeZoomState` - 清理缩放状态
- `setTargetZoom` - 设置目标缩放

**src\bridge\resume_ai_controller.ts**
- `applySuggestion` - 应用建议
- `buildDiff` - 构建差异
- `clearAiSession` - 清除AI会话
- `createResumeAiController` - 创建恢复AI控制器
- `describeError` - 描述错误
- `markAiChanges` - 标记AI更改
- `planAiEdit` - 计划AI编辑
- `submitPrompt` - 提交提示
- `syncViewerState` - 同步查看器状态
- `tokenizeDiff` - 标记化差异
- `updateSuggestion` - 更新建议

**src\core\event-bus.ts**
- `emit` - 发出事件
- `off` - 取消订阅
- `on` - 订阅事件
- `once` - 一次性订阅

**src\core\interfaces.ts**
- `injectComponent` - 注入组件
- `injectContent` - 注入内容
- `loadComponent` - 加载组件
- `replaceComponent` - 替换组件
- `replaceComponentContent` - 替换组件内容

**src\core\plugin-loader.ts**
- `getAll` - 获取所有
- `getDiagnostics` - 获取诊断
- `getLoaded` - 获取已加载
- `getMetadata` - 获取元数据
- `isLoaded` - 是否已加载
- `load` - 加载
- `register` - 注册
- `unload` - 卸载

**src\core\router.ts**
- `getCurrentRoute` - 获取当前路由
- `goBack` - 返回
- `navigateTo` - 导航到
- `registerRoute` - 注册路由

**src\core\template-loader.ts**
- `loadComponent` - 加载组件
- `renderTemplate` - 渲染模板

**src\core\types.ts**
- `CommandResult` - 命令结果
- `Process` - 进程
- `FileInfo` - 文件信息
- `LogEntry` - 日志条目
- `CommandHistory` - 命令历史
- `AppSettings` - 应用设置
- `FeatureModule` - 功能模块
- `PluginCapabilities` - 插件能力
- `PluginContext` - 插件上下文
- `Plugin` - 插件
- `PluginMetadata` - 插件元数据

**src\core\utils.ts**
- `colorToCss` - 颜色转CSS
- `escape` - 转义
- `hexToRgb` - 十六进制转RGB
- `objectToCompactString` - 对象转紧凑字符串
- `objectToCompactValue` - 对象转紧凑值
- `objectToTerminalString` - 对象转终端字符串
- `parseColor` - 解析颜色
- `rgbToHex` - RGB转十六进制
- `stringifyTerminalValue` - 字符串化终端值

**src\core\algorithm-manager.ts**
- `getNamespaced` - 获取命名空间
- `getRegistry` - 获取注册表
- `registerAlgorithm` - 注册算法
- `resolveOptions` - 解析选项

**src\core\platform.ts**
- `getPlatform` - 获取平台
- `isTauri` - 是否Tauri
- `getTauri` - 获取Tauri

**src\core\window-manager.ts**
- `getCurrentWindow` - 获取当前窗口
- `registerWindowAction` - 注册窗口动作
- `unregisterWindowAction` - 注销窗口动作

**src\main.ts**
- `createDemoPdf` - 创建演示PDF
- `initializePdfViewer` - 初始化PDF查看器
- `setupKeyboardShortcuts` - 设置键盘快捷键
- `setupPdfViewer` - 设置PDF查看器

**src\bridge\index.ts**
- `initialize` - 初始化
- `destroy` - 销毁

### Utils 层 (51 个)

**utils\pdf-utils.ts**
- `calculatePdfPageCount` - 计算PDF页数
- `extractPdfText` - 提取PDF文本
- `getPdfMetadata` - 获取PDF元数据
- `isPdfFile` - 是否PDF文件
- `parsePdfPage` - 解析PDF页面

**utils\file-utils.ts**
- `getFileExtension` - 获取文件扩展名
- `getFileSize` - 获取文件大小
- `isImageFile` - 是否图片文件
- `isTextFile` - 是否文本文件
- `readFileAsText` - 读取文件为文本

**utils\string-utils.ts**
- `capitalize` - 首字母大写
- `truncate` - 截断
- `escapeHtml` - 转义HTML
- `unescapeHtml` - 反转义HTML
- `formatBytes` - 格式化字节

**utils\date-utils.ts**
- `formatDate` - 格式化日期
- `formatDateTime` - 格式化日期时间
- `getCurrentTime` - 获取当前时间
- `parseDate` - 解析日期
- `timeAgo` - 时间前

**utils\array-utils.ts**
- `chunk` - 分块
- `flatten` - 扁平化
- `unique` - 去重
- `sortBy` - 排序
- `groupBy` - 分组

**utils\math-utils.ts**
- `clamp` - 限制
- `lerp` - 线性插值
- `randomBetween` - 随机数
- `roundTo` - 四舍五入
- `toRadians` - 转弧度

---

## 需要用户确认的孤儿方法（关键问题）

### Kernel 层关键问题

1. **`chars_count`** (多处出现) - 字符计数功能，请问：
   - 是用于文本编辑的字数统计？
   - 还是用于布局分析的字符量计算？
   - 是否可以统一到一个工具模块？

2. **`test_*` 方法** (如 `test_cjk_no_start_rule`) - 明显是测试方法：
   - 是否应该移至 `tests/` 目录？
   - 还是这些是生产代码中的断言函数？

3. **`mock_run`** - 模拟运行：
   - 是用于调试的模拟？
   - 还是用于渲染的占位符？

4. **`get_core_version`** - 获取核心版本：
   - 是否需要保留？
   - 还是仅用于调试？

### Frontend 层关键问题

1. **大量UI组件方法** (如 `createDiagnosticPanel`, `buildCommentOverlay`)：
   - 这些是UI构建函数，是否属于"UI组件"功能域？
   - 还是应该归类到"DOM与UI"功能域？

2. **诊断相关方法** (`*diagnostics*`)：
   - 是否应该新增"诊断面板"功能域？
   - 还是归入"调试诊断"？

3. **消息气泡/建议卡片** (`createMessageBubble`, `createSuggestionCard`)：
   - 是否属于"AI辅助"的UI部分？
   - 还是独立的"通知系统"？

4. **WASM加载器** (`loadWasm`, `loadWasmWithProgress`)：
   - 是否属于"基础设施"？
   - 还是"插件系统"的一部分？

### Utils 层关键问题

1. **通用工具函数** (如 `clamp`, `truncate`, `capitalize`)：
   - 是否需要保留？
   - 还是应该使用外部库（如 lodash）？

2. **PDF工具** (`calculatePdfPageCount`, `extractPdfText`)：
   - 这些与内核的PDF功能重复，是否需要删除？
   - 还是用于前端快速访问？

---

## 重构建议

### 1. 功能域重新划分建议

基于孤儿方法分析，建议新增以下功能域：

- **F29 UI组件系统** - 处理所有UI组件创建/渲染
- **F30 诊断面板** - 处理诊断信息的UI展示
- **F31 通知系统** - 处理消息气泡/建议卡片/通知
- **F32 WASM基础设施** - 处理WASM加载/初始化
- **F33 通用工具库** - 处理字符串/数组/数学等工具函数

### 2. 方法删除建议

- **测试方法** - 所有 `test_*` 方法移至 `tests/`
- **重复工具** - Utils层与Kernel层重复的PDF工具函数
- **调试方法** - 仅用于调试的方法（如 `mock_run`）可标记为内部API

### 3. 方法合并建议

- **字符计数** - 多处 `chars_count` 合并为 `utils/text-utils.ts`
- **组件构建** - `build*` 和 `create*` 组件方法合并到UI组件系统
- **诊断相关** - 所有诊断方法合并到诊断面板功能域

请确认以上问题，以便完成最终的重构方案。
