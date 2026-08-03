# 编辑渲染 Bug 架构分析

## Bug 现象
删除文字后：
1. 列表符号跑位（跑到尾部）
2. 下方出现重复文字（原文 + overlay 同时显示）

## 根本原因

**编辑管线有两条独立的数据路径，状态不统一：**

### 路径 A：打开编辑器时（固化状态）
```
GlyphPaintParagraph
  → from_target_id()
  → resolve_preferred_editor_session()
  → resolve_marker_split()
  → split_editor_session()
  → EditContext {
      body_session: ParagraphEditContext { runs, anchor_bbox },
      marker: ParagraphEditorMarker { runs, advance },
      original_runs: Vec<GlyphPaintRun>,
    }
```

这个 `EditContext` 存入 `active_target.scene`，**状态在打开时固化，编辑后不更新**。

### 路径 B：渲染时（使用固化状态 + draft_text）
```
active_target.scene.document_plan
  → build_persisted_overlay_render_plan(document_plan, draft_text)
  → build_draft_paragraph_with_policy()
  → rebuild_layout_pipeline()
  → LayoutParagraph { runs: 新计算的 }
```

渲染时用：
- `document_plan.body_session.anchor_bbox`（固化的）
- `document_plan.marker.runs`（固化的 origin_x/origin_y）
- `document_plan.original_runs`（固化的 object_ids）
- 新计算的 `LayoutParagraph.runs`（基于 draft_text）

**问题：marker.runs 的 origin_x 是固化的绝对坐标，不会随 body 变化调整。**

**问题：original_runs 的 object_ids 是完整列表，编辑删字后 draft_text 变短，但 suppress 照旧用完整列表。**

## 正确架构

**单一状态源 + 编辑驱动更新：**

```rust
struct EditorState {
    // 唯一状态
    paragraph_id: String,
    anchor_bbox: BoundingBox,       // 固定的页面区域
    source_runs: Vec<TextRun>,      // PDF 原始 runs（不变）
    source_text: String,            // PDF 原始文本（不变）
    draft_text: String,             // 用户编辑的文本（变化）
}

impl EditorState {
    /// 打开编辑器时从 Paragraph 创建
    fn from_paragraph(p: &GlyphPaintParagraph) -> Self

    /// 用户编辑后更新 draft_text
    fn apply_draft(&mut self, new_text: &str)

    /// 计算 marker/body 分割（基于 draft_text）
    fn compute_marker_split(&self) -> (Option<MarkerInfo>, BodyInfo)

    /// 构建渲染计划
    fn render_plan(&self) -> RenderPlan {
        // marker 位置 = anchor_bbox.left + marker_advance（相对于 anchor）
        // body 位置 = anchor_bbox.left + body_offset
        // suppress object_ids = 根据编辑范围重新计算
    }

    /// 计算 suppress 的 object_ids（根据编辑范围）
    fn compute_suppress_indices(&self) -> Vec<usize>
}
```

**关键改动：**
1. marker 位置存储为 **相对于 anchor 的偏移**，渲染时转为绝对坐标
2. suppress object_ids **根据 draft_text 与 source_text 的差异重新计算**
3. 所有渲染都从 `EditorState.render_plan()` 出，没有第二条路径

## 需要删除的冗余路径

1. `split_editor_session` → 用 `EditorState.compute_marker_split` 替代
2. `build_persisted_overlay_render_plan` → 用 `EditorState.render_plan` 替代
3. `build_draft_paragraph_with_policy` → 用 `EditorState.render_plan` 替代
4. `rebuild_layout_pipeline` → 用 `EditorState.render_plan` 替代
5. 所有 `ParagraphEditContext` → 用 `EditorSession` 替代
6. 所有 `LayoutRun` → 用 `TextRun` 替代

## 需要修复的命名违规

| 当前 | 正确 |
|------|------|
| `build_editor_document_plan_for_target` | `from_target` |
| `build_editor_document_plan_from_session` | `From<Session>` trait |
| `build_persisted_overlay_render_plan` | `render_plan` |
| `resolve_preferred_editor_session` | `resolve_session` |
| `split_editor_session` | `split_session` 或直接在 `EditorState` 内部 |

## 实施步骤

1. 先修复 suppress bug：根据 draft 变化重新计算 suppress indices
2. 再修复 marker bug：marker 位置相对于 anchor 存储
3. 然后重构：引入 EditorState，删除冗余路径
4. 最后清理命名