# pdf-viewer-core · document / annotation / history / persistence（文档域与持久化）

> 范围：`crates/pdf-viewer-core/src/{document,annotation,history,persistence}/`（11 个文件，约 2430 行，不含各 `mod.rs`）。
> 上游：`crate::models`（NativePageModel / NativeTextModel / StyledRun / FontHints / EditableSegment / PaginationCommand 等）、`crate::text::list_semantics`（列表标记语义推断）、`crate::edit::active_target`（ActiveEditorTarget）、`crate::geometry::layout_engine`（ParagraphLayout）。
> 下游：core 内 `geometry/reflow_engine.rs`（读 GLOBAL_PATCH_STATE）、`edit/bridge.rs`、`edit/replacement_snapshot.rs`（PersistableRegionPatch）；`pdf-viewer-ui` 的 `ui_state_store.rs`（re-export `patch_store::*`）、`review_api.rs`（review_types）、`annotation_api.rs` / `document/comment.rs`（批注类型）、`projection_workflow.rs`（build_pagination_commands）。

## 职责
- `document/`：把解析后的页面物理模型（NativePageModel）降维为可穿越 WASM 边界的展平区域上下文（行、字段行、段落、列表项）及全部 Output DTO。
- `annotation/`：定义 PDF `/Annot` 批注子类型与评论审阅域的纯数据类型、请求/响应信封。
- `history/`：定义文档撤销历史域的错误、状态快照与响应 DTO。
- `persistence/`：以全局补丁状态容器（GLOBAL_PATCH_STATE）存取区域文本补丁，收集可持久化保存计划，并提供撤销/重做命令栈与审阅响应类型。

## 文件与方法

### `document/document_types.rs` — DocumentSession 域的对外错误与响应信封 DTO
| 条目 | 类型 | 含义 |
|---|---|---|
| `DocumentError` | enum | 文档域错误分类，serde tag = "type" |
| `DocumentError::InvalidInput` | variant | 输入反序列化失败或含非法值（附字段与原因） |
| `DocumentError::NotImplemented` | variant | 方法保留但尚未实现 |
| `DocumentError::IoError` | variant | 文档字节读写 IO 失败 |
| `DocumentError::Internal` | variant | 不可恢复的内部错误 |
| `DocumentResponse<T>` | struct | 统一响应信封：ok / data / error，camelCase |

### `document/list_item_region_builder.rs` — 从单个文本对象构建列表项区域输出
| 条目 | 类型 | 含义 |
|---|---|---|
| 私有 | `chars_count` | 按字符（char）统计文本长度 |
| 私有 | `split_runs_by_body_start` | 按正文起点把样式 run 拆成标记与正文两组，重编 id 与区间 |
| 私有 | `resolve_body_left` | 三级回退推算正文起始 X 坐标（字符原点→标记 run 尾→标记宽度累加） |
| pub(crate) | `build_list_item_region` | 组装 ListItemRegionOutput：几何换算、列表语义拆分、标记/正文 run、行投影 |

### `document/page_region_context.rs` — 页面区域上下文投影：物理态到纯渲染态的结构降维
| 条目 | 类型 | 含义 |
|---|---|---|
| 私有 | `read_object_display_text` | 有 runs 则拼接 run 文本，否则回退 obj.text |
| 私有 | `chars_count` | 按字符统计文本长度 |
| 私有 | `resolve_run_visible_glyph_width` | run 宽度缺失时按父对象均分或字号回退 |
| 私有 | `build_style_source` | 把字体/颜色/粗斜/间距/缩放打包成 StyleSource |
| 私有 | `build_style_runs_from_text_object` | 把文本对象展开为行内样式 run 快照，补齐字符原点/宽度回退值 |
| 私有 | `build_paragraph_line_from_text_object` | 单文本对象转段落行输出，附包围盒与行投影 |
| 私有 | `infer_scene_hint` | 按前 40 个对象关键词猜场景：resume / form-like / generic |
| 私有 | `is_standalone_paragraph_candidate` | 判定独立段落：大字号、标题角色或加粗短行 |
| 私有 | `should_merge_paragraph_objects` | 判定相邻对象能否并入同段（同 paragraph_id、左右/字号/行距阈值内） |
| 私有 | `build_paragraph_region_from_objects` | 一组对象按行序构建段落区域输出、联合包围盒与投影 |
| 私有 | `split_key_value_text` | 按全角/半角冒号把文本切成键与值 |
| pub | `build_page_region_context` | 主入口：排序文本对象，生成字段行/段落/列表项区域并回填行区域上下文 |

