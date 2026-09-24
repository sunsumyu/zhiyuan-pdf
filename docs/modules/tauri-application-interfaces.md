# src-tauri · 应用层与命令面（application / interfaces / 根文件）

> 范围：`src-tauri/src/` 根文件（`main.rs`、`lib.rs`、`app_state.rs`、`state.rs`、`error.rs`）+ `application/`（mod.rs 与 `pdf/` 7 文件）+ `interfaces/`（mod.rs 与 `pdf/` 9 文件），共 23 个文件，约 2640 行。
> 上游：`src-tauri/src/infrastructure/`（document_service / document_resolver / save_engine / commands / page_intermediate_service / preview_engine / annotation_store / region_materializer / log_service / cache / models、`pdf_fallback/`）；`crates/pdf-viewer-core`（annotation 请求/结果类型、`persistence::models::PersistableRegionPatch`、`document::page_region_context`、NativePageModel / NativeTextModel）。
> 下游：前端经 Tauri `invoke` 调用（桥接层见 `src/bridge/`，命令面总表见 `docs/guide/architecture-map.md` §2.1，此处不重复复制）；`lib.rs` 的 `invoke_handler!` 注册表；`pdfasset://` 自定义协议向 WebView 提供内存图像。

## 职责
`lib.rs` 是 Tauri 启动入口：装配插件、注册 `pdfasset://` 内存图像协议、注入 `AppState` 并登记 30 个 IPC 命令；`app_state.rs` / `state.rs` / `error.rs` 提供按域分组的共享状态与（预留的）类型化错误。`interfaces/` 是命令面——薄封装，只做日志事件跨度与参数转发；`application/` 是应用服务层——持有事务历史、磁盘持久化、缓存失效、页资产准入（防并发重复与过期请求）以及批注 / 评论 / 搜索的业务编排。

## IPC 命令面
全部注册于 `lib.rs` `generate_handler!`（共 30 个，逐项核对与 `docs/guide/architecture-map.md` §2.1 一致），经 `interfaces/pdf/mod.rs` glob 再导出为扁平的 `crate::interfaces::pdf::*`。

**文档生命周期（document.rs + system.rs 的 pick_file）：**

| 命令 | 文件 | 功能 |
|---|---|---|
| `open_pdf` | `interfaces/pdf/document.rs` | 按路径加载 PDF 入缓存，返回页数 |
| `save_pdf` | `interfaces/pdf/document.rs` | 应用 region patches / text reflows 并写盘 |
| `undo` | `interfaces/pdf/document.rs` | 回滚到上一文档快照 |
| `redo` | `interfaces/pdf/document.rs` | 重做被撤销的快照 |
| `clear_cache` | `interfaces/pdf/document.rs` | 释放全部文档资源并清预览缓存 |
| `pick_file` | `interfaces/pdf/system.rs` | 原生对话框选择 PDF 文件 |

**页面数据（page.rs + render.rs）：**

| 命令 | 文件 | 功能 |
|---|---|---|
| `read_preview` | `interfaces/pdf/page.rs` | 轻量页预览：尺寸、扫描/矢量类型、光栅图 URL |
| `read_page_asset_bundle` | `interfaces/pdf/render.rs` | 矢量页模型 + 字形绘制计划组合包 |
| `read_vector` | `interfaces/pdf/render.rs` | 完整矢量页模型（可只留图或只留文） |
| `read_glyph_plan` | `interfaces/pdf/render.rs` | 文本渲染用的字形绘制计划 |
| `read_images` | `interfaces/pdf/render.rs` | 内存图像缓存快照为 base64 data URL |
| `diagnose_page` | `interfaces/pdf/render.rs` | 调试 JSON：页字典、内容流算符、解析结果 |

**编辑（replace.rs）：**

| 命令 | 文件 | 功能 |
|---|---|---|
| `apply_region_patches` | `interfaces/pdf/replace.rs` | 对指定页应用区域文本替换补丁 |

**批注 / 评论（annotation.rs + comment.rs）：**

