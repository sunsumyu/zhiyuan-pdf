# src-tauri · 字体引擎与服务层

> 范围：`src-tauri/src/infrastructure/pdf/font/`（10 个文件，2682 行：mod / catalog / embed / face / layout / match_mod / metrics / parse / path / ttc）+ `infrastructure/pdf/` 服务层 7 个文件（2159 行：annotation_store / commands / document_resolver / document_service / log_service / page_intermediate_service / pdf_loader）+ `infrastructure/mod.rs`（2 行），共 18 个文件约 4843 行。
> 上游：lopdf（Document / Dictionary / Stream / Object）；ttf_parser / fontdb / cosmic_text + swash（字体解析、查询与度量）；windows-sys GDI（系统字体枚举）；`pdf_viewer_core`（models 的 FontHints / LayoutRun / NativeTextModel、typography 的 TypographyEngine 与 matcher、geometry::layout_engine 的 layout_paragraph、render::paint_plan、common::trace）；本地 `models.rs`（PathSegment / TextReflowPatch / PageDisplayList / PdfModifications 等再导出）；`AppState`（docs / cache / history 三组共享锁）。
> 下游：`pdf_write/reflow.rs`（resolve_text_write_font / break_text_into_lines / PdfTextWriteFont::encode_text 的调用方）、`pdf_read/content_parser.rs` 与 `path_resolver.rs`（parse_font_from_dict / resolve_glyph_geom / ResourceCache）、`vector_engine.rs`（显示列表 / 矢量模型 / 布局推断）、`region_materializer.rs`（save_pdf 物化计划）、`application/pdf/edit_commands.rs` 与 `page_annotation.rs`、`interfaces/pdf/document.rs` / `render.rs` / `system.rs`（Tauri 命令入口）、`lib.rs`（日志宏与 trace 桥安装）。

## 职责
字体引擎负责"写路径"的字体解析链：按需把 PDF 原字体或系统替换字体解析为可编码、可嵌入的 Type0 字体对象，并为读路径提供 CMap / 宽度 / 字形几何解析。文档与页面服务管理文档生命周期（打开 / 保存 / 回滚 / 重做 / 工作副本）与页中间产物的三级缓存（显示列表 → 矢量模型 → 布局推断 / 绘制计划）。存储与日志提供注释只读提取、编辑命令对象，以及分级日志、环形事件日志与 core trace 桥接。

## 文件与方法
（各文件内联 `#[cfg(test)] mod tests` 不列入；match_mod / parse / embed / mod 的内联测试覆盖纯辅助函数，pdf_loader 覆盖 xref 修复分支，document_service 覆盖 transfer_snapshot。）

### `font/mod.rs` — 字体链枢纽：SystemFont 数据契约 + PdfTextWriteFont + resolve_text_write_font 编排
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | 九个子模块声明 + `pub use` | 汇出 parse / layout / path 关键类型与 core FontHints |
| pub | `PdfTextWriteFont` (struct) | 写路径字体句柄：资源别名 + ParsedFont + 编码策略 + 来源标签 |
| pub | `PdfTextWriteFont::encode_text(text)` | 按编码策略产出文本字节：原字体 CMap 编码或 TrueType glyph id 双字节序列 |
| pub | `PdfTextWriteFont::source_label()` | 返回字体来源标签（诊断用） |
| 私有 | `PdfTextWriteEncoding` (enum) | OriginalPdfFont / TrueTypeGlyphIds 两种编码策略 |
| pub(crate) | `SystemFont` (struct) | 解析链数据契约：候选系统字体的字节 / face_index / 字形子集 / 度量全量信息 |
| pub | `resolve_text_write_font(doc, page_id, alias, current_font, text)` | 链入口：原字体可编码则复用；否则 match_mod 找系统字体 → embed 入页资源 → face 转 ParsedFont，跨族替换记告警 |
| 私有 | `can_pdf_font_encode_text(font, text)` | 逐字符判原字体可编码，全空白视为不可编码 |
| 私有 | `target_weight_for(current_font)` | 名含 bold 或 weight≥600 时要求粗体替换面 |

