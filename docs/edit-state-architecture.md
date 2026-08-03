# 编辑态全链路架构分析

## 一、状态存储（谁存了什么）

### 1.1 全局状态

| 位置 | 结构 | 存了什么 |
|------|------|---------|
| `EDITOR_MODE_STATE` (thread_local) | `EditorModeState` | text_edit_enabled, active_paragraph_id, live_state, history |
| `read_patch_state()` (thread_local) | `PatchState` | persisted patches (original_text, new_text, target_indices) |
| `PAGE_STATE` (thread_local) | `HostPageState` | paint_plan, vector_model |

### 1.2 LiveEditorParagraphState（编辑中的段落状态）

```
LiveEditorParagraphState {
    target: ActiveEditorTarget,       // ← 核心状态
    text_model: EditorTextModel,      // 文本模型（draft_text + caret）
    style_mapper: StyleMapper,        // 样式映射
    list_kind: ListMarkerKind,        // 列表类型
    caret_index: usize,               // 光标位置
    selection_start/end: Option<usize>,// 选区
}
```

### 1.3 ActiveEditorTarget（❌ 状态混乱的核心）

```
ActiveEditorTarget {
    paragraph_id, region_id, page_index,
    text, bbox_left/top/right/bottom,  // ← 冗余！与 scene 中的 bbox 重复
    font_family, font_size, ...        // ← 冗余！与 scene.document_plan 中的 style 重复
    editor_session: ParagraphEditContext,  // ← 重复 A：完整 session
    scene: ParagraphEditorScene,           // ← 重复 B：又存了一份 session
}
```

### 1.4 ParagraphEditorScene（❌ 状态分叉）

```
ParagraphEditorScene {
    target_id, base_paragraph_id,
    shell_bbox,
    document_plan: EditContext,            // ← 包含 body_session + marker
    body_text,                             // ← 冗余！= document_plan.source_body_text
    body_session: ParagraphEditContext,     // ← 分叉！与 document_plan.body_session 重复
    body_initial_caret,                    // ← 冗余！= document_plan.body_initial_caret
    marker: Option<ParagraphEditorMarker>, // ← 分叉！与 document_plan.marker 重复
    original_runs: Vec<GlyphPaintRun>,     // ← 分叉！与 document_plan.original_runs 重复
}
```

**关键问题：`scene.body_session` 与 `scene.document_plan.body_session` 可能不一致！**
**`scene.marker` 与 `scene.document_plan.marker` 可能不一致！**

## 二、编辑动作执行链

### 2.1 打开编辑器

```
用户点击段落
  → editor_api::open_editor()
  → activation::activate_editor()
  → build_paragraph_render_target()          // 构建 ActiveEditorTarget
    → from_paragraph(paragraph, vector_model) // 调用 core 的 document_plan
      → resolve_preferred_editor_session()   // 选 source runs
      → resolve_marker_split()               // marker/body 分割
      → split_editor_session()               // 执行分割（split_run）
      → 产出 EditContext
    → 构建 ParagraphEditorScene
      → scene.document_plan = EditContext
      → scene.body_session = document_plan.body_session  // 复制
      → scene.marker = document_plan.marker              // 复制
      → scene.original_runs = ...                        // 复制
    → 构建 ActiveEditorTarget
      → target.editor_session = ...  // 又复制一份
  → set_paragraph(paragraph_id)
  → set_live_state(state)
```

**问题：同一个 session 被复制了 3 份（document_plan.body_session, scene.body_session, target.editor_session），后续更新可能只更新其中一份。**

### 2.2 用户编辑文字

```
用户在 textarea 输入
  → editor_api::sync_editor_input()
  → text_model.replace_range()              // 更新 text_model（draft_text）
  → caret_index 更新
  → scene_dirty = true
  → 触发渲染
```

**问题：编辑只更新了 text_model（draft_text），没有更新 scene/body_session/document_plan 中的任何 runs！**

### 2.3 渲染 overlay

```
渲染帧
  → canvas.rs::draw_editor_overlay()
  → draw_editor_marker_page()               // 画 marker（用 scene.document_plan.marker.runs 的固化坐标）
  → build_persisted_overlay_render_plan()   // 画 body（用 document_plan + draft_text）
    → build_draft_paragraph_with_policy()   // 从 document_plan.body_session 克隆 paragraph
    → runs = build_styles() 或 template     // 创建新 runs（无 object_ids、无 char_origins）
    → paragraph.runs = runs                  // 覆盖
    → layout_paragraph()                     // 排版
  → 逐行逐 run 画 body 文字
```

