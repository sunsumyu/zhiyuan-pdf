# 命名原则参考

本文件定义 fn-analyzer 命名一致性验证所使用的六条原则体系。

## P1 — 动词单层

**规则**：函数名中动词只能有一层，不能叠床架屋。

- ✅ `build_context()` — 单层动词 build
- ❌ `build_editor_document_plan()` — 三层动词叠词
- ❌ `create_new_build()` — create + new + build
- ✅ `resolve_target()` — 单层 resolve
- ❌ `resolve_and_build_target()` — 两层

**原理**：A Philosophy of Software Design 提出"深层模块"概念——接口简洁（少量方法、短名称），功能强大。长名称意味着在暴露实现细节而非抽象意图。

## P2 — Prefix 语义精准

**规则**：动词前缀必须唯一对应一种语义，不能混用。

| Prefix | 唯一语义 | 示例 |
|--------|---------|------|
| `new` / `from_` | 构造函数（轻量/转换） | `from_paragraph`, `from_target_id` |
| `to_` / `into_` | 类型转换（有开销/所有权转移） | `String::into_bytes` |
| `as_` | 零开销借用转换 | `as_bytes`, `as_str` |
| `is_` / `has_` | 布尔谓词 | `is_empty`, `has_marker` |
| `find_` / `resolve_` | 查找（可能失败） | `resolve_click_caret` |
| `compute_` | 纯计算（无副作用） | `compute_bbox_from_runs` |
| `build_` | 对象构建（复杂组装） | `EditContext::build` |
| `apply_` | 生效变更 | `apply_persisted_override` |
| `validate_` | 校验（返回 Result） | `validate_body_text` |
| `normalize_` | 规范化/标准化 | `normalize_pdf_font_identity` |
| `collect_` | 集合操作 | `collect_all` |
| `detect_` | 启发式探测 | `detect_graphic_markers` |
| `insert_` | 插入 | `insert_text` |
| `delete_` | 删除 | `delete_backward` |

**原理**：Rust API Guidelines C-GETTER（避免 `get_` 前缀）和 C-CONV（`as_` 免费、`to_` 有开销、`into_` 拥有权转移）。

## P3 — 消除模块重复

**规则**：模块名已提供的上下文，函数名中不要重复出现。

- ✅ `edit_target.rs` → `collect_targets(session)` （模块提供了 `edit_target` 上下文）
- ❌ `edit_target.rs` → `collect_edit_targets_from_session` （模块 + 函数名都出现 `edit_target`）
- ✅ `document_plan.rs` → `from_paragraph()` （模块提供了 `document_plan` 上下文）
- ❌ `document_plan.rs` → `build_editor_document_plan()` （模块 + 函数名重复）

**例外**：当函数需要和同模块的其他函数区分时，可以加限定词：
- `from_paragraph` vs `from_target_id` — 区分的限定词允许多次出现

**原理**：Clippy lint `module_name_repetitions`（默认 warn 级别）。短名字信任上下文——模块路径已提供消歧义。

## P4 — 动词 + 宾语固定语序

**规则**：函数名语序固定为 `动词 + 宾语`，所有状语（参数）放到参数列表。

- ✅ `build_segment_id` — 动词 build + 宾语 segment_id
- ✅ `insert_text(state, text)` — 动词 insert + 宾语 text
- ❌ `build_editor_document_plan_for_target` — "for_target" 是状语/参数，不是函数名的一部分
- ❌ `build_editor_document_plan_from_session` — "from_session" 是参数

**原理**：方法签名的参数名已经是描述"状语"的地方，函数名不需要重复参数信息。Clean Code "Use one word per concept"。

## P5 — 转换方法契约

**规则**：类型转换方法必须使用 `as_` / `to_` / `into_` / `from_` 体系，且遵守 Rust 社区约定的成本语义。

| Prefix | 成本 | 所有权语义 | 示例 |
|--------|------|-----------|------|
| `as_` | O(1), 免费 | 借入 → 借出 | `str::as_bytes` |
| `to_` | 有开销 | 借入 → 拥有/借出 | `str::to_lowercase` |
| `into_` | O(1) 或重 | 拥有 → 拥有 | `String::into_bytes` |
| `from_` | 有开销 | 参数 → Self | `String::from_utf8` |

