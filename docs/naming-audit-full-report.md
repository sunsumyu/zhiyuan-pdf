# 命名审查完整报告

> 审查日期：2026-06-18
> 规则依据：docs/naming-conventions.md
> 审查范围：pdf-viewer-core、pdf-viewer-ui、src-tauri

---

## 一、审查规则与命名约定

### 1.1 动词前缀约定

| 能力类型 | 约定动词 | 语义 | 使用场景 |
|---------|---------|------|----------|
| Query | `read_` | 查询状态或IO-backed数据 | `read_metadata()` — 读已有数据 |
| Query | `find_` | 返回Option/可空值 | `find_paragraph()` — 可能找不到 |
| Query | `list_`/`search_` | 返回集合 | `list_regions()` |
| Resolve | `resolve_` | 由输入+回退规则导出决定 | `resolve_layout()` — 有决策逻辑 |
| Resolve | `compute_` | 纯数学/几何计算 | `compute_baseline()` — 无决策 |
| Create | `new` | 默认/最小构造器 | `Document::new()` |
| Create | `create_` | 带副作用创建 | `create_document()` |
| Create | `build_` | 无副作用纯值构造 | `build_styles()` |
| Create | `init_` | 初始化上下文 | `init_page()` |
| Validate | `is_` | 判断状态（bool） | `is_preview_active()` |
| Validate | `has_` | 判断存在（bool） | `has_bold_span()` |
| Validate | `should_` | 判断行为建议（bool） | `should_replace_source()` |
| Validate | `can_` | 判断能力（bool） | `can_edit()` |
| Mutate | `set_` | 单字段变更 | `set_alignment()` |
| Mutate | `update_` | 多字段/状态变更 | `update_viewport()` |
| Mutate | `apply_` | 对集合应用操作 | `apply_bold()` |
| Mutate | `toggle_` | 状态翻转 | `toggle_bold()` |
| Execute | `execute_` | 执行命令/操作 | `execute_save()` |
| Execute | `commit_` | 提交变更 | `commit_edit()` |
| Lifecycle | `start_` | 启动流程 | `start_render_frame()` |
| Lifecycle | `advance_` | 推进步骤 | `advance_preview_host()` |
| Lifecycle | `cancel_` | 取消流程 | `cancel_progressive()` |
| Sync | `sync_` | 跨边界同步 | `sync_editor_input()` |

### 1.2 介词规则

- `from_xxx` / `with_xxx` — 仅在**必须区分时使用**（如 Rust 不支持重载时多入口区分、同一模块相反方向转换）
- **禁止冗余介词**：当参数名/类型已说明来源时，不加 `from_xxx`
- `xxx_for_yyy` — 禁止，描述调用场景而非核心行为
- `xxx_to_yyy` — 禁止，应改用 `read_yyy`（输出类型已说明目标）
- `xxx_into_yyy` — 禁止，同上
- `xxx_by_yyy` — 禁止，应改用 `with_yyy`（仅当必须区分时）

### 1.3 冗余禁止

- **模块路径已承载的词不加**：在 `edit_target` 模块，函数名不加 `edit`、`target`
- **参数名已说明的来源不加**：参数是 `session`，不加 `from_session`
- **废话修饰词不加**：`whole`/`all`/`full`（除非必须区分数量）

### 1.3 禁止模式

- `process_*` → 改用具体动词（模糊）
- `handle_*` → 改用具体动词（模糊）
- `do_*` → 改用具体动词（模糊）
- `make_*` → 改用 `From` trait、`build_*` 或 `create_*`（随意）
- 名词短语函数名 → 必须动词开头

### 1.4 模块路径承载上下文

函数名不应重复完整模块路径。在 `edit::document_plan` 模块下，函数可以用 `impl From<&ParagraphEditContext> for EditContext` trait，调用形式 `EditContext::from(session)`。

---

## 二、edit 模块审查

### 2.1 不合规项