| 命令 | 文件 | 功能 |
|---|---|---|
| `read_annotation_targets` | `interfaces/pdf/annotation.rs` | 列出可批注 region 及包围盒 |
| `read_highlights` | `interfaces/pdf/annotation.rs` | 列出页内高亮 |
| `apply_highlight` | `interfaces/pdf/annotation.rs` | 对 region 写入高亮 |
| `delete_annotation` | `interfaces/pdf/annotation.rs` | 按 annotation id 删除批注 |
| `read_comments` | `interfaces/pdf/comment.rs` | 列出页内评论 |
| `read_comment_review` | `interfaces/pdf/comment.rs` | 全文档（或单页）评论汇总与过滤 |
| `apply_comment` | `interfaces/pdf/comment.rs` | 对 region 新增文本评论 |
| `apply_comment_update` | `interfaces/pdf/comment.rs` | 更新评论内容 |

**搜索（search.rs）：**

| 命令 | 文件 | 功能 |
|---|---|---|
| `find_in_page` | `interfaces/pdf/search.rs` | 单页 region 级文本搜索 |
| `find_in_document` | `interfaces/pdf/search.rs` | 逐页解析后全文档搜索 |

**系统（system.rs）：**

| 命令 | 文件 | 功能 |
|---|---|---|
| `create_demo_pdf` | `interfaces/pdf/system.rs` | 生成硬编码演示 PDF |
| `set_log_level` | `interfaces/pdf/system.rs` | 设置后端日志级别（u8） |
| `clear_pdf_event_log` | `interfaces/pdf/system.rs` | 清空事件环形日志 |
| `read_pdf_event_log` | `interfaces/pdf/system.rs` | 读取事件日志行 |
| `set_page_asset_test_delay_ms` | `interfaces/pdf/system.rs` | 调试：人为延迟页资产请求 |
| `terminal_log` | `interfaces/pdf/system.rs` | 前端日志直通终端打印 |
| `resolve_asset_url` | `interfaces/pdf/system.rs` | 磁盘路径转 asset.localhost URL |

## 文件与方法
（各文件内联 `#[cfg(test)] mod tests` 不列入；覆盖点见各行文件说明。）

### `main.rs` — 二进制入口：转发到库 crate 的 run()
| 可见性 | 方法 | 功能 |
|---|---|---|
| 私有 | `main()` | release 构建隐藏 Windows 控制台窗口，调用 `pdf_viewer_standalone::run()` |

### `lib.rs` — Tauri 启动装配与命令注册表
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod` ×6 | 声明 app_state / application / error / infrastructure / interfaces / state |
| pub | `pub use app_state::*`、`pub use error::*` | 再导出 AppState 与错误类型到 crate 根 |
| pub | `run()` | 构建 AppState；挂 dialog/fs/shell 插件；注册 `pdfasset://` 协议回调；debug 构建自动开 DevTools；`manage` 状态；`generate_handler!` 登记 30 个命令并进入事件循环 |
| 私有 | `pdfasset` 协议闭包 | 从 `PDF_IMAGE_CACHE` 取图像，按 JPEG 魔数嗅探 MIME 返回（未命中 404），带 CORS 头 |

### `app_state.rs` — 按域分组的根应用状态（Tauri `State` 注入点）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `DocumentStore` (struct) | 拥有的 PDF 文档：`pdf_documents`（路径→`Arc<lopdf::Document>`）与 `loading_docs`（加载状态） |
| 私有 | `DocumentStore::new()` | 两张空表初始化 |
| pub | `CacheStore` (struct) | 派生视图缓存六件套：页中间列表 / 矢量页模型 / 布局推断 / 页资产 in-flight 锁 / 页预览 / 物料化报告 |
| 私有 | `CacheStore::new()` | 六张空表初始化 |
| pub | `HISTORY_LIMIT` (const) | 每路径撤销快照上限（20） |
| pub | `HistoryStore` (struct) | 撤销栈 `pdf_transactions` 与重做栈 `pdf_redo_transactions`（路径→快照向量） |
| 私有 | `HistoryStore::new()` | 两张空表初始化 |
| pub | `AppState` (struct) | 根状态：docs / cache / history 三个子仓 + `active_pages`（路径→当前活动页号） |
| pub | `AppState::new()` | 全空初始化（`impl Default` 委托于此） |