### `document/page_region_models.rs` — 页面区域输出 DTO 汇总（纯数据，全展平 camelCase）
| 条目 | 类型 | 含义 |
|---|---|---|
| `ParagraphRegionSnapshotLine` | struct | 段落快照单行：文本、几何、样式 run 与标记/正文拆分结果 |
| `ParagraphRegionSnapshot` | struct | 段落区域纯渲染快照（区域 id、kind、行列表、run） |
| `FieldGroupSnapshot` | struct | 字段组快照：键/值文本与 run、键值包围盒、对象索引 |
| `BoundingBoxOutput` | struct | 包围盒 left/top/width/height，统一 Y 向下坐标 |
| `ParagraphProjectionOutput` | struct | 段落投影：区域框 + 行框 + 紧行框 |
| `ParagraphLineProjectionOutput` | struct | 行投影：位置尺寸与基线 y |
| `FieldGroupProjectionOutput` | struct | 字段组五框投影：文本/外壳/标签/值/编辑器 |
| `StyleSource` | struct | 样式源：字体、颜色、粗斜下划线、字距、横向缩放（默认 100） |
| `default_scale_x_persistence` | fn（私有） | serde 默认函数：scale_x 缺省 100.0 |
| `StyleRunSnapshot` | struct | 行内样式 run：文本区间、样式、宽度、字符原点/宽度、对象索引 |
| `ParagraphLineOutput` | struct | 段落行完整输出：文本、几何、样式 run、字符度量、投影 |
| `ParagraphRegionOutput` | struct | 段落区域输出：行区间、联合框、全文、字符度量、投影 |
| `ListItemRegionOutput` | struct | 列表项区域输出：标记/正文拆分字段（body_char_start、body_left 等） |
| `KeyBox` | struct | 键值对单侧框（left/right/top/bottom，PDF 坐标） |
| `KeyValuePairOutput` | struct | 键值对输出：键值文本/样式/框、run 键、对象索引 |
| `FieldRowRegionGroupOutput` | struct | 字段行内分组：槽位/标签/值边界、键值对、可编辑段、投影 |
| `FieldRowRegionOutput` | struct | 字段行区域：置信度、语义来源、列带、分组列表 |
| `LineProjectionOutput` | struct | 行投影（位置尺寸，无基线） |
| `LineRegionModelOutput` | struct | 行区域模型：对象列表 + 可选字段行/段落/列表区域挂载 |
| `PageRegionContextOutput` | struct | 页面上下文总输出：场景提示、文本对象、行/字段/段落/列表区域 |

### `annotation/annotation_types.rs` — PDF 批注领域类型与评论审阅视图 DTO
| 条目 | 类型 | 含义 |
|---|---|---|
| `AnnotationKind` | enum | PDF 1.7 §12.5.6 批注子类型子集，未知归 Other |
| `AnnotationKind::TextNote` | variant | 便签（/Text） |
| `AnnotationKind::Highlight` | variant | 高亮（/Highlight） |
| `AnnotationKind::Underline` | variant | 下划线（/Underline） |
| `AnnotationKind::Squiggly` | variant | 波浪线（/Squiggly） |
| `AnnotationKind::Strikeout` | variant | 删除线（/StrikeOut） |
| `AnnotationKind::Ink` | variant | 手绘墨迹（/Ink） |
| `AnnotationKind::Stamp` | variant | 图章（/Stamp） |
| `AnnotationKind::Link` | variant | 超链接（/Link） |
| `AnnotationKind::Signature` | variant | 数字签名占位（/Widget + /FT /Sig） |
| `AnnotationKind::FreeText` | variant | 自由文本标签（/FreeText） |
| `AnnotationKind::Other` | variant | 其他/未支持子类型 |
| `Annotation` | struct | PDF /Annot 对象 UI 视图：id、页码、类型、包围盒、内容/作者/时间戳 |
| `AnnotationBBox` | struct | 批注包围盒（PDF 用户空间，原点左下） |
| `AnnotationError` | enum | 批注域错误：非法输入/未找到/未实现/IO/内部 |
| `AnnotationResponse<T>` | struct | 批注域统一响应信封 |
| `CommentBoxRect` | struct | 评论框矩形（页坐标 left/top/width/height） |
| `PdfPageAnnotationBox` | type alias | CommentBoxRect 的别名 |
| `CommentPercentFrame` | struct | 百分比定位框（覆盖层用） |
| `PdfPageCommentItem` | struct | 单条页面评论：颜色、内容、框矩形 |
| `PdfPageCommentList` | struct | 某页评论列表 |
| `PdfPageAnnotationTarget` | struct | 页面批注定位目标：标签与框矩形 |
| `PdfPageAnnotationTargetResult` | struct | 定位目标集合响应 |
| `PdfCommentTargetOverlayMarker` | struct | 批注目标覆盖层标记（百分比框） |
| `PdfCommentTargetOverlayDisplay` | struct | 覆盖层标记集合 |
| `PdfCommentReviewPageSummary` | struct | 审阅分页摘要：总数/过滤数 |
| `PdfCommentReviewRequest` | struct | 审阅请求：页码过滤 + 查询词 |
| `PdfCommentReviewResult` | struct | 审阅结果：计数、分页摘要、评论列表 |
| `PdfCommentReviewSummaryChip` | struct | 摘要 chip：页码 + 标签 |
| `PdfCommentReviewCardAction` | struct | 审阅卡片动作：id/标签/色调 |
| `PdfCommentReviewCard` | struct | 审阅卡片：内容、页码位置标签、选中态、动作 |
| `PdfCommentReviewPanel` | struct | 审阅面板：元信息、空态、chips、卡片 |
| `PdfCommentOverlayMarker` | struct | 评论覆盖层标记：标题、百分比框、选中态 |
| `PdfCommentOverlayDisplay` | struct | 评论覆盖层标记集合 |
| `PdfCommentReviewDisplay<TSession>` | struct | 组合展示：会话 + 审阅结果 + 面板 + 覆盖层 |
| `PdfRegionCommentRequest` | struct | 区域评论新增请求：页码、区域 id、内容、颜色 |
| `PdfRegionCommentResult` | struct | 区域评论新增结果 |
| `PdfDeleteAnnotationRequest` | struct | 删除批注请求 |
| `PdfDeleteAnnotationResult` | struct | 删除批注结果 |
| `PdfUpdateCommentRequest` | struct | 更新评论内容请求 |
| `PdfUpdateCommentResult` | struct | 更新评论结果 |

