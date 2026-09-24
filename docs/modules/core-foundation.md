# pdf-viewer-core · 基础层（models / geometry / text / typography / common）

> 范围：`crates/pdf-viewer-core/src/` 下 `lib.rs`、`models.rs`（壳）与 `models/`（9 文件）、`geometry/`（8 文件）、`text/`（16 文件，含子目录 `glyph_layout/`）、`typography/`（5 文件）、`common/`（4 文件）——共 44 个文件，约 7900 行（含内联测试与注释；`text/glyph_layout/tests.rs` 独立测试文件仅列覆盖点）。`models/` 无独立 `mod.rs`，由 `models.rs` 充当类型枢纽壳。
> 上游：`document/`（读路径解析产出 StyledRun / VectorPageModel / NativeTextModel）、`persistence::patch_store`（reflow_engine 读取段落补丁快照）、`document::page_region_context`（style_preservation 消费行样式快照类型）、`edit::debug_trace`（reflow / layout_engine 调试打点）。
> 下游：core 内部 `render/`（消费 models 全家与 `typography::font_resolver`）、`edit/`（draft_layout / document_plan 消费 `geometry::layout_engine`）、`persistence/patch_store`（消费 ParagraphLayout）；对外 `pdf-viewer-ui` 的 `editor/`（text_geometry、session、orchestrator）、`projection_workflow.rs`、`geometry_api.rs`、`render/` 等。

## 职责
承载全库共享的领域数据模型与基础计算：`models/` 是跨 WASM 边界的 serde DTO 层（camelCase 序列化 + `flip_y` 坐标极性规范化公约）；`geometry/` 提供坐标投影、包围盒运算与流式段落折行重排引擎；`text/` 处理编辑态文本（段落聚合、光标几何、样式映射保持、列表与查找替换）；`typography/` 负责字体解析与系统字体匹配；`common/` 为调试、数值净化与结构化追踪等横切工具。除 `common/trace.rs` 的 thread_local subscriber 注册表外，全层为纯 Rust 领域计算，无 DOM / 无 wasm_bindgen 依赖。

## 文件与方法
（各文件内联 `#[cfg(test)] mod tests` 不列入；纯数据文件用"条目/类型/含义"表，方法行以"方法"标注；逻辑文件用"可见性/方法/功能"表。）

### `lib.rs` — crate 模块清单与版本号
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `read_core_version()` | 返回 CARGO_PKG_VERSION 字符串 |

### `models.rs` — models 域类型枢纽壳：声明 9 个子模块并整体再导出（保持 `use models::Foo` 旧路径）
| 条目 | 类型 | 含义 |
|---|---|---|
| `pub mod document_runtime / font / geometry / glyph / interaction / layout / marker / styled_run / vector` | 模块声明 | 九个单一职责子模块 |
| `pub use …::*` | 再导出 | 各子模块类型平铺至 `models::` 顶层 |

### `models/document_runtime.rs` — 文档级运行时 DTO：页面状态、编辑意图、文档分类与分页/页面操作命令
| 条目 | 类型 | 含义 |
|---|---|---|
| `PageState` | struct | 页面渲染状态快照：视口/缩放/锚点/绘制计划/页尺寸表 |
| `BaseEditIntent` | struct | 基础编辑意图：选区起止 + 新文本 |
| `EditIntent` | enum | 编辑意图：段落（BaseEditIntent）或字段（active_part + key/value 新文本） |
| `LightPageKind` | enum | 轻量页类型：Pending / Scanned / Mixed / Text |
| `LightPageModel` | struct | 轻量页模型：索引/尺寸/类型/预览图 URL |
| `PdfDocumentKind` | enum | 文档分类：Unknown / Scanned / Mixed / Vector |
| `ClassificationReason` | enum | 分类依据：整页图/OCR 层/文本算符/字体资源/低置信回退 |
| `ReadDocumentMeta` | struct | 打开文档元信息：doc_id/路径/页数/分类/置信度/扫描预览开关 |
| `PaginationAction` | enum | 分页动作：Prefetch / Release / Upgrade |
| `PaginationCommand` | struct | 分页命令：动作 + 页码 + 路径 + zoom |
| `DeletePageCommand` | struct | 删除页命令（页号） |
| `RotatePageCommand` | struct | 旋转页命令（页号 + 角度增量） |
| `InsertPageCommand` | struct | 插入页命令（插入位置） |
| `AddHighlightCommand` | struct | 添加高亮命令（页号 + rect + color） |
| `UpdateMetadataCommand` | struct | 更新文档元数据（title/author/subject/keywords） |

### `models/font.rs` — 字体提示与解析结果 DTO（FontHints / ResolvedFontFace）
| 条目 | 类型 | 含义 |
|---|---|---|
| `FontHints` | struct | 字体提示：flags/weight/斜度/上下高度/等宽/serif/bold 等布尔标志 |
| `FontSourceKind` | enum | 字体来源：Embedded / SystemMatched / Substituted / Fallback |
| `SymbolClass` | enum | 符号字体类别：None / Symbol / Dingbat |
| `ResolvedFontIdentity` | struct | 解析出的字体身份：规范族名/样式名/字重/斜体/符号类/子集剥离标记 |
| `ResolvedFontFace` | struct | 渲染用字体面：身份 + render/metrics CSS 族 + 来源 + 置信度 |

