# src-tauri · infrastructure/pdf 读写管线（解析 / 修补 / 布局 / 预览）

> 范围：`src-tauri/src/infrastructure/pdf/`（21 个文件，约 4830 行：`pdf_read/` 7 文件 + 模块清单、`pdf_write/` 4 文件 + 模块清单、`pdf_utils.rs`、`layout_engine.rs`、`layout_analyzer.rs`、`preview_engine.rs`、`glyph_mapping.rs`、`models.rs`、`cache.rs`、`color.rs`）。
> 上游：lopdf（Document / Content / Object / Stream）；`crates/pdf-viewer-core::models`（StyledRun / LayoutRun / LayoutInferenceResult / NativeTextModel / NativePageModel 等，经本层 `models.rs` 再导出）；本地 `font/` 子模块（ParsedFont / ResourceCache / resolve_glyph_geom / break_text_into_lines / resolve_text_write_font / simplify_path_segments）；`text_state.rs`（TextState / TextMatrixCore）；`spatial_graph.rs`（SpatialGraph）；`crate::AppState`（cache.rs 失效入口）。
> 下游：`vector_engine.rs`（resolve_paths + LayoutGraphAnalyzer）、`page_intermediate_service.rs`（缓存键与 PageDisplayList）、`save_engine.rs` / `document_service.rs` / `commands.rs`（PdfDocExt 调用方）、`region_materializer.rs`（PdfMaterializationReport）、`interfaces/pdf/page.rs`（build_light_page_model）、`interfaces/pdf/render.rs`（snapshot_image_cache_as_data_urls）、`application/pdf/edit_commands.rs`（truncate_for_log）。

## 职责
桌面后端 PDF 读写管线的基础设施层：读路径把页面内容流算符解析为 RenderObject / StyledRun 矢量模型；写路径按 TextReflowPatch 重排文本并修补内容流，另提供注释 / 页面 / 元数据等文档级编辑（PdfDocExt trait）；辅以布局语义推断、扫描页预览提取、字形映射纯函数与进程内缓存。格式术语（读路径 / 写路径 / TextState / GraphicsState）与根 CONTEXT.md 一致，其中 CONTEXT.md 所写"写路径 pdf_write.rs"现拆分为 `pdf_write/` 四子模块。

## 文件与方法
（各文件内联 `#[cfg(test)] mod tests` 不列入；`pdf_write/reflow.rs` 内联测试仅覆盖 PdfTextState 算符状态机与纯辅助函数。）

### `pdf_read/mod.rs` — 读路径模块清单与再导出
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | 七个子模块声明 + `pub use` | 汇出 parse_content_stream / GraphicsState / resolve_paths / read_resources 等 |
| pub(crate) | `pub(crate) use build_image_as_jpeg` | 供 preview_engine 经 pdf_read 路径取用图像转码 |

### `pdf_read/content_parser.rs` — 内容流算符遍历：状态机驱动，产出路径 / 图像 / 文本对象
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `parse_content_stream(doc, content, flat_resources, res_cache, state, objects, text_runs, obj_counter)` | 遍历算符：q/Q/cm 栈、颜色（g/G/k/K/rg/SC 等）、透明度（ca/CA/gs→ExtGState）、路径算符（m/l/h/re/W/n）、文本算符（BT/Tf/TL/Tc/Tw/Tz/Ts/Tr/Td/TD/T*/Tm/Tj/TJ）逐类改写 GraphicsState；绘图算符经 simplify_path_segments 后压入 NativePathModel；Do 递归解析 Form XObject、Image XObject 入 PDF_IMAGE_CACHE；单方法内联全部算符分派 |
| 私有 | `apply_alpha_to_color(color, alpha)` | alpha<1 时把 #rrggbb 扩为 #rrggbbaa |
| 私有 | `compute_segments_bbox(segments)` | 求路径段轴对齐包围盒，仅供诊断日志 |
| 私有 | `resolve_tj_array_text(items, font, …)` | TJ 数组合并字符串与字距调整，返回统一文本几何 |

### `pdf_read/graphics_state.rs` — 读路径图形状态容器（TextState + 图形专属字段）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `GraphicsState` (struct) | TextState + line_width/cap/join/miter、stroke/fill 颜色与 alpha、current_font、text_rise，字段公开由算符分派直接内联改写 |
| pub | `new() -> Self` | 默认值：线宽 1.0、miter 10.0、alpha 1.0、无字体 |
| pub | `impl Default` | 委托 new() |