| # | 文件 | 当前名 | 中文功能 | 问题 | 建议改名 | 命名理由 |
|---|------|--------|----------|------|----------|----------|
| 1 | edit_target.rs | `make_edit_segment_target_id` | 拼接基础段落ID和segment key生成目标ID字符串 | make非约定动词 | `build_segment_id` | Create类用`build_`；模块`edit_target`已承载target上下文，在模块内或调用处`segment_id`语义已清晰；无需重复target |
| 2 | edit_target.rs | `edit_target_base_paragraph_id` | 从目标ID字符串中截取段落ID部分（去掉`::segment`后缀） | 无动词前缀，读起来像字段名 | `get_paragraph_id` | Query类纯属性获取用`get_`；产出是`paragraph_id`，与`get_segment_key`对称；`base`由对比关系隐含，无需显式表达 |
| 3 | edit_target.rs | `edit_target_segment_key` | 从目标ID字符串中截取segment key部分（`::`后面的部分） | 无动词前缀，读起来像字段名 | `get_segment_key` | Query类纯属性获取用`get_`；与`get_paragraph_id`对称；模块上下文已说明"从target中" |
| 4 | edit_target.rs | `whole_session_target` | 构造整段会话的编辑目标（不分segment，覆盖所有runs） | 无动词前缀，whole多余修饰词 | 实现 `From<&ParagraphEditContext>` trait | Create类用`From` trait；模块`edit_target`已承载target语义；调用形式`EditorEditTarget::from(session)`；`session`参数隐含来源，无需`from_session` |
| 5 | replacement_snapshot.rs | `replacement_target_from_patch_snapshot` | 从持久化补丁JSON中反序列化替换目标（可能不存在） | 无动词前缀，`from_patch_snapshot`冗余，`replacement_`重复模块路径 | `find_target` | Query类用`find_`；返回Option表示可能找不到；模块`replacement_snapshot`已承载replacement上下文，`target`在此模块中语义唯一；参数类型`Patch`已说明来源 |
| 6 | replacement_snapshot.rs | `replacement_object_indices` | 收集替换区域中所有源对象索引的有序列表 | 无动词前缀，`replacement_`重复模块路径 | `collect_object_indices` | Query类返回集合用`collect_`；模块`replacement_snapshot`已承载replacement上下文 |
| 7 | replacement_region.rs | `paragraph_replacement_region` | 计算段落替换区域（外壳bbox、清除区域、路径抑制区域等） | 无动词前缀，读起来像类型名而非函数，`paragraph_`冗余，`replacement_`重复模块路径 | `build_region` | Create类用`build_`；模块`replacement_region`已承载replacement上下文，`region`在此模块中语义唯一；`paragraph`是参数名冗余 |
| 6 | source_runs.rs | `target_paint_runs` | 收集编辑目标的绘制runs列表（按对象ID或几何筛选） | 无动词前缀 | `collect_paint_runs` | Query类返回集合，`collect_`表达筛选+汇聚；模块`source_runs`已承载target/runs上下文 |
| 9 | bridge.rs | `active_editor_target_from_scene` | 从编辑器场景数据中构造当前活动编辑目标 | 无动词前缀 | `build_editor_target` | Create类用`build_`；产出是EditorTarget，参数类型`Scene`已说明来源，无需`from_scene` |
| 10 | document_plan.rs | `build_editor_document_plan_from_session` | 从编辑会话构建EditContext | 函数名重复模块路径`document_plan`，且未用`From` trait | 实现 `From<&ParagraphEditContext>` trait | Create类用`From` trait；调用形式 `EditContext::from(session)`；模块路径和参数类型已隐含上下文 |
| 11 | draft_style.rs | `shell_width` | 计算编辑器外壳宽度（取anchor bbox的宽度值） | 无动词前缀，像字段名 | `compute_shell_width` | Resolve类纯数学计算（取bbox宽），`compute_`表达"算出一个数值" |
| 12 | draft_style.rs | `paragraph_preserve_underline` | 判断段落是否应保留下划线样式（检查源文本下划线覆盖率） | 无动词前缀，像布尔字段 | `should_preserve_underline` | Validate类布尔判断，`should_`表达"行为建议"——应不应该保留下划线 |
| 13 | draft_style.rs | `same_existing_layout_line` | 判断run是否与参考基线Y在同一视觉行 | same非标准动词，命名不表达判断行为 | `is_same_layout_line` | Validate类，`is_`表达布尔判断，`same_layout_line`表达"是否同一布局行" |
| 14 | draft_style.rs | `source_baseline_y` | 获取源runs的基线Y坐标（取第一个run的baseline_y） | 无动词前缀，像字段名 | `get_baseline_y` | Query类纯属性获取用`get_`；模块上下文已承载source语义 |
| 15 | draft_text_diff.rs | `body_runs_text` | 拼接所有body runs的文本为一个字符串 | 无动词前缀 | `read_body_runs_text` | Query类，拼接文本返回字符串，`read_`表达"读取并组合" |
| 16 | draft_text_diff.rs | `body_runs_match_source_text` | 判断body runs拼接文本是否与源文本一致 | 无动词前缀，像布尔字段 | `is_body_matching_source` | Validate类布尔判断，`is_`表达"是否匹配" |
| 17 | draft_text_diff.rs | `build_source_to_runs_index_map` + `build_runs_to_source_index_map` | 构建源文本与runs文本的字符索引双向映射 | 成对转换函数，参数相同方向相反，`source`/`runs`写死在函数名中 | 定义 `SourceIndex`/`RunsIndex` 类型 + `to_runs()`/`to_source()` 方法，或合并为一个函数返回双向映射 | 空间域由类型承载；两个函数参数完全一致，可合并为 `build_index_map()` 返回 `(Vec<usize>, Vec<usize>)` |
| 17 | paragraph_scene.rs | `paragraph_editor_scene_from_plan` | 从EditContext组装ParagraphEditorScene数据 | 名词开头缺动词 | 实现 `From<EditContext>` trait | Create类用`From` trait；调用形式`ParagraphEditorScene::from(ctx)`；三个入口改为 `impl From<EditContext>` + `impl From<(&Paragraph, Model, Point)>` + `impl From<(&Paragraph, Id, Point)>` |
| 18 | replacement_region.rs | `preferred_source_bbox` | 在两个可选bbox中选择首选（有面积优先，否则回退） | 无动词前缀 | `resolve_preferred_bbox` | Resolve类有回退逻辑（有面积优先否则回退），用`resolve_`；模块上下文已承载source语义 |

