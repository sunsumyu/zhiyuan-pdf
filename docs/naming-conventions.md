# 命名规范（Naming Conventions）

> 综合自 `origin/pdf-engine-naming-guide.md`、`archive/naming_convention_rules.md`、
> `naming-refactor-review-plan.md`、`architecture-principles.md` 以及 Rust API Guidelines。
>
> 最后更新：2026-06-17

---

## 1. 核心原则

1. **动词 + 名词** — 清楚表达"做什么" + "对什么做"
2. **语义明确** — 避免模糊词汇（get, do, process, handle）
3. **能力分层** — 按函数能力类型选择动词
4. **模块路径承载上下文** — 函数名不应重复完整模块路径
5. **介词前置连接** — `from_xxx` / `with_xxx`，禁止 `xxx_for_yyy` 后缀模式

---

## 2. 能力分类与动词选择

| 能力 | 动词 | 副作用 | 示例 |
|------|------|-------|------|
| **Query** | `read`, `find`, `list`, `search`, `get` | 无 | `read_metadata()`，`get_scale()` |

**`get_*` vs `read_*` 区分：**
- `get_*` — 纯属性获取，无计算成本（Rust 标准库大量使用 `get()`，无 JS getter 冲突）
- `read_*` — 有IO/反序列化/拼接等计算成本的查询
| **Resolve** | `resolve`, `compute` | 无 | `resolve_layout()` |
| **Transform** | `convert`, `transform`, `project` | 无 | `convert_to_layout_runs()` |
| **Validate** | `is`, `has`, `should`, `can` | 无 | `is_preview_active()` |
| **Create** | `new`, `create`, `build`, `init` | 有 | `create_document()` |
| **Destroy** | `delete`, `remove`, `clear`, `close` | 有 | `close_pdf_resources()` |
| **Mutate** | `set`, `update`, `apply`, `toggle`, `preserve` | 有 | `apply_highlight()`，`preserve_line_styles()` |
| **Execute** | `execute`, `commit`, `dispatch` | 有 | `execute_save()` |
| **Lifecycle** | `start`, `stop`, `schedule`, `advance` | 有 | `start_render_frame()` |
| **Sync** | `sync` | 有（跨边界） | `sync_editor_input()` |

---

## 3. 构造器命名（Rust API Guidelines）

遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/naming.html)：

| 模式 | 用途 | 示例 |
|------|------|------|
| `new()` | 默认/最小构造器 | `Document::new()` |
| `from_xxx()` | 从 xxx 输入构造 | `EditContext::from_paragraph()` |
| `from_target_id()` | 从 target_id 构造 | `EditContext::from_target_id()` |
| `with_xxx()` | 非 builder 类型带配置 | `HashMap::with_capacity(100)` |
| `xxx()` | Builder setter（裸名） | `builder.timeout(30)` |

**禁止模式：**

```rust
// ❌ 介词后缀
build_plan_for_target()
prepare_context_for_session()

// ✅ 介词前置连接
build_target_plan()
prepare_session_context()
from_target_id()

// ❌ new_with 风格（改用 builder）
MyStruct::new_with_config()

// ✅ builder 模式
MyStruct::new().with_config()
```

---

## 4. 函数前缀详解

### Query 查询类

- `read_*` — 查询状态或 IO-backed 数据（明确语义，避免与 JS getter 混淆）
- `find_*` — 返回 `Option<T>` / 可空值
- `list_*` / `search_*` — 返回集合

```rust
// ✅ 正确
read_metadata()
find_paragraph()
search_regions()

// ❌ 避免
get_metadata()      // get 与 JS getter 易混淆
get_preview_active() // 返回 bool 应用 is_
```

### Resolve 计算类

- `resolve_*` — 由输入 + 回退规则导出决定
- `compute_*` — 纯数学计算

```rust
// ✅ 正确
resolve_layout()
resolve_edit_context()
compute_baseline()

// ❌ 避免（已完成迁移）
extract_*()
measure_*()  // 改用 resolve_
```

### Create 创建类

- `new` — trait/struct 默认构造器
- `create_*` — 带副作用创建
- `build_*` — 无副作用纯值构造
- `init_*` — 初始化上下文

```rust
// ✅ 正确
Document::new()
create_page_context()
build_styles()
init_page()

// ❌ 避免
open_document()     // 改用 create_document 或 read_document
generate_demo_pdf() // 改用 create_demo_pdf
```

### Validate 验证类

- `is_*` — 返回 bool，判断状态
- `has_*` — 返回 bool，判断存在
- `should_*` — 返回 bool，判断行为建议
- `can_*` — 返回 bool，判断能力