### `models/geometry.rs` — 2D 绝对包围盒 BoundingBox 与 Y 轴极性翻转原语
| 条目 | 类型 | 含义 |
|---|---|---|
| `BoundingBox` | struct | Y-Down 规范包围盒，不变式 top 严格 < bottom |
| `flip_y` | pub 方法 | 以页高 h 为反射轴原地反转 Y 极性 |

### `models/glyph.rs` — 编辑器字形绘制计划 DTO 树（Plan→Region→Paragraph→Run）与整页 Y 翻转
| 条目 | 类型 | 含义 |
|---|---|---|
| `GlyphPaintRun` | struct | 单游程绘制单元：文本/bbox/origin/字符原点/颜色/字体/绘制模式 |
| `EditorControlStyle` | struct | 编辑器控件 CSS 样式（family/size/weight/style/color/decoration） |
| `GlyphPaintParagraph` | struct | 绘制段落：样式 + 编辑会话 + 控件样式 + 语义角色 + runs |
| `ExternalObject` | enum | 页面外部对象：Image（几何）或 Path（命令串） |
| `GlyphPaintRegion` | struct | 绘制区域：LayoutRole/LayoutMode + bbox + 段落集 + 对象引用 |
| `GlyphPaintPlan` | struct | 整页字形绘制计划：页索引/尺寸 + regions + external_objects |
| `flip_y` | pub 方法 | 整计划 Y 翻转：区域/段落/编辑会话/双份 runs/Image 外部对象 |

### `models/interaction.rs` — 交互投影 DTO：字段编辑器五盒矩形与点击命中请求/结果
| 条目 | 类型 | 含义 |
|---|---|---|
| `RectBox` | struct | left/top/width/height 矩形 |
| `FieldProjection` | struct | 字段五盒投影：text/shell/label/value/editor |
| `FieldProjectionRequest` | struct | 投影计算入参：组/槽/标签/值边界 + 页高 + 字段元信息标记 |
| `FieldPartKind` | enum | 命中部分：Key / Value |
| `FieldHitRequest` | struct | 单字段点击命中请求：投影 + 双侧可编辑文本与编辑会话 |
| `FieldHitResolution` | struct | 命中结果：active_part + 初始光标索引 + 实测 key/value 宽 |
| `FieldHitTarget` | struct | 批量命中目标：投影 + 双侧文本与会话 |
| `FieldHitBatchRequest` | struct | 批量命中请求：目标集 + 点击页面坐标 |
| `FieldHitMatch` | struct | 命中匹配：目标索引 + 解析结果 |
| `FieldEditorParamsRequest` | struct | 字段编辑器参数请求：runs + anchor_bbox + 行高 |
| `FieldEditorParams` | struct | 字段编辑器参数：编辑会话 + 控件样式 |
| `InteractionProjection` | struct | 交互投影：区域盒 + 行盒列表 + label/value 盒 |
| `InteractionTarget` | struct | 可交互目标：kind/region_id/object_id/text + 投影 |
| `FieldEditorProjection` | struct | 字段编辑器 DOM 投影：像素矩形/scale_x/字号/渲染族/颜色 |

### `models/layout.rs` — 布局语义 DTO：LayoutRun/段落/语义区域与全部布局枚举
| 条目 | 类型 | 含义 |
|---|---|---|
| `FieldKind` | enum | 字段类型：Unknown / LabelValue |
| `SemanticRole` | enum | 语义角色：Title/Header/Date/Amount/Email/PhoneNumber/Contact/Address/GenericField/BodyText |
| `EditableFieldGroup` | struct | K-V 字段分组：标签/值文本、值起始索引与 run 区间 |
| `EditableSegment` | struct | 可编辑段：run 区间聚合 + 几何 + 样式 + 字符原点/宽度 + 字段组 |
| `LayoutRole` | enum | 布局角色：Title/SectionHeader/KvField/ListItem/Paragraph/PageMeta/FixedBlock/AnchoredObject |
| `LayoutAlignment` | enum | 对齐：Left / Center / Right / Justify |
| `LayoutMode` | enum | 布局模式：Flow / Fixed / Anchored |
| `RunStyle` | struct | 游程样式快照：字体/字号/颜色/bold/italic/underline/char_spacing/scale_x |
| `LayoutRun` | struct | 布局游程：文本 + 样式 + bbox + origin + 字符原点/宽度 + 对象引用 |
| `from_styled` | pub 方法 | StyledRun → LayoutRun：推导 bbox 并换算 scale_x（horizontal_scaling/100） |
| `ParagraphStyle` | struct | 段落样式：对齐/行距/首行缩进/左缩进/tab 停靠位 |
| `LayoutParagraph` | struct | 布局段落：runs + 样式 + bbox + origin + wrap_width |
| `flip_y` | pub 方法 | 段落 bbox/origin 及其 runs 的 Y 翻转 |
| `ParagraphEditContext` | struct | 段落编辑会话：锚点 bbox + 可变 LayoutParagraph |
| `SemanticRegion` | struct | 语义区域：role/mode/bbox + 段落集 + 对象引用 |
| `flip_y` | pub 方法 | 区域 bbox 与其段落的 Y 翻转 |
| `LayoutInferenceResult` | struct | 整页布局推断结果：页索引/尺寸 + 区域集 + 列带 |
| `flip_y` | pub 方法 | 整页全部区域的 Y 翻转 |
| `PaintMode` | enum | 绘制模式：Fill / Stroke / FillStroke |