### `state.rs` — 文档加载状态枚举
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `LoadingStatus` (enum) | Loading / Ready / Error(String)，由 infrastructure 的加载器写入 `loading_docs` |

### `error.rs` — 类型化错误枚举（迁移预留，见疑点）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PdfError` (enum) | 8 变体：DocumentNotFound / PageOutOfRange / AnnotationNotFound / LopdfError(`#[from]`) / IoError(`#[from]`) / JoinError(`#[from]`) / SaveFailed / Other |
| pub | `other(msg)` | Other 兜底变体构造器 |
| pub | `impl From<PdfError> for String` | 兼容桥：使 `?` 能从 `Result<_, String>` 命令面直接上抛 |
| pub | `PdfResult<T>` (type) | `Result<T, PdfError>` 别名 |

### `application/mod.rs` — 应用层模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod pdf` | 声明 pdf 子层 |

### `application/pdf/mod.rs` — pdf 应用服务模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(crate) | 六个子模块声明 | comment_review / edit_commands / page_annotation / page_asset / page_context / page_search（均不对外导出） |

### `application/pdf/edit_commands.rs` — 编辑命令编排：历史快照、克隆应用、写盘、缓存失效
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `ensure_document_loaded(app_state, path)` | 确保文档已在内存缓存（委托 document_resolver） |
| pub(crate) | `execute_region_patches(app_state, path, page_index, patches)` | 构建 region 物料化计划，非空 reflow 转 BatchTextReflowCommand 执行 |
| pub(crate) | `apply_highlight_annotation(app_state, path, page_index, rect, color)` | 构造 AddHighlightCommand 执行（页号 +1 转 1-based） |
| pub(crate) | `apply_text_comment(app_state, path, page_index, rect, color, contents)` | 构造 AddCommentCommand 执行 |
| pub(crate) | `delete_annotation_internal(app_state, path, page_index, annot_id)` | 构造 DeleteAnnotationCommand 执行 |
| pub(crate) | `update_text_comment(app_state, path, page_index, annot_id, contents)` | 构造 UpdateCommentCommand 执行 |
| 私有 | `execute_commands(app_state, path, page_index, commands)` | 四步编排：当前文档压撤销栈（超 20 弹最旧）并清重做栈 → 克隆文档应用命令 → 写盘 → 替换内存文档并按 `path::` 前缀失效三个视图缓存 |

### `application/pdf/comment_review.rs` — 全文档评论审阅汇总
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | 再导出 core 类型 | PdfCommentReviewPageSummary / Request / Result（IPC DTO 来源） |
| pub(crate) | `review_document_comments(app_state, path, request)` | 按指定页或全文档逐页取评论，按 query 小写包含过滤，产出总数 / 过滤数 / 有评论页数 / 页级摘要 / 过滤评论列表（内联测试覆盖 request 默认值） |

### `application/pdf/page_context.rs` — 矢量页模型 → core region 上下文适配
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `build_page_region_context_from_vector_model(page_model)` | 先转 NativePageModel 再调 core 的 `build_page_region_context` |
| pub(crate) | `native_page_from_vector_model(page_model)` | 仅保留 Text 对象组装 NativePageModel（丢弃路径与图像） |
| 私有 | `native_text_from_vector_text(text)` | infra 文本模型到 core 文本模型的克隆转换 |