### `pdf_read/image_builder.rs` — 非 JPEG 图像 XObject 解码并转码为 JPEG
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `apply_png_predictor(raw, bytes_per_row, bpp)` | 反滤波 PNG 预测器（None/Sub/Up/Average/Paeth） |
| pub(crate) | `read_decode_params(doc, stream)` | 读 DecodeParms 的 Predictor/Columns/Colors/BitsPerComponent，支持间接引用 |
| pub(crate) | `manual_flate_decompress(compressed)` | flate2 兜底解压：先 zlib 后裸 deflate |
| pub(crate) | `build_image_as_jpeg(doc, stream, w, h)` | lopdf 解压失败转手动手 flate；按 ColorSpace（RGB/Gray/CMYK/1-bit Gray）组 RGBA，编码 JPEG（质量 80）返回 Arc<[u8]> |

### `pdf_read/metadata.rs` — 文档 Info 字典元数据提取
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `extract_metadata(doc) -> PdfMetadata` | 读 trailer→Info 的 Title/Author/Subject，统计页数 |

### `pdf_read/path_resolver.rs` — 页面矢量解析入口：缓存 + 页级锁 + 算符解析编排
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `PAGE_LOCKS` (lazy_static) | 按缓存键串行化并发重复解析的页级互斥表 |
| pub | `resolve_paths(doc, page_index) -> (Vec<RenderObject>, Vec<StyledRun>, w, h)` | 以"Document 指针地址_页号"为键查 PDF_RESOLVE_PATHS_CACHE，双检锁防击穿；解 MediaBox（90/270 旋转交换宽高）、继承 /Rotate，扁平化资源后调 parse_content_stream，结果回写缓存 |

### `pdf_read/resource_reader.rs` — 页资源字典沿 /Parent 链扁平化
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `FlatResources` (type) | 类别名 → 资源名 → ObjectId 的扁平映射 |
| pub | `read_resources(doc, page_id)` | 沿 Parent 链合并各级 Resources 字典，近端优先，visited 防环 |
| pub | `find_xobject_by_name(doc, flat_resources, name)` | 先查本页扁平表，未命中则全局逐页兜底搜索 |

### `pdf_read/utils.rs` — 读路径小工具（算符操作数与矩阵乘法）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `operands_to_f32(ops)` | 算符操作数 Real/Integer 统一转 f32 向量 |
| pub | `multiply_matrices(current, new)` | 3x2 仿射矩阵乘（被 text_state.rs 的 TextMatrixCore 复用） |

### `pdf_write/mod.rs` — 写路径枢纽：PdfDocExt trait 定义与 Document 实现（重活分派四子模块）
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `mod annotations / emitters / pages / reflow` | 四个子模块：注释 CRUD、算符发射、页面操作、文本重排 |
| pub(crate) | `pub(crate) use emitters::*; use reflow::{…}` | 转出发射器与重排类型（PdfTextState / PersistedTextLinePlan / ReflowCluster 等） |
| pub | `PdfDocExt` (trait) | lopdf::Document 的 PDF 编辑操作扩展契约 |
| pub (trait) | `apply_text_patch(page, old, new, target_index, offset_x)` | 简单文本替换：patch_content_recursive 命中后重编内容流并换 Contents 引用 |
| pub (trait) | `apply_atomic_reflow_to_doc(…11 参数)` | 单补丁原子重排：打包 TextReflowPatch 后转调 apply_batch_reflow_to_doc |
| pub (trait) | `apply_batch_reflow_to_doc(page, patches)` | 批量重排主干：读页尺寸 / UserUnit，内容流头部插 q、尾部补 Q，ReflowCluster 分簇后交 patch_atomic_reflow_recursive，末尾追加 emit_deferred_text_block 并重写 Contents |
| pub (trait) | `replace_image_xobject(object_id, bytes)` | 转调 pages 实现（当前未实现） |
| pub (trait) | `delete_page(page_num)` | 转调 lopdf delete_pages |
| pub (trait) | `rotate_page(page_num, rotation)` | 设置页字典 /Rotate |
| pub (trait) | `insert_blank_page(at_index)` | 转调 pages 实现（当前未实现） |
| pub (trait) | `add_highlight(page, rect, color)` | 转调 annotations 高亮实现 |
| pub (trait) | `add_text_comment(page, rect, color, contents)` | 转调 annotations 文本批注实现 |
| pub (trait) | `update_text_comment(page, annot_id, contents)` | 转调 annotations 批注更新实现 |
| pub (trait) | `delete_annotation(page, annot_id)` | 转调 annotations 批注删除实现 |
| pub (trait) | `update_metadata(title, author, subject, keywords)` | 转调 pages 元数据写回实现 |