### `models/marker.rs` — 统一视觉 marker 抽象（文本 bullet/编号与图形 bullet）
| 条目 | 类型 | 含义 |
|---|---|---|
| `VisualMarkerKind` | enum | marker 类型：None / TextBullet / TextNumbering / GraphicBullet / Custom |
| `GraphicType` | enum | 图形对象类型：Image / Path |
| `VisualMarkerContent` | enum | marker 内容：Text{text,runs} 或 Graphic{对象索引/类型/ID} |
| `VisualMarker` | struct | 统一 marker：kind + content + bbox + advance（body 缩进量）+ object_indices |
| `from_text_marker` | pub 方法 | 由文本 runs 构造文本 marker（汇总 object_indices，bbox 取并集） |
| `from_graphic` | pub 方法 | 由图形对象构造图形 marker（advance = 宽 + 6px 间隙） |
| `is_graphic` | pub 方法 | 是否图形型 marker |
| `contains_object_index` | pub 方法 | 是否包含指定对象索引 |

### `models/styled_run.rs` — 内容流原子游程 StyledRun 与前端原生文本模型 NativeTextModel
| 条目 | 类型 | 含义 |
|---|---|---|
| `StyledRun` | struct | 解析管线的原子数据载体：文本/颜色/Tm 仿射矩阵分量/字距/字符原点与宽度/render_mode/object_id 回溯 ID/嵌入字体线索 |
| `flip_y` | pub 方法 | 仅反转 ty（层级过低不内置 bbox，bbox 延后至 LayoutRun/GlyphPaintRun） |
| `NativeTextModel` | struct | 面向前端的原生文本对象 DTO：多版本演进字段（glyph_bounds/调色板索引/角色/对齐/字符级位置）+ 内嵌 runs |
| `flip_y` | pub 方法 | 反转 ty/baseline_y/top 并镜像 char_origins 的 dy（基线相对偏移取负号） |
| `NativePathObject` | struct | 空占位结构（路径对象） |
| `NativeImageObject` | struct | 空占位结构（图像对象） |
| `NativePageObject` | enum | 原生页面对象：Text / Path / Image |
| `NativePageModel` | struct | 原生页模型：页索引/尺寸 + 对象列表 |

### `models/vector.rs` — 页面矢量对象模型（Text/Path/Image）与调色板
| 条目 | 类型 | 含义 |
|---|---|---|
| `VectorPathSegment` | struct | 路径段：SVG 式命令 + 点列 |
| `VectorPathObject` | struct | 矢量路径对象：segments/填充/描边/宽度/z_index/调色板索引 |
| `VectorPalette` | struct | 颜色调色板（字符串颜色表） |
| `VectorImageObject` | struct | 矢量图像对象：位置尺寸 + z_index |
| `VectorTextObject` | struct | 矢量文本对象：StyledRun 集 + z_index |
| `VectorRenderObject` | enum | 渲染对象：Text / Path / Image |
| `VectorPageModel` | struct | 大一统页面矢量容器：Core 离线计算与前端桥接的骨架 |
| `flip_y` | pub 方法 | "The Great Normalization Gate"：整页对象由 Y-Up 不可逆转为 Y-Down |
| `decompress_palette` | pub 方法 | 将路径的调色板索引解压回实际颜色字符串（WASM 序列化后调用） |

### `geometry/mod.rs` — geometry 子模块清单（7 个子模块声明）

### `geometry/bbox_ops.rs` — 包围盒基础运算
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `bbox_width(bbox)` | 包围盒宽（负值钳 0） |
| pub | `bbox_height(bbox)` | 包围盒高（负值钳 0） |
| pub | `bbox_intersects(a, b)` | 两包围盒是否相交 |
| pub | `union_bbox(a, b)` | 两包围盒并集 |