- ❌ `session_source_text(session)` — 应改为 `EditContext::session_source_text(&self)` 或 free fn `source_text(session)`
- ✅ `bbox_from_runs(runs)` — 符合 `from_` 模式，表明有开销的构建
- ❌ `chars_to_text(chars)` — 应直接用 `String::from_iter(chars)` 或 `chars.into_iter().collect()`

**原理**：Rust API Guidelines C-CONV。

## P6 — 术语一致性（Ubiquitous Language）

**规则**：同一个概念只能有一个词，一个词只能有一个概念。

**检查方法**：
1. 函数参数名是否和类型名一致？如果参数类型是 `ParagraphEditContext` 但参数名叫 `session`，这是术语不一致。
2. 同一模块内是否用不同词指代同一个概念？
3. 同一个词是否在不同位置指代不同概念？

**常见不一致模式**：
- 类型是 `FooContext`，变量/参数叫 `session` ← 不一致
- 类型是 `EditorDocumentPlan`，改名为 `EditContext` 后变量名还叫 `plan` ← 不一致
- 同一模块内 `EditContext` 既叫 `context` 又叫 `plan` 又叫 `document_plan` ← 不一致

**原理**：Eric Evans DDD 的 Ubiquitous Language。语言必须精确、一致、无歧义。软件不允许歧义。

## P7 — 语义清晰度（核心词义不污染）

**规则**：函数名中的每个核心词在项目中必须只有一个精确含义。如果一个词在不同位置指代不同概念，或词义过于抽象模糊，必须重命名。

**检查方法**：
1. 函数名中的每个词是否都有具体、精确的含义？
2. 同一个词在项目其他地方是否指代不同的概念？
3. 是否有抽象万能词（plan / data / info / config / manager）？

**常见语义污染模式**：
- `plan` → 同时指"配置"、"计算结果"、"位置列表" ← ❌ 语义污染
- `data` → 同时指"runs 数据"、"patch 数据"、"任何数据" ← ❌ 万能词
- `info` → 没有具体含义，不如 summary / metadata / details ← ❌ 无意义
- `manager` → 没有具体含义，不如 coordinator / registry / pool ← ❌ 无意义

**抽象词替代方案**：

| 抽象词 | 具体替代 | 场景 |
|--------|---------|------|
| `plan` | `context`（配置）| 编辑配置 |
| `plan` | `layout`（布局）| 几何计算结果 |
| `plan` | `positions`（位置）| 光标位置列表 |
| `data` | `runs` | glyph runs 数据 |
| `data` | `segments` | 文本段数据 |
| `data` | `patches` | patch 数据 |
| `info` | `summary` | 摘要信息 |
| `info` | `metadata` | 元数据 |
| `info` | `details` | 详细信息 |
| `manager` | `coordinator` | 协调器 |
| `manager` | `registry` | 注册表 |
| `manager` | `pool` | 池 |

**原理**：Clean Code "一个概念一个词"（One Word per Concept）。抽象词是命名的坟墓——它们掩盖了真实意图。

## P8 — 类型命名反模式（Type Naming Anti-pattern）

**规则**：struct / enum / type alias 名称存在语义不清、一词多义、抽象词或动词堆砌时触发。**函数名不应为了对齐有问题的类型名而跟着错**——类型名和函数名应该一起重构。

**检查方法**：
1. 类型名是否包含抽象词（`plan`/`data`/`info`/`manager`）？
2. 类型名是否在项目不同模块中指代不同概念？
3. 类型名是否包含动词（`Build...`/`Compute...`/`Resolve...`）？
4. 函数名是否为了对齐有问题的类型名而被迫使用抽象词？

**常见类型命名反模式**：
- ❌ `EditorDraftRenderPlan` → `plan` 抽象，同时指"配置"、"计算结果"、"位置列表"
- ❌ `BuildParagraphResult` → 类型名包含动词
- ❌ `ComputeLayoutData` → 类型名包含动词 + `data` 万能词
- ❌ `EditorDocumentParagraphEditorMarker` → 过度描述，重复上下文

**函数名不应为了对齐类型名而跟着错**：
```rust
// ❌ 错误：为了对齐有问题的类型名而保留抽象词
pub fn compute_draft_render_plan(...) -> EditorDraftRenderPlan

// ✅ 正确：类型名和函数名一起重构
pub fn compute_draft_layout(...) -> DraftLayout
```