```rust
// ✅ 正确
is_preview_active()
has_bold_span()
should_replace_source()
can_edit()

// ❌ 避免
requires_source_replacement()  // 改用 should_replace_source
is_bold_any()                   // 改用 has_bold_span
```

### Mutate 修改类

- `set_*` — 单字段变更
- `update_*` — 多字段或状态变更
- `apply_*` — 对集合应用操作
- `toggle_*` — 状态翻转

```rust
// ✅ 正确
set_alignment()
update_viewport()
apply_bold_to_all()
toggle_bold()

// ❌ 避免
set_bold_all()      // 对集合应用改用 apply_
toggle_bold_all()   // all 多余
```

### Execute 执行类

- `execute_*` — 执行命令/操作
- `commit_*` — 提交变更
- `dispatch_*` — 分发事件

```rust
// ✅ 正确
execute_save()
commit_edit()
dispatch_event()

// ❌ 避免
save_pdf()      // 改用 execute_save()
rollback_pdf()  // 改用 execute_undo() 或 execute_rollback()
```

### Lifecycle 生命周期类

- `start_*` — 启动流程
- `stop_*` — 停止流程
- `schedule_*` — 安排调度
- `advance_*` — 推进步骤
- `cancel_*` — 取消流程

```rust
// ✅ 正确
start_render_frame()
advance_preview_host()
cancel_progressive()

// ❌ 避免
begin_render_frame()  // 改用 start_
step_preview_host()   // 改用 advance_
```

---

## 5. 模块命名

- 使用 `snake_case`
- 领域名词优于实现标签
- 禁止 `utils`、`helper`、`manager`、`misc`（除明确临时）
- 不重复 crate/plugin 名

```rust
// ✅ 正确
editor_runtime_workflow.rs    // 导出 sync_editor_input
geometry/coordinate_transform.rs
text/glyph_layout.rs

// ❌ 避免
utils/debug.rs               // 改用 diagnostics/text.rs
helper/sanitize.rs           // 改用 geometry/sanitize.rs
pdf_viewer_core_utils.rs     // 重复 crate 名
```

---

## 6. 禁止模式

### 边界词堆叠

```rust
// ❌ 禁止
runtime_workflow_action_host()
sync_active_editor_input_runtime_v3()

// ✅ 正确
sync_editor_input()  // 模块名已承载 runtime_workflow
```

### 历史标签污染

```rust
// ❌ 禁止
open_editor_v19()
commit_sovereign()
migrated_open_editor()

// ✅ 正确
open_editor()
commit_edit()
```

### 描述重构过程

```rust
// ❌ 禁止
migrated_open_editor()
refactored_build_plan()
audit_resolve_layout()

// ✅ 正确
open_editor()
build_plan()
resolve_layout()
```

### 一次性包装区分

```rust
// ❌ 禁止 — 仅以 runtime/workflow/host 区分
open_editor_runtime()
open_editor_workflow()
open_editor_host()

// ✅ 正确 — 合并到单一入口
open_editor()
```

### 测试场景长句

```rust
// ❌ 禁止
source_text_stays_canonical_when_text_plan_has_synthetic_gap_slots()

// ✅ 正确 — 场景细节移到注释或 mod 层级
preserves_canonical_source()  // 注释说明 synthetic gap 场景
```

---

## 7. WASM API 命名

面向 TS 的公开接口使用 camelCase，内部 Rust 保持 snake_case：

```rust
// ✅ 正确
#[wasm_bindgen(js_name = "openDocument")]
pub fn open_document() { ... }

#[wasm_bindgen(js_name = "resolveFrame")]
pub fn resolve_frame() { ... }

// ❌ 避免 — 裸 snake_case 导出
#[wasm_bindgen]
pub fn resolve_frame_plan() { ... }  // TS 端看到 resolve_frame_plan
```

### WASM 命名简化原则

在 `render/free_api.rs` 等模块中，模块上下文已知，无需重复：

| 当前 | 建议 | 理由 |
|------|------|------|
| `resolve_frame_plan` | `resolve_frame` | plan 由返回类型体现 |
| `schedule_render_frame` | `schedule_frame` | render_api 模块已知 |
| `start_progressive_render` | `start_progressive` | 模块已知是 render |

---

## 8. 类型/结构体命名

- PascalCase
- 短语义名
- 避免 `Plan`、`Context`、`State` 等泛词堆砌