### `geometry/coordinate_transform.rs` — 宿主 client ↔ 页面坐标投影与 Y-Up 归一化屏障
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PageViewPoint` / `EditorLocalPoint` (struct) | 页面绝对坐标点 / 编辑器局部坐标点（Y-Down 语义） |
| pub | `HostReferenceRect` / `ClientPoint` / `PageSize` / `PageScale` (struct) | 宿主参考框 / client 点 / 逻辑页尺寸 / 缩放比例 |
| pub | `HostPageTransform` (struct) | 宿主 client 坐标与页面坐标的统一投影器 |
| pub | `HostPageTransform::new / scale` | 构造 / 计算宿主→页面比例（非法值兜底 1.0） |
| pub | `HostPageTransform::client_to_page` | client 坐标 → 页面绝对坐标 |
| pub | `HostPageTransform::client_to_page_in_box` | client → 指定页盒内坐标（clamp 到盒内） |
| pub | `HostPageTransform::client_to_local_in_box` | client → 页盒内局部坐标 |
| 私有 | `positive_ratio(num, den)` | 安全正值比例（非有限/非正回退 1.0） |
| pub | `PdfToPageViewTransform` (struct) | 全局点→视图系投影屏障（Y 翻转已前置，O(1) 透传） |
| pub | `PdfToPageViewTransform::new / point` | 构造（页高为占位参数）/ 原样透传坐标 |
| pub | `PdfCoordinateSpace` (struct) | PDF 原始物理空间归一化关卡（无状态） |
| pub | `PdfCoordinateSpace::normalize_y` | PDF Y-Up → Y-Down |
| pub | `PdfCoordinateSpace::denormalize_y` | Y-Down → PDF Y-Up 反向投影 |
| pub | `EditorViewportTransform` (struct) | 页面绝对系 → 编辑视区局部系变换（锚点盒 + 顶部缓冲） |
| pub | `EditorViewportTransform::new` | 由锚点盒与 top_buffer 构造 |
| pub | `EditorViewportTransform::point_from_pdf` | 页面点 → 编辑器局部点（含顶部缓冲） |
| pub | `EditorViewportTransform::x_from_pdf` | 一阶横向脱钩换算 |
| pub | `EditorViewportTransform::baseline_y_from_pdf` | 基线映射到缓冲后的局部 Y |
| pub | `EditorViewportTransform::baseline_y_from_anchor_relative` | 锚点相对 Y + 缓冲叠加 |

### `geometry/dom_projection.rs` — DOM rect/point 的 serde DTO 与缩放比例计算
| 条目 | 类型 | 含义 |
|---|---|---|
| `DomRectLike` | struct | DOM 矩形 DTO（left/top/width/height） |
| `DomPointLike` | struct | DOM 点击点 DTO（clientX/clientY） |
| `ScalePair` | struct | 缩放比对（scaleX/scaleY） |
| `measure_dom_to_page_scale` | pub 方法 | 由 DOM 参考框与页面尺寸求宿主→页面缩放对 |

### `geometry/field_projection.rs` — K-V 字段编辑器五盒投影
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `resolve_field_projection(request)` | 由组/槽/标签/值边界计算五盒投影（含最小宽高与页高换算、左右负边距） |

### `geometry/layout_engine.rs` — 流式段落折行排版引擎与编辑器投影、段落命中
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `VisualLine` (struct) | 折行后的视觉行：runs/宽/高/基线 Y/横向偏移补偿/文本 |
| pub | `ParagraphLayout` (struct) | 段落排版结果：视觉行瀑布流集合 + 总高 |
| pub | `ParagraphLayout::find_run_at_text_offset` | 文本偏移 →（行, run, run 内偏移）三元组 |
| 私有 | `CJK_NO_START / CJK_NO_END` (const) | CJK 避头尾禁则字符表 |
| 私有 | `is_no_start / is_no_end / is_forced_line_break_run` | 避头尾成员判定 / 强制换行 run 判定 |
| pub | `layout_paragraph(paragraph, wrap_width, measure)` | 贪心折行主算法：避头尾/强制换行/首行缩进/tab 停靠/四向对齐/行高累加，附排版诊断打点 |
| 私有 | `finish_line(runs, …)` | 行收尾：Center/Right 偏移与 Justify 均摊间隙 |
| pub | `layout_anchored_pair(anchor, body, gap)` | 锚定排版：body 放置到 anchor 右侧并基线对齐 |
| pub | `find_paragraph_at(plan, x, y)` | 点命中段落：run 命中优先、面积最小者胜 |
| pub | `is_point_in_bbox(bbox, x, y)` | 点是否在包围盒内 |
| pub | `resolve_editor_projection(box_rect, zoom, …)` | RectBox × zoom → 编辑器 DOM 投影（scale/字号缩放） |

### `geometry/reflow_engine.rs` — 段落高度变化引起的纵向位移累计计算
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `calculate_reflow_displacements(v3_model)` | 读取补丁快照行数推算区域新高，按 Y 自底向上累计各区域位移 |

### `geometry/source_geometry.rs` — 源游程可视 bbox 推导（编辑会话/光标行高亮用）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `source_session_visual_bbox(session)` | 会话段落整体可视 bbox |
| pub | `source_visual_bbox_from_runs(runs)` | 非空 runs 的联合可视 bbox |
| pub | `source_line_visual_bbox_for_caret(session, baseline_y)` | 光标基线所在行的可视 bbox（字号 0.45 容差匹配） |
| pub | `source_run_visual_bbox(run)` | 单 run 可视 bbox：基线推导优先，原 bbox 有面积时兜底 |
| 私有 | `source_run_horizontal_span / inferred_run_width` | 横向跨度推导 / 由 char_origins+widths 推断 run 宽 |
| 私有 | `bbox_width / bbox_height / bbox_has_area / union_bbox` | NaN 过滤版包围盒工具（与 bbox_ops 平行的私有变体） |

### `text/mod.rs` — text 子模块清单（10 个子模块声明）

### `text/text_model.rs` — 编辑器文本双态模型
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorTextModel` (struct) | 源文本 / 当前文本双态容器 |
| pub | `new(source_text)` | 构造（current 初始化为 source） |
| pub | `source_text / current_text` | 读取两个态 |
| pub | `current_char_count()` | 当前文本 char 数 |
| pub | `is_pristine()` | 当前是否与源文本一致（未改动） |
| pub | `set_current_text(next)` | 更新当前文本，返回是否发生变化 |

