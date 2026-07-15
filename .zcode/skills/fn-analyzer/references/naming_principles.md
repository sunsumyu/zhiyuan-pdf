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