### `font/face.rs` — 系统字体字节解析：TTF 提取、覆盖校验、字形子集与度量
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `font_from_bytes(data, face_index, family, label, text)` | 字节→SystemFont：提取 TTF、缺字即拒绝、收集字形子集、度量换算 1/1000 em |
| pub(crate) | `standalone_ttf_bytes(data, face_index)` | TTF 直传 / TTC 抽 face / CFF（OTTO）拒绝并记日志 |
| 私有 | `font_covers_text(face, text)` | 除换行外每个字符都有 glyph |
| 私有 | `missing_chars(face, text)` | 缺字码点清单，供拒绝日志 |
| 私有 | `glyph_subset(face, text)` | 去重收集 (char, gid, advance)，advance 换算 PDF 单位 |
| pub(crate) | `encode_text_as_glyph_ids(bytes, face_index, text)` | 文本→2 字节 glyph id 序列（Identity-H），缺字报错 |
| pub(crate) | `parsed_font_from_system_font(font)` | SystemFont→ParsedFont：双向宽度表 + CMap + Type0 / 已嵌入标记 |
| 私有 | `post_script_name(face)` | 从 name 表读 PostScript 名 |

### `font/match_mod.rs` — 系统字体查找器（finder）：候选名展开 + fontdb 查询 + 兜底 + 匹配器
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PdfSystemFontMatcher` (struct) | PDF 字体名→系统字体的带缓存匹配器（包装 core TypographyEngine） |
| pub | `new(candidates, fallback_family)` | 注入候选清单与兜底族名 |
| pub | `resolve(pdf_font_name, hints)` | 缓存命中直返；否则 TypographyEngine 解析并记缓存与日志 |
| pub | `candidate_count()` | 返回候选数量 |
| pub | `resolve_native_text(text)` | NativeTextModel 构造 PdfFontDescriptor 后走 core 解析 |
| 私有 | `maybe_log_resolution(…)` | 仅低置信 / 回退结果记日志，附 top3 候选排名 |
| 私有 | `build_cache_key(name, hints)` | 名称 + 五项 hints 拼缓存键 |
| 私有 | `build_native_text_cache_key(text)` | 追加描述符字段的完整缓存键 |
| 私有 | `map_embedded_font_kind(subtype)` | 子类型字符串→PdfEmbeddedFontKind |
| pub(crate) | `candidate_font_names(current_font)` | 原字体各名称变体优先，后接固定 CJK 兜底名单 |
| 私有 | `name_variants(name)` | 分隔符变体展开 + GBK 乱码名映射规范字体名 |
| 私有 | `push_unique(out, value)` | 忽略大小写去重收集 |
| pub(crate) | `strip_subset_prefix(name)` | 去 "ABCDEF+" 子集前缀（6 大写字母+加号） |
| pub(crate) | `find_system_font(names, text, weight)` | fontdb（系统 + 受管目录）逐名查询，首个全覆盖者胜，失败附缺字诊断 |
| 私有 | `load_managed_font_dirs(db)` | 加载 assets/fonts 等四个受管字体目录 |
| 私有 | `query_fontdb(db, family, text, weight)` | fontdb 命中→face::font_from_bytes，未中转 known files 探测 |
| 私有 | `probe_known_font_files(family, …)` | 硬编码 C:\Windows\Fonts 的 msyh / simsun / simhei 兜底读取 |
| 私有 | `missing_text_diagnostics / source_label` | 失败诊断串 / 字体来源标签构造 |

### `font/catalog.rs` — Windows GDI 系统字体清单枚举
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `load_system_font_candidates()` | Windows 枚举全系统字体为 SystemFontCandidate 并富化去重排序；非 Windows 恒空 |
| 私有 | `enum_font_proc` | GDI 回调：LOGFONTW/TEXTMETRICW→候选（风格 / 衬线 / 等宽 / 符号 / 覆盖分） |
| 私有 | `enrich_and_dedupe_candidates` | 别名展开 + 七字段键去重 + 按覆盖分排序 |
| 私有 | `wide_to_string(buffer)` | UTF-16 宽字符转 String |
| 私有 | `expand_candidate_aliases(candidate)` | 主条目 + 族别名变体，覆盖分取两者最大 |
| 私有 | `infer_style_name / build_full_name / build_postscript_name / sanitize_postscript_token` | 风格名与全名 / PostScript 名推导（保留汉字） |
| 私有 | `alias_families(family)` | 宋体 / 黑体 / 雅黑 / 楷体等族别名静态表 |
| 私有 | `estimate_windows_coverage_score(family, symbolic)` | 按族名启发式打覆盖分（CJK 40 / 常用西文 24） |

### `font/embed.rs` — PDF 对象构造：把 SystemFont 嵌为页资源的 Type0 字体
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub(crate) | `sanitize_pdf_name(value)` | 过滤 PDF 名非法字符，空回退 HsaWriteFont，截 48 字节 |
| pub(crate) | `truncate_log(value, limit)` | 按字符数截断加省略号 |
| pub(crate) | `ensure_font_in_page(doc, page_id, font, text)` | 按 alias 查重后把 Type0 字体挂入页资源 Font 字典，返回资源别名 |
| 私有 | `type0_font_object(doc, font)` | 构造 FontFile2 流 + FontDescriptor + CIDFontType2 + ToUnicode + Type0 五对象 |
| 私有 | `width_array(font)` | 生成 W 数组：连续 gid 合并为 [start [w…]] 段 |
| 私有 | `to_unicode_cmap(ps_name, font)` | 生成 bfchar CMap（100 条/块），gid→UTF-16BE hex |
| 私有 | `utf16be_hex(ch)` | 字符→UTF-16BE 十六进制（含代理对） |
| 私有 | `page_resources / page_resource_dictionary` | 读页字典 Resources（直接字典或间接引用两种形态） |
| 私有 | `font_alias(ps_name, text)` | PS 名 + 排序去重字符集哈希 → "HSAW_名_哈希" 别名 |

### `font/layout.rs` — 写路径重排的段落排版适配（core 委托）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `break_text_into_lines(text, runs, font, size, max_width, align, line_height, char_spacing, scale_x)` | 组装 LayoutParagraph 委托 core layout_paragraph，宽度回调 ParsedFont.resolve_text_width |

### `font/metrics.rs` — cosmic-text/swash 字符宽度查询（当前无调用方）
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `FONT_SYSTEM / METRIC_CACHE` (lazy_static) | 全局 FontSystem 互斥体 / 按族名+字符的宽度缓存 |
| pub | `get_character_width_pdf_units(family, ch)` | 缓存查→cosmic_text 排版取 glyph→swash advance 换算 1/1000 em，回写缓存 |

### `font/parse.rs` — PDF 字体字典解析与字形几何（读路径）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `CMap` (struct) | 码→文本 与 文本→码 双向映射表 |
| pub | `CMap::new() / from_codepoint_pairs(pairs)` | 空构造 / 按 (code, text) 对构建双向映射 |
| pub | `ParsedFont` (struct) | 字体解析结果：名称 / 子类型 / CMap / 宽度表 / hints / 嵌入标记 |
| pub | `ParsedFont::is_multibyte()` | 子类型为 Type0 即双字节编码 |
| pub | `ParsedFont::can_encode(ch)` | CMap 反查存在或 ASCII 即可编码 |
| pub | `ParsedFont::encode_text(text)` | 按 CMap 编码字节（Type0 两字节），缺字产 ? / 0x0000 |
| pub | `ParsedFont::resolve_text_width(text, size, spacing, h_scale)` | 宽度表逐字累计 PDF 单位文本宽度 |
| pub | `resolve_glyph_geom(data, font, …)` | 原始字节解码：双/单字节码→文本（缺 CMap 只留几何）、符号码两段补丁，返回 (文本 / 原点 / 宽 / 码 / 总宽) |
| pub | `read_cmap(data)` | 解析 ToUnicode CMap 的 bfchar / bfrange（数组与标量两式） |
| 私有 | `hex_to_string(hex)` | hex→UTF-16BE 解码（含代理对），退化按 2 位字节 |
| pub | `ParsedImage` (struct) | 提取图像：data_url / mime / 提取方式 |
| pub | `ResourceCache` (struct) | ObjectId→ParsedFont / ParsedImage 两张页级缓存 |
| pub | `ResourceCache::new()` | 空缓存构造 |
| pub | `parse_font_from_dict(doc, font_id, name_bytes)` | 字典解析：BaseFont、DescendantFonts 的 DW/W、FontDescriptor→hints、ToUnicode→CMap |

### `font/path.rs` — 路径段简化（Douglas-Peucker）
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `simplify_path_segments(segments, epsilon)` | 连续 move/line 折线合并抽稀，bezier/close 原样保留 |
| 私有 | `simplify_points(points, epsilon)` | 递归 Douglas-Peucker 抽稀点列 |
| 私有 | `perpendicular_distance(p, p1, p2)` | 点到线段的垂直距离 |

### `font/ttc.rs` — TTC 容器抽取独立 TTF
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `extract_ttc_face_as_ttf(data, face_index)` | 校验 ttcf 头与 face_index，按表拷贝重建 sfnt（补 4 字节对齐、重算校验和与 head checkSumAdjustment） |
| 私有 | `SfntTableRecord` (struct) | 表记录：tag / checksum / offset / length |
| 私有 | `sfnt_search_params(num_tables)` | 计算 searchRange / entrySelector / rangeShift |
| 私有 | `checksum(data)` | 4 字节分块累计和 |
| 私有 | `align4(value)` | 4 字节向上对齐 |

### `document_service.rs` — 文档生命周期：打开 / 保存 / 回滚 / 重做 / 资源释放
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `PdfDocumentService` (struct) | 文档级服务空结构（纯命名空间） |
| pub(crate) | `resolve_working_path(original_path)` | 转调 document_resolver 同名函数 |
| pub | `release_pdf_resources(state, path)` | 释放工作副本 + 文档 / 事务 / 加载状态 / 物化报告并清图像缓存、失效页与布局缓存 |
| pub | `release_all_pdf_resources(state)` | 全量清空各缓存、历史与加载状态并释放全部工作副本 |
| pub | `open_pdf(app_handle, state, path)` | 缓存命中返页数；否则后台线程 lenient 加载入缓存；lopdf 0 页时回退 pdf-rs 计页 |
| pub | `save_pdf(state, path, modifications)` | 物化区域补丁→逐页 apply_batch_reflow_to_doc→落盘→换缓存→失效缓存→存物化报告 |
| pub | `rollback_pdf(state, path)` | 从 undo 快照恢复（transfer_snapshot）并落盘、失效缓存 |
| pub | `redo_pdf(state, path)` | 从 redo 快照恢复并落盘、失效缓存 |
| pub | `generate_demo_pdf(path)` | 写一张 Helvetica "Demo" 的最小 PDF 文件 |
| 私有 | `transfer_snapshot(from, to, path, current)` | 弹最新快照，当前档入目标栈（限 HISTORY_LIMIT） |

### `document_resolver.rs` — 工作副本与文档加载的统一入口
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `WORKING_COPIES / COPY_LOCKS` (lazy_static) | 原路径→临时工作副本路径表 / 每路径复制互斥锁 |
| pub(crate) | `resolve_working_path(original_path)` | md5(路径) 命名 temp 工作副本，锁内不存在才复制 |
| pub(crate) | `release_working_copy(path)` | 删工作副本与锁条目及临时文件 |
| pub(crate) | `release_all_working_copies()` | 清空注册表并删除全部临时文件 |
| pub(crate) | `ensure_loaded(app_state, path)` | 缓存命中直返；否则从工作副本 load_pdf_public 阻塞加载入缓存并更新加载状态 |

### `page_intermediate_service.rs` — 页中间产物服务：显示列表 / 矢量模型 / 布局推断 / 绘制计划的三级缓存
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(crate) | `PageIntermediateBundle` (struct) | 矢量页模型 + 字形绘制计划打包 |
| pub | `PdfPageIntermediateService` (struct) | 页中间服务空结构（纯命名空间） |
| pub(crate) | `resolve_page_display_list_from_app_state(…)` | 版本化键查显示列表缓存，miss 时 ensure_loaded + vector_engine.resolve_display_list 回填 |
| pub(crate) | `resolve_vector_page_model(state, …)` | 转调 _from_app_state（tauri::State 门面） |
| pub(crate) | `resolve_vector_page_model_from_app_state(…)` | 查页缓存→显示列表→build_vector_page_model_from_display_list 回填 |
| pub(crate) | `resolve_layout_inference_from_app_state(…)` | 查布局缓存→显示列表→resolve_layout_inference_from_display_list 回填 |
| pub(crate) | `resolve_glyph_paint_plan(state, …)` | 转调 _from_app_state（tauri::State 门面） |
| pub(crate) | `resolve_glyph_paint_plan_from_app_state(…)` | 布局推断→core build_glyph_paint_plan |
| pub(crate) | `resolve_page_asset_bundle(…)` | 组合 model+paint_plan，按 image_only / text_only 过滤 RenderObject |

### `pdf_loader.rs` — 宽松 PDF 加载与 xref 修复
| 可见性 | 方法 | 功能 |
|---|---|---|
| pub | `load_pdf_public(path)` | load_pdf_lenient 的公开包装 |
| pub(crate) | `load_pdf_lenient(path)` | 三级策略：直接 load → 读字节 load_mem → 修复后加载 |
| pub(crate) | `repair_and_load(raw)` | 3a 截 %%EOF 尾部垃圾重试；3b/3c 追加修正 trailer 重试 |
| pub(crate) | `repair_startxref(raw)` | 定位最新 xref 段，追加修正 startxref 的 trailer（append-only，不删原字节） |
| 私有 | `rfind_bytes(haystack, needle)` | 字节级反向查找 |
| 私有 | `last_declared_startxref_offset(raw)` | 读文件尾声明的 startxref 值 |
| 私有 | `verify_startxref_target(raw, offset)` | 32 字节窗口判定偏移处形似 xref 表或 obj 头 |
| 私有 | `locate_last_xref_offset / locate_last_xref_stream_object / obj_header_start` | 找最新经典 xref 表或 /Type /XRef 流对象的对象头 |

### `annotation_store.rs` — PDF 注释只读提取（高亮 / 文本批注）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `StoredPdfHighlight` (struct) | 高亮：id / rect / color |
| pub | `StoredPdfComment` (struct) | 批注：id / rect / color / contents |
| pub | `read_page_highlights(doc, page_num)` | 遍历页 Annots 取 Highlight，Rect 转 top-down 坐标，色缺省黄 |
| pub | `read_page_comments(doc, page_num)` | 遍历页 Annots 取 Text 注释读 Contents，色缺省蓝 |
| 私有 | `read_page_annotation_refs(doc, page_id)` | Annots 数组 / 间接引用统一取 ObjectId 列表 |
| 私有 | `read_page_height(doc, page_id)` | MediaBox 高 \|y1−y0\| |
| 私有 | `parse_rect_array / parse_color_array` | 4 元 / 3 元数值数组解析 |
| 私有 | `pdf_rect_to_top_down_box(rect, page_height)` | PDF 坐标转左上原点 [left, top, w, h] |

### `log_service.rs` — PDF 日志服务：分级终端输出 + 环形事件日志 + core trace 桥接
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `PDF_LOG_LEVEL` (static AtomicU8) | 全局日志级别（0 静默 / 1 INFO / 2 DEBUG / 3 TRACE） |
| pub | `PDF_EVENT_LOG_MUTEX` | 事件日志互斥（测试隔离用） |
| pub | `set_pdf_log_level / get_pdf_log_level` | 写 / 读全局级别 |
| pub | `clear_pdf_event_log / read_pdf_event_log` | 清空 / 快照环形事件日志 |
| 私有 | `timestamp / level_label / level_color / layer_for_event / format_fields` | 行格式化辅助（层级按事件名归类 DOC/ASSET/PREVIEW/CACHE） |
| 私有 | `format_layered_line / format_plain_event_line` | ANSI 彩色 / 纯文本行组装 |
| 私有 | `record_pdf_event(line)` | 写入 512 条环形缓冲 |
| pub | `log_pdf_event(level, event, fields)` | 级别门槛 + 记录 + 终端输出 |
| pub | `log_terminal_message(message)` | 直写 stderr |
| pub | `PdfEventSpan` (struct) | begin/end 成对事件，Drop 兜底记 aborted |
| pub | `PdfEventSpan::begin / finish` | 记 begin / 记 end + result + 耗时 |
| pub | `ProfileSpan` (struct) | Drop 时输出 [PROF][SPAN] 耗时 |
| pub | `LogServiceSubscriber` (struct) | TraceSubscriber 适配器：core trace 事件转 log_pdf_event |
| pub | `install_core_trace_bridge()` | 安装适配器（幂等，启动时调用一次） |
| 宏 | `pdf_log! / log_step! / log_audit! / prof_span!` | 分级 eprintln / INFO 捷径 / TRACE 捷径 / 耗时 span |

### `commands.rs` — PDF 编辑命令对象（命令模式，转调 PdfDocExt）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub use pdf_viewer_core::models::{…}` | 再导出核心命令类型（删页 / 旋转 / 插页 / 高亮 / 元数据） |
| pub | `PdfEditCommand` (trait) | execute(doc, page_num) 编辑契约 |
| pub | `ReplaceTextCommand` | 简单文本替换 → apply_text_patch |
| pub | `PersistableRegionPatchCommand` | 区域补丁 → apply_atomic_reflow_to_doc |
| pub | `TextReflowCommand` | 物化重排 → apply_atomic_reflow_to_doc |
| pub | `BatchTextReflowCommand` | 批量重排 → apply_batch_reflow_to_doc |
| pub | `ReplaceImageCommand` | 图像替换 → replace_image_xobject（底层恒 Err 占位） |
| pub | `DeletePageCommand / RotatePageCommand / InsertPageCommand / AddHighlightCommand` | trait 实现，转调 PdfDocExt 对应方法 |
| pub | `AddCommentCommand / UpdateCommentCommand / DeleteAnnotationCommand` | 注释增 / 改 / 删转调 |
| pub | `UpdateMetadataCommand` | 元数据更新转调 |

