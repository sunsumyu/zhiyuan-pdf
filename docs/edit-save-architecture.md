# 编辑 / 保存架构分析

> 目的：彻底厘清"用户编辑文字 → 退出编辑 → 修改持久显示"的完整链路，定位反复出现的"退出编辑后修改丢失 / 圆点消失"问题的架构成因，并给出重构方案。

---

## 1. 现有链路（实测代码 trace）

### 1.1 入口：JS → WASM facade

文件 `crates/pdf-viewer-ui/src/editor/facade.rs` 暴露 25+ `wasm_bindgen` 入口。与编辑生命周期相关的有：

| JS 名 | 对应 tx | 是否持久化 patch |
|---|---|---|
| `editorFacadeOpen` / `editorFacadeOpenRegion` | `open_editor_tx` | — |
| `editorFacadeSyncInput` / `editorFacadeApplyCommand` | `apply_input_tx` | 否（仅改 live state） |
| **`editorFacadeCommit`** | `commit_editor_tx` | **是** |
| `editorFacadeCommitSilent` | `commit_editor_silent_tx` | 是（无渲染） |
| **`editorFacadeClose`** | **`close_editor_tx`** | **否（直接丢弃 live state）** |

### 1.2 commit 路径（持久化）

```
facade_commit_editor (facade.rs:243)
  → commit_editor_tx (render_transaction.rs:135)
    → sync_editor_input               # 同步前端文本到 live state
    → commit_active_editor_text       # ★ 真正持久化
        ├─ build_active_editor_patch  (runtime.rs:344)
        │   ├─ 读 active live state
        │   ├─ patch_is_noop 判定     # 文本/样式/对齐/marker 都没变 → None
        │   └─ build_edit_replacement_snapshot  (replacement_snapshot.rs)
        ├─ remember_paragraph_replacement_target  # ★ 写 GLOBAL_PATCH_STATE.paragraph_replacement_targets
        ├─ apply_document_patch_direct (patch_persistence.rs:14)
        │   └─ record_patch (state_manager.rs:174)
        │       └─ apply_patch_maps   # ★ 写 paragraph_texts/snapshots/patches
        └─ close_active_editor        # 清 live state
```

### 1.3 close 路径（**直接丢弃**）

```
facade_close_editor (facade.rs:338)
  → close_editor_tx (render_transaction.rs:198)
    → close_active_editor          # 仅清 HOST_EDITOR_MODE.live_state
                                   # ★ 完全不调用 commit_active_editor_text
                                   # ★ patch 永远不进 GLOBAL_PATCH_STATE
```

### 1.4 渲染读取路径

```
渲染循环 → collect_paragraph_render_overlays (paragraph_overlay.rs:55)
  ├─ 1. 遍历 GLOBAL_PATCH_STATE.paragraph_patches  → PersistedPageCanvas overlay
  │     target 解析顺序：
  │     a. paragraph_replacement_targets.get(id)             ← commit.rs 写入
  │     b. replacement_target_from_patch_snapshot(patch)     ← 实际生产返回 None
  │     c. build_paragraph_render_target(plan, vector, id)   ← fallback
  └─ 2. 若有 active editor → ActiveEditorShell overlay

build_effective_vector_render_plan (effective_page_plan.rs:271)
  ├─ overlay_suppresses_text_source(overlay)         # 让原 PDF 文本不显示
  ├─ text_object_index_match                         # 路径 ① 整对象 suppress
  ├─ overlay_suppresses_row_paths                    # 路径 ② row 装饰线 suppress
  ├─ text_object_should_be_suppressed (object_ids)   # 路径 ③ 整对象 suppress
  └─ matching_text_run_refs (spatial)                # 路径 ④ 单 run suppress

draw_persisted_paragraph_overlay_page (canvas.rs:1303)
  ├─ draw_editor_marker_page                         # 绘 overlay 自己的 marker
  └─ build_persisted_overlay_render_plan(draft_text) # 绘新 body 文本
```

---

## 2. 已确认的 Bug 根因（按可能性排序）

### 2.1 **退出编辑被错调成 close 而非 commit**（强候选）