### `text/index_convert.rs` — UTF-16 偏移与 char 索引互转（JS 边界契约）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `utf16_offset_to_char_index(text, offset)` | UTF-16 偏移 → char 索引（代理对取整） |
| pub | `char_index_to_utf16_offset(text, index)` | char 索引 → UTF-16 偏移 |

### `text/semantic_axiom.rs` — 语义角色公理推断引擎
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `AxiomEngine` (struct) | 无状态语义推断器 |
| pub | `infer_role(segment, parent, page_height)` | 几何公理（顶部大字→标题）+ 内容公理（日期/金额/联系方式/地址模式）推断 SemanticRole |

### `text/caret_geometry.rs` — 纯计算光标几何：停靠点、点击命中、可视位置、键盘导航
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorCaretVisualPosition` (struct) | 光标可视位置：left / baseline_y / height |
| pub | `CaretStop` / `CaretLine` (struct) | 光标停靠点（索引+left）/ 光标行（基线+高+停靠点集） |
| pub | `same_existing_session_line(ref_y, run, anchor_top)` | run 与参考基线是否同行（字号容差） |
| pub | `resolve_caret_index_from_lines(lines, x, y)` | 点击坐标 → 最近停靠点索引（y 距离权重 ×4） |
| pub | `dedupe_caret_stops(line)` | 去重相邻停靠点（同索引且间距 ≤0.5px） |
| pub | `caret_index_at_page_point(session, x, y)` | 页面点 → 光标索引（构建文本计划后解析） |
| pub | `resolve_index(session, plan, x, y)` | 会话 + 文本计划 → 点击光标索引 |
| pub | `caret_visual_for_session(session, index, h)` | 光标索引 → 可视位置（内部重建计划） |
| pub | `caret_visual_for_session_plan(session, plan, index, h)` | 同上但复用既有文本计划 |
| pub | `build_session_caret_lines(session, plan, h)` | 按 run 基线分行并从文本计划填充停靠点 |
| pub | `populate_line_stops_from_text_plan(line, plan, …)` | 由计划槽位生成行内停靠点序列 |
| pub | `resolve_navigation_from_lines(lines, index, key)` | Home/End/ArrowUp/ArrowDown 键盘导航解析 |

### `text/editable_segments.rs` — NativeTextModel 游程聚合为可编辑段与 K-V 字段分组
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `build_editable_segments(model, page_height)` | 主入口：字段分组 + 连续段聚合 + 语义推断，产出 EditableSegment 列表 |
| 私有 | `FieldLabelAnchor` / `FieldGroup` (struct) | 标签锚点 run 区间 / 标签+值区间分组 |
| 私有 | `resolve_segment_patch_key` | 段补丁键：`{object_id}::{start}-{end}` |
| 私有 | `is_colon_token / looks_like_short_field_token` | 冒号 run 判定 / 短标签块判定（≤6 字符） |
| 私有 | `resolve_run_visible_glyph_width` | run 可见宽度兜底链：自身宽→父均宽→字号 |
| 私有 | `resolve_run_style_signature` | run 样式签名串（字体/字号/色/矩阵/hints） |
| 私有 | `normalize_field_label` | 标签归一化：去冒号与空格 |
| 私有 | `detect_field_label_anchors` | 几何阵型探测 `[文本]+[:]` 的 K-V 标签锚点 |
| 私有 | `build_field_groups` | 由锚点构建标签/值区间分组 |
| 私有 | `create_editable_segment` | 由 run 区间构造 EditableSegment（含跨 run 字符原点合并） |
| 私有 | `build_contiguous_segments_in_range` | 按样式签名 + 几何间隙阈值聚合连续段 |

### `text/list_semantics.rs` — 列表行语义：marker（bullet/编号）识别与正文剥离
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ListMarkerKind` (enum) | marker 类型：None / Bullet / Numbering / Symbol / Custom |
| pub | `ListTextSemantic` (struct) | 列表行语义：是否有 marker/kind/marker 与 body 文本及字符区间 |
| pub | `parse_numbering_value(marker_text)` | 从编号 marker 文本解析数值 |
| pub | `format_numbering_marker(value, template)` | 按模板括号/分隔符格式化编号 |
| pub | `derive_list_text_semantics(text)` | 行首装饰 bullet / 编号前缀识别 + 尾部装饰字符与空白剥离 |
| 私有 | `extract_numbering_prefix(chars)` | 识别 `(1)` / `一、` / `Ⅰ.` 等编号前缀 |

### `text/search_replace.rs` — char 级查询替换
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `SearchReplaceOptions` (struct) | 大小写敏感 / 全部替换开关 |
| pub | `replace_query_matches(text, query, repl, opts)` | char 级查找替换，无匹配或结果未变返回 None |
| 私有 | `matches_query_at / slice_chars` | 位置匹配（可选忽略大小写）/ char 切片 |