### `infrastructure/mod.rs` — 基础设施层模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod pdf / pdf_fallback` | 两个子层声明（pdf 为主管线，pdf_fallback 为 pdf-rs 回退后端） |

## 疑点
- **死代码**：`font/metrics.rs` 的 `get_character_width_pdf_units`（连同 `FONT_SYSTEM` / `METRIC_CACHE` 两个全局量）全 workspace 无任何调用方；`font/catalog.rs` 的 `load_system_font_candidates` 全 workspace 无调用方（其产出的 SystemFontCandidate 本应喂给字体匹配器）；`font/match_mod.rs` 的 `PdfSystemFontMatcher`（resolve / resolve_native_text / candidate_count）仅被自身内联测试使用，生产代码无消费方——三者均为悬空 / 预留代码。
- **疑似重复（font/ 与 crates 侧）**：match_mod.rs 内同时存在两套字体查找机制——自有 finder 链（`find_system_font` + `probe_known_font_files`，供写路径嵌入）与包装 core `TypographyEngine` 的 `PdfSystemFontMatcher`（读路径匹配，现无调用方）；后者与 `pdf_viewer_core::typography`（engine / matcher / font_resolver）职责重叠。另有小口径重复：`strip_subset_prefix` 与 metrics.rs 内联的子集前缀剥离逻辑重复；`annotation_store::read_page_height` 与 `pdf_utils::read_page_size`、`pdf_write/annotations.rs::read_page_height` 三处各自读 MediaBox。
- **与 CONTEXT.md 术语出入**：CONTEXT.md 写"字体解析链 — pdf_write_font/ 子目录：finder→face→embed"，实际目录为 `src-tauri/src/infrastructure/pdf/font/`（finder 职责现落在 `match_mod.rs`，文件头注释仍留 "from finder.rs" / "from matching.rs" / "from pdf_write_font/mod.rs" 的迁移痕迹）；`SystemFont` 数据契约与链入口 `resolve_text_write_font` 定义在 `font/mod.rs`，并非独立 finder 文件。CONTEXT.md 称 PdfTextState "仅在 pdf_write.rs 内部可见"亦与现状不符（见 tauri-infra-readwrite.md）。
- **其他**：`document_service.rs::release_pdf_resources` 首尾各调一次 `release_working_copy`（行 20 与行 55，幂等但冗余）；`open_pdf` 直读原始路径加载，而 `ensure_loaded` / `save_pdf` 走 temp 工作副本——同一文档缓存存在两种装载来源，缓存内容可能因路径不同而分叉；`commands.rs` 的 `ReplaceImageCommand` / `InsertPageCommand` 指向恒 Err 的占位实现。