### 2.2 合规项（部分示例）

| 文件 | 当前名 | 中文功能 | 前缀分析 | 清楚表达? |
|------|--------|----------|----------|-----------|
| document_edit_ops.rs | `insert_text` | 在光标位置插入文本 | Mutate:insert | 是 |
| document_runtime.rs | `resolve_document_state` | 从运行时状态解析完整文档状态 | Resolve:resolve | 是 |
| bridge.rs | `collect_paragraph_interaction_targets` | 收集页面中所有段落交互目标 | Query:collect | 是 |
| bridge.rs | `build_rich_patch` | 构建带自定义runs的替换补丁 | Create:build | 是 |
| source_identity.rs | `collect_object_index_set` | 收集对象索引集合 | Query:collect | 是 |

---

## 三、text 模块审查

### 3.1 不合规项

| # | 文件 | 当前名 | 中文功能 | 问题 | 建议改名 | 命名理由 |
|---|------|--------|----------|------|----------|----------|
| 1 | list_semantics.rs | `ListMarkerKind` | 列表标记类型枚举（None/Bullet/Numbering/Symbol/Custom） | Kind后缀冗余 | `ListMarkerType` | `Type`比`Kind`更通用更直白；Rust社区`Kind`主要用于`ErrorKind`等特定场景，类型枚举用`Type`更自然 |
| 2 | list_semantics.rs | `ListTextSemantic` | 列表语义解析结果：标记类型、标记文本、正文文本等 | Semantic作名词不直观 | `ListSemanticResult` | `Result`表达"解析后的输出"，比泛词`Semantic`更清晰；与项目其他`xxxResult`后缀统一 |
| 3 | list_semantics.rs | `derive_list_text_semantics` | 从整行文本推导出列表语义（标记类型+标记文本+正文） | derive语义偏弱不如resolve | `resolve_list_text_semantics` | 按约定Resolve类用`resolve_`；推导过程有回退逻辑（先试编号再试bullet），属于"输入+规则导出决定" |
| 4 | editable_segments.rs | `looks_like_short_field_token` | 判断StyledRun是否像短字段标记（冒号前缀、短文本） | looks_like非标准Validate动词 | `is_short_field_token` | Validate类用`is_`；`looks_like`语义模糊——"看起来像"不是精确判断 |
| 5 | editable_segments.rs | `detect_field_label_anchors` | 从runs中探测字段标签锚点位置 | detect非约定动词 | `find_field_label_anchors` | Query类用`find_`；探测是"搜索定位"过程，可能找不到，`find_`返回Option语义匹配 |
| 6 | editable_segments.rs | `build_contiguous_segments_in_range` | 在指定字符范围内构建连续样式段 | `_in_range`介词后缀 | `build_segments` | Create类用`build_`；参数`range`的类型`CharRange`已说明范围来源，无需`from_range`重复；模块名`editable_segments`已承载上下文 |
| 7 | style_preservation.rs | `make_style_run` | 创建一个StyleRunSnapshot实例（设置id/text/style） | make非约定动词 | `create_style_run` | Create类用`create_`；这是带初始化的构造，`create_`比`build_`更适合（有初始赋值） |
| 8 | style_preservation.rs | `line_selection_range` | 计算指定行中选区的起止范围 | 无动词前缀 | `read_line_selection_range` | Query类读取数据，`read_`表达查询 |
| 9 | style_preservation.rs | `preserve_changed_line_styles` | 保留变更行的原有样式 | preserve非约定动词 | `apply_changed_line_style_preservation` | Mutate类对集合操作用`apply_`；保留样式是对runs列表应用操作，不是简单的set |
| 10 | style_mapper.rs | `dominant_style` | 返回第一个非空非装饰span的样式 | 无动词前缀 | `resolve_dominant_style` | Resolve类有回退逻辑（空列表回退default），属于"输入+规则导出" |
| 11 | style_mapper.rs | `set_bold_all` | 设置所有span为粗体 | set_对集合操作应用apply_，_all后缀冗余 | `apply_bold` | Mutate类对集合操作用`apply_`；方法本身就作用于整个集合，隐含"所有"，无需`_all` |
| 12 | style_mapper.rs | `set_italic_all` | 设置所有span为斜体 | 同上 | `apply_italic` | 同上 |
| 13 | style_mapper.rs | `set_underline_all` | 设置所有span为下划线 | 同上 | `apply_underline` | 同上 |
| 14 | style_mapper.rs | `set_color_all` | 设置所有span的颜色 | 同上 | `apply_color` | 同上 |
| 15 | style_mapper.rs | `set_font_name_all` | 设置所有span的字体名 | 同上 | `apply_font_name` | 同上 |
| 16 | style_mapper.rs | `set_font_size_all` | 设置所有span的字号 | 同上 | `apply_font_size` | 同上 |
| 17 | style_mapper.rs | `set_char_spacing_all` | 设置所有span的字间距 | 同上 | `apply_char_spacing` | 同上 |
| 18 | style_mapper.rs | `has_style_changes_against_paragraph` | 判断当前样式与原始段落样式是否有差异 | `_against_paragraph`介词后缀 | `has_style_changes` | Validate类用`has_`；参数类型`Paragraph`已说明比较基准，无需`from_paragraph`重复 |
| 19 | style_mapper.rs | `to_layout_runs` | 将spans转换为排版引擎LayoutRun列表 | `to_`介词后缀 | `build_layout_runs` | Create类纯值构造用`build_`；`to_`描述转换目标而非行为，`build_`聚焦"构造输出" |
| 20 | index_convert.rs | `utf16_offset_to_char_index` + `char_index_to_utf16_offset` | UTF-16偏移与char索引互转 | 编码类型`utf16`写死在函数名中，方向也写死 | 定义 `CharIndex`/`Utf16Offset` 类型 + `to_utf16(text)`/`to_char(text)` 方法 | 编码由类型承载，方向由方法名表达；`CharIndex(5).to_utf16(text)` 语义清晰 |
| 22-24 | glyph_layout.rs | `map_raw_to_reconstructed` + `map_reconstructed_to_raw` | 原始PDF索引与重建索引互转 | 成对转换函数，空间域`raw`/`reconstructed`写死在函数名中 | 定义 `RawIndex`/`ReconstructedIndex` 类型 + `to_reconstructed()`/`to_raw()` 方法 | 空间域由类型承载，方向由方法名表达；`RawIndex(5).to_reconstructed()` 语义清晰 |
| 23 | glyph_layout.rs | `reconstructed_char_count` | 返回重建后文本的字符数 | 无动词前缀 | `get_char_count` | Query类纯属性获取用`get_`；`reconstructed`由模块上下文隐含 |
| 25 | glyph_layout.rs | `extract_decorative_prefix` | 从段落上下文中提取装饰前缀布局 | extract非约定动词 | `read_decorative_prefix` | Query类，从上下文中读取数据，`read_`表达查询而非物理"提取" |
| 26 | glyph_layout.rs | `glyph_left` | 读取字形左侧X坐标 | 无动词前缀 | `get_glyph_left` | Query类纯属性获取用`get_` |
| 27 | glyph_layout.rs | `glyph_right` | 读取字形右侧X坐标 | 无动词前缀 | `get_glyph_right` | 同上 |
| 28 | glyph_layout.rs | `glyph_visual_width` | 读取字形可视宽度 | 无动词前缀，`visual`冗余 | `get_glyph_width` | Query类纯属性获取用`get_`；`visual`冗余，`width`已隐含是视觉宽度 |
| 29 | glyph_layout.rs | `estimated_gap_source_advance` | 估算间隙源步进值 | 无动词前缀 | `compute_estimated_gap_advance` | Resolve类纯计算，`compute_`表达"算出一个估算值" |
| 30 | glyph_layout.rs | `needs_gap` | 判断两Run间是否需要间隙 | needs非约定Validate动词 | `should_insert_gap` | Validate类用`should_`；表达"行为建议——是否应插入间隙" |
| 31 | glyph_layout.rs | `infer_run_advance` | 推断Run的平均字符步进宽度 | infer非约定动词 | `resolve_run_advance` | Resolve类用`resolve_`；推断有回退逻辑（估计+校正），属于"输入+规则导出决定" |
| 32 | glyph_layout.rs | `EditorSessionTextPlan` | 编辑会话文本计划：含重建文本、字形槽、双向索引映射 | Plan含义模糊 | `EditorSessionTextLayout` | `Layout`比`Plan`更具体——这是已完成的排版结果而非计划 |
| 33 | glyph_layout.rs | `EditorGlyphSlotKind` | 编辑器字形槽类型枚举 | Kind后缀冗余 | `EditorGlyphSlotType` | 同#1，`Type`比`Kind`更自然 |
| 34 | glyph_layout.rs | `has_suspicious_run_geometry` | 判断是否有几何可疑的Run | suspicious主观不精确 | `has_abnormal_run_geometry` | `abnormal`是客观描述（偏离正常范围），`suspicious`是主观判断 |
| 35 | caret_geometry.rs | `same_existing_session_line` | 判断Run是否与参考基线Y在同一视觉行 | same非标准动词，`visual`冗余 | `is_same_line` | Validate类用`is_`；`line`已隐含是视觉行 |
| 36 | caret_geometry.rs | `caret_index_at_page_point` | 根据页面坐标计算光标字符索引 | 无动词前缀 | `resolve_caret_index_at_page_point` | Resolve类，坐标→索引有查找+就近回退逻辑，用`resolve_` |
| 37 | caret_geometry.rs | `resolve_index` | 从会话和文本计划解析光标索引 | 过于泛化 | `resolve_caret_index` | 加上名词限定`caret`，让调用者知道解析的是光标索引而非任意索引 |
| 38 | caret_geometry.rs | `session_caret_visual` | 计算编辑会话中指定光标索引的视觉位置 | session_非动词 | `compute_session_caret_visual` | Resolve类纯计算用`compute_`；输入索引→输出坐标，无决策逻辑 |
| 39 | caret_geometry.rs | `session_plan_caret_visual` | 基于已有文本计划计算光标视觉位置 | session_plan_非动词 | `compute_caret_visual` | Resolve类用`compute_`；参数类型`TextPlan`已说明输入是计划 |
| 40 | caret_geometry.rs | `populate_line_stops_from_text_plan` | 从文本计划填充行停靠点 | `_from_text_plan`介词后缀冗余 | `populate_line_stops` | Mutate类用动词`populate`；参数类型`TextPlan`已说明来源 |
| 41 | caret_geometry.rs | `resolve_navigation_from_lines` | 从行数据中解析导航目标索引 | `_from_lines`介词后缀 | `resolve_navigation` | Resolve类用`resolve_`；参数类型`Vec<CaretLine>`已说明是lines |
| 42 | caret_geometry.rs | `CaretStop` | 光标停靠点：字符索引与左侧X坐标 | Stop含义不直观 | `CaretAnchor` | `Anchor`表达"锚定位置"——光标停在某处就锚定在那里，比`Stop`（停止）更精确 |
| 43 | semantic_axiom.rs | `AxiomEngine` | 语义角色推断引擎 | Axiom含义不直观 | `RoleInferenceEngine` | `RoleInference`直接表达功能——推断文本段的角色（Title/Date/Amount），比`Axiom`（公理）直白 |