**问题链：**
1. marker 用固化坐标画 → 删字后 marker 位置不变
2. body 用 `anchor_bbox.left + line.offset_x + run.origin_x` → 新 runs 的 origin_x 是从 0 开始的相对坐标
3. 新 runs 无 object_ids → suppress 可能失效
4. 新 runs 无 char_origins → 字距丢失 → 字体漂移

### 2.4 光标计算

```
用户按左右键
  → caret 计算用 compute_run_aware_caret_left()
  → 基于 body_session.paragraph.runs（固化的原始 runs）
  → 但实际显示的是 draft_layout 新排版的 runs
  → 二者不一致 → 光标位置错误
```

## 三、状态分叉汇总

| 概念 | 存储位置 1 | 存储位置 2 | 存储位置 3 | 是否一致 |
|------|-----------|-----------|-----------|---------|
| body runs | document_plan.body_session.paragraph.runs | scene.body_session.paragraph.runs | target.editor_session.paragraph.runs | ❌ 编辑后只有 document_plan 的是原始的 |
| marker | document_plan.marker | scene.marker | — | ❌ 可能不一致 |
| original_runs | document_plan.original_runs | scene.original_runs | — | ❌ 可能不一致 |
| body bbox | document_plan.body_session.anchor_bbox | scene.body_session.anchor_bbox | — | ✅ 打开后不变 |
| draft text | text_model.text | — | — | ✅ 唯一源 |
| caret | live_state.caret_index | — | — | ✅ 唯一源 |

## 四、根因分析

**所有 bug 的根因是同一个：编辑后只更新了 text_model（draft_text），没有重新计算 document_plan 和 scene 中的 runs/geometry/suppress 信息。**

这导致：
1. **marker 跑位** — marker.runs 的 origin_x 是固化的绝对坐标
2. **文字重复** — suppress 用的是固化 object_ids，可能不匹配当前渲染
3. **列表符号变大** — 新 runs 无 char_origins，fallback 字距不对
4. **光标跑位** — caret 基于 runs（固化的）计算，但显示的是新排版

## 五、修复方案

### 原则：单一状态源 + 编辑驱动更新

引入 `EditorPipeline` 统一管理编辑态：

```rust
struct EditorPipeline {
    // 不可变状态（打开时固定）
    paragraph_id: String,
    anchor_bbox: BoundingBox,
    source_runs: Vec<TextRun>,          // PDF 原始 runs
    source_text: String,                // PDF 原始文本
    marker_info: Option<MarkerInfo>,    // marker 位置相对于 anchor
    suppress_ids: HashSet<String>,      // 需要 suppress 的 object ids

    // 可变状态（编辑时更新）
    draft_text: String,
    caret_index: usize,
}

struct MarkerInfo {
    text: String,
    advance: f32,    // 相对于 anchor_bbox.left 的偏移
    style: RunStyle,
}

impl EditorPipeline {
    /// 打开编辑器
    fn open(paragraph, vector_model, target_id) -> Self

    /// 用户编辑 → 唯一更新入口
    fn apply_edit(&mut self, new_text: &str, new_caret: usize)

    /// 渲染 marker（相对于 anchor）
    fn render_marker(&self) -> MarkerRenderInfo

    /// 渲染 body（基于 draft_text 重新排版）
    fn render_body(&self, measure: MeasureFn) -> BodyRenderInfo

    /// 计算 suppress（不可变，始终 suppress 整个 source 区域）
    fn suppress_object_ids(&self) -> &HashSet<String>
}
```

### 关键改动

1. **消除状态分叉** — 删除 scene.body_session、scene.marker、scene.original_runs，全部走 document_plan
2. **marker 相对定位** — MarkerInfo.advance 相对于 anchor_bbox.left，渲染时转绝对坐标
3. **每次渲染重新排版** — render_body 基于 draft_text + source runs style 重新排版
4. **caret 基于新排版** — caret 计算用 render_body 的结果，不用固化 runs
5. **suppress 不变** — 打开时固定，始终 suppress 整个 source 区域

### 实施步骤

1. 先在 ParagraphEditorScene 中消除分叉：删除 body_session/marker/original_runs 冗余字段，统一走 document_plan
2. 修改 draw_editor_marker_page：marker 位置用 advance 相对定位
3. 修改 build_persisted_overlay_render_plan：draft runs 继承 source 的 object_ids 和 char_origins
4. 修改 caret 计算：基于 render_plan 的 layout 而非固化 runs
5. 加调试日志框架：每个关键动作前后打印状态快照