### `pdf_write/reflow.rs` — 写路径核心：两个内容流遍历器 + 纯辅助函数 + PdfTextState
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(crate) | `PersistedTextLinePlan` (struct) | 单条重排文本行的持久化计划：字体别名 / 字号 / 编码字节 / 坐标 / 宽 / 色 / 下划线 / 渲染模式 / 补丁序号 |
| pub(crate) | `PdfTextState` (struct) | 写路径状态：TextState + Tf 操作数字体名字节（font_alias） |
| pub(crate) | `PdfTextState::new()` | 默认字号 12、水平缩放 100、单位矩阵 |
| pub(crate) | `ReflowCluster` (struct) | 重排补丁簇：min_idx/max_idx 与补丁列表 |
| pub(crate) | `ReflowCluster::build(patches)` | 按目标索引最小锚点聚合补丁，BTreeMap 保序 |
| pub(crate) | `compute_silenced_indices(cluster_map)` | 汇总将被静默（替换）的全部目标索引 |
| pub(crate) | `compute_micro_fit(layout, target_wrap, text, h, cs)` | 微调水平缩放（±15% 内按比例）或字距使替换文本贴合原槽位 |
| pub(crate) | `mute_show_op(op, op_str)` | 置空 Tj/'/TJ/" 的文本操作数，产出无渲染副本 |
| pub(crate) | `apply_text_state_op(op, op_str, state, stack)` | 处理纯文本状态算符（BT/ET/Tc/Tw/Tz/Tr/TL/Tm/Td/TD/T*/q/Q/cm），返回是否接管 |
| pub(crate) | `resolve_line_color(line)` | 取 VisualLine 首个非空 run 的颜色，缺省黑 |
| pub(crate) | `resolve_line_underline(line)` | 任一 run 带下划线即为真 |
| pub(crate) | `patch_content_recursive(doc, content, …)` | 简单替换遍历器：跟踪 Tf/Tc/Tw/Tz，解码 Tj/TJ 匹配 old_text 后以字体编码回填，Do 递归 Form |
| pub(crate) | `patch_atomic_reflow_recursive(doc, page_id, content, …)` | 布局感知重排遍历器：按 obj_counter 对齐目标索引，命中簇时 resolve_text_write_font + break_text_into_lines（先初排再微调复排），换算页面坐标后生成 PersistedTextLinePlan，原 show 算符静音，Do 递归 Form 并应用其 Matrix |

### `pdf_write/annotations.rs` — 注释 CRUD：高亮、文本批注、删除
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(super) | `add_highlight_impl(doc, page_num, rect, color)` | 构造 Highlight Annot（QuadPoints/CA=0.35），页面坐标翻转 Y 后挂入 Annots |
| pub(super) | `add_text_comment_impl(doc, page_num, rect, color, contents)` | 构造 Text/Comment Annot，图标贴区域右上角 |
| pub(super) | `update_text_comment_impl(doc, page_num, annot_id, contents)` | 校验归属页与 Subtype=Text 后改写 Contents |
| pub(super) | `delete_annotation_impl(doc, page_num, annot_id)` | 从 Annots 摘除并删除注释对象 |
| pub(super) | `read_page_height(doc, id)` | 经 pdf_utils::read_page_size 取有效页高 |
| 私有 | `append_page_annotation(doc, page_id, annot_id)` | Annots 数组 / 间接引用两种形态统一追加 |
| 私有 | `remove_page_annotation(doc, page_id, annot_id)` | 过滤目标引用，空数组则移除 Annots 键 |
| pub(super) | `read_page_annotation_refs(doc, page_id)` | 读页面全部注释 ObjectId |

