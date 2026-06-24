# PDF Viewer 项目架构文档

## 一、项目结构

```
pdf-viewer-standalone/
├── crates/
│   ├── pdf-viewer-core/    # 纯 Rust 核心库（无 WASM 依赖）
│   └── pdf-viewer-ui/      # WASM/UI 层（依赖 web_sys）
├── src-tauri/              # Tauri 桌面应用后端
└── src/                    # 前端 TypeScript/React
```

## 二、核心模块划分

### 2.1 pdf-viewer-core（纯 Rust）

| 模块 | 职责 | 关键文件 |
|------|------|----------|
| `models/` | 数据模型定义 | layout.rs, glyph.rs, vector.rs |
| `edit/` | 编辑态核心逻辑 | document_plan.rs, draft_layout.rs, engine_state.rs |
| `text/` | 文本处理 | text_model.rs, list_semantics.rs, style_mapper.rs |
| `geometry/` | 几何计算 | layout_engine.rs, bbox_ops.rs |
| `render/` | 渲染计划构建 | overlay_ops.rs, source_suppression.rs |
| `document/` | 文档结构 | page_region_models.rs |

### 2.2 pdf-viewer-ui（WASM 层）

| 模块 | 职责 | 关键文件 |
|------|------|----------|
| `render/` | Canvas 渲染 | canvas.rs, canvas_overlay.rs |
| `editor/` | 编辑器状态管理 | session/session.rs, engine_state.rs |
| `page/` | 页面状态存储 | page_store.rs |

## 三、编辑态完整渲染链路

### 3.1 数据流

```
用户输入
  → editor_api::sync_input()
  → LiveEditorParagraphState.text_model.current_text 更新
  → 触发渲染帧

渲染帧
  → canvas.rs::render_page()
  → 遍历 overlays
  → ActiveEditorShell 分支
  → draw_active_editor_shell_overlay_page()
    → if replaces_source:
        → draw_persisted_paragraph_overlay_page()
          → build_region() 计算遮盖区域
          → ctx.fill_rect() 画白色遮盖矩形
          → build_persisted_overlay_render_plan() 构建渲染计划
            → build_draft_paragraph_with_policy() 构建 draft paragraph
            → if has marker: 插入 marker run 到 paragraph.runs[0]
            → layout_paragraph() 排版
          → 逐行逐 run 渲染文字
```

### 3.2 关键状态

| 状态 | 位置 | 说明 |
|------|------|------|
| `EditorModeState` | thread_local | 全局编辑模式状态 |
| `LiveEditorParagraphState` | `EditorModeState.live_state` | 当前编辑段落状态 |
| `ActiveEditorTarget` | `LiveEditorParagraphState.target` | 编辑目标数据 |
| `ParagraphEditorScene` | `ActiveEditorTarget.scene` | 场景数据（含 document_plan） |
| `EditContext` | `scene.document_plan` | 文档计划（含 marker, body_session） |

### 3.3 Marker + Body 统一渲染流程

```rust
// 1. draw_persisted_paragraph_overlay_page 入口
let document_plan = &active_target.scene.document_plan;
let draft_text = &overlay.draft_text;  // 不含 marker

// 2. 构建渲染计划
let render_plan = build_persisted_overlay_render_plan(document_plan, draft_text, measure);

// 3. 在 build_persisted_overlay_render_plan 内部
let mut paragraph = build_draft_paragraph_with_policy(document_plan, draft_text, ...);

// 4. 插入 marker run
if let Some(marker) = &document_plan.marker {
    let mut marker_run = marker.runs[0].clone();
    marker_run.text = marker.text.clone();
    marker_run.origin_x = 0.0;
    paragraph.runs.insert(0, marker_run);
}

// 5. 统一排版
let plan = rebuild_layout_pipeline(paragraph, document_plan, draft_text, measure);

// 6. 渲染
for line in plan.layout.lines {
    for run in line.runs {
        let run_x = session.anchor_bbox.left + line.offset_x + run.origin_x;
        renderer.draw_text_run(&run.text, run_x, baseline_y, ...);
    }
}
```