### `application/pdf/page_annotation.rs` — 批注 / 评论业务编排（DTO 定义 + region 定位 + 增删改查）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | 再导出 core 批注类型 11 个 | PdfPageAnnotationBox / Target(Result) / PdfPageCommentItem / List / PdfRegionCommentRequest / Result / PdfUpdateCommentRequest / Result / PdfDeleteAnnotationRequest / Result |
| pub | `PdfPageHighlightItem` (struct) | 单条高亮 DTO：id、页号、页尺寸、颜色、包围盒 |
| pub | `PdfPageHighlightList` (struct) | 页高亮列表 DTO |
| pub | `PdfRegionHighlightRequest` (struct) | 高亮请求：页号、region_id、kind、颜色（默认淡黄） |
| pub | `PdfRegionHighlightResult` (struct) | 高亮结果：added / 页号 / region_id |
| 私有 | `default_highlight_color()` | 默认颜色 `[1.0, 0.92, 0.4]` |
| pub(crate) | `collect_page_annotation_targets(page_context, page_index, w, h)` | 把段落 / 列表项 / 字段行三类 region 汇总为可批注目标（标签截断 48 字符） |
| pub(crate) | `list_page_annotation_targets(app_state, path, page_index)` | 解析矢量页模型 → region 上下文 → 目标列表 |
| pub(crate) | `list_page_highlights(app_state, path, page_index)` | 确保加载后由 annotation_store 后台线程读页高亮并转 DTO |
| pub(crate) | `list_page_comments(app_state, path, page_index)` | 同上读取页评论并转 DTO |
| pub(crate) | `add_region_highlight(app_state, path, request)` | 解析 region 包围盒后经 edit_commands 写高亮 |
| pub(crate) | `add_region_comment(app_state, path, request)` | 校验内容非空、解析 region 框、写文本评论 |
| pub(crate) | `delete_page_annotation(app_state, path, request)` | 解析 annotation id 后删除批注 |
| pub(crate) | `update_page_comment(app_state, path, request)` | 校验非空、解析 id、更新评论内容 |
| 私有 | `resolve_region_box(page_context, region_id, kind)` | 按 kind（paragraph-region / list-item-region / field-row）查 region 框 |
| 私有 | `from_region_box(region_box)` | core BoundingBoxOutput → PdfPageAnnotationBox |
| 私有 | `summarize_label(text)` | 截取前 48 字符，超出补省略号 |
| 私有 | `parse_annotation_object_id(value)` | 解析 `"obj-gen"` 字符串为 `(u32, u16)` PDF 对象号 |

### `application/pdf/page_asset.rs` — 页资产准入服务：in-flight 去重与过期请求拒绝
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `PAGE_ASSET_TEST_DELAY_MS` (static AtomicU64) | 调试注入延迟毫秒数（仅 debug 构建生效） |
| pub(crate) | `PageAssetRole` (enum) | 请求角色：Current（前台） / Prefetch（预取）；`from_request` 按 `"prefetch"` 判别，`as_str` 供日志 |
| pub(crate) | `PageAssetKind` (enum) | 资产类型：Preview / PageBundle / VectorModel / GlyphPlan（私有 `as_str` 供日志） |
| pub(crate) | `PageAssetAdmissionService` (struct) | 准入服务（无字段，纯静态方法） |
| pub(crate) | `set_test_delay_ms(delay_ms)` | 设定测试延迟，上限 5000ms，release 下为空操作 |
| pub(crate) | `apply_test_delay()` | 按设定值异步 sleep |
| 私有 | `emit_event(level, event, fields)` | 写 PDF 事件日志 |
| 私有 | `lock_for(state, path, page_index, revision, kind)` | 以 `path::revN::page::kind` 为键取/建 tokio 互斥锁 |
| pub(crate) | `acquire_inflight_lock(...)` | try_lock 失败则等待并记录 `pageAsset.dedupeWait` 起止事件（同键请求串行去重） |
| pub(crate) | `admit_before_work(...)` | Current 直接登记活动页；Prefetch 先做窗口检查 |
| pub(crate) | `mark_current_page(...)` | 写 `active_pages` 并记录 `pageAsset.admit` 事件 |
| pub(crate) | `admit_after_wait(...)` | 拿到 in-flight 锁后复检，委托 admit_after_work |
| pub(crate) | `admit_after_work(...)` | Current 检查是否已被翻页取代；Prefetch 再查窗口 |
| 私有 | `admit_prefetch(...)` | 无活动页或超出窗口（Preview ±8 页、其余 ±2 页）即拒绝 |
| 私有 | `reject(page_index, role, kind, latest, reason)` | 记录拒绝事件并返回 `stale page asset request` 错误（内联测试覆盖去重等待 / 修订隔离 / 失效清锁 / 预取窗口） |