### `history/history_types.rs` — HistoryController 域对外 DTO
| 条目 | 类型 | 含义 |
|---|---|---|
| `HistoryError` | enum | 历史域错误：无可撤销 / 无可重做 / 内部状态损坏 |
| `HistoryState` | struct | 历史快照：can_undo / can_redo、双栈深度、单调修订号 |
| `HistoryStepResult` | struct | 单次 undo/redo 结果：是否变更 + 新状态 |
| `HistoryResponse<T>` | struct | 历史域统一响应信封 |

### `persistence/engine.rs` — 从页面上下文与全局补丁状态收集保存计划
| 条目 | 类型 | 含义 |
|---|---|---|
| pub | `collect_persistable_region_patches` | 读全局补丁状态，为字段行/段落/列表项生成可持久化补丁 |
| pub | `collect_legacy_text_reflows` | 为未被区域覆盖的对象收集 run 级/对象级遗留文本重排 |
| pub | `build_persistable_save_plan` | 合并区域补丁与遗留重排，按键去重并归入有效/被抑制两组 |
| 私有 | `resolve_reflow_key` | 以页号 + 对象索引序列生成重排去重键 |

### `persistence/history_store.rs` — 基于 PatchCommand 的全局撤销/重做历史栈
| 条目 | 类型 | 含义 |
|---|---|---|
| pub | `PatchCommand::execute` | 执行命令 = 应用新补丁到全局状态 |
| pub | `PatchCommand::undo` | 回滚：有旧补丁则应用，否则按来源删除补丁条目 |
| `HistoryStore` | struct | undo/redo 命令双栈 |
| pub | `HistoryStore::new` | 建空栈 |
| pub | `HistoryStore::push` | 入栈并可选立即执行，清空 redo 栈，超 50 条淘汰最旧 |
| pub | `HistoryStore::undo` | 弹出 undo 栈顶执行回滚并转入 redo 栈 |
| pub | `HistoryStore::redo` | 弹出 redo 栈顶重新执行并转回 undo 栈 |
| pub | `HistoryStore::clear` | 清空双栈 |
| `GLOBAL_HISTORY` | static | lazy_static 全局 RwLock\<HistoryStore\> 单例 |
| pub | `push_command` | 全局包装：入栈并立即执行 |
| pub | `undo` | 全局包装：撤销一步 |
| pub | `redo` | 全局包装：重做一步 |
| pub | `clear_history` | 全局包装：清空历史 |

### `persistence/models.rs` — 可持久化补丁与保存计划 DTO
| 条目 | 类型 | 含义 |
|---|---|---|
| `PersistableRegionPatch` | struct | 区域补丁全量字段：原/新文本、来源、快照、目标索引、排版参数（字距/缩放/对齐/换行宽等） |
| `RegionTextReflow` | struct | 区域文本重排：页号、目标对象索引、新文本、来源 |
| `PersistableSavePlan` | struct | 保存计划：区域补丁 + 有效/被抑制重排 + 两类已覆盖对象索引集合 |