### `text/style_mapper.rs` — 编辑态"文本→样式"映射器（LCP/LCS diff 保持样式）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `StyleSpan` (struct) | 样式分片：文本 + 样式 + 是否装饰文本 |
| pub | `StyleMapper` (struct) | 样式映射器：span 列表即"文本→样式"对应关系 |
| 私有 | `new_from_paragraph(paragraph)` | 由段落 runs 初始化（按下划线占比判定是否保留 underline） |
| pub | `new_from_paragraph_for_text(paragraph, text)` | 初始化并对齐到给定文本 |
| pub | `read_full_text()` | 拼接全部 span 文本 |
| pub | `update_with_text(new_text)` | LCP/LCS 差分重建 span：前缀映射、新内容继承活动样式、后缀位移映射，最后合并相邻 |
| pub | `set_bold_all / set_italic_all / set_underline_all` | 全量设置布尔样式并合并 span |
| pub | `set_color_all / set_font_name_all / set_font_size_all / set_char_spacing_all` | 全量设置颜色/字体/字号/字距并合并 |
| pub | `is_bold_any / is_italic_any / is_underline_any` | 任一 span 为真 |
| pub | `is_bold_all / is_italic_all / is_underline_all` | 全部 span 为真 |
| pub | `dominant_style()` | 首个非空非装饰 span 的样式 |
| pub | `has_style_changes_against_paragraph(paragraph)` | 与源段落逐字比较绘制样式是否变化 |
| pub | `to_layout_runs()` | 转为排版引擎 LayoutRun（坐标置零） |
| pub | `should_preserve_editor_underline(paragraph)` | 可见字符下划线占比 ≥0.8 才保留（排除装饰与符号字体） |
| 私有 | `compute_lcp_len / compute_lcs_len` | 公共前缀 / 公共后缀长度（字节） |
| 私有 | `merge_adjacent_spans / is_style_equal` | 合并相邻同样式 span / 样式等价判定 |
| 私有 | `style_spans_have_same_paint_style / expand_style_signature_by_char` | 逐字样式签名比较 |
| 私有 | `is_decorative_text` | 装饰字符表判定（与 text_predicates 平行的私有副本） |

### `text/style_preservation.rs` — 行级样式快照保持与按旧比例重新分摊文本
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `make_style_run(id, text, style)` | 构造零几何的样式快照 run |
| pub | `reindex_style_runs(runs)` | 重排 start/end 字符索引并清空几何字段 |
| pub | `resolve_dominant_paragraph_style(runs, fallback)` | 取首个 run 样式，空则回退 |
| pub | `distribute_text_across_runs(prefix, text, prev, fallback)` | 按旧 run 长度比例把新文本分摊到各 run |
| pub | `preserve_changed_line_styles(region_id, line_index, …)` | 行变更后保持装饰前缀 runs 并以光标处活动 run 样式承载正文 |
| 私有 | `is_decorative_run_text / line_selection_range` | 装饰 run 判定 / 计算行内选区范围 |

### `text/glyph_layout/mod.rs` — 微观排版引擎入口：类型定义与子模块 API 再导出
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `DecorativePrefixLayout` (struct) | 装饰前缀布局：文本/字符数/宽度/runs |
| pub | `EditorSessionTextPlan` (struct) | 编辑会话文本计划：重建文本 + 字形槽位 + raw↔重建双向索引映射 |
| pub | `EditorGlyphSlotKind` / `EditorGlyphSlot` (struct/enum) | 槽位类型（Glyph/Gap）/ 槽位：字符 + raw 索引 + 左右边界 |
| pub | `EditorSessionTextPlan::map_raw_to_reconstructed` | raw 索引 → 重建文本索引 |
| pub | `EditorSessionTextPlan::reconstructed_char_count` | 重建文本字符数 |
| pub | `EditorSessionTextPlan::map_reconstructed_to_raw` | 重建索引 → raw 索引 |
| pub | `pub use caret / layout_engine / text_predicates` | 再导出 caret 定位、text_plan 构建、装饰判定等公共 API |

### `text/glyph_layout/caret.rs` — run advance 推测、光标定位、字段点击命中、装饰前缀提取
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `infer_run_advance(run)` | 推测 run 典型 advance：字符间距差分优先，宽度/字数兜底 |
| pub | `compute_run_aware_caret_left(session, index)` | 光标索引 → 相对锚点左缘像素（前缀和 + char_origins 精确定位） |
| pub | `resolve_caret_index_for_click(session, x)` | 点击 x → 最近字符缝隙索引（O(N) 穷举最近邻） |
| pub | `resolve_field_hit_for_click(request)` | 字段点击 → Key/Value 归属 + 初始光标 + 实测双侧宽度 |
| pub | `resolve_field_hit_target_for_click(request)` | 批量字段命中（5px 容差矩形检测） |
| pub | `extract_decorative_prefix(session, is_symbol_font)` | 提取行首装饰/符号字体的前缀布局（bullet 与正文分离） |

### `text/glyph_layout/glyph_geometry.rs` — glyph 左右物理边界、视觉宽度、同行判定、典型 advance
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `glyph_left(run, i)` | 第 i 个字形左边界（char_origins 优先，advance 推算兜底） |
| pub(crate) | `glyph_right(run, i, n)` | 第 i 个字形右边界（origins/char_widths/单字形 bbox 逐级兜底） |
| pub(crate) | `glyph_visual_width(run, i, n)` | 字形视觉宽（下限 1.0） |
| pub(crate) | `same_visual_line(prev, next)` | 两 run 是否同一视觉行（字号 0.45 容差） |
| pub(crate) | `typical_contiguous_advance(run)` | 典型连续 advance（差分 35% 分位，避免词间隙充当基准） |
| pub(crate) | `should_insert_internal_gap_space(run, i, chars)` | run 内相邻字形间是否插入合成空格 |
| pub(crate) | `should_insert_visual_gap_space(prev, next)` | 相邻 run 间是否按几何间隙插入合成空格 |