---

## 四、geometry 模块审查

### 4.1 不合规项

| # | 文件 | 当前名 | 中文功能 | 问题 | 建议改名 | 命名理由 |
|---|------|--------|----------|------|----------|----------|
| 1 | bbox_ops.rs | `bbox_width` | 计算边界框宽度（right-left，最小0） | 名词短语非动词开头 | `compute_bbox_width` | Resolve类纯数学计算，`compute_`表达"算出一个数值"；当前名像属性访问而非计算函数 |
| 2 | bbox_ops.rs | `bbox_height` | 计算边界框高度（bottom-top，最小0） | 同上 | `compute_bbox_height` | 同上 |
| 3 | bbox_ops.rs | `union_bbox` | 合并两个边界框为并集bbox | union名词非动词 | `merge_bbox` | Mutate类合并操作，`merge`是动词表达"合并"；`union`是名词（集合论术语），不符合动词+名词约定 |
| 4 | coordinate_transform.rs | `HostPageTransform::scale` | 获取当前变换的缩放比例值 | 名词方法缺动词 | `get_scale` | Query类纯属性获取用`get_`；当前`scale`像字段名而非方法 |
| 5 | coordinate_transform.rs | `PdfToPageViewTransform::point` | 将逻辑页面坐标投射到视图坐标系 | 名词方法缺动词 | `project_point` | Transform类用`project_`；投射是坐标变换动作，动词`project`精确表达行为 |
| 10 | coordinate_transform.rs | `normalize_y` + `denormalize_y` | Y轴极性归一化与反归一化（实现相同：`page_height - y`） | 成对转换函数，方向写死在函数名中 | 定义 `PdfYUp`/`YDown` 类型 + `to_y_down()`/`to_y_up()` 方法 | 坐标系由类型承载，方向由方法名表达；两个函数实现完全相同，类型区分即可 |
| 11 | coordinate_transform.rs | `client_to_page` / `client_to_page_in_box` / `client_to_local_in_box` | client坐标转换到page/local坐标 | 空间域`client`/`page`/`local`硬编码在函数名中；`client_to_page`和`client_to_page_in_box`是功能变体（仅差clamp） | 定义 `ClientPoint`/`PagePoint`/`LocalPoint` 类型 + `to_page()`/`to_local()` 方法；clamp由参数控制 | 空间域由类型承载；功能变体合并，clamp由参数`clamp: Option<BoundingBox>`控制 |
| 12 | coordinate_transform.rs | `point_from_pdf` / `x_from_pdf` / `baseline_y_from_pdf` + `baseline_y_from_anchor_relative` | PDF坐标转局部坐标 | 空间域`pdf`/`anchor_relative`硬编码在函数名中；`baseline_y_from_pdf`和`baseline_y_from_anchor_relative`是功能变体 | 定义 `PdfPoint`/`AnchorRelativePoint` 类型 + `to_local()` 方法 | 空间域由类型承载；功能变体由输入类型区分 |
| 6 | source_geometry.rs | `source_session_visual_bbox` | 从编辑上下文中计算所有run的合并视觉bbox | `source_`前缀语义模糊，`visual`冗余 | `compute_session_bbox` | Resolve类纯几何计算；去掉`source_`和`visual`冗余；`bbox`已隐含是视觉边界框 |
| 7 | source_geometry.rs | `caret_line_bbox` | 根据光标基线Y计算同一行所有run的合并bbox | 无动词前缀 | `compute_caret_line_bbox` | Resolve类纯计算，`compute_`表达"算出行bbox" |
| 8 | source_geometry.rs | `source_run_visual_bbox` | 计算单个LayoutRun的视觉bbox | `source_`前缀模糊，`visual`冗余 | `compute_run_bbox` | 去掉`source_`和`visual`冗余 |
| 9 | source_geometry.rs | `source_visual_bbox_from_runs` | 从run数组中计算合并后的视觉bbox | `source_`前缀模糊，`visual`和`from_runs`冗余 | `compute_bbox` | 去掉`source_`、`visual`冗余；参数类型已说明是runs |