```rust
// ✅ 正确
EditContext          // 编辑准备数据
ParagraphEditSession
GlyphPaintPlan

// ❌ 避免
EditorDocumentPlan   // "Plan" 泛词，改用 EditSetup 或 EditContext
EditorSessionContext // Context 重复
```

---

## 9. 例外规则

以下情况保留原命名：

1. **标准 trait 实现** — `Default::default()`、`Clone::clone()`
2. **构造函数** — `Document::new()`、`Page::new()`
3. **已符合规范的函数** — 所有 `resolve_*`、`read_*`、`find_*`
4. **WASM 绑定入口** — 修改需同步前端，保留兼容 alias
5. **测试 helper** — 可接受稍长命名，但应简化

---

## 10. 兼容性规则

- 私有 Rust 测试/helper：可直接重命名
- 公开 Rust/WASM 导出：保留兼容 wrapper 或 `#[deprecated]`
- TS 公开 API：补 JSDoc `@deprecated`
- window 全局变量：保留一个周期的兼容 alias
- 每批重命名后：重新生成 method inventory，运行受影响测试

---

## 11. 常见错误示例

### ❌ 错误：介词后缀

```rust
build_plan_for_target_session()
collect_editor_document_target_plans()
prepare_edit_context_for_target()
```

### ✅ 正确：介词前置或极简

```rust
build_target_plan()           // 或 from_target_id()
collect_target_plans()        // 或 collect_all()
prepare_target_context()      // 或 from_target()
```

### ❌ 错误：动词选择不当

```rust
get_metadata()        // 应改用 read_
get_preview_active()  // 应改用 is_
open_document()       // 应改用 create_ 或 read_
```

### ✅ 正确

```rust
read_metadata()
is_preview_active()
create_document()
```

---

## 12. 冗余禁止规则（审查补充）

### 12.1 模块路径已承载的词不加

函数名不应重复模块路径中已有的词。调用形式 `module::function()` 已经提供了上下文。

```rust
// ❌ 禁止 — 重复模块路径
// 模块: edit_target
edit_target::read_base_paragraph_id_from_target()  // edit/target/from_target 三重冗余
edit_target::build_edit_segment_target_id()         // edit/target 冗余

// ✅ 正确 — 模块路径承载上下文
edit_target::read_base_paragraph_id()
edit_target::build_segment_id()
```

### 12.2 参数名已说明的来源不加 `from_xxx`

当参数类型/名称已隐含来源时，`from_xxx` 是冗余的。

```rust
// ❌ 禁止 — from_xxx 冗余（参数名就是来源）
read_base_paragraph_id_from_target(target: &str)    // target 参数已说明来源
read_replacement_target_from_patch(snapshot: Patch)  // snapshot 参数已说明来源
build_active_editor_target_from_scene(scene: Scene)  // scene 参数已说明来源
has_style_changes_from_paragraph(para: &Paragraph)   // para 参数已说明来源

// ✅ 正确 — 参数名已承载来源信息
read_base_paragraph_id(target: &str)
read_replacement_target(snapshot: Patch)
build_editor_target(scene: Scene)
has_style_changes(para: &Paragraph)
```

**`from_xxx` 仅在以下场景使用：**

1. **Rust 不支持函数重载**，同一模块需要多个同名功能不同参数的入口时：
   ```rust
   // ✅ from_xxx 有区分价值（Rust 无重载）
   build_scene_from_context(ctx: EditContext)     // 场景一
   build_scene_from_paragraph(para: GlyphPaint)   // 场景二
   build_scene_from_target(id: &str)              // 场景三
   ```

2. **同模块有反向转换函数**，必须区分方向时：
   ```rust
   // ✅ from_xxx 有方向区分价值
   read_char_index_from_utf16(utf16_offset: usize) -> usize
   read_utf16_offset_from_char(char_index: usize) -> usize
   ```

### 12.3 废话修饰词不加

`whole`、`full`、`all` 等修饰词在语义上通常是冗余的，除非必须区分数量。

```rust
// ❌ 禁止 — 废话修饰词
build_whole_session_target()    // session 本身是整体概念
build_full_render_plan()        // render_plan 本身就是完整的
collect_all_targets()           // collect 默认就是全部，除非有 collect_partial

// ✅ 正确 — 除非必须区分
build_session_target()          // 与 build_segment_target() 对称
build_render_plan()
collect_targets()               // 如果有对应的 collect_visible_targets() 则 collect_all_targets() 可接受
```

### 12.4 `build_` 后必须跟产物名词

`build_` 表达"构造什么"，裸 `build` 过于泛化。