**抽象词替代方案**（同 P7）：
| 抽象词 | 具体替代 | 场景 |
|--------|---------|------|
| `plan` | `context`（配置）| 编辑配置 |
| `plan` | `layout`（布局）| 几何计算结果 |
| `plan` | `positions`（位置）| 光标位置列表 |
| `data` | `runs` | glyph runs 数据 |
| `data` | `segments` | 文本段数据 |
| `info` | `summary` | 摘要信息 |
| `manager` | `coordinator` | 协调器 |

---

# 优秀开源框架命名模式对比（Cross-Reference）

本章节提供知名 Rust 开源项目的命名习惯作为基准，用于在命名审查时对比参考。

## 1. serde — 前缀一致性

**模式**：`serialize_*` / `deserialize_*` 前缀约定 — 同一类操作使用一致的前缀，而不是每个函数独立取名。

```rust
// ✅ 一致的前缀体系
serialize_u64, serialize_str, serialize_bytes
serialize_struct, serialize_map

// ❌ 如果混用则不一致
// to_bytes, as_string, convert_u64  ← 这样不好
```

**启示**：项目中如果有一组函数做同一类操作，应该用统一前缀，而非各取各的名字。

## 2. tokio — 极简动词

**模式**：函数名尽量短，参数名和类型签名提供上下文。

```rust
// tokio::sync::mpsc::Sender
pub async fn send(&self, value: T) -> Result<(), SendError<T>>

// tokio::time::sleep
pub async fn sleep(duration: Duration) -> Sleep

// tokio::fs::read
pub async fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>>
```

**启示**：模块路径已经提供了上下文，函数名不需要重复。`sync::mpsc::Sender::send` 比 `sync::mpsc::Sender::send_message_to_channel` 好得多。

## 3. clap — `get_` 前缀的合理使用

**模式**：`get_` 前缀表示"查询"，但只在真正需要区分 getter 和 action 方法时使用。

```rust
// clap::ArgMatches
pub fn get_one<T>(&self, name: &str) -> Option<&T>
pub fn get_many<T>(&self, name: &str) -> Option<Values<T>>
pub fn get_count(&self, name: &str) -> u8

// 注意：这些函数返回的是查询结果，不是简单的字段访问
```

**启示**：`get_` 不是绝对禁用——当需要区分"获取"和"构建"时使用。但绝大多数情况应直接去掉 `get_`。

## 4. rustc — 谓词命名

**模式**：布尔返回函数必须使用 `is_` / `has_` 前缀。

```rust
// rustc internals
fn is_copy(&self) -> bool
fn has_trait(&self, trait: &str) -> bool
fn is_sized(&self) -> bool
fn can_coerce(&self) -> bool
```

**启示**：返回 `bool` 的函数不要用 `get_`（如 ❌ `get_enabled()`），要用 `is_enabled()`、`has_marker()` 等谓词形式。

## 5. anyhow — 极简主义

**模式**：库本身名字就很短，API 也追求极简。

```rust
// anyhow
pub fn anyhow<T>(msg: T) -> anyhow::Error
pub fn ensure<T>(cond: bool, msg: T)
```

**启示**：当功能简单且上下文明确时，函数名可以极端短。`anyhow()` 只有一个词，但在 `use anyhow::anyhow` 的上下文中足够清晰。

## 6. std — 转换契约

**模式**：`as_` / `to_` / `into_` 的严格区分是 Rust 命名最核心的约定。

```rust
// as_ → 免费转换，返回引用
"hello".as_bytes()     // &[u8]
path.as_os_str()       // &OsStr

// to_ → 有开销转换，返回拥有值
"hello".to_uppercase() // String（分配）
str.to_string()        // String（分配）

// into_ → 所有权转移，O(1) 或重
string.into_bytes()    // Vec<u8>（所有权转移）
vec.into_boxed_slice() // Box<[T]>（所有权转移）
```

**启示**：如果你的函数做"转换"，必须选择正确的 `as_`/`to_`/`into_`/`from_` 前缀，不能随意命名。

---

# 重构经验总结（来自实战）

## 常见"坏名字"模式及修复策略