---

## 五、render 模块审查

### 5.1 不合规项

| # | 文件 | 当前名 | 中文功能 | 问题 | 建议改名 | 命名理由 |
|---|------|--------|----------|------|----------|----------|
| 1 | text_suppression.rs | `process_visible_objects` | 遍历可见对象并应用文本/路径压制规则 | process禁用词 | `apply_suppression` | Mutate类对集合操作用`apply_`；`process`语义模糊，`apply_suppression`精确表达 |
| 2 | glyph_plan.rs | `process_glyph_paragraph` | 处理单个段落的overlay压制 | process禁用词 | `apply_overlay_suppression` | 同上，`apply_overlay_suppression`表达"对段落应用覆盖层压制" |
| 3 | overlay_ops.rs | `insert_overlay_if_needed` | 按需插入overlay条目到渲染列表 | `_if_needed`后缀描述条件 | `try_insert_overlay` | Lifecycle类用`try_`表达"尝试操作可能不执行"；`_if_needed`是自然语言描述场景而非精炼动词 |
| 4 | prepared_scene.rs | `build` | 构建预处理页面场景 | 缺名词 | `build_prepared_scene` | Create类，`build_`后必须接名词表达构造什么；裸`build`过于泛化 |
| 5 | prepared_scene.rs | `visible_vector_indices` | 获取可见矢量对象索引集合 | 无动词前缀 | `find_visible_vector_indices` | Query类用`find_`；这是查找筛选过程，`find_`表达"定位可见的" |
| 6 | prepared_scene.rs | `active_text_object_ids` | 获取活跃文本对象ID集合 | 无动词前缀 | `find_active_text_object_ids` | 同上 |
| 7 | progressive.rs | `total_items` | 返回渐进式渲染任务的总条目数 | 无动词前缀 | `get_total_items` | Query类纯属性获取用`get_` |
| 8 | renderer.rs | `name` | 返回渲染后端名称 | 无动词/get语义 | `get_name` | Query类纯属性获取用`get_`；模块renderer已承载backend语义 |
| 9 | viewport_culling.rs | `styled_run_bbox` | 计算run的包围盒 | 无动词前缀，`styled`冗余 | `compute_run_bbox` | Resolve类纯几何计算；`run`是排版术语保留，`styled`冗余（run本身就是带样式的文本段） |
| 10 | viewport_culling.rs | `path_object_bbox` | 计算路径对象的包围盒 | 无动词前缀，`object`冗余 | `compute_path_bbox` | Resolve类纯几何计算；`path`已隐含是对象，无需`object`修饰 |
| 11 | zoom_host.rs | `resolve_anchor_from_visible_preview_state` | 从可见预览状态解析锚点 | `_from_visible_preview_state`介词后缀冗余 | `resolve_visible_preview_anchor` | Resolve类，去掉`from_`介词后缀；`visible_preview_anchor`已表达完整语义——"可见预览的锚点" |