`editorFacadeClose` 不调 `commit_active_editor_text`，**所有未提交的 live state 直接被丢弃**。前端任何走 close 的退出路径（点编辑器外部、按 ESC、blur 事件、Ctrl+R 前的清理）都会丢失编辑。

**证据**：
- `close_editor_tx` 内只有 `close_active_editor()`
- 离线测试已证明：只要正常走 commit，patch 持久化和 overlay collection **完全正常**（4 个 wasm 测试全过）

### 2.2 marker run 被 suppress（已修复）

`text_object_should_be_suppressed` / `text_object_index_match` 命中后整对象 suppress，把同对象内的 `●` run 一并干掉。已改为 run 级 suppress + marker 字符黑名单。

### 2.3 marker 重复绘制 / 错位

`draw_persisted_paragraph_overlay_page` 调 `draw_editor_marker_page` 绘 overlay 的 marker，同时 marker run 现在又被保留了 → 可能出现两个 marker 重叠。需要二选一。

---

## 3. 架构问题诊断

### 3.1 **职责边界模糊**

| 函数 | 同时承担的职责 |
|---|---|
| `commit_active_editor_text` | ① 构建 patch ② 写 replacement_targets ③ 写 patches map ④ 关闭编辑器 ⑤ 触发渲染调度 |
| `apply_patch_maps` | ① 写 paragraph_texts ② 写 paragraph_snapshots ③ 写 paragraph_patches ④ 条件写 paragraph_replacement_targets |
| `collect_paragraph_render_overlays` | ① 读 patches ② 三路 fallback 解析 target ③ 读 active editor ④ 合并 marker overrides |
| `build_effective_vector_render_plan` | ① 视口裁剪 ② 文本 suppress ③ 路径 suppress ④ overlay 排序 ⑤ marker 决策 |

### 3.2 **状态分散且语义重叠**

```
┌─ HOST_EDITOR_MODE (thread_local)             ─┐  active live state（编辑中）
└───────────────────────────────────────────────┘

┌─ GLOBAL_PATCH_STATE (OnceLock<RwLock>) ────────────────────────┐
│  paragraph_texts:               id → new_text                   │
│  paragraph_snapshots:           id → ParagraphRegionSnapshot    │
│  paragraph_patches:             id → PersistableRegionPatch     │
│  paragraph_replacement_targets: id → ActiveEditorTarget         │  ← 同一信息有两条写入路径
│  history / redo_stack / accepted_patch_keys                     │
└─────────────────────────────────────────────────────────────────┘
```

`paragraph_replacement_targets` 由 **两条不一致的路径** 写入：
- `commit.rs::commit_active_editor_text` → `remember_paragraph_replacement_target`（生产路径）
- `state_manager.rs::apply_patch_maps` → 从 `patch.snapshot.replacementTarget` 读取（**但生产 patch 的 snapshot 不写这个字段**——见 `replacement_snapshot.rs` 自身的测试）

→ 哪条路径生效完全依赖 commit.rs 的副作用调用，**没有任何不变量保护**。

### 3.3 **Source suppression 启发式分散**

`build_effective_vector_render_plan` 内部对每个对象逐个 overlay 应用 4 条独立检查：

```
text_object_index_match  ─┐
text_object_should_be_     ├─ 任一命中即"整对象 suppress"
suppressed (object_ids)   ─┤   （直到我刚改成 run 级才避免误杀 marker）
matching_text_run_refs    ─┘
overlay_suppresses_row_paths（路径装饰）
```

每条规则的"什么时候应该 suppress"**没有形式化**，靠注释和经验。新增 list-marker 这类边界场景时，三条规则都要改。

### 3.4 **退出编辑器的语义二义性**

业界常见三种退出语义：

| 语义 | 应该做什么 | 当前实现 |
|---|---|---|
| Cancel（ESC） | 丢弃 live state，**保留之前已 commit 的 patch** | ✅ close 路径正确 |
| Commit（Enter / blur / 切换段落） | 把 live state 转 patch，再清 live state | ✅ commit 路径正确 |
| Close（Ctrl+R 前 / 应用关闭） | 同 Commit | ❌ **若调 close 即丢失** |

UI 层有没有选错语义？——facade 层无法保证。

---

## 4. 重构提案（设计模式 + 边界划分）