### `application/pdf/page_search.rs` — region 级文本搜索（DTO 定义 + 三类 region 匹配）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PdfPageSearchRequest` (struct) | 搜索请求：query 与 case_sensitive |
| pub | `PdfPageSearchBox` (struct) | 匹配框：left / top / width / height |
| pub | `PdfPageSearchMatch` (struct) | 单条匹配：id、kind、页号与尺寸、行号、原文、预览、命中文本、对象索引、包围盒 |
| pub | `PdfPageSearchResult` (struct) | 单页搜索结果（含 total_matches） |
| pub | `PdfDocumentSearchResult` (struct) | 全文档搜索结果 |
| pub(crate) | `search_page_regions(page_model, request)` | 单页搜索：匹配后按页号 / 行号 / top / left 排序 |
| pub(crate) | `search_document_regions(page_models, request)` | 多页合并匹配后统一排序 |
| 私有 | `search_page_matches(page_model, request)` | 构建 region 上下文并依次收集三类匹配 |
| 私有 | `collect_paragraph_matches(...)` | 段落 region 文本包含查询即产出匹配 |
| 私有 | `collect_list_item_matches(...)` | 列表项 region（优先 body_text）匹配 |
| 私有 | `collect_field_row_matches(...)` | 字段行组（键值拼接）匹配 |
| 私有 | `contains_query(text, query, case_sensitive)` | 大小写敏感 / 不敏感包含判断 |
| 私有 | `summarize_preview(text)` | 压缩空白后截断至 96 字符补省略号 |
| 私有 | `from_region_box(box_rect)` | core BoundingBoxOutput → PdfPageSearchBox |

### `interfaces/mod.rs` — 接口层模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod pdf` | 声明 pdf 命令面 |

### `interfaces/pdf/mod.rs` — 命令面模块清单与扁平再导出
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | 八个子模块声明 | annotation / comment / document / page / render / replace / search / system |
| pub | `pub use *` ×8 | glob 再导出，保持 `crate::interfaces::pdf::<命令>` 的扁平调用路径不变 |

### `interfaces/pdf/document.rs` — 文档生命周期命令（open / save / undo / redo / clear_cache）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `#[command] open_pdf(app_handle, state, path)` | 记 `document.open` 事件跨度（含路径 md5），委托 PdfDocumentService::open_pdf，返回页数 |
| pub | `#[command] clear_cache(state)` | 记事件；释放全部文档资源并清页预览缓存 |
| pub | `#[command] save_pdf(state, path, modifications)` | 记 `document.save` 事件（补丁 / reflow 计数），委托保存 |
| pub | `#[command] undo(state, path)` | 记事件后回滚到上一快照 |
| pub | `#[command] redo(state, path)` | 记事件后重做 |

### `interfaces/pdf/page.rs` — 页预览命令（lopdf 轻模型优先，扫描页回退）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `#[command] read_preview(state, path, page_index, request_role)` | 准入检查 → 查预览缓存 → 确保文档加载 → lopdf 0 页时回退 ScannedReadBackend，否则 build_light_page_model 转 PagePreview → 写缓存返回 |

### `interfaces/pdf/render.rs` — 渲染数据命令（矢量模型 / 字形计划 / 图像缓存 / 诊断）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PageAssetBundle` (struct) | 序列化组合包：`model`（NativeVectorPageModel）+ `paint_plan`（GlyphPaintPlan） |
| pub | `#[command] read_page_asset_bundle(...)` | 准入三段检查 + in-flight 锁 + 测试延迟后，委托 PdfPageIntermediateService 取模型与绘制计划 |
| pub | `#[command] read_vector(...)` | 同准入流程取矢量页模型，按 image_only / text_only 保留或剔除 Text 对象 |
| pub | `#[command] read_glyph_plan(...)` | 取字形绘制计划 |
| pub | `#[command] read_images(_path)` | 快照进程内图像缓存为 data URL 映射 |
| pub | `#[command] diagnose_page(state, path, page_index)` | 诊断 JSON：页字典键、Contents / Resources 类型、内容流字节数与首 30 算符、display list 对象 / 文本运行数 |