---

## 六、src-tauri 审查

### 6.1 不合规项

| # | 文件 | 当前名 | 中文功能 | 问题 | 建议改名 | 命名理由 |
|---|------|--------|----------|------|----------|----------|
| 1 | log_service.rs | `get_pdf_log_level` / `set_pdf_log_level` / `clear_pdf_event_log` / `read_pdf_event_log` / `log_pdf_event` | PDF日志级别读写和事件记录 | `pdf_`重复模块路径`pdf::log_service` | `get_log_level` / `set_log_level` / `clear_event_log` / `read_event_log` / `log_event` | 模块路径已承载pdf上下文，去掉`pdf_`冗余 |
| 2 | font/metrics.rs | `get_character_width_pdf_units` | 获取字符在PDF单位下的宽度 | get_禁用前缀 | `get_character_width` | Query类纯属性获取用`get_`；模块路径`pdf::font::metrics`已承载pdf上下文；`pdf_units`由返回类型体现 |
| 3 | font/matching.rs | `candidate_count` | 返回候选字体数量 | 无动词前缀 | `get_candidate_count` | Query类纯属性获取用`get_` |
| 4 | pdf_utils.rs | `effective_height` | 计算页面有效高度（考虑旋转） | 无动词前缀 | `compute_effective_height` | Resolve类纯计算（旋转校正），`compute_`表达"算出有效高度" |
| 5 | pdf_write_font_resolver.rs | `source_label` | 返回字体来源标签字符串 | 无动词前缀 | `get_source_label` | Query类纯属性获取用`get_` |
| 6 | cache.rs | `page_cache_key` | 生成页面缓存键字符串 | 无动词前缀 | `build_page_cache_key` | Create类纯值构造，`build_`表达"组装键字符串" |
| 7 | cache.rs | `page_revision_cache_key` | 生成带版本的页面缓存键 | 无动词前缀 | `build_page_revision_cache_key` | 同上 |
| 8 | cache.rs | `light_page_cache_key` | 生成轻量页面缓存键 | 无动词前缀 | `build_light_page_cache_key` | 同上 |
| 9 | font/ttc.rs | `extract_ttc_face_as_ttf` | 从TTC集合中提取TTF字体数据 | `_as_ttf`介词后缀 | `read_ttf` | Query类用`read_`；参数类型`TTC`已说明来源；产出TTF由返回类型体现 |
| 10 | pdf_font.rs | `break_text_into_lines` | 将文本按宽度断为多行 | `_into_lines`介词后缀 | `compute_text_lines` | Resolve类纯计算，`compute_`表达"算出多行排版"，去掉`into_`后缀 |
| 11 | pdf_read/resource_reader.rs | `find_xobject_by_name` | 按名称查找XObject资源 | `_by_name`介词后缀 | `find_xobject` | Query类用`find_`；参数名`name`已说明来源，无需`with_name` |
| 12 | save_text_write_plan.rs | `truncate_for_log` | 截断文本用于日志显示 | `_for_log`介词后缀 | `truncate_for_logging` | 日志截断是特定场景，保留限定词表达目的 |
| 13 | vello_renderer.rs | `render_objects_to_png` | 渲染PDF对象为PNG图片 | `_to_png`介词后缀 | `render_png` | Execute类用`render_`；参数类型已说明是objects；PNG由返回类型体现 |
| 14 | geometry_service.rs | `resolve_layout_inference_revisioned` | 带版本校验解析布局推断 | `_revisioned`后缀不统一 | `resolve_layout_inference` | Resolve类；版本校验是内部行为，无需在命名中暴露 |
| 16 | document_service.rs | `release_pdf_resources` / `release_all_pdf_resources` / `read_last_pdf_materialization_report` / `generate_demo_pdf` / `load_pdf_public` | PDF文档资源管理和加载 | `pdf_`重复模块路径`pdf::document_service` | `release_resources` / `release_all_resources` / `read_materialization_report` / `generate_demo` / `load_public` | 模块路径已承载pdf上下文 |
| 17 | font/matching.rs | `resolve_pdf_font` | 解析PDF字体 | `pdf_`重复模块路径`pdf::font::matching` | `resolve_font` | 模块路径已承载pdf上下文 |
| 18 | color_utils.rs | `parse_pdf_hex_color` | 解析PDF十六进制颜色 | `pdf_`重复模块路径`pdf::color_utils` | `parse_hex_color` | 模块路径已承载pdf上下文 |
| 19 | pdf_write_service.rs | `generate_demo_pdf` | 生成演示PDF | `pdf_`重复模块路径`pdf::pdf_write_service` | `generate_demo` | 模块路径已承载pdf上下文 |
| 20 | save_engine.rs | `apply_pdf_commands` | 应用PDF命令 | `pdf_`重复模块路径`pdf::save_engine` | `apply_commands` | 模块路径已承载pdf上下文 |