### 4.1 **顶层：Editor Lifecycle 状态机**

把 editor 状态收敛为有限状态机，所有 facade 入口都是状态迁移：

```
        ┌─────────────────────────────────────────────────┐
        │                                                 │
        │   Idle ──open──▶ Editing ──input──▶ Editing     │
        │                     │                           │
        │                     ├──commit────▶ Persisting ──┘
        │                     │                  │
        │                     │                  ▼
        │                     ├──cancel────▶ Idle (live state 丢弃，patches 不变)
        │                     │
        │                     └──close─────▶ Persisting (force-commit) ──▶ Idle
        │                                                                  │
        └──────────────────────────────────────────────────────────────────┘
```

**关键不变量**：
> Editing → Idle 的迁移**只能**经过 Persisting（commit）或 cancel（明确丢弃）。
> "Close without commit" 不是合法迁移。

实施：把 `close_editor_tx` 删除（或重命名 `cancel_editor_tx`，明确丢弃语义）；新增 `force_commit_editor_tx` 给 reload / app close 用。前端永远只能调 commit / cancel 二选一。

### 4.2 **中层：分层职责**

```
┌──────────────────────────────────────────┐
│  Facade Layer  (facade.rs)               │   仅做 JsValue ↔ Rust 转换
└──────────────────────────────────────────┘
              ↓
┌──────────────────────────────────────────┐
│  Application Layer  (use cases)          │   单一用例 = 单个 pub fn
│   - OpenEditorUseCase                    │
│   - InputEditorUseCase                   │
│   - CommitEditorUseCase                  │   ← 唯一持久化入口
│   - CancelEditorUseCase                  │
└──────────────────────────────────────────┘
              ↓                ↓
┌─────────────────────┐  ┌─────────────────────┐
│  Domain Layer       │  │  Render Layer       │
│   - EditSession     │  │   - OverlayCollector│
│   - PatchBuilder    │  │   - SourceSuppressor│
│   - PatchStore      │  │   - OverlayPainter  │
└─────────────────────┘  └─────────────────────┘
```

**Domain 不依赖 Render，Render 不依赖 Domain 内部，两者通过 `PatchSnapshot` value object 通讯**。

### 4.3 **PatchStore：单一写入路径（Repository pattern）**

```rust
trait PatchStore {
    fn put(&mut self, patch: Patch) -> Result<PatchHandle>;  // 唯一写入
    fn remove(&mut self, id: &PatchId) -> Option<Patch>;
    fn get(&self, id: &PatchId) -> Option<&Patch>;
    fn iter_by_page(&self, page: PageIndex) -> impl Iterator<Item = (&PatchId, &Patch)>;
}
```

`Patch` value object **自带** `replacement_target`（不再依赖 commit.rs 的副作用）：

```rust
struct Patch {
    id: PatchId,
    page: PageIndex,
    region_id: RegionId,
    original: TextSnapshot,
    edited:   TextSnapshot,
    replacement_target: ReplacementTarget,  // ← 从 EditSession 提取，强制内嵌
    marker_override: Option<String>,
    style_changes: Option<StyleChanges>,
}
```

**没有"snapshot.replacementTarget vs paragraph_replacement_targets"的歧义**——target 在 `Patch` 内部，没有第二条路径。

### 4.4 **SourceSuppressor：策略模式**

把"原 PDF 哪些东西要被 suppress"显式建模：

```rust
trait SuppressionRule {
    fn applies_to(&self, obj: &VectorRenderObject, overlay: &Overlay) -> SuppressionDecision;
}

enum SuppressionDecision {
    Keep,                            // 完全保留
    SuppressEntireObject,
    SuppressRuns(BitSet),            // 按 run index suppress
}

struct SourceSuppressor {
    rules: Vec<Box<dyn SuppressionRule>>,
}
```

规则按职责拆分：
- `OwnedTextObjectRule`（object_id 命中，但**保留** marker run）
- `BodyAreaSpatialRule`（空间相交，**排除** marker 字符）
- `DecorativePathRule`（细水平线在 body 区域）
- `RowBoundingPathRule`（路径相交 body row）

**每条规则单独可测；marker 保留是 `OwnedTextObjectRule` 内部的不变量，不再散落多处**。

