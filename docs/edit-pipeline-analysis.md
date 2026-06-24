---
name: edit-pipeline-analysis
description: 编辑管线架构分析 — 分叉逻辑、命名违规、统一方案
type: project
---

# 编辑管线架构分析

## 一、当前管线分叉

编辑管线存在 **3 条独立路径**，没有任何统一的状态管理：

### 路径 A：打开编辑器（document_plan.rs）
- `from_target_id` → `resolve_preferred_editor_session` → `resolve_marker_split` → `split_editor_session`
- 产出 `EditContext`（含 `ParagraphEditContext` + marker + source_text）
- marker/body 分割点在打开时固定，**编辑后不重新计算**

### 路径 B：编辑渲染（draft_layout.rs）
- `build_persisted_overlay_render_plan` → `build_draft_paragraph_with_policy` → `rebuild_layout_pipeline`
- 输入 `EditContext` + draft_text
- `build_draft_paragraph_with_policy` 创建新 LayoutParagraph，覆盖 body_session.paragraph.runs
- **不处理 marker**，marker 信息来自 EditContext 中打开时固化的值

### 路径 C：overlay 重建（pdf-viewer-ui canvas.rs）
- `build_persisted_overlay_render_plan` 同路径 B
- 但在 `paragraph_overlay.rs` 中手动构建 `ParagraphEditContext`
- **第二条独立的 session 构建路径**

### 问题：分叉导致状态不一致

1. marker 分割点打开时固定 → 编辑删字后分割点不变 → 列表符号跑位
2. 两条 session 构建路径 → 可能产生不同 anchor_bbox/wrap_width
3. `build_styles` vs `build_draft_paragraph_with_policy` 两条风格路径并存
4. `split_run`（旧） vs `TextRun::split_at`（新）并存，刚替换了 document_plan.rs 但 draft_style.rs 的 `slice_runs_by_char_range` 还走旧路径

## 二、命名违规清单（按严重程度排序）

### 🔴 严重违规（4词+介词）

| 当前命名 | 违规点 | 正确命名 |
|---------|--------|---------|
| `build_editor_document_plan_for_target` | 4词+介词for | `from_target_id`（已有，旧名应删） |
| `build_editor_document_plan_from_session` | 4词+介词from | `From<ParagraphEditContext>` trait |
| `build_editor_document_plan` | 4词+废话plan | `from_paragraph`（已有，旧名应删） |
| `collect_editor_document_target_plans` | 4词+废话plan/target | `collect_all`（已有，旧名应删） |
| `collect_edit_targets_from_session` | 3词+介词from | `collect_targets(&session)` |
| `resolve_edit_target_from_session` | 3词+介词from | `resolve_target(&session, id)` |
| `compute_bbox_from_runs` | 介词from | `compute_bbox(runs)` |
| `build_paragraph_editor_scene` | 3词+废话plan | `build_scene`（模块edit已承载editor） |
| `paragraph_editor_scene_from_plan` | 3词+介词from | `From<EditContext>` trait |
| `build_effective_vector_render_plan` | 4词+废话plan | `build_render_plan`（模块render已承载） |
| `build_effective_glyph_render_plan` | 4词+废话plan | `build_glyph_plan`（模块render已承载） |
| `resolve_caret_index_from_lines` | 介词from | `resolve_caret(lines)` |
| `populate_line_stops_from_text_plan` | 介词from | `populate_stops(plan)` |
| `resolve_navigation_from_lines` | 介词from | `resolve_navigation(lines)` |
| `resolve_anchor_from_visible_preview_state` | 4词+介词from | `resolve_anchor(state)` |
| `build_persisted_overlay_render_plan` | 4词+废话plan | `build_overlay_plan` |
| `build_edit_replacement_snapshot` | 3词 | `build_snapshot`（模块edit已承载） |
| `build_persistable_save_plan` | 3词+废话plan | `build_save_plan`（模块persistence已承载） |
| `collect_persistable_region_patches` | 3词 | `collect_patches`（模块persistence已承载） |
| `collect_legacy_text_reflows` | 3词 | `collect_reflows` |

### 🟡 中等违规（3词无介词）

| 当前命名 | 违规点 | 正确命名 |
|---------|--------|---------|
| `compute_run_aware_caret_left` | 3词 | `compute_caret_left` |
| `build_editor_session_text_plan` | 3词+废话plan | `build_text_plan`（模块text已承载） |
| `resolve_preferred_editor_session` | 3词 | `resolve_session`（模块edit/source_runs已承载） |
| `resolve_paragraph_shell_bbox` | 3词 | `resolve_shell_bbox`（模块edit已承载paragraph） |
| `build_paragraph_render_target` | 3词 | `build_target`（模块edit已承载paragraph） |
| `collect_paragraph_interaction_targets` | 3词 | `collect_targets`（模块edit/bridge已承载） |
| `normalize_pdf_font_identity` | 3词+领域词pdf | `normalize_identity`（模块typography已承载） |
| `score_system_font_candidate` | 3词 | `score_candidate` |
| `resolve_system_or_fallback_font` | 4词 | `resolve_font` |
| `find_run_at_text_offset` | 介词at | `find_run(offset)` |
| `caret_index_at_page_point` | 介词at | `caret_index(point)` |
| `is_point_in_bbox` | 介词in | `contains(bbox, point)` |
| `same_existing_session_line` | 3词 | `same_line` |
| `has_style_changes_against_paragraph` | 4词+介词against | `has_changes(paragraph)` |
| `should_preserve_editor_underline` | 3词 | `should_preserve_underline`（模块edit已承载editor） |
| `distribute_text_across_runs` | 介词across | `distribute(text, runs)` |
| `preserve_changed_line_styles` | 3词 | `preserve_styles` |

## 三、统一架构方案

### 核心原则：单一管线 + 状态驱动

当前问题根源：**3条路径各自构建状态，没有统一的状态对象驱动渲染**。

### 方案：引入 `EditorState` 状态机

```rust
/// 编辑器的唯一状态源
struct EditorState {
    session: EditorSession,       // 替代 ParagraphEditContext
    source_text: String,          // 原始文本
    marker: Option<Marker>,       // 列表标记
    draft_text: String,           // 用户编辑的文本
}

impl EditorState {
    /// 从段落构建（打开编辑器时）
    fn from_paragraph(paragraph, vector_model, target_id) -> Option<Self>
    
    /// 用户编辑后更新 draft_text，重新计算 marker/body 分割
    fn apply_draft(&mut self, new_text: &str)
    
    /// 构建渲染计划（唯一渲染路径）
    fn render_plan(&self, measure: &dyn Measure) -> RenderPlan
}
```

### 关键改进：

1. **marker/body 分割跟随 draft 变化** — `apply_draft` 根据新文本重新计算分割点
2. **单一渲染路径** — 删除 `build_persisted_overlay_render_plan` 和 `build_draft_render_plan`，统一为 `EditorState::render_plan`
3. **内部使用 TextRun** — `EditorSession` 用 `TextRun`，不再有 `LayoutRun ↔ TextRun` 转换循环
4. **split 统一为 TextRun::split_at** — 删除所有 `split_run`、`split_runs_at_char_index` 等旧实现