---

## 七、pdf-viewer-ui 审查（关键不合规项）

| # | 文件 | 当前名 | 中文功能 | 问题 | 建议改名 | 命名理由 |
|---|------|--------|----------|------|----------|----------|
| 1 | application.rs | `snapshot_state` | 读取所有域的聚合状态快照 | snapshot非标准动词 | `read_aggregated_state` | Query类用`read_`；`aggregated_state`表达"聚合状态"，比`snapshot_state`更精确 |
| 2 | application.rs | `reset_all` | 重置所有WASM会话到空状态 | all后缀冗余，不精确 | `reset_sessions` | Destroy类用`reset_`；方法本身就作用于所有会话，隐含"所有"；`sessions`名词限定产物 |
| 3 | free_api(render) | `schedule_render_frame` | 安排渲染帧请求 | render重复模块上下文 | `schedule_frame` | Lifecycle类`schedule_`；在render/free_api模块下，render是冗余上下文 |
| 4 | free_api(render) | `commit_render_result` | 提交渲染结果 | render重复模块上下文 | `commit_frame` | Execute类`commit_`；模块路径已承载render语义 |
| 5 | free_api(render) | `is_render_frame_current` | 判断渲染帧是否当前 | render重复模块上下文 | `is_frame_current` | Validate类`is_`；同上 |
| 6 | free_api(render) | `clear_zoom_preview_host_state` | 清除缩放预览宿主状态 | host/state冗余，zoom重复模块上下文 | `clear_preview` | Destroy类用`clear_`；模块zoom_api已承载zoom；预览状态隐含在"清除预览"中；调用形式`zoom_api::clear_preview()`语义完整 |
| 7 | free_api(zoom) | `resolve_wheel_zoom` | 解析滚轮缩放请求 | zoom重复模块上下文 | `resolve_wheel` | Resolve类；zoom_api模块已承载zoom语义 |
| 8 | free_api(zoom) | `read_zoom_state` | 读取缩放状态 | zoom重复模块上下文 | `get_state` | Query类纯属性获取用`get_`；模块路径已承载zoom |