## 四、已知死代码

| 文件 | 函数/模块 | 状态 | 原因 |
|------|-----------|------|------|
| canvas_overlay.rs | `draw_editor_marker_page` | ❌ 死代码 | marker 已整合到统一渲染流程 |

## 五、状态分叉问题（已修复）

### 5.1 原问题

`ParagraphEditorScene` 中存在冗余字段：
- `body_session` 和 `document_plan.body_session` 重复
- `marker` 和 `document_plan.marker` 重复
- `original_runs` 和 `document_plan.original_runs` 重复

### 5.2 解决方案

所有读取通过 accessor 方法走 `document_plan`：
- `scene.body_session()` → `document_plan.body_session`
- `scene.marker()` → `document_plan.marker`
- `scene.original_runs()` → `document_plan.original_runs`

## 六、核心数据结构

### 6.1 LayoutRun vs TextRun

| 特性 | LayoutRun | TextRun |
|------|-----------|---------|
| 坐标系 | 绝对页面坐标 | 可相对可绝对 |
| char_origins | 绝对坐标 | 相对偏移 |
| 分割操作 | 归零后重新计算 | 零变换直接切割 |
| 用途 | PDF 解析结果 | 编辑态中间表示 |

### 6.2 EditorDocumentPlan

```rust
struct EditorDocumentPlan {
    target_id: String,
    base_paragraph_id: String,
    shell_bbox: BoundingBox,           // 整行边界（含 marker）
    source_body_text: String,          // body 文本（不含 marker）
    body_session: ParagraphEditContext, // body 区域会话
    body_initial_caret: usize,         // 初始光标位置
    marker: Option<ParagraphEditorMarker>, // marker 信息
    original_runs: Vec<GlyphPaintRun>, // PDF 原始 runs
}
```

### 6.3 ParagraphEditorMarker

```rust
struct ParagraphEditorMarker {
    kind: ListMarkerKind,
    text: String,        // marker 文本（如 "•", "1."）
    advance: f32,        // 相对于 anchor_bbox.left 的偏移
    runs: Vec<LayoutRun>, // marker 的渲染信息
}
```

## 七、关键不变量

1. **anchor_bbox 必须包含整行**：`body_session.anchor_bbox` 应该是整行边界（含 marker），而非仅 body 边界
2. **draft_text 不含 marker**：用户在 textarea 中看到的只是 body 文本
3. **marker 在渲染时插入**：`build_persisted_overlay_render_plan` 中将 marker run 插入到 paragraph.runs[0]
4. **白色遮盖覆盖整行**：`resolve_preferred_bbox` 返回的区域必须包含 marker 和 body

## 八、测试覆盖

### 8.1 核心测试

| 测试 | 文件 | 验证点 |
|------|------|--------|
| `unified_layout_with_marker` | draft_layout.rs | marker + body 统一布局 |
| `preserves_active_geometry` | draft_layout.rs | 编辑后保留 PDF char_origins |
| `preserves_origins` | draft_layout.rs | compact PDF 场景 |

## 九、调试日志

### 9.1 关键日志点

| 事件 | 标签 | 位置 |
|------|------|------|
| 打开编辑器 | `document-plan.marker-split` | document_plan.rs |
| 渲染计划构建 | `unified-layout.*` | draft_layout.rs |
| overlay 渲染 | `paint.overlay.*` | canvas_overlay.rs |

### 9.2 查看日志

浏览器 F12 Console 中搜索：
- `unified-layout` — marker 统一布局
- `paint.overlay` — overlay 渲染
- `document-plan` — 文档计划构建

## 十、编译命令

```powershell
# 编译 WASM
npm run wasm:pdf-viewer-ui

# 启动开发服务器
npm run tauri:dev
```
