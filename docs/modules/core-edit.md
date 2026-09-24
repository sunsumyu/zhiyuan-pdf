# pdf-viewer-core · edit（文本编辑领域逻辑）

> 范围：`crates/pdf-viewer-core/src/edit/`（30 个文件，约 6860 行，含子目录 `document_plan/`、`draft_layout/` 与内联测试）。
> 上游：`crate::models`（BoundingBox / GlyphPaintParagraph / GlyphPaintRun / LayoutParagraph / LayoutRun / ParagraphEditContext / PageState / VectorPageModel / StyledRun / VisualMarker 等）、`crate::text`（glyph_layout / list_semantics / style_mapper / text_model）、`crate::geometry`（source_geometry / bbox_ops / layout_engine）、`crate::typography::font_resolver`、`crate::persistence::models`（PersistableRegionPatch）、`crate::common`（trace / debug）。
> 下游：core 内 `render/effective_page_plan`（覆盖层与源抑制，消费 paragraph_overlay / replacement_region / active_target / source_identity / paragraph_scene）、`annotation` / `document` / `history`（editor_types DTO）、`persistence::patch_store`；UI crate `pdf-viewer-ui` 的 `editor/*`（editor_controller、workflow、activation、editor_api/helpers、overlay/*、format/list_format、session、engine_state、debug_trace）与 `render/canvas_overlay.rs`。
> 深入阅读：[docs/edit-save-architecture.md](../edit-save-architecture.md)、[docs/editor-render-architecture.md](../editor-render-architecture.md)。

## 职责
把 PDF 段落（vector 模型 / paint 计划）转换为可编辑的领域对象：编辑目标解析与段内分段、marker/body 会话切分、源文本重建与索引映射。编辑会话的活状态（文本、样式、caret、选区）与文本增删、草稿排版计划（布局 + caret 停靠点）、替换区域/替换快照等持久化前置数据也在此定义。全模块为纯 Rust 领域逻辑（无 wasm 依赖），唯一的 thread_local 是调试事件环形缓冲。

## 文件与方法
（各文件内联 `#[cfg(test)] mod tests` 不列入；`document_plan/tests.rs`、`draft_layout/tests.rs` 仅列覆盖点。）

### `mod.rs` — 模块清单
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `pub mod` × 18 | 声明全部子模块，无再导出、无别名 |

### `active_target.rs` — 活动编辑目标：正在编辑段落的完整快照 DTO
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ActiveEditorTarget` (struct) | 段落 id、bbox、字体样式、初始 caret、编辑会话与场景 |
| pub | `Default::default()` | 空目标默认值 |
| pub | `source_body_text(&self) -> &str` | 取文档计划中的 body 源文本 |
| pub | `initial_body_caret_index(&self) -> usize` | 取 body 初始 caret |

### `bridge.rs` — 渲染计划与持久化补丁之间的装配桥（交互目标收集、编辑目标/补丁组装）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ParagraphInteractionTarget` (struct) | 交互目标 DTO：文本、bbox、样式与源对象索引 |
| pub | `collect_paragraph_interaction_targets(plan, vm) -> Vec<…>` | 遍历区域段落收集全部可交互编辑目标 |
| pub | `build_paragraph_patch(plan, vm, paragraph_id, new_text)` | 构建段落文本替换补丁（无自定义 runs） |
| pub | `build_paragraph_patch_with_runs(…, new_runs)` | 同上，可携带新样式 runs |
| 私有 | `active_editor_target_from_scene(plan, paragraph, scene)` | 由场景组装 ActiveEditorTarget |
| pub | `build_active_editor_target(plan, vm, id, click_x, click_y)` | 按点击点构建目标（打开编辑器） |
| pub | `build_paragraph_render_target(plan, vm, id)` | 无点击点构建渲染用目标 |
| pub | `resolve_paragraph_shell_bbox(plan, id) -> Option<BoundingBox>` | 解析段落 shell 包围盒 |
| 私有 | `resolve_target_indices_from_runs(runs, vm) -> Vec<usize>` | 提取 runs 的源对象索引（转调 source_identity） |

### `debug_trace.rs` — 编辑器调试事件追踪（thread_local 环形缓冲）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorDebugField` (struct) | 键值调试字段 |
| pub | `EditorDebugTraceEvent` (struct) | seq / node / action / details 事件 |
| 私有 | `EditorDebugTraceState` (struct) | seq 计数与事件缓冲 |
| thread_local | `EDITOR_DEBUG_TRACE` | 事件环形缓冲，上限 240 条 |
| pub | `editor_debug_field(key, value) -> EditorDebugField` | 构造调试字段 |
| pub | `record_editor_debug_event(node, action, details)` | 转发统一 trace 并写入本地缓冲 |
| pub | `resolve_editor_debug_trace() -> Vec<…>` | 导出全部缓冲事件 |

### `document_edit_ops.rs` — 文本增删纯操作（对活状态应用变更）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorTextMutation` (struct) | 变更结果：新文本 + 新 caret |
| pub | `insert_text(state, inserted_text) -> EditorTextMutation` | 在 caret 处插入文本 |
| pub | `delete_backward(state) -> EditorTextMutation` | 退格删除前一字符（起点 no-op） |
| pub | `delete_forward(state) -> EditorTextMutation` | 删除 caret 处字符（末尾 no-op） |

### `document_runtime.rs` — 编辑段落文档状态解析（供增删操作读取的统一视图）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorResolvedDocumentState` (struct) | 源/当前文本、字符向量、caret、pristine/slot 标志 |
| pub | `char_count(&self) -> usize` | 可变字符数 |
| pub | `chars_to_text(chars) -> String` | 字符向量转字符串 |
| pub | `resolve_document_state(state) -> EditorResolvedDocumentState` | 汇总文本/caret/pristine 等编辑前状态 |

### `edit_target.rs` — 编辑目标切分：段落按视觉行与间隙切成可编辑分段
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditorEditTarget` (struct) | 目标 id、基段落 id、会话、源 run 索引与对象 id |
| pub | `make_edit_segment_target_id(base, key) -> String` | 生成 `base::edit-segment::key` 复合 id |
| pub | `edit_target_base_paragraph_id(target_id) -> &str` | 从复合 id 还原基段落 id |
| pub | `edit_target_segment_key(target_id) -> Option<&str>` | 提取 segment key |
| pub | `collect_edit_targets_from_session(base, session) -> Vec<…>` | 按视觉行/间隙把会话切为编辑目标 |
| pub | `resolve_edit_target_from_session(base, id, session, click)` | 显式 segment 优先、否则按点击最近打分 |
| 私有 | `VisualSegment` (struct) | 行内连续 run 段（键 + run 索引） |
| 私有 | `build_visual_segments(session)` | 排序 run、按行分组、按间隙切段（列表行整行成段） |
| 私有 | `group_runs_by_visual_line(runs)` | 按 y 容差聚合视觉行 |
| 私有 | `line_is_list_like(runs) -> bool` | 判定行是否列表项（语义/符号字体/符号字符） |
| 私有 | `visual_segment_from_indices(indices)` | 生成 `r{start}-{end}` 键 |
| 私有 | `build_segment_target(base, session, segment)` | 由段构建独立编辑目标 |
| 私有 | `whole_session_target(base, session)` | 整段回退目标 |
| 私有 | `normalize_paragraph_to_bbox(paragraph, bbox)` | 以 bbox 归一化段落原点与 wrap 宽 |
| 私有 | `bbox_from_layout_runs(runs)` | 源视觉 bbox 优先的 run 并集 |
| 私有 | `line_sort_key(run) -> f32` | 行 y 量化键 |
| 私有 | `same_visual_line(ref_y, run) -> bool` | 行归属容差判定（0.55×字号） |
| 私有 | `segment_break_gap(prev, next) -> f32` | 断段间隙阈值（字号/均宽启发） |
| 私有 | `target_hit_score(bbox, x, y) -> f32` | 点击到 bbox 的加权距离（垂直优先） |

### `editor_types.rs` — 会话状态、错误、统一响应与编辑器 DTO（*_types 纯 DTO）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `SessionState` (enum) | Viewing / Editing / EditingBlock / Saving 四态 |
| pub | `SessionState::as_str()` | 状态名 |
| pub | `EditorError` (enum) | InvalidState / NotFound / NotImplemented / Internal / IoError |
| pub | `EditorResponse<T>` (struct) | ok / data / error / render 统一响应封套 |
| pub | `HitTestResult` / `PagePointDto` / `ClientPointDto` | 命中测试与坐标 DTO |
| pub | `OpenBlockResult` / `MoveCaretResult` / `SyncInputResult` / `ApplyCommandResult` / `SetEditModeResult` / `CommitResult` | 打开块、移 caret、输入同步、命令应用等操作结果 DTO |
| pub | `SnapshotResult` / `TextBlockInfo` / `FormatState` / `TextSelection` / `TextLineDto` | 会话快照、块信息、格式状态、选区与文本行 DTO |

### `engine_state.rs` — 段落编辑会话活状态：文本模型 + 样式映射 + 格式命令
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `LiveEditorParagraphState` (struct) | target、text_model、style_mapper、列表/对齐/行高、caret/选区、revision/dirty |
| 私有 | `alignment_label(align) -> &str` | 对齐枚举转标签 |
| 私有 | `list_kind_label(kind) -> &str` | 列表类型转标签 |
| 私有 | `derive_next_marker_text(next, source, marker)` | 依新旧类型推导补丁用 marker 文本 |
| pub | `new(target)` | 从目标初始化（文本模型、样式、列表类型、caret） |
| pub | `paragraph_id(&self) -> &str` | 目标段落 id |
| pub | `text_char_count(&self) -> usize` | 当前文本字符数 |
| pub | `normalize_caret(&mut self)` | caret 夹到文本长度内 |
| pub | `set_caret_index(i) -> bool` | 设置 caret 并清除选区 |
| pub | `set_selection_range(start, end) -> bool` | 设置选区（退化即设 caret） |
| pub | `clear_selection(&mut self) -> bool` | 清除选区 |
| pub | `selection_range() -> Option<(start, end)>` | 归一化（小在前）选区 |
| pub | `selection_text() -> Option<String>` | 选区文本切片 |
| pub | `set_draft_text(text) -> bool` | 整体替换草稿并同步样式/caret/revision |
| pub | `current_text` / `draft_text` / `source_text` | 当前/草稿/源文本读取（前两者同值） |
| pub | `normalized_caret_index(&self) -> usize` | 夹取后的 caret |
| pub | `toggle_bold_all` / `toggle_italic_all` / `toggle_underline_all` | 全文粗体/斜体/下划线切换 |
| pub | `is_bold_active` / `is_italic_active` / `is_underline_active` | 三种格式是否激活 |
| pub | `active_color` / `active_font_family` / `active_font_size` / `active_char_spacing` | 主导样式读取 |
| pub | `active_line_height` / `source_line_height` | 当前/源行高 |
| pub | `active_paragraph_mode_label() -> String` | 行高映射 compact/normal/relaxed/custom |
| pub | `active_alignment` / `active_alignment_label` / `source_alignment` | 当前/源对齐及标签 |
| pub | `source_list_kind` / `active_list_kind` / `active_list_kind_label` | 源/当前列表类型及标签 |
| pub | `has_style_changes(&self) -> bool` | 样式是否偏离源段落 |
| pub | `requires_source_replacement(&self) -> bool` | 文本/样式/对齐/行高/列表任一变化 |
| pub | `has_session_changes` / `mark_session_clean` | 会话 dirty 标志读/清 |
| pub | `draft_runs() -> Vec<LayoutRun>` | 草稿样式转布局 runs |
| pub | `sync_target_control_style(&mut self)` | 主导样式回写 target 控制样式字段 |
| pub | `set_alignment(align) -> bool` | 设置段落对齐 |
| pub | `set_list_kind(kind) -> bool` | 设置列表类型（重复设置即取消） |
| pub | `restore_list_kind_from_marker_text(marker_text)` | 从 marker 文本恢复列表类型 |
| pub | `resolved_marker_text_for_patch() -> Option<String>` | 补丁 marker 文本决策 |
| pub | `source_marker_text_for_patch() -> Option<&str>` | 源 marker 文本 |
| pub | `set_color_all` / `set_font_family_all` / `set_font_size_all` / `set_char_spacing_all` | 全局颜色/字体/字号/字距设置（归一化+去抖） |
| pub | `set_line_height(lh) -> bool` | 设置行高（0.8–4.0 夹取） |
| pub | `set_paragraph_mode(mode) -> bool` | 模式标签映射行高设置 |

### `paragraph_overlay.rs` — 段落渲染覆盖层 DTO（纯数据）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ParagraphRenderOverlayOwner` (enum) | 覆盖层归属：活动编辑壳 / 持久化页面画布 |
| pub | `ParagraphRenderOverlay` (struct) | owner、target、对象索引、图形 marker、源/草稿文本等 |

### `paragraph_scene.rs` — 段落编辑器场景（文档计划的展平视图）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ParagraphEditorScene` (struct) | target/base id、shell bbox、document_plan、body 会话、marker、原始 runs |
| pub | `graphic_markers(&self) -> &[VisualMarker]` | 图形 marker 访问器 |
| pub | `Default::default()` | 空场景 |
| pub | `paragraph_editor_scene_from_plan(plan) -> Option<Scene>` | 计划 → 场景纯组装 |
| pub | `build_paragraph_editor_scene(paragraph, vm, click)` | 从段落构建场景 |
| pub | `build_paragraph_editor_scene_for_target(paragraph, vm, id, click)` | 按目标 id 构建场景 |

### `replacement_region.rs` — 替换区域几何：渲染抑制所需的 bbox 组
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ParagraphReplacementRegion` (struct) | shell / 源 / 文本清除 / 路径抑制 bbox 与行带上下界 |
| pub | `row_path_suppression_bbox_for_page_width(page_width)` | 整行宽路径抑制 bbox |
| pub | `viewport_cull_bbox_for_page_width(page_width)` | 视口剔除并集 bbox |
| pub | `cache_invalidation_bbox_for_page_width(page_width)` | 缓存失效 bbox（同视口剔除） |
| pub | `paragraph_replacement_region(target)` | 由目标计算替换区域（含边距启发） |
| 私有 | `preferred_source_bbox(target)` | 四级回退选源 bbox |
| 私有 | `bbox_has_area(bbox) -> bool` | bbox 是否有面积 |

### `replacement_snapshot.rs` — 替换快照（editReplacementSnapshot.v3）构建与回读
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `EditReplacementSnapshot` (struct) | 快照 DTO（含遗留 replacement_target 兼容字段） |
| pub | `build_edit_replacement_snapshot(target, kind, body_text, marker, new_marker) -> Value` | 组装 JSON 快照（合并 marker、收集对象索引） |
| pub | `replacement_target_from_patch_snapshot(patch)` | 从旧补丁快照回读完整目标 |
| 私有 | `replacement_object_indices(target) -> Vec<usize>` | 三级来源合并对象索引 |

### `source_identity.rs` — 源对象身份收集（对象 id / 索引）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `collect_target_source_object_ids(target) -> HashSet<String>` | 四路来源并集对象 id |
| pub | `collect_target_source_object_indices_set(target) -> HashSet<usize>` | 同上索引集合 |
| pub | `collect_target_source_object_indices(target) -> Vec<usize>` | 排序去重索引向量 |
| pub | `collect_object_indices_from_runs(runs, vm) -> Vec<usize>` | 直接索引优先，否则按对象 id 反查 vector 模型 |

### `source_runs.rs` — 编辑会话源 runs 解析（vector 模型优先，paint 计划回退）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `original_paint_runs_for_target(paragraph, body_session, target)` | 五级回退取原始 paint runs |
| 私有 | `summarize_layout_runs(runs) -> String` | 调试摘要（前 10 run） |
| pub | `resolve_preferred_editor_session(paragraph, vm)` | 组装权威编辑会话（anchor bbox + runs） |
| 私有 | `resolve_vector_model_source_runs(paragraph, vm)` | object-id 优先、几何次之 |
| 私有 | `resolve_vector_model_runs_by_object_id(paragraph, vm)` | 按对象 id 精确匹配 vector runs |
| 私有 | `bbox_intersection_width` / `bbox_intersection_height` | bbox 交集宽/高 |
| 私有 | `expand_bbox(bbox, x_pad, y_pad)` | bbox 四向扩展 |
| 私有 | `vector_run_matches_paragraph_geometry(run, bbox) -> bool` | 垂直重叠 + 水平包含/中心点判定 |
| 私有 | `resolve_vector_model_runs_by_geometry(paragraph, vm)` | 扩展 bbox 内几何匹配并排序 |
| 私有 | `resolve_vector_source_object_order(paragraph) -> Vec<String>` | paint runs 优先、editor_session 回退的对象顺序 |
| 私有 | `resolve_glyph_paint_runs(paragraph)` | paint runs 转 LayoutRun |
| 私有 | `build_layout(run, owner_id, index)` | StyledRun → LayoutRun 补 id/对象归属 |
| 私有 | `layout_run_from_glyph_paint(run, index)` | GlyphPaintRun → LayoutRun |

### `source_text.rs` — 源文本重建（为 PDF 视觉间距合成空格）
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `run_gap(prev, next) -> f32` | 两 run 间距 |
| 私有 | `boundary_needs_visual_space(prev, next) -> bool` | 标点/大小写/数字字母边界启发 |
| 私有 | `should_insert_run_space(prev, next) -> bool` | run 间是否补空格（阈值启发） |
| 私有 | `char_gap_threshold(font_size, w1, w2) -> f32` | 字符间空格阈值 |
| 私有 | `should_insert_char_space(prev, next, gap, …) -> bool` | 字符间是否补空格 |
| 私有 | `is_ascii_word_char` / `is_pdf_text_separator` | 词字符 / PDF 分隔符判定 |
| 私有 | `starts_with_compact_word_boundary(chars, index) -> bool` | Framework/Program/Library 词边界特判 |
| 私有 | `needs_compact_text_space(chars, index) -> bool` | 紧凑文本是否补空格 |
| 私有 | `normalize_compact_pdf_text(text) -> String` | 全文规范化补空格 |
| 私有 | `run_text_with_visual_spaces(run)` | 依 char_origins 间距重建 run 文本 |
| pub | `session_source_text(session) -> String` | 全 runs 拼接 + 规范化（body 源文本权威来源） |

### `target_resolution.rs` — 持久化补丁的目标回解析
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `resolve_region_target_from_page_state(page_state, …, original_text)` | 页状态 → 交互目标（含 kind 校验） |
| pub | `is_supported_region_kind(kind) -> bool` | 仅 paragraph-region / list-item-region |
| pub | `resolve_region_text_target(targets, page_index, region_id, original_text)` | 文本匹配 → 区域唯一 → 同页回退 |
| 私有 | `normalize_target_text(value) -> String` | 去空白、全角冒号转半角 |

## 文件与方法 · `document_plan/`（编辑器文档计划：类型定义与构建管线）

### `document_plan/mod.rs` — 计划类型定义与构建入口再导出
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `ParagraphEditorMarker` (struct) | marker 类型 / 文本 / advance / runs |
| pub | `EditorDocumentPlan` (struct) | 目标 id、shell bbox、body 会话、源文本、行计划、marker、原始 runs |
| pub | `EditorDocumentLinePlan` (struct) | 行模板 / 源 runs 与重建字符数 |
| pub | `source_body_text(&self) -> &str` | 源 body 文本 |
| pub | `body_char_count(&self) -> usize` | 源 body 字符数 |
| pub | `pub use plan_builder::{…}` × 4 | 再导出四个构建入口 |

### `document_plan/geometry.rs` — bbox 工具与图形 marker 检测
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `vector_object_bbox(object)` | Path/Image 的 bbox（Text 排除） |
| 私有 | `bbox_width` / `bbox_height` / `vertical_overlap_height` | bbox 度量工具 |
| 私有 | `object_marker_kind` / `object_id` | 对象的图形类型 / id |
| 私有 | `graphic_marker_candidate(obj, body, shell) -> bool` | 尺寸/垂直对齐/重叠/水平范围判定 |
| pub(super) | `detect_graphic_markers(vm, body_session, shell) -> Vec<VisualMarker>` | 扫描 vector 对象收集图形 marker |

### `document_plan/line_plans.rs` — body 行计划与草稿模板 run
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(super) | `split_run_at_char_index(run, i)` | 字符索引处拆 run（marker/body 两侧坐标分治） |
| pub(super) | `bbox_from_runs(runs)` | 源视觉 bbox 优先并集 |
| 私有 | `normalize_draft_template_run(run)` | 清几何与对象归属 |
| pub(super) | `select_draft_template_run(session, body_lines)` | 三级回退选草稿模板 run |
| 私有 | `normalize_template_run_for_draft(run)` | 与上者逐字重复的同一实现 |
| pub(super) | `build_body_line_plans(session, text_plan)` | 按 y 容差聚合行计划 |
| 私有 | `same_document_line(ref_y, run) -> bool` | 行归属容差判定（0.45×字号） |

### `document_plan/plan_builder.rs` — 计划构建入口与 open-caret 追踪
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `build_editor_document_plan_from_session(session)` | 纯会话构建（无 marker） |
| pub | `build_editor_document_plan(paragraph, vm, click)` | 委托 for_target（段落自身 id） |
| pub | `collect_editor_document_target_plans(paragraph, vm)` | 收集全部目标计划（交互用） |
| pub | `build_editor_document_plan_for_target(paragraph, vm, id, click)` | 权威入口：解析会话 → 目标 → 计划 |
| 私有 | `codepoint_preview(text, limit)` | 码点诊断串 |
| 私有 | `trace_open_caret_resolved(…)` | open-caret 追踪事件（纯观测） |
| 私有 | `build_plan_for_target_session(…)` | 三步 marker 切分 + 图形 marker + 行计划组装 |

### `document_plan/session_split.rs` — marker/body 会话切分（三级策略链）
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(super) | `resolve_shell_bbox(target_session, split, graphic_markers)` | body ∪ marker ∪ 图形 marker 的 shell bbox |
| pub(super) | `SessionSplit` (struct) | body 会话 + 可选 marker |
| pub(super) | `split_editor_session(session, body_char_start, kind)` | 按字符起点切 run 为 marker/body 两会话 |
| 私有 | `detect_symbolic_font_marker(session)` | 连续符号字体首 runs 识别为 marker |
| pub(super) | `synthesize_marker_from_paragraph(paragraph, body_session)` | 几何合成：body 左侧同行候选合成 marker |
| pub(super) | `resolve_marker_split(paragraph, session, text, plan)` | 策略链：文本语义 → 符号字体 → 几何合成 |

### `document_plan/tests.rs` — 覆盖点（11 项）
canonical 源文本保持、视觉间隙与 run 间空格恢复、marker 切分 advance/宽度保持、可视→raw 索引映射、几何合成候选约束、vector 源优先、overlay 源保留、vector 几何回退、图形 marker 检测与 shell 扩展。

## 文件与方法 · `draft_layout/`（草稿排版：布局 + caret 计划构建管线）

### `draft_layout/mod.rs` — 计划类型定义与构建入口再导出
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub | `DraftCaretStop` (struct) | caret 停靠点：字符索引 + x 坐标 |
| pub | `DraftCaretLine` (struct) | 基线 y、行高与停靠点序列 |
| pub | `EditorDraftRenderPlan` (struct) | 段落布局 + caret 行集合 |
| pub | `pub use plan_builder::{…}` × 2 | 再导出两个构建入口 |

### `draft_layout/text_mapping.rs` — source / runs / draft 三空间索引映射
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(super) | `body_runs_text(plan) -> String` | raw runs 拼接文本 |
| pub(super) | `body_runs_match_source_text(plan) -> bool` | runs 与源文本是否一致 |
| pub(super) | `build_source_to_runs_index_map(source, runs)` | 源→raw 索引映射（合成空格跳过） |
| pub(super) | `build_runs_to_source_index_map(source, runs)` | raw→源逆映射（越界 clamp 到句末） |
| pub(super) | `remap_caret_indices_to_draft_space(lines, plan, draft)` | caret 停靠点重映射到 draft 空间 |
| pub(super) | `TextDiff` (struct) | 公共前后缀：inserted_start / inserted_end / has_inserted |
| pub(super) | `compute_text_diff(source, draft) -> TextDiff` | 计算公共前后缀长度 |

### `draft_layout/styles.rs` — 样式规范化、style 选择与 diff 切片
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(super) | `shell_width(session) -> f32` | 会话 anchor 宽度 |
| pub(super) | `paragraph_preserve_underline(paragraph) -> bool` | 是否保留源下划线 |
| pub(super) | `resolve_draft_template_run(plan)` | 计划模板 run 或首个 body run 回退 |
| pub(super) | `resolve_template(plan, preserve_underline)` | 模板选取并清几何/净化样式 |
| pub(super) | `sanitize_draft_run_style(run, preserve)` | scale_x 夹取、条件清除下划线 |
| 私有 | `normalize_style_run(run, preserve)` | 清全部几何的归一化 |
| 私有 | `normalize_preserved_geometry_run(run, preserve)` | 保留 char_origins/widths 的归一化 |
| 私有 | `find_source_run_index_at_char(runs, index)` | 字符索引 → run 序号 |
| 私有 | `is_good_body_style(run) -> bool` | 非空 / 非装饰 / 非符号字体 |
| 私有 | `select_style(plan, runs, anchor_index, preserve)` | 锚点优先、左右扩展找好样式 |
| 私有 | `slice_runs_by_char_range(runs, start, end)` | 字符区间切片（保留 run 局部 origins） |
| pub(super) | `build_styles(plan, draft_text, preserve)` | diff → 前缀/插入/后缀三段组装草稿 runs（统一渲染链） |

### `draft_layout/source_layout.rs` — 未编辑态的 source 布局重建
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(super) | `same_existing_layout_line(ref_y, run, anchor_top) -> bool` | 行归属容差判定 |
| pub(super) | `build_source_layout(plan) -> ParagraphLayout` | runs 按 anchor 平移重组为视觉行（PDF 原几何） |
| pub(super) | `source_baseline_y(plan) -> f32` | 首行基线（无 run 时模板字号回退） |

### `draft_layout/caret_plan.rs` — caret 停靠点构建
| 可见性 | 条目 | 功能 |
|---|---|---|
| pub(super) | `build_editor_draft_caret_plan_from_layout(layout, measure_width)` | 依 char_origins 或前缀度量生成每行 stops |

### `draft_layout/plan_builder.rs` — 草稿渲染计划构建入口
| 可见性 | 条目 | 功能 |
|---|---|---|
| 私有 | `summarize_render_plan_lines(plan) -> String` | 计划调试摘要 |
| 私有 | `build_draft_paragraph(plan, draft_text, measure)` | 默认下划线策略组装草稿段落 |
| 私有 | `build_draft_paragraph_with_policy(…, preserve_underline)` | 可指定下划线保留策略 |
| 私有 | `align_layout_baseline(layout, target_y)` | 首行基线整体平移 |
| 私有 | `build_empty_render_plan(plan)` | 空文本单行计划 |
| 私有 | `rebuild_layout_pipeline(paragraph, plan, draft, measure)` | 布局 + caret + 重映射流水线 |
| 私有 | `trace_render_plan(action, …)` | 渲染计划追踪事件 |
| pub | `build_draft_render_plan(plan, draft_text, measure_width)` | 编辑器草稿计划（未编辑走 source 布局，空文本走空计划，其余统一排版） |
| pub | `build_persisted_overlay_render_plan(plan, draft_text, measure_width)` | 持久化覆盖层计划（强制不保留下划线） |

### `draft_layout/tests.rs` — 覆盖点（8 项）
下划线净化、紧凑 runs、活动几何保持、overlay 几何保持、origins 保持、分词几何、合成空格映射、缺源字符 clamp。

## 疑点
- 疑似重复：`document_plan/line_plans.rs` 内 `normalize_draft_template_run` 与 `normalize_template_run_for_draft` 是逐字相同的两份实现，应合并。
- 死代码/冗余调用：`draft_layout/plan_builder.rs::build_persisted_overlay_render_plan` 中 `build_draft_paragraph_with_policy` 被连续调用两次，第一次的结果被直接丢弃重建。
- 疑似重复：`bridge.rs` 中 `active_editor_target_from_scene`、`build_active_editor_target`、`build_paragraph_render_target` 三处近乎相同的 `ActiveEditorTarget` 字段装配。
- 命名与公约不符：`bridge.rs`（公约无 *_bridge，实为交互目标/补丁装配层）；`document_runtime.rs`（非 thread_local 容器也非流程编排，实为文档状态解析器，更贴近 *_ops）；`debug_trace.rs` 持有 thread_local 缓冲但未按 *_store 命名。
- 边界模糊：`engine_state.rs::LiveEditorParagraphState` 名为状态容器，实为可 serde 的值类型并承担格式命令执行（toggle/set 系列），介于 *_types（纯 DTO）与有状态业务之间。
- 逻辑重复：`edit_target.rs::bbox_from_layout_runs`/`same_visual_line` 与 `document_plan/line_plans.rs::bbox_from_runs`/`same_document_line` 近似（行容差 0.55× 与 0.45× 字号不同），疑似有意微调但构成术语与实现重复。