| 坏名字模式 | 示例 | 问题 | 修复 |
|-----------|------|------|------|
| **术语漂移** | `session: ParagraphEditContext` | "session" 指代不明 | 参数名改为 `context` |
| **类型重命名残留** | `plan: EditContext`（原叫 EditorDocumentPlan） | 类型改了但变量名没改 | 统一改为 `context` |
| **模块名重复** | `build_paragraph_editor_scene`（在 paragraph_scene.rs 中） | 模块名已提供上下文 | 改为 `build_scene` |
| **名词堆叠** | `collect_target_source_object_ids` | 4 层名词 | 改为 `object_ids` |
| **get_ 冗余** | `get_base_paragraph_id` | 纯读取器不需要 get_ | 改为 `base_paragraph_id` |
| **resolve_ 滥用** | `resolve_paragraph_shell_bbox` | 直接构建而非"解决" | 改为 `paragraph_shell_bbox` |
| **execute_ 过重** | `execute_marker_injection` | 听起来像系统命令 | 改为 `inject_marker` |
| **find_ 不准确** | `find_target` | 实际是反序列化而非搜索 | 改为 `extract_target` |
| **collect_ 多余** | `collect_paragraph_interaction_targets` | 不是集合操作 | 改为 `interaction_targets` |
| **参数名暴露** | `build_editor_document_plan_from_session` | "from_session" 是参数 | 改为 `build_context(ctx)` |
| **输入类型命名** | `build_caret_plan_from_layout` | 用输入类型而非行为动词 | 改为 `compute_caret_positions(layout)` |

## 重构 checklist

在重命名前执行以下检查：

1. **术语一致性检查**：参数类型名和参数名是否一致？有无术语漂移？
2. **模块重复检查**：函数名是否重复了模块名已提供的上下文？
3. **动词精准性检查**：动词前缀是否唯一匹配函数的实际语义？
4. **转换契约检查**：如果是转换方法，是否使用了正确的 `as_`/`to_`/`into_`/`from_`？
5. **输入类型命名检查**：是否用 `from_layout`/`from_runs` 替代了行为动词？输入类型已放在参数列表中，名字里不应重复。
6. **开源对比检查**：对照 serde/tokio/clap/rustc/anyhow/std 的命名习惯，是否有更好的选择？
7. **变量遮蔽检查**：重命名后函数名是否可能被局部变量遮蔽？（如 `base_paragraph_id` 函数 vs 局部变量）
8. **调用点影响评估**：有多少调用点需要更新？是否值得重命名？
9. **对称性检查**：相关的函数（如 `build_target_at_point` / `build_render_target`）是否命名对称？

## 实战教训

### 教训 1：变量名遮蔽函数名

```rust
// ❌ 错误：局部变量遮蔽函数名
let base_paragraph_id = base_paragraph_id(&patch.region_id).to_string();

// ✅ 正确：变量名与函数名区分
let base_id = base_paragraph_id(&patch.region_id).to_string();
```

**教训**：重命名函数后，检查所有文件中的局部变量名，确保不与新函数名冲突。

### 教训 2：术语漂移的连锁反应

`ParagraphEditContext` 被参数名叫做 `session`，导致 6 个函数名中出现 `session`：
- `session_source_text(session)`
- `collect_edit_targets_from_session(session)`
- `resolve_edit_target_from_session(session)`

**教训**：术语漂移会产生连锁反应——一个参数名的错误会导致一系列函数名都跟着错。修复时应从源头（参数名）开始。

### 教训 3：对称性命名的重要性

```rust
// ❌ 不对称，看不出区别
pub fn build_editor_target(..., click_x: f32, click_y: f32)      // 有 click
pub fn build_paragraph_render_target(...)                         // 无 click

// ✅ 对称，一看就懂
pub fn build_target_at_point(..., x: f32, y: f32)                 // 有 click
pub fn build_render_target(...)                                   // 无 click
```

**教训**：相关函数应该用对称的命名，让"区别"一目了然。

### 教训 4：`from_` 不适合返回 Vec

```rust
// ❌ from_context 暗示构造单个值，但实际返回 Vec
pub fn from_context(base_id: &str, ctx: &ParagraphEditContext) -> Vec<EditorEditTarget>

// ✅ build_targets 准确表达"从上下文构建目标列表"
pub fn build_targets(base_id: &str, ctx: &ParagraphEditContext) -> Vec<EditorEditTarget>
```

**教训**：`from_` 前缀暗示构造一个 Self 类型的值。如果返回 Vec 或其他类型，应该用 `build_` 或其他合适的前缀。

### 教训 5：注释中的函数名也要更新

```rust
// ❌ 注释引用旧函数名
// session_source_text 注入合成空格 → "智能合约: Anchor Framework, ..."

// ✅ 注释引用新函数名
// source_text 注入合成空格 → "智能合约: Anchor Framework, ..."
```

**教训**：grep 搜索调用点时，不要忘记搜索注释和 doc comments 中的函数名引用。