### `pdf_write/emitters.rs` — PDF 算符发射器（命令模式）：文本行 / 下划线 / 延迟文本块
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(crate) | `UnderlineSpec` (struct) | 单条下划线几何：x/y/宽/线宽/颜色 |
| pub(crate) | `emit_text_line_ops(run, user_unit)` | 发射单行：rg/RG 着色、Tr 渲染模式（1/2 加伪粗描边）、Tm/Tf/Tj，返回可选下划线规格 |
| pub(crate) | `emit_underline_ops(spec)` | 发射单条下划线路径：RG/w/m/l/S |
| pub(crate) | `emit_deferred_text_block(lines, page_height, user_unit)` | 组装整块：q + cm 翻转 Y + 重置字距 + BT，逐行发射去重（patch_idx+line_seq），ET 后补下划线，Q 收尾 |

### `pdf_write/pages.rs` — 页面级操作：删除 / 旋转 / 插页 / 元数据 / 图像替换
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(super) | `delete_page_impl(doc, page_num)` | 调 lopdf delete_pages |
| pub(super) | `rotate_page_impl(doc, page_num, rotation)` | 写页字典 /Rotate |
| pub(super) | `insert_blank_page_impl(_doc, _at_index)` | 未实现，恒返回 Err |
| pub(super) | `replace_image_xobject_impl(_doc, _object_id, _bytes)` | 未实现，恒返回 Err |
| pub(super) | `update_metadata_impl(doc, title, author, subject, keywords)` | 写 trailer Info 字典四个字段 |

### `pdf_utils.rs` — 共享解析工具：MediaBox 页尺寸、继承 /Rotate、数值与日志截断
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `DEFAULT_PAGE_WIDTH/HEIGHT` (const) | MediaBox 缺失时的 A4 回退 595×842 |
| pub | `obj_to_f32(obj)` / `obj_to_f32_or(obj, default)` | lopdf Object 转 f32（Real/Integer 通吃），带/带默认值 |
| pub | `PageSize` (struct) | 页几何：width/height/y_origin |
| pub | `PageSize::effective_height()` | height − y_origin 取绝对值 |
| pub | `PageSize::default_a4()` | A4 默认构造 |
| pub | `read_page_size(doc, page_id)` | 解析 MediaBox 数组，异常逐级回退 A4 |
| pub | `read_page_rotation(doc, page_id)` | 沿 Parent 链读继承 /Rotate，归一化到 0/90/180/270 |
| pub | `apply_rotation(width, height, rotation)` | 90/270 时交换宽高 |
| pub | `truncate_for_log(value, limit)` | 按字符数截断并追加省略号，用于日志 |

### `layout_engine.rs` — 核心布局语义分析器（自 pdf-viewer-core 迁入）：逆排版推演
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `LayoutGraphAnalyzer` (struct) | 单页分析上下文：page_index/width/height |
| pub | `new(page_index, width, height)` | 注入页面几何参数 |
| pub | `resolve_regions(runs) -> LayoutInferenceResult` | SpatialGraph 建邻接（水平容差 8.0 防跨列合并）→ 连通分量 → 阅读序排序 → 模式判定 → 产出 SemanticRegion 集合 |
| 私有 | `detect_layout_pattern(runs)` | 启发式判型：含冒号→KvField、大字加粗短文本→SectionHeader、项目符号/编号→ListItem、否则 Paragraph |
| 私有 | `create_semantic_region(id, kind, mode, runs)` | 聚合分量包围盒并装配默认段落样式的 SemanticRegion |

### `layout_analyzer.rs` — 布局分析 Tauri 宿主代理（V3 薄封装）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `LayoutGraphAnalyzer` (struct) | 包装 layout_engine 核心分析器（`use … as CoreAnalyzer`） |
| pub | `new(page_index, width, height)` | 构造内层 CoreAnalyzer |
| pub | `analyze(runs) -> LayoutInferenceResult` | StyledRun 经 LayoutRun::from_styled 转换后委派 resolve_regions |
| pub | `detect_column_bands(_runs) -> Vec<f32>` | 未迁移，恒返回空（TODO 注明） |