### `interfaces/pdf/replace.rs` — 区域文本替换命令
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `#[command] apply_region_patches(state, path, page_index, patches)` | 记 V3-SAVE-CMD 日志后委托 execute_region_patches |

### `interfaces/pdf/search.rs` — 全文搜索命令
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `#[command] find_in_page(state, path, page_index, query, case_sensitive)` | 解析矢量页模型后单页 region 搜索 |
| pub | `#[command] find_in_document(state, path, page_count, query, case_sensitive)` | 逐页解析模型后合并全文档搜索 |

### `interfaces/pdf/annotation.rs` — 批注命令（纯转发到 page_annotation）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `#[command] read_annotation_targets(state, path, page_index)` | 列出可批注 region 及包围盒 |
| pub | `#[command] read_highlights(state, path, page_index)` | 列出页内高亮 |
| pub | `#[command] apply_highlight(state, path, request)` | 对 region 写入高亮 |
| pub | `#[command] delete_annotation(state, path, request)` | 按 annotation id 删除批注 |

### `interfaces/pdf/comment.rs` — 评论命令（纯转发到 page_annotation / comment_review）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `#[command] read_comments(state, path, page_index)` | 列出页内评论 |
| pub | `#[command] read_comment_review(state, path, request)` | 全文档（或单页）评论汇总与过滤 |
| pub | `#[command] apply_comment(state, path, request)` | 对 region 新增文本评论 |
| pub | `#[command] apply_comment_update(state, path, request)` | 更新评论内容 |

### `interfaces/pdf/system.rs` — 系统级工具命令（演示 PDF / 日志 / 资产 URL / 文件选择）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `#[command] create_demo_pdf(path)` | 生成硬编码演示 PDF |
| pub | `#[command] set_log_level(level)` | 设置后端日志级别 |
| pub | `#[command] clear_pdf_event_log()` | 清空事件环形日志 |
| pub | `#[command] read_pdf_event_log()` | 读取事件日志行 |
| pub | `#[command] set_page_asset_test_delay_ms(delay_ms)` | 设定页资产测试延迟 |
| pub | `#[command] terminal_log(message)` | 前端消息直通终端打印 |
| pub | `#[command] resolve_asset_url(path)` | 反斜杠转正斜杠后编码为 `http://asset.localhost/` URL（走 Tauri 内置 asset 协议，tauri.conf.json 已配 assetProtocol） |
| pub | `#[command] pick_file(app_handle)` | 原生对话框选 PDF（过滤 `.pdf`），阻塞式返回路径 |

## 疑点
1. **命令数**：`lib.rs` `generate_handler!` 实际注册 30 个，与旧表 `docs/guide/architecture-map.md` §2.1（2026-08）逐项一致，无滞后；但旧表把 `pick_file` 归入"文档生命周期"、本文按其实际文件归入"系统"。
2. **error.rs 未接线**：`PdfError` / `PdfResult` 与 `From<PdfError> for String` 桥已定义并带测试，但 interfaces 与 application 两层现仍全部返回 `Result<_, String>`，范围内无任何调用点——类型化错误仍是迁移预留。
3. **双资产通道**：`resolve_asset_url` 生成 `http://asset.localhost/`（Tauri 内置 asset 协议，服务磁盘文件），而 `lib.rs` 注册的自定义协议是 `pdfasset://`（服务 `PDF_IMAGE_CACHE` 内存图像）；`read_preview` 产出的 `image_url` 用后者。两条通道并存，命名易混淆。
4. **`admit_after_wait` 与 `admit_after_work` 完全同义**（前者直接委托后者），语义上"等待后复检"与"完成后复检"未区分逻辑。
5. **`find_in_document` 逐页串行解析**：循环内逐页 `resolve_vector_page_model`，大文档全量搜索时无并行 / 进度回报，page_count 由前端传入而非后端校验。
6. **`app_state.active_pages` 仅单值**：每路径只记一个活动页号，同文档多窗口 / 双视图场景下 Prefetch 准入窗口会以最后登记页为准。