---

## 八、前后缀一致性分析

### 8.1 前缀使用统计

| 前缀 | 出现次数 | 统一性 | 问题 |
|------|---------|--------|------|
| `resolve_` | ~35 | ⚠️ 与compute/infer/derive混用 | 同为"计算型"，有的用resolve有的用compute，应按有无回退逻辑区分 |
| `compute_` | ~5 | ⚠️ 与resolve混用 | 无回退的纯计算用compute，有回退用resolve |
| `read_` | ~10 | ✅ 一致 | 用于有IO/计算成本的查询 | |
| `build_` | ~25 | ✅ 一致 | |
| `find_` | ~8 | ✅ 一致 | 但有detect/extract混用 |
| `is_` | ~15 | ✅ 一致 | 但有needs/same/looks_like混用 |
| `has_` | ~5 | ✅ 一致 | |
| `should_` | ~5 | ⚠️ 与needs混用 | `should_insert_gap` vs `needs_gap` |
| `apply_` | ~5 | ⚠️ 与set_xxx_all混用 | 对集合操作应统一用apply_ |
| `collect_` | ~8 | ✅ 一致 | |

### 8.2 后缀使用统计

| 后缀 | 出现次数 | 统一性 | 问题 |
|------|---------|--------|------|
| `Result` | ~20 | ✅ 一致 | 操作结果结构体统一用Result |
| `Output` | ~15 | ✅ 一致 | 序列化输出结构体 |
| `Dto` | ~5 | ✅ 一致 | 数据传输对象 |
| `Request` | ~10 | ✅ 一致 | |
| `State` | ~8 | ✅ 一致 | 但有Session/Store混用 |
| `_all` | ~12 | ⚠️ set和apply都有_all | 对集合操作应统一用apply_前缀+all后缀 |
| `Kind` | ~3 | ⚠️ 与Type混用 | 建议统一用Type |
| `Plan` | ~8 | ⚠️ 含义模糊 | 有的Plan是已完成结果，有的确实是计划，建议区分 |

### 8.3 统一建议

1. **resolve vs compute**：有回退逻辑用`resolve_`，纯数学用`compute_`。当前混用需逐一确认。
2. **set vs apply**：单字段用`set_`，对集合批量操作用`apply_`。`set_bold_all`改为`apply_bold_all`。
3. **Kind vs Type**：统一用`Type`，除非是Rust社区惯例的`ErrorKind`。
4. **Plan vs Layout**：已完成排版结果用`Layout`，待执行步骤用`Plan`。
5. **缺动词**：所有名词短语函数名必须加动词前缀，按能力分类选动词。

---

## 九、修改优先级

### P0 必须立即修改

1. `get_*`用于纯属性获取，`read_*`用于有IO/计算成本的查询
2. 所有`process_*` → `apply_*`（2处）
3. 所有`make_*` → `build_*`/`create_*`（2处）
4. 所有缺动词的名词短语函数名（~20处）

### P1 应在本轮修改

1. 所有介词后缀`_for_`/`_to_`/`_into_`/`_by_`/`_as_` → 前置`_from_`/`_with_`（~15处）
2. 所有非约定动词：`detect`→`find`、`extract`→`read`、`map`→`read`、`needs`→`should`、`dedupe`→`apply`、`same`→`is`、`looks_like`→`is`（~8处）
3. `set_xxx_all` → `apply_xxx_all`（7处）
4. `Kind` → `Type`（3处）
5. 过于泛化的名字如`resolve_index`→`resolve_caret_index`（~3处）

### P2 可后续修改

1. `Plan`含义模糊处→`Layout`或具体名词（~5处）
2. 模块路径冗余处（如render/free_api中的render重复）（~8处）
3. `_revisioned` → `_with_revision`（1处）
4. 主观词`suspicious` → `abnormal`（1处）
