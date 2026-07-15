# 重构总结报告（2026-07-05）

> 分支：`codex/refactor-split`
> 跨度：21 commits / 243 files / ~核心 4500 行
> 状态：编译通过，118 项核心测试全部通过；但**已知 bug 未解决**——marker 跑到尾部。

---

## 目录
1. [重构目标](#1-重构目标)
2. [提交清单与时间线](#2-提交清单与时间线)
3. [三 Crate 模块结构变化](#3-三-crate-模块结构变化)
4. [核心架构变更](#4-核心架构变更)
5. [Semantic Block 模型详解](#5-semantic-block-模型详解)
6. [编辑→保存数据流](#6-编辑保存数据流)
7. [Region Materializer v2](#7-region-materializer-v2)
8. [Marker 相关修复历史](#8-marker-相关修复历史)
9. [测试覆盖](#9-测试覆盖)
10. [已知问题与未完成工作](#10-已知问题与未完成工作)
11. [关键文件清单](#11-关键文件清单)

---

## 1. 重构目标

本次重构在 `codex/refactor-split` 分支上完成三个目标：

1. **God File 拆分**：将超大的单文件按职责拆分为子模块（`pdf_write.rs`、`host/`、`document_plan.rs` 等）。
2. **状态集中化**：将分散的 `thread_local` 状态统一到 `ui_state_store`、`patch_store`、`editor_store`。
3. **语义块模型**：引入 `SemanticBlock` 抽象，把 list item（marker + body）当作统一编辑单元，避免编辑时 marker 与正文互相污染。

---

## 2. 提交清单与时间线

按时间正序排列（最早 → 最新）：

| # | Commit | 类型 | 描述 |
|---|--------|------|------|
| 1 | `2830b7f` | refactor | pdf_write: 提取 trait_def 和 helpers 子模块 |
| 2 | `f4c689a` | chore | 删除被 pdf_write/ 模块取代的旧 pdf_write.rs |
| 3 | `9d8d420` | feat | 新增 working_copy 模块，集中管理工作路径 |
| 4 | `2443c57` | refactor | 把 color/path utils 和 working_copy 迁到独立模块 |
| 5 | `95856ed` | fix | 修复 pdf-viewer-ui 和 src-tauri 的编译错误 |
| 6 | `6827c4b` | refactor | ui: host/ 重命名为 platform/，删除过时文件 |
| 7 | `e4dc02e` | feat | 三个 crate 全部新增拆分子模块 |
| 8 | `0091d80` | refactor | core: 完成编辑引擎和渲染管线拆分 |
| 9 | `4b5599e` | refactor | ui,tauri: 完成 UI 状态集中化和后端拆分 |
| 10 | `e50190a` | refactor | bridge: 拆分 editor 和 vector_host 模块 |
| 11 | `7b825d4` | docs | 新增架构设计文档和重构实施报告 |
| 12 | `bdae8e8` | chore | .gitignore 加入 .zed/ |
| 13 | `04d7fcf` | refactor | edit: 合并 paragraph scene 状态访问 |
| 14 | `a49c39c` | refactor | ui: 把 editor callbacks 集中到 app stores |
| 15 | `ecf34e4` | fix | marker: 保留绝对 char_origins 防止 marker 位移 |
| 16 | `79d0743` | chore | 删除未使用的 normalize_style_run_origins 函数 |
| 17 | `590cd78` | fix | pdf-open: 防止重复对话框并增加详细错误日志 |
| 18 | `baa09de` | debug | marker: 增加 split 和 detection 的 trace 日志 |
| 19 | `5822a6c` | feat | marker: 编辑和提交时保留图形 bullet |
| 20 | `34ec3ea` | fix | marker: 从 list body 中剥离尾部装饰字符 |
| 21 | `6ff7512` | feat | persistence: 新增 semantic block 模型用于列表编辑 |

**关键观察**：第 15-20 提交都是针对同一类 marker bug 的反复修补，每次都号称"修复"但 bug 依旧存在——表明根因未定位。

---

## 3. 三 Crate 模块结构变化

### 3.1 `crates/pdf-viewer-core`（核心引擎）

**新增模块**：
- `crates/pdf-viewer-core/src/models/semantic_block.rs`（509 行）—— 语义块模型
- `crates/pdf-viewer-core/src/edit/document_plan/`（marker.rs + tests.rs 子模块）
- `crates/pdf-viewer-core/src/edit/` 下新增多个职责模块：
  - `engine_state.rs`（651 行）—— 编辑器活跃状态
  - `paragraph_scene.rs`（315 行）—— 段落编辑场景
  - `replacement_snapshot.rs`（165 行）—— 编辑替换快照
  - `source_runs.rs`（423 行）—— 源 run 解析
  - `active_target.rs`、`bridge.rs`、`debug_trace.rs`、`draft_*.rs`

**修改模块**：
- `models.rs`：导出新增的 `semantic_block` 模块
- `models/marker.rs`：`VisualMarker` 类型整理
- `persistence/models.rs`：新增 `PersistableSemanticBlockSummary`、`PersistableSemanticOperation`
- `persistence/patch_store.rs`：新增 `semantic_ops` 跟踪与生命周期
- `edit/bridge.rs`：`build_rich_patch` 填充 semantic_block 和 semantic_ops
- `edit/engine_state.rs`：`LiveEditorParagraphState` 改用 SemanticBlock
- `edit/document_plan.rs`：`EditContext::semantic_block()` 适配方法
- `text/list_semantics.rs`：剥离尾部装饰字符的逻辑

### 3.2 `crates/pdf-viewer-ui`（UI 层）

**重命名**：
- `host/` → `platform/`（platform_bridge、command、layout、scroll、mod）

**新增/拆分**：
- `editor/orchestrator/`：commit、render_transaction、replace_pipeline
- `editor/overlay/`：navigation、paragraph_overlay、projection、visual
- `editor/editor_api/`：block、format、text 子模块
- `editor/format/`：list_format、text_geometry、text_index
- `editor/session/`：history、session
- `render/canvas/`：vector_draw
- `find/`、`present/`、`presentation/`、`review/`、`viewer/`、`zoom/` 等子模块全部拆分

**修改**：
- `editor/editor_controller.rs`：`build_patch` 检测 list kind / marker 变化并生成 semantic_ops
- `document/patch_persistence.rs`：`save_persistable_patches` 收集并发送 semantic_ops
- `ui_state_store.rs`：新增 `collect_persistable_semantic_ops`

### 3.3 `src-tauri`（后端）

**拆分**：
- `infrastructure/pdf/pdf_write/`：trait_def、helpers、merged_impl、annotation、mod
- `infrastructure/pdf/pdf_read/`：content_parser、graphics_state、image_builder、metadata、page_model、path_resolver、utils、mod
- `infrastructure/pdf/font/`：matching、metrics

**新增**：
- `infrastructure/pdf/working_copy.rs` —— 集中化工作路径管理
- `infrastructure/pdf/region_materializer.rs` 中的 `semantic_list_item_unit_text_reflow` 和 `build_region_materialization_plan_v2`

**修改**：
- `infrastructure/pdf/models.rs`：`PdfModifications` 新增 `semantic_ops` 字段
- `infrastructure/pdf/document_service.rs`：调用 v2 materialization
- `interfaces/pdf/replace.rs`：`apply_region_patches` 命令接受 `semantic_ops` 参数
- `interfaces/pdf/ipc_converters.rs`：传递 semantic_ops 到 v2

---

## 4. 核心架构变更

### 4.1 状态集中化（Before → After）

**Before**：分散的 `thread_local` 状态散落各处。
**After**：统一到三个 store：

```
ui_state_store.rs  ← UI 侧全局状态（patch_state、editor callbacks）
patch_store.rs     ← core 侧 GlobalPatchState（paragraph_patches、semantic_ops）
editor_store.rs    ← 编辑器会话状态
```

`GlobalPatchState` 现在的字段：
```rust
pub struct GlobalPatchState {
    pub paragraph_texts: HashMap<String, String>,
    pub paragraph_snapshots: HashMap<String, ParagraphRegionSnapshot>,
    pub paragraph_layout_snapshots: HashMap<String, ParagraphLayout>,
    pub paragraph_patches: HashMap<String, PersistableRegionPatch>,
    pub semantic_ops: HashMap<String, Vec<PersistableSemanticOperation>>,  // ← 新增
    pub paragraph_replacement_targets: HashMap<String, ActiveEditorTarget>,
    pub field_group_texts: HashMap<String, String>,
    pub field_group_snapshots: HashMap<String, serde_json::Value>,
    pub field_group_patches: HashMap<String, PersistableRegionPatch>,
    pub history: Vec<PatchCommand>,
}
```

### 4.2 host/ → platform/ 重命名

UI 侧的 `host/` 目录（host_mode、host_pipeline、host_snapshot、host_workflow 等）全部重命名为 `platform/`，语义更清晰：
- `platform_bridge.rs` —— 平台桥接
- `platform/command.rs`、`platform/layout.rs`、`platform/scroll.rs`

### 4.3 God File 拆分

| 原 God File | 拆分后 |
|-------------|--------|
| `pdf_write.rs` | `pdf_write/{trait_def, helpers, merged_impl, annotation, mod}.rs` |
| `pdf_read_service.rs` | `pdf_read/{content_parser, graphics_state, image_builder, metadata, page_model, path_resolver, utils, mod}.rs` |
| `document_plan.rs` | `document_plan/{marker, tests}.rs` + 主文件 |
| `editor_controller.rs` | `editor_api/{block, format, text}.rs` + controller |

---

## 5. Semantic Block 模型详解

### 5.1 设计动机

**问题**：编辑 list item 时，marker（如"•"、"1."）和 body（正文）作为整体存储在一个 `LayoutRun` 序列里。编辑流程把它们当成纯文本处理，导致：
- 改 body 时 marker 被错改
- 改 marker 时 body 被错改
- 图形 bullet（图片蓝点）被当作文本 run 误抑制
- 保存到 PDF 时 marker 和 body 拼接顺序错乱

**方案**：引入 `SemanticBlock` 显式区分 marker 和 body，记录各自的 object indices，并在 patch 上以"语义操作"形式描述变化。

### 5.2 核心类型（`crates/pdf-viewer-core/src/models/semantic_block.rs`）

```rust
pub struct SemanticBlock {
    pub id: SemanticBlockId,
    pub base_id: String,
    pub region_id: String,
    pub page_index: u16,
    pub kind: SemanticBlockKind,        // Paragraph | ListItem | FieldRow | Unknown
    pub shell_bbox: BoundingBox,
    pub body: SemanticTextBody,
    pub provenance: SourceProvenanceLite,
    pub validation: SemanticModelValidation,
}

pub enum SemanticBlockKind {
    Paragraph,
    ListItem(SemanticListItem),
    FieldRow,
    Unknown,
}

pub struct SemanticListItem {
    pub marker: Option<SemanticMarker>,         // 文本 marker（"•"、"1."）
    pub graphic_markers: Vec<SemanticMarker>,   // 图形 marker（图片蓝点）
    pub layout: SemanticListLayout,
}

pub struct SemanticMarker {
    pub kind: ListMarkerKind,                  // Bullet | Numbering | Custom | None
    pub content: SemanticMarkerContent,        // Text { text } | Graphic { object_index, ... }
    pub bbox: BoundingBox,
    pub advance: f32,
    pub runs: Vec<LayoutRun>,
    pub object_indices: Vec<usize>,
}

pub struct SourceProvenanceLite {
    pub body_object_indices: Vec<usize>,
    pub marker_object_indices: Vec<usize>,
    pub graphic_marker_object_indices: Vec<usize>,
}
```

### 5.3 验证规则

`SemanticBlock::validate()` 强制三个不变量：

1. **body 文本必须匹配 body runs 文本**（`validate_body_text_matches_body_runs`）
2. **list body 文本不得以 marker 文本开头**（`validate_list_item_body_excludes_marker`）
3. **body 和 marker 的 object_indices 不得重叠**（`validate_marker_body_object_sets_do_not_overlap`）

### 5.4 适配器方法

在 `EditContext`（原 `EditorDocumentPlan`）上新增：
```rust
impl EditContext {
    pub fn semantic_block(&self) -> SemanticBlock {
        // 从 source_body_text、body_session、marker、graphic_markers 构造 SemanticBlock
        // 区分 Paragraph 和 ListItem 两种构造路径
    }
}
```

`ParagraphEditorScene::semantic_block()`、`ActiveEditorTarget::semantic_block()` 转发到这个方法。

### 5.5 持久化类型

```rust
pub struct PersistableSemanticBlockSummary {
    pub block_id: String,
    pub region_id: String,
    pub kind: String,                        // "list-item" | "paragraph" | ...
    pub body_text: String,
    pub marker_text: Option<String>,
    pub body_object_indices: Vec<usize>,
    pub marker_object_indices: Vec<usize>,
    pub graphic_marker_object_indices: Vec<usize>,
}

pub enum PersistableSemanticOperation {
    ReplaceBodyText { block_id, old_text, new_text },
    SetListKind { block_id, list_kind, marker_text },
    SetListMarker { block_id, marker_text },
}

pub struct PersistableRegionPatch {
    // ... 既有字段 ...
    pub semantic_block: Option<PersistableSemanticBlockSummary>,  // ← 新增
    pub semantic_ops: Vec<PersistableSemanticOperation>,          // ← 新增
}
```

### 5.6 Patch Store 生命周期

`GlobalPatchState.semantic_ops` 按 region_id 索引：

```rust
pub fn apply_patch_maps(state, patch) {
    if patch.semantic_ops.is_empty() {
        state.semantic_ops.remove(&patch.region_id);   // 清空旧 ops
    } else {
        state.semantic_ops.insert(patch.region_id.clone(), patch.semantic_ops.clone());
    }
}

pub fn collect_semantic_ops(state) -> Vec<PersistableSemanticOperation> {
    state.semantic_ops.values().flat_map(|ops| ops.iter().cloned()).collect()
}
```

`reject_review_change`、`reject_all_changes`、`clear_persistable_patches` 都同步清理 semantic_ops。

---

## 6. 编辑→保存数据流

完整链路（标注每一步所在文件）：

```
┌─────────────────────────────────────────────────────────────────────────┐
│ 1. 用户编辑 list item                                                    │
│    src/bridge/editor/input_handler.ts                                    │
│    → editor_api/text.rs                                                  │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 2. 构造 EditContext + SemanticBlock                                      │
│    crates/pdf-viewer-core/src/edit/document_plan.rs                      │
│       └─ EditContext::semantic_block()                                   │
│    crates/pdf-viewer-core/src/edit/paragraph_scene.rs                    │
│       └─ ParagraphEditorScene::semantic_block()                          │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 3. LiveEditorParagraphState 初始化（用 SemanticBlock 替代直接访问 scene） │
│    crates/pdf-viewer-core/src/edit/engine_state.rs                       │
│       └─ LiveEditorParagraphState::new(target)                           │
│           source_text = semantic_block.body.text                         │
│           list_kind = semantic_block.kind.list_item.source_list_kind()   │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 4. 用户提交 → build_patch                                                │
│    crates/pdf-viewer-ui/src/editor/editor_controller.rs                  │
│       └─ build_patch(new_text)                                           │
│           调用 build_rich_patch → 填充 semantic_block、semantic_ops       │
│           检测 list kind 变化 → push SetListKind                         │
│           检测 marker 文本变化 → push SetListMarker                      │
│           构造 snapshot via build_edit_replacement_snapshot              │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 5. build_rich_patch（core 侧构造 patch）                                 │
│    crates/pdf-viewer-core/src/edit/bridge.rs                             │
│       └─ build_rich_patch(paint_plan, vector_model, paragraph_id, ...)   │
│           semantic_summary_from_scene(scene) → PersistableSemanticBlock- │
│             Summary                                                     │
│           if original_text != new_text:                                  │
│             semantic_ops.push(ReplaceBodyText)                           │
│           PersistableRegionPatch { semantic_block, semantic_ops, ... }    │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 6. record_patch（存入 GlobalPatchState）                                 │
│    crates/pdf-viewer-ui/src/ui_state_store.rs                            │
│       └─ record_patch → apply_patch_maps                                 │
│    crates/pdf-viewer-core/src/persistence/patch_store.rs                 │
│       └─ apply_patch_maps：按 region_id 写入 semantic_ops                │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 7. save_persistable_patches（保存时收集并 IPC 发送）                     │
│    crates/pdf-viewer-ui/src/document/patch_persistence.rs                │
│       └─ save_persistable_patches(path, page_index)                      │
│           patches = collect_persistable_patches()                        │
│           semantic_ops = collect_persistable_semantic_ops()              │
│           raw_invoke("apply_region_patches", { patches, semanticOps })   │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 8. apply_region_patches（Tauri 命令）                                    │
│    src-tauri/src/interfaces/pdf/replace.rs                               │
│       └─ apply_region_patches(path, page_index, patches, semantic_ops)   │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 9. execute_region_patches                                                │
│    src-tauri/src/interfaces/pdf/ipc_converters.rs                        │
│       └─ build_region_materialization_plan_v2(patches, semantic_ops, []) │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 10. build_region_materialization_plan_v2                                 │
│     src-tauri/src/infrastructure/pdf/region_materializer.rs              │
│        └─ semantic_list_item_unit_text_reflow(patch)                     │
│            target_indices = marker_object_indices ++ body_object_indices  │
│            new_text = combine_list_item_text(marker, body)               │
│            → TextReflowPatch { target_indices, new_text, new_runs: None }│
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 11. BatchTextReflowCommand 写入 PDF                                      │
│     src-tauri/src/infrastructure/pdf/pdf_write/merged_impl.rs            │
│        └─ ReflowCluster::build：anchor = min(target_indices)             │
│        └─ patch_atomic_reflow_recursive：                                │
│            遍历 PDF content operations                                   │
│            遇到 target_idx 在 silenced 集合 → 静音原 Tj                  │
│            遇到 target_idx == cluster.min_idx → 注入 new_text 的 layout  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Region Materializer v2

### 7.1 v1 vs v2

```rust
// v1（向后兼容包装）
pub fn build_region_materialization_plan(
    region_patches: &[PersistableRegionPatch],
    text_reflows: &[TextReflowPatch],
) -> RegionMaterializationPlan {
    build_region_materialization_plan_v2(region_patches, &[], text_reflows)
}

// v2（接受显式 semantic_ops）
pub fn build_region_materialization_plan_v2(
    region_patches: &[PersistableRegionPatch],
    semantic_ops: &[PersistableSemanticOperation],
    text_reflows: &[TextReflowPatch],
) -> RegionMaterializationPlan { ... }
```

**所有生产代码已迁移到 v2**（document_service、ipc_converters）。v1 仅保留给旧测试。

### 7.2 semantic_list_item_unit_text_reflow

```rust
fn semantic_list_item_unit_text_reflow(
    patch: &PersistableRegionPatch,
) -> Option<(Vec<TextReflowPatch>, RegionMaterializationDecision)> {
    let summary = patch.semantic_block.as_ref()?;
    if summary.kind != "list-item" { return None; }

    let marker_text = patch.new_marker_text.clone()
        .or_else(|| summary.marker_text.clone())
        .or_else(|| patch.marker_text.clone())
        .filter(|text| !text.is_empty())?;

    if summary.marker_object_indices.is_empty()
        || summary.body_object_indices.is_empty() { return None; }

    // ⚠️ 关键：target_indices 用 BTreeSet 排序，丢失 marker-first 顺序
    let mut target_indices = summary.marker_object_indices.iter()
        .chain(summary.body_object_indices.iter())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    target_indices.sort_unstable();

    let body_text = patch.snapshot.as_ref()
        .and_then(|v| v.get("bodyText"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .unwrap_or_else(|| patch.new_text.clone());

    let new_text = combine_list_item_text(&marker_text, &body_text);

    Some((
        vec![TextReflowPatch {
            target_indices,
            new_text: normalize_region_text(new_text),
            new_runs: None,  // ⚠️ 故意丢弃 body-only new_runs
            ...
        }],
        RegionMaterializationDecision {
            reason: "semantic-list-item-text-marker-unit".to_string(),
            ...
        },
    ))
}
```

**设计意图**：marker + body 作为整体文本单元写入 PDF，避免 body-only 的 legacy `new_runs` 与 marker+body 文本不匹配。

**潜在问题（待验证）**：`target_indices` 用 `BTreeSet` + `sort_unstable` 排序，丢失了 marker 在前、body 在后的文档流顺序。PDF 写入侧 `ReflowCluster::build` 用 `min(target_indices)` 做 anchor——如果 marker 的 object index 数值大于 body，anchor 落在 body 位置，可能导致 marker 文本视觉位置错误。**这正是当前 marker 跑到尾部 bug 的疑似根因，尚未最终确认。**

### 7.3 materialize_list_item_patch_to_text_reflow（非 semantic 路径）

```rust
fn materialize_list_item_patch_to_text_reflow(patch) -> (...) {
    if let Some(semantic_result) = semantic_list_item_unit_text_reflow(patch) {
        return semantic_result;   // ← 优先走 semantic 路径
    }
    // 否则走 legacy 路径：用 full_target_indices 或 target_indices
    let target_indices = if has_marker_update && !patch.full_target_indices.is_empty() {
        patch.full_target_indices.clone()
    } else {
        patch.target_indices.clone()
    };
    let new_text = if has_marker_update {
        combine_list_item_text(&marker_text, &body_text)
    } else {
        patch.new_text.clone()
    };
    ...
}
```

---

## 8. Marker 相关修复历史

下表是过去几次 marker 相关"修复"的演进。**每次都号称修复了，但 bug 依旧**——这反映了根因定位的失败：

| Commit | 描述 | 改动文件 | 实际效果 |
|--------|------|---------|---------|
| `ecf34e4` | 保留绝对 char_origins 防止 marker 位移 | document_plan/marker.rs | 部分缓解，未根治 |
| `baa09de` | 增加 marker split 和 detection 的 trace 日志 | 多文件 | 仅诊断，无修复 |
| `5822a6c` | 编辑和提交时保留图形 bullet | list_semantics、bridge | 图形 marker 不再丢失，但位置问题仍在 |
| `34ec3ea` | 从 list body 中剥离尾部装饰字符 | list_semantics.rs（+142/-12） | body 不再含装饰字符，但 marker 位置问题仍在 |
| `6ff7512` | semantic block 模型 | 多文件 | 引入新抽象，但 bug 依旧 |

**教训**：所有修复都集中在"marker 文本内容"层面（剥离、保留、检测），没有触及"marker 在 PDF 写入时的位置锚点"层面。真正根因很可能在 `region_materializer.rs` 的 `target_indices` 构造和 `merged_impl.rs` 的 `ReflowCluster::build` 锚点选择上。

---

## 9. 测试覆盖

### 9.1 通过的测试（118 项核心 + 22 项 src-tauri）

**semantic_block.rs 单元测试**：
- `list_item_body_excludes_text_marker` —— body 不含 marker 文本
- `validation_rejects_marker_in_body_text` —— body 以 marker 开头时拒绝
- `validation_rejects_overlapping_marker_and_body_objects` —— object indices 重叠时拒绝

**patch_store.rs 单元测试**：
- `apply_and_remove_patch_maps_tracks_semantic_ops` —— apply/remove 生命周期
- `applying_patch_without_semantic_ops_clears_previous_region_ops` —— 替换时清空旧 ops

**region_materializer.rs 单元测试**：
- `semantic_text_marker_list_item_materializes_marker_and_body_as_unit` —— marker+body 单元化
- `semantic_graphic_marker_list_item_keeps_body_only_legacy_materialization` —— 图形 marker 走 legacy
- `body_only_list_patch_materializes_body_targets_only` —— 仅 body 时不合并
- `marker_update_uses_full_targets_and_combines_marker_before_body` —— marker 改变时用 full_targets
- `v2_keeps_legacy_materialization_compatible_when_semantic_ops_exist` —— v2 兼容性

**document_plan/tests.rs**：
- `semantic_block_adapter_keeps_marker_out_of_body` —— 适配器保持分离

### 9.2 失败的测试（1 项，与重构无关）

- `infrastructure::pdf::preview_engine::tests::test_diagnose_scanned_pdf`
  - 原因：硬编码本地路径 `C:\Users\AREN\Documents\刘---20250514 - 副本 (3) - 副本.pdf` 不存在
  - 与本次重构无关，是预先存在的环境依赖测试

### 9.3 测试缺口

**没有端到端测试**覆盖以下场景：
1. 编辑 list item body 后保存 PDF，验证 marker 位置正确
2. 编辑 marker 文本后保存 PDF，验证 body 位置正确
3. 图形 bullet + 文本 marker 混合场景的保存验证

**这正是 bug 反复出现的根本原因**——单元测试只验证了数据结构的内部一致性，没有验证 PDF 写入后的视觉效果。

---

## 10. 已知问题与未完成工作

### 10.1 ⚠️ 未解决的核心 Bug：marker 跑到尾部

**现象**：编辑 list item 后保存，marker 在 PDF 中从段落开头跑到了尾部。

**疑似根因**（待最终确认）：
- `region_materializer.rs:219-230` 的 `semantic_list_item_unit_text_reflow` 用 `BTreeSet` + `sort_unstable` 对 `target_indices` 排序，丢失 marker-first 顺序
- `bridge.rs:117-127` 的 `full_target_indices` 构造同样用 `BTreeSet` 排序
- `replacement_snapshot.rs:81-108` 的 `collect_object_indices` 同样用 `BTreeSet` 排序
- PDF 写入侧 `merged_impl.rs:448` 的 `ReflowCluster::build` 用 `min(target_indices)` 做 anchor，当 marker 的 object index 数值大于 body 时，anchor 落在 body 位置

**待验证**：需要实际 PDF 复现 + 日志确认 anchor 位置。

### 10.2 待清理的兼容代码

- `document_plan.rs:711-746` 的 deprecated 别名（`EditorDocumentPlan`、`build_editor_document_plan` 等），注释说"保留一个周期后删除"
- `region_materializer.rs:668-673` 的 v1 包装函数，可在所有调用方迁移完后删除

### 10.3 Clippy 警告

37 项警告（大多是预先存在的）：
- `deprecated_semver`：`#[deprecated(since = "2026.6", ...)]` 应改为 semver 格式如 `"0.2.0"`
- `too_many_arguments`：多个函数参数超过 7 个
- `large_enum_variant`：`NativePageObject`、`EffectiveVectorRenderEntry` 等枚举变体大小差异大
- `derivable_impls`：`SemanticBlockId`、`SemanticBlockKind`、`ParagraphEditorScene` 的 Default 可改为 derive

### 10.4 前端 TS 类型未同步

`src/bridge/ai/resume_ai_types.ts` 的 `PdfPersistableRegionPatch` 接口缺少 `semanticOps`、`semanticBlock` 字段。由于 semantic_ops 在 Rust 层自动生成，前端不手动构造，所以不影响功能，但类型定义不完整。

### 10.5 测试缺口

缺少端到端的 PDF 保存验证测试（见 9.3）。

---

## 11. 关键文件清单

### 核心新增
| 文件 | 行数 | 职责 |
|------|------|------|
| `crates/pdf-viewer-core/src/models/semantic_block.rs` | 509 | 语义块模型定义与验证 |
| `src-tauri/src/infrastructure/pdf/working_copy.rs` | - | 工作路径集中管理 |

### 核心修改
| 文件 | 行数 | 职责 |
|------|------|------|
| `crates/pdf-viewer-core/src/edit/document_plan.rs` | 746 | EditContext + semantic_block() 适配 |
| `crates/pdf-viewer-core/src/edit/engine_state.rs` | 651 | LiveEditorParagraphState（用 SemanticBlock） |
| `crates/pdf-viewer-core/src/edit/source_runs.rs` | 423 | target_paint_runs（body runs 提取） |
| `crates/pdf-viewer-core/src/edit/bridge.rs` | 339 | build_rich_patch（填充 semantic_block/ops） |
| `crates/pdf-viewer-core/src/edit/paragraph_scene.rs` | 315 | ParagraphEditorScene |
| `crates/pdf-viewer-core/src/edit/replacement_snapshot.rs` | 165 | build_edit_replacement_snapshot |
| `crates/pdf-viewer-core/src/persistence/models.rs` | 106 | PersistableSemantic* 类型 |
| `crates/pdf-viewer-core/src/persistence/patch_store.rs` | 286 | semantic_ops 生命周期 |
| `crates/pdf-viewer-core/src/edit/document_plan/marker.rs` | 352 | ParagraphEditorMarker + split_editor_session |
| `crates/pdf-viewer-core/src/text/list_semantics.rs` | 313 | 装饰字符剥离 |
| `crates/pdf-viewer-ui/src/editor/editor_controller.rs` | - | build_patch（生成 SetListKind/SetListMarker） |
| `crates/pdf-viewer-ui/src/document/patch_persistence.rs` | - | save_persistable_patches |
| `crates/pdf-viewer-ui/src/ui_state_store.rs` | - | collect_persistable_semantic_ops |
| `src-tauri/src/infrastructure/pdf/region_materializer.rs` | - | semantic_list_item_unit_text_reflow + v2 |
| `src-tauri/src/infrastructure/pdf/models.rs` | - | PdfModifications.semantic_ops |
| `src-tauri/src/infrastructure/pdf/document_service.rs` | - | 调用 v2 |
| `src-tauri/src/interfaces/pdf/replace.rs` | - | apply_region_patches 命令 |
| `src-tauri/src/interfaces/pdf/ipc_converters.rs` | - | execute_region_patches |
| `src-tauri/src/infrastructure/pdf/pdf_write/merged_impl.rs` | - | ReflowCluster + PDF 写入 |

---

## 附：如何继续定位 marker bug

1. **复现**：准备一个包含文本 marker 的 list item PDF，编辑 body，保存。
2. **加日志**：在 `semantic_list_item_unit_text_reflow` 输出 `target_indices`、`marker_object_indices`、`body_object_indices`、`new_text`。
3. **在 `ReflowCluster::build` 输出 `min_idx`、`max_idx`**。
4. **在 `patch_atomic_reflow_recursive` 输出每个 `target_idx` 命中时的 `(ax, ay)` 位置**。
5. **对比**：marker 的 object index 是不是大于 body？cluster anchor 落在哪？注入位置在哪？
6. **定位后**：根据 anchor 落点决定是改 `target_indices` 顺序，还是改 cluster anchor 选择策略（改为按文档流顺序的第一个 index 而非数值 min）。

---

*文档生成于 2026-07-05，基于 commit `6ff7512`。*