### `text/glyph_layout/layout_engine.rs` — 行级上下文 delta 与 build_editor_session_text_plan 微排主循环
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `line_contextual_run_delta(runs, i)` | 行内相邻 run 间距的典型值（35% 分位） |
| pub(crate) | `needs_gap(prev, next, typical)` | 两 run 之间是否需要合成空格（单字形对用行级 delta，否则用视觉间隙） |
| pub | `build_editor_session_text_plan(session)` | 微排主循环：重建文本 + 槽位（含 run 间/run 内合成空格）+ 双向索引映射 |
| pub | `has_suspicious_run_geometry(…)` | run 几何可疑检测（bbox 与实测宽偏差超 3 倍/0.4 倍）；`#[allow(dead_code)]` |

### `text/glyph_layout/text_predicates.rs` — CJK/标点/装饰字符判定与合成间隙谓词
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `is_decorative_glyph(ch)` | 是否装饰字符（•●▪◦·○-▶➤） |
| pub | `is_decorative_text(text)` | 整串是否全为装饰字符 |
| pub(crate) | `is_cjk_unified(ch)` | CJK 统一表意/假名/谚文区段判定 |
| pub(crate) | `is_open_punctuation / is_close_punctuation(ch)` | 开/闭标点判定（ASCII + 全角） |
| pub(crate) | `should_allow_synthetic_gap(prev, next)` | 相邻字符对是否允许合成间隙（CJK 对与标点邻接除外） |
| pub(crate) | `should_insert_gap_from_origin_delta(…)` | 按原点间距与典型 advance 的阈值判定插空格 |
| 私有 | `is_spacing_punctuation / is_ascii_word_start / estimated_gap_source_advance` | 间隔标点判定 / 词首判定 / 期望 advance 估算 |

### `text/glyph_layout/tests.rs` — 微排引擎测试（原内联测试模块搬移）
覆盖点：装饰/CJK/标点谓词、合成间隙允许与阈值、同行判定、infer_run_advance、text_plan 空文本/越界/间隙映射/往返一致性、has_suspicious_run_geometry 各分支。

### `typography/mod.rs` — typography 子模块清单（4 个子模块声明）

### `typography/engine.rs` — 排版引擎门面：PDF 字体名 → 系统字体解析
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `TypographyEngine` (struct) | 字体解析门面：持有系统候选列表与回退族名 |
| pub | `new(candidates, fallback_family)` | 构造引擎 |
| pub | `resolve_pdf_font(name, hints)` | 构建匹配请求并执行系统匹配或回退，产出 ResolvedPdfFont |

### `typography/models.rs` — 字体匹配领域模型（身份、描述符、候选、结果）
| 条目 | 类型 | 含义 |
|---|---|---|
| `PdfFontSourceKind` | enum | PDF 字体来源：EmbeddedSubset / EmbeddedFull / SystemMatched / Fallback |
| `RenderFontKind` | enum | 渲染方式：Embedded / System / Fallback |
| `PdfEmbeddedFontKind` | enum | 嵌入字体技术类型：Type1 / TrueType / CidType0 / CidType2 / OpenType / Unknown |
| `NormalizedPdfFontIdentity` | struct | 归一化字体身份：原名/净名/规范族/样式/子集标记/符号性 |
| `PdfFontDescriptor` | struct | PDF 字体描述符：来源/嵌入类型/字重/斜体/等宽/serif/嵌入文件与 cmap 有无 |
| `PdfFontMatchRequest` | struct | 匹配请求：身份 + 描述符 + hints |
| `SystemFontCandidate` | struct | 系统字体候选：族/全名/PS 名/样式/字重/风格标志/覆盖分 |
| `MatchReason` | struct | 打分理由：代码 + 详情 + 分值增量 |
| `SystemFontMatchResult` | struct | 单候选打分结果：候选 + 总分 + 理由列表 |
| `ResolvedPdfFont` | struct | 最终解析结果：匹配族/渲染方式/嵌入渲染可行性/置信分/理由 |

### `typography/font_resolver.rs` — PDF 字体名 → CSS font-family 快速解析（中文字体映射表）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `resolve_font_face(name, hints)` | 剥子集前缀、拆族/样式、判符号类，映射到 CSS 渲染族并给出来源与置信度 |
| pub | `looks_like_symbolic_font(name)` | 名称是否解析为 Symbol/Dingbat 类符号字体 |
| 私有 | `strip_subset_prefix` | 剥离 `ABCDEE+` 六字符子集前缀 |
| 私有 | `split_family_and_style` | 按后缀拆分族名与样式名（regular/bold/italic…） |
| 私有 | `classify_symbol_family` | wingdings/webdings/zapfdingbats/symbol 家族归类 |
| 私有 | `resolve_render_family` | 中文字体（楷/宋/黑/雅黑/仿宋）与西文常见族的 CSS 族映射 + 兜底链 |