### `persistence/patch_store.rs` — 全局补丁状态容器与补丁写入
| 条目 | 类型 | 含义 |
|---|---|---|
| `GlobalPatchState` | struct | 全局补丁状态：段落/字段文本与快照映射、补丁映射、替换目标、历史/重做栈、已接受键、修订号、遗留 run/对象文本映射 |
| pub | `GlobalPatchState::new` | 构造空状态（同 Default） |
| pub | `GlobalPatchState::find_paragraph_snapshot` | 按键查段落布局快照（ParagraphLayout） |
| `GLOBAL_PATCH_STATE` | static | lazy_static 全局 RwLock\<GlobalPatchState\> 单例 |
| `PatchCommand` | struct | 单条补丁命令：patch_key + 旧补丁（可空）+ 新补丁，可序列化 |
| `ReviewChangeEntry` | struct | 审阅条目：补丁键、页码、区域、来源、原/当前文本 |
| `ReviewBulkChangeResult` | struct | 批量变更结果：是否变更、修订号、影响补丁数 |
| pub | `bump_patch_revision` | 修订号饱和自增 1 |
| pub | `has_visible_patches` | 判定段落/字段各映射是否存在任何补丁 |
| pub | `apply_patch_maps` | 按来源把补丁写入段落或字段组的文本/快照/补丁映射 |
| pub | `remove_patch_maps` | 按来源从上述映射中移除该区域补丁 |
| pub | `capture_existing_patch` | 捕获区域现有补丁并回填当前文本，作为回滚副本 |
| pub | `apply_patch` | 把补丁按 patch_key 直接写入全局状态（field-row 走字段组，其余走段落） |
| pub | `should_prefetch_page` | 判定目标页是否落在当前页 ±buffer 预取窗口内 |
| pub | `build_pagination_commands` | 生成 [current-1, current+1] 窗口的分页预取命令列表 |

### `persistence/review_types.rs` — 审阅接口响应 DTO
| 条目 | 类型 | 含义 |
|---|---|---|
| `ReviewFeedResult` | struct | 审阅变更列表响应：修订号、待定数、变更条目 |
| `RejectReviewChangeResult` | struct | 拒绝单条变更响应：是否变更、修订号、补丁键 |
| `AcceptReviewChangeResult` | struct | 接受单条变更响应：是否变更、修订号、补丁键 |

## 疑点
- `persistence/history_store.rs` 整体疑似死代码：`GLOBAL_HISTORY` / `push_command` / `undo` / `redo` / `clear_history` 在全仓无调用者；实际撤销/重做由 `pdf-viewer-ui/src/ui_state_store.rs` 直接操作 `GlobalPatchState.history` / `redo_stack` 实现，两套历史机制并存。
- `history/history_types.rs`（HistoryState / HistoryStepResult / HistoryError）无任何下游引用，与 ui_state_store 的历史实现重复，属未接线的 DTO。
- `persistence/engine.rs` 三个 pub 函数（collect_persistable_region_patches / collect_legacy_text_reflows / build_persistable_save_plan）无外部调用者，保存计划实际由 UI 侧 `document/patch_persistence` 组装；`collect_legacy_text_reflows` 内遗留死变量 `_obj_key_field` 与 TS 移植注释，疑似半成品。
- `patch_store.rs` 职责混杂：`should_prefetch_page` / `build_pagination_commands` 属分页预取，与补丁存取无关；其中 `should_prefetch_page` 无调用者。
- `GlobalPatchState.history` / `redo_stack` / `accepted_patch_keys` 实际消费者是 UI 层的 ui_state_store，而 core 内 `patch_store::ReviewBulkChangeResult`、`patch_store::apply_patch`（仅 history_store 使用）随之闲置。
- `annotation/annotation_types.rs` 文件头声明 comment 域归 `CommentManager` 分层，但 PdfCommentReviewPanel / PdfCommentOverlayDisplay 等评论审阅视图 DTO 仍集中在本文件，超出"纯 /Annot 领域类型"的范围。
- 命名公约核对：`*_store` 确为全局状态容器（lazy_static RwLock，非 thread_local 但同语义）、`*_types` 均为纯 DTO，符合；`document/document_types.rs` 的 DocumentError/DocumentResponse 在 core 内无消费者（UI 侧存在镜像副本），属"镜像先行、接线在后"。