### `preview_engine.rs` — 扫描页判定与轻量页模型：选最大图作预览
| 可见性 | 方法 | 功能 |
|---|---|---|
| 私有 | `collect_page_xobjects(doc, page_id)` | 沿 Parent 链收集 XObject 引用；空则解析内容流 Do 名字跨页兜底搜索 |
| 私有 | `page_has_font_resources(doc, page_id)` | 沿 Parent 链探测 Font 资源 |
| 私有 | `page_has_text_operators(doc, page_id)` | 内容流是否含 Tj/TJ/'/"/BT/Tf/Td/TD/Tm |
| 私有 | `cache_image_asset(doc, xobj_stream, w, h)` | DCTDecode 直取（支持前置 FlateDecode 解压），非 JPEG 转调 build_image_as_jpeg；入 PDF_IMAGE_CACHE 并返回 pdfasset.localhost URL |
| pub | `build_light_page_model(doc, page_index) -> LightPageModel` | 综合文本算符 / 字体资源 / XObject：无文本取最大面积图为 Scanned 预览；有文本仅当图像覆盖 ≥85% 页面才作背景预览，产出 LightPageModel |

### `glyph_mapping.rs` — 嵌入字体矢量文本的字形映射纯函数（run → glyph id）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `real_font_size(text)` | scale_y 绝对值 >1 时取矩阵缩放，否则声明字号 |
| pub | `glyph_count(text)` | 有 pdf_char_codes 按码计数，否则按 Unicode 字符数 |
| pub | `build_glyph_positions(text)` | 逐字形基线原点：优先 char_origins，其次按 char_widths 自 tx 累加，单字形退 run 原点 |
| pub | `prefers_pdf_code_glyph_mapping(text)` | TrueType/OpenType/Type1 简单字体子类型可直接用 PDF 字符码作 glyph id |
| pub | `resolve_glyph_id(text, glyph_index, map_charcode)` | 四级回退求 glyph id：原始码过 charmap → Unicode 码过 charmap（跳控制/空白）→ 简单字体直用原始码 → 0（notdef） |

### `models.rs` — 管线数据模型：core 再导出 + 写路径补丁与渲染对象类型
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use pdf_viewer_core::models::{…}` | 再导出 document_runtime/font/geometry/glyph/interaction/layout/styled_run/vector 各组及 NativeTextModel、PersistableRegionPatch |
| 私有 | `is_false / default_alpha / is_default_alpha / default_scale_x / is_zero_u8` | serde skip/default 辅助函数 |
| pub | `TextReflowPatch` (struct) | 文本重排补丁：页号、目标索引、新文本、多样式 runs、对齐 / 行高 / Y 位移 / 换行宽 / 字距 / 水平缩放 |
| pub | `PdfMaterializationDecisionReport / SourceStats / Report` (struct) | 区域补丁物化决策报告：路径、计数、按来源统计、逐条决策 |
| pub | `PdfModifications` (struct) | 保存聚合：旋转表 + region_patches + text_reflows + text_patches |
| pub | `PathSegment` (struct) | 路径段：command（move/line/bezier/close）+ 点集 |
| pub | `NativePathModel` (struct) | 矢量路径对象：segments、填充 / 描边色与开关、线属性、alpha、z_index、V197 调色板索引 |
| pub | `NativePathModel::flip_y(h)` | 段点 Y 坐标按页高翻转 |
| pub | `NativeImageModel` (struct) | 图像对象：pdfasset URL、位置尺寸、六元 CTM 分量、提取方式标记 |
| pub | `VectorPalette` (struct) | 页级颜色 / 字体调色板 |
| pub | `TextPatch` (struct) | 简单文本替换补丁：old/new 文本、X 偏移、目标序号 |
| pub | `RenderObject` (enum) | Text/Path/Image 三态渲染对象（serde tag=type） |
| pub | `PageDisplayList` (struct) | 页显示列表：页号 + 尺寸 + objects + text_runs |
| pub | `NativeVectorPageModel` (struct) | 矢量页模型：objects + palette + 背景图 |
| pub | `NativeVectorPageModel::flip_y()` | 全对象 Y 翻转到页面坐标 |
| pub | `LightPageKind` (enum) | 页类型：Pending/Scanned/Mixed/Text |
| pub | `LightPageModel` (struct) | 轻量页模型：尺寸 + 类型 + 预览图 URL |
| pub | `PdfMetadata` (struct) | 文档元数据：标题 / 作者 / 主题 / 关键词 / 创建者 / 日期 / 页数 |

### `cache.rs` — 进程内缓存与失效：图像 / 字节 / 解析结果三张全局表
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PDF_IMAGE_CACHE` (lazy_static) | 资产 id → 图像字节（pdfasset.localhost 协议取数） |
| pub | `PDF_FONT_PROGRAM_CACHE` (lazy_static) | 字体程序字节缓存（当前无读写方，见疑点） |
| pub | `PDF_RESOLVE_PATHS_CACHE` (lazy_static) | "Document 指针_页号" → 解析结果 Arc |
| pub(crate) | `page_cache_key(path, page_index)` | 路径::页号 缓存键 |
| pub(crate) | `page_revision_cache_key(path, page_index, revision)` | 带文档修订号的缓存键，无修订退化为普通键 |
| pub(crate) | `invalidate_pdf_page_cache(state, path)` | 按前缀清理页中间缓存 / 页缓存 / 资产锁，并按指针地址清 resolve_paths 缓存 |
| pub(crate) | `snapshot_image_cache_as_data_urls()` | 图像缓存按魔数判 MIME 转 data: URL 快照 |
| pub(crate) | `invalidate_pdf_layout_cache(state, path)` | 按前缀清理布局缓存 |