### 4.5 **OverlayCollector：消除 fallback 三连**

当前 target 解析有 3 条 fallback：
```
paragraph_replacement_targets → snapshot.replacementTarget → build_paragraph_render_target
```

**根因**：`Patch` 没自带 target。按 §4.3 内嵌后，collector 直接：
```rust
for patch in store.iter_by_page(page) {
    overlays.push(Overlay::from_patch(patch));   // 无 fallback
}
```

### 4.6 **Marker 渲染：单一来源（Single Source of Truth）**

当前 marker 的可能来源：
- 原 PDF 文本对象内的 marker run（保留）
- `marker_text_override`（overlay 字段）
- `draw_editor_marker_page`（独立绘制）

**统一策略**：每个 paragraph overlay **只信任一处** marker 决策，由 `MarkerResolver` 在构建 overlay 时计算唯一结果：

```rust
struct MarkerResolver;
impl MarkerResolver {
    fn resolve(&self, source: &SourceParagraph, edit: &EditSession) -> MarkerSource {
        // 优先级：用户显式 override > 原 PDF marker run > 推断的 list 序号
    }
}

enum MarkerSource {
    PreserveOriginal,      // 不绘 overlay marker，让原 PDF marker run 显示
    DrawOverlay(String),   // 绘 overlay marker，suppress 原 PDF marker run
}
```

**渲染端只看 `MarkerSource`，不再有"两个都画 / 两个都不画"的中间态**。

---

## 5. 与当前问题的对应

| 当前症状 | 当前架构成因 | 重构后如何避免 |
|---|---|---|
| 退出编辑修改丢失 | close 路径绕过 commit | §4.1 状态机移除非法迁移 |
| 圆点（marker）消失 | 三条 suppression 规则各自整对象 suppress | §4.4 规则化 + §4.6 marker 单一来源 |
| target 解析有时失败 | snapshot/global map 双路径，写入不同步 | §4.3 Patch 内嵌 target |
| 改一处坏多处 | use case 跨层（commit 同时管 4 件事） | §4.2 分层 + 单一职责 |
| 调试加日志要散布多处 | 同一逻辑分散在 commit / state_manager / paragraph_overlay 三处 | §4.3 单一 PatchStore |

---

## 6. 实施路线（不破坏现有功能的渐进式重构）

| 阶段 | 改动 | 风险 |
|---|---|---|
| P0（1 步） | `close_editor_tx` 改为内部走 commit；`facade_close_editor` 文档化为"force commit & close" | 极低，纯收口 |
| P1（2-3 步） | 把 `replacement_target` 内嵌到 `PersistableRegionPatch`；删 `paragraph_replacement_targets` map | 中，需迁移序列化兼容 |
| P2（5+ 步） | 抽 `PatchStore` trait；commit/use case 分层 | 中 |
| P3（持续） | 把 4 条 suppression 规则抽成 `Vec<Box<dyn SuppressionRule>>` | 低，每条独立可测 |
| P4（持续） | 引入 `MarkerResolver`，移除 `draw_editor_marker_page` 与 marker run 共存的歧义 | 中 |

每阶段都要补 wasm-pack 离线测试（已有 4 个，作为基线）。

---

## 7. 立即行动建议（针对当前 bug）

**P0 一行修复**（不重构，仅止血）：

```rust
// editor/render_transaction.rs:198
pub fn close_editor_tx(frame_request: FramePlanRequest) -> EditorRenderTransactionResult {
    // 旧：let changed = close_active_editor();
    // 新：force commit before close
    let live_text = active_editor_draft_text();
    let action = if let Some(text) = live_text {
        commit_active_editor_text(text)
    } else {
        EditorVisibilityAction { changed: close_active_editor(), request_visibility_render: true }
    };
    EditorRenderTransactionResult {
        changed: action.changed,
        render_frame: schedule_editor_render(&frame_request, action.changed || action.request_visibility_render),
    }
}
```

加一个 wasm-pack 测试 `close_editor_persists_pending_edit`：在 live state 有 draft text 的情况下调 close → 验证 `paragraph_patches.len() == 1`。

如果这个改完问题就消失，根因就是 §2.1。