### `typography/matcher.rs` — 系统字体候选多维打分匹配与回退决策
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `build_match_request(name, hints)` | 由字体名 + hints 构建匹配请求 |
| pub | `build_match_request_with_descriptor(name, hints, desc)` | 带描述符构建匹配请求 |
| pub | `normalize_pdf_font_identity(name)` | 字体名归一化：剥子集/拆族/提取样式/判符号性 |
| pub | `score_system_font_candidate(request, candidate)` | 多维打分：族精确/别名/部分匹配、PS 名、全名、字重、风格、覆盖分等 |
| pub | `choose_best_match(request, candidates)` | 取最高分候选 |
| pub | `choose_top_matches(request, candidates, limit)` | 打分排序取前 N |
| pub | `resolve_system_or_fallback_font(request, candidates, fallback)` | 匹配或回退，并按嵌入文件 + cmap 判定嵌入渲染可行性 |
| 私有 | `strip_subset_prefix / split_family_name / extract_style_name` | 子集剥离 / 族名与样式名拆分 |
| 私有 | `push_reason / normalized_font_key` | 追加打分理由 / 中英文别名归一为标准键（宋体→songti 等） |

### `common/mod.rs` — common 子模块清单（debug / sanitize / trace）

### `common/debug.rs` — 调试文本截断
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `truncate_debug_text(text, limit)` | 按 char 截断超限文本并追加 "..." |

### `common/sanitize.rs` — 浮点数合法性净化
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `sanitize_positive(value, fallback)` | 非有限或 ≤0 时回退默认值 |
| pub | `sanitize_non_negative(value, fallback)` | 非有限或 <0 时回退默认值 |

### `common/trace.rs` — 统一结构化追踪设施（可插拔 subscriber + RAII span）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `TraceLevel` (enum) | 级别：Info / Debug / Trace（由详到简） |
| pub | `TraceField` (struct) | 结构化字段：key（Cow）+ value |
| pub | `TraceEvent` (struct) | 结构化事件：级别 + node + action + fields |
| pub | `TraceSubscriber` (trait) | 可插拔后端接口（buffer/console/log_service/no-op） |
| pub | `NoOpSubscriber` (struct) | 空实现（默认，零成本） |
| pub | `set_subscriber(Option<Box<dyn …>>)` | 安装/还原 subscriber（thread_local 注册表） |
| pub | `set_max_level(level)` | 设置最大输出级别，超出即静默丢弃 |
| pub | `emit(level, node, action, fields)` | 事件统一发射口：级别过滤后分发 |
| pub | `field(key, value)` | TraceField 便捷构造 |
| pub | `TraceSpan` (struct) | RAII span：构造发 begin，finish 发 end，drop 未 finish 发 aborted |
| pub | `TraceSpan::begin / finish` | 开始计时并发射 begin / 发射 end（result + elapsedMs） |
| pub | `trace_span!` (macro) | 便捷宏：字段打包并绑定 span 到局部变量 |

## 疑点
- **同名不同物**：`models/document_runtime.rs` 与 `edit/document_runtime.rs` 同名但毫无关联——前者是文档级 serde DTO（PageState / EditIntent / ReadDocumentMeta / 分页与页面操作命令），后者是编辑器运行时逻辑（`EditorResolvedDocumentState` + `resolve_document_state`，由 `edit/document_edit_ops.rs` 与 UI 侧消费）。查代码时极易误导航，建议其一改名。
- **疑似死代码**：`geometry/reflow_engine.rs::calculate_reflow_displacements` 全仓无调用方（仅 mod.rs 挂载）；`geometry/layout_engine.rs` 的 `layout_anchored_pair` 与 `find_paragraph_at` 亦无调用方；`glyph_layout/layout_engine.rs::has_suspicious_run_geometry` 已标 `#[allow(dead_code)]`，仅经 `test_reexports` 被测试引用。
- **reflow_engine 隐藏依赖与坐标疑点**：纯几何模块内直读 `persistence::patch_store::GLOBAL_PATCH_STATE` 全局锁；且按"bbox.bottom 越大越靠上"的 Y-Up 语义排序累计位移，与全库 Y-Down 规范相悖——若上游未保证传入未翻转的 LayoutInferenceResult，位移方向会算反（鉴于该函数无调用方，风险暂为潜伏）。
- **疑似重复实现**：`geometry/source_geometry.rs` 私有重写了与 `bbox_ops` 平行的 `union_bbox/bbox_width/bbox_height`（仅多 NaN 过滤）；`text/style_mapper.rs` 私有 `is_decorative_text` 与 `text_predicates::is_decorative_glyph` 重复维护同一装饰字符表；`text/caret_geometry.rs` 与 `glyph_layout/caret.rs` 存在两套点击→光标索引算法（CaretLine 停靠点最近邻 vs run 遍历最近邻），分别服务 body 与 field 路径。
- **字段/结构遗留**：`models/styled_run.rs::NativeTextModel` 同时保留 `rendering_mode(i32)` 与 `render_mode(i64)` 两个历史版本字段；`NativePathObject/NativeImageObject` 为空占位结构；`models/glyph.rs::GlyphPaintPlan::flip_y` 只翻转 `ExternalObject::Image`，Path 变体的坐标不做处理。
- **两套字体解析模型并存**：`models/font.rs`（ResolvedFontIdentity/ResolvedFontFace，CSS 快路径）与 `typography/models.rs`（NormalizedPdfFontIdentity/ResolvedPdfFont，打分匹配）语义相近、命名重叠，消费方分别集中在 models 与 render，易混淆但分工明确（快速渲染族映射 vs 系统字体精确匹配）。