```rust
// ❌ 禁止 — build_ 缺产物名词
build_from_session()    // 不知道构建什么
build()                 // 完全不表达产出

// ✅ 正确 — build_ + 产出名词
build_context()         // 构建 EditContext
build_scene()           // 构建 ParagraphEditorScene
build_styles()          // 构建样式列表
build_segment_id()      // 构建目标 ID 字符串
```

### 12.5 `set_` vs `apply_` 统一规则

- `set_` — 单字段变更（目标明确）
- `apply_` — 对集合/批量应用操作

```rust
// ❌ 禁止 — 对集合用 set_
set_bold_all()         // 对所有 span 批量设置，应该是 apply_
set_font_size_all()    // 同上

// ✅ 正确
set_alignment()        // 单字段变更
apply_bold_all()       // 对集合批量应用
apply_font_size_all()  // 同上
```

### 12.6 `Kind` vs `Type` 统一规则

统一使用 `Type`，除非是 Rust 社区惯例的 `ErrorKind`。

```rust
// ❌ 避免
EditorGlyphSlotKind    // Kind 冗余且不一致
ListMarkerKind         // 同上

// ✅ 正确
EditorGlyphSlotType    // Type 更自然通用
ListMarkerType         // 同上
DocumentErrorKind      // Rust 惯例，可保留
```

---

## 13. 多入口设计模式

Rust **不支持函数重载**（同名不同参数）。当同一模块需要多种来源创建同类产出时，推荐以下模式：

### 13.1 模式一：只暴露核心方法（推荐）

只保留一个核心入口，创建步骤由调用方组合：

```rust
// 公开核心方法
pub fn build_scene(ctx: EditContext) -> Option<ParagraphEditorScene> { ... }

// 创建方法也在同一模块公开
pub fn from_paragraph(...) -> Option<EditContext> { ... }
pub fn from_target_id(...) -> Option<EditContext> { ... }

// 调用方自己组合（两步法）
let ctx = paragraph_scene::from_paragraph(para, model, point)?;
let scene = paragraph_scene::build_scene(ctx)?;
```

**优点：** 命名简洁，无 `from_xxx` 区分负担，调用方可见完整流程。

### 13.2 模式二：`From` Trait 转换

让产出类型实现 `From` trait：

```rust
impl From<EditContext> for ParagraphEditorScene {
    fn from(ctx: EditContext) -> Self { ... }
}

// 调用
let scene = ParagraphEditorScene::from(ctx);
```

**适用：** 产出类型与输入类型之间有自然转换关系时。

### 13.3 模式三：Enum 参数统一入口

```rust
pub enum SceneSource<'a> {
    Context(EditContext),
    Paragraph { para: &'a GlyphPaintParagraph, point: Option<(f32, f32)> },
    Target { para: &'a GlyphPaintParagraph, id: &'a str },
}

pub fn build_scene(source: SceneSource) -> Option<ParagraphEditorScene> {
    let ctx = match source {
        SceneSource::Context(c) => c,
        SceneSource::Paragraph { .. } => from_paragraph(..),
        SceneSource::Target { .. } => from_target_id(..),
    };
    Some(ParagraphEditorScene::from(ctx))
}
```

**适用：** 来源类型有限且稳定时。

### 13.4 模式四：`from_xxx` 区分（当前项目使用）

```rust
pub fn build_scene_from_context(ctx: EditContext) -> Option<Scene> { ... }
pub fn build_scene_from_paragraph(...) -> Option<Scene> { ... }
pub fn build_scene_from_target(...) -> Option<Scene> { ... }
```

**缺点：** 命名冗余，增加间接层，后两个只是"创建 + 构建"两步拼接。

**仅当**无法用模式一/二/三时才使用此模式。

---

## 14. 快速检查清单

命名前问自己：

1. 动词是否匹配能力类型？（Query→read，Resolve→resolve，Create→build）
2. `build_` 后是否有产物名词？（`build_context` 而非 `build_from_session`）
3. 是否避免了 `get_*` / `process_*` / `handle_*` / `make_*`？
4. 模块路径是否已承载上下文？（去掉冗余词）
5. 参数名是否已说明来源？（不加冗余 `from_xxx`）
6. 是否有废话修饰词？（去掉 `whole` / `full` / `all`，除非必须区分）
7. 对集合操作是否用 `apply_` 而非 `set_`？
8. 类型枚举是否用 `Type` 而非 `Kind`？
9. WASM 导出是否有 camelCase `js_name`？
10. 多入口是否用了最简设计模式？（优先模式一或二）