### `color.rs` — 颜色转换：写路径严格解析 + 读路径宽松转换
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `parse_pdf(color) -> Option<[f32; 3]>` | 严格解析 6 位 hex 为 [0,1] RGB，坏值返回 None 供写路径拒绝 |
| pub(crate) | `cmyk_to_rgb(c, m, y, k)` | 朴素 CMYK→RGB（逐通道 clamp） |
| pub(crate) | `gray_to_hex(gray)` | 灰度值转 #rrggbb |

## 疑点
- **死代码**：`cache.rs` 的 `PDF_FONT_PROGRAM_CACHE` 全 workspace 无任何读写方；`glyph_mapping.rs` 五个纯函数全 workspace 无消费方（仅 mod.rs 声明 + 自带测试），其文档注释称"自 vello_renderer 提取以便测试"，但当前矢量渲染引擎未接线，属悬空 / 预留代码。
- **疑似重复**：`pdf_read/path_resolver.rs` 内联手写 MediaBox 解析 + 宽高交换，与已抽象的 `pdf_utils::read_page_size` + `apply_rotation` 重复（其仅复用了 read_page_rotation）；`preview_engine.rs` 的 `collect_page_xobjects` / `page_has_font_resources` 各自重写 Parent 链遍历，与 `read_resources` 逻辑重叠；`path_resolver` 与 `pdf_utils::read_page_size` 对 MediaBox 的宽度口径不一致（`x1.abs()` vs `|x1−x0|`，y0≠0 时结果不同）。
- **命名冲突**：`layout_engine::LayoutGraphAnalyzer`（核心分析器）与 `layout_analyzer::LayoutGraphAnalyzer`（薄代理）同名同目录层级，仅靠 `as CoreAnalyzer` 区分，易混淆。
- **术语 / 版本口径**：CONTEXT.md 仍写"写路径 pdf_write.rs、PdfTextState 仅在 pdf_write.rs 内部可见"，实际已拆分为 `pdf_write/` 四子模块且 PdfTextState 定义于 reflow.rs（pub(crate)）；models.rs / preview_engine.rs / layout_engine.rs 散布 V3 / V197 / V206 / V265 / V267 / V4.1 等版本号注释，与 core 文档口径未统一。
- **设计风险**：`resolve_paths` 以 `&Document` 的内存地址作缓存键，文档重载后地址复用可能命中脏缓存（cache.rs 失效逻辑依赖同一指针约定）；`PDF_IMAGE_CACHE` 只增不减（无容量上限 / 淘汰）；`pdf_write/pages.rs` 的 insert_blank_page 与 replace_image_xobject 为恒 Err 占位；`content_parser.rs` 的 `compute_segments_bbox` 结果仅喂诊断日志；`preview_engine.rs` 内联测试引用本机固定路径并故意 `panic!` 输出调试信息，依赖机器夹具。
