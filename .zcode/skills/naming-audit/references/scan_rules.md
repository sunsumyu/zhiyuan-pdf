# 扫描规则参考

本文件定义 naming-audit 扫描时所使用的规则体系，包含问题模式识别规则和开源框架命名基准。

## 扫描规则体系

### R1 — 术语漂移检测

**规则**：参数类型名和参数名不一致时触发。

**检测模式**：
```
参数类型：ParagraphEditContext
参数名：session
→ 术语漂移："session" 是通用术语，不能精确表达 "ParagraphEditContext"
```

**严重级别**：🔴 高（术语漂移会产生连锁反应，导致多个函数名跟着错）

**修复策略**：
1. 参数名改为与类型名一致：`ctx: &ParagraphEditContext` 或 `context: &ParagraphEditContext`
2. 如果模块名已提供上下文，可进一步简化为：`context`（因为模块是 `edit_target`，context 足够）

---

### R2 — 模块名重复检测

**规则**：函数名中包含模块名已提供的上下文信息时触发。

**检测模式**：
```
模块名：edit_target.rs
函数名：collect_edit_targets_from_session
→ 重复："edit_target" 在模块名和函数名中都出现了
```

**严重级别**：🟡 中（影响可读性，但不影响功能）

**修复策略**：
1. 去掉模块名部分：`collect_targets_from_session`
2. 如果上下文足够明确，可以进一步缩短：`build_targets`

---

### R3 — 动词前缀误用检测

**规则**：动词前缀与实际语义不匹配时触发。

**检测模式**：

| 前缀 | 正确语义 | 误用示例 |
|------|---------|---------|
| `build_` | 复杂组装 | 用于简单字段访问 |
| `resolve_` | 从多个候选中选择 | 用于直接构建 |
| `collect_` | 遍历集合并收集 | 用于转换/构建 |
| `find_` | 搜索（可能失败） | 用于反序列化提取 |
| `get_` | 简单字段访问 | 用于复杂查询 |
| `execute_` | 执行系统命令/重大操作 | 用于简单注入 |

**严重级别**：🟡 中（动词不精准会导致调用者误解函数行为）

**修复策略**：
1. 根据实际语义选择正确的动词前缀
2. 参考 P2 Prefix 语义精准表

---

### R4 — `get_` 前缀冗余检测

**规则**：无副作用的纯读取器使用 `get_` 前缀时触发。

**检测模式**：
```rust
// ❌ 冗余 get_
pub fn get_base_paragraph_id(target_id: &str) -> &str

// ✅ 去掉 get_
pub fn base_paragraph_id(target_id: &str) -> &str
```

**严重级别**：🟡 中（Rust 社区惯例，不影响功能）

**修复策略**：直接去掉 `get_` 前缀。

**例外**：
- 当需要区分 getter 和 action 时，如 `get_count()` vs `set_count()`
- 当 getter 涉及复杂计算时，如 `get_computed_style()`

---

### R5 — 转换契约检测

**规则**：转换方法没有使用 `as_`/`to_`/`into_`/`from_` 前缀，或使用了错误的前缀时触发。

**检测模式**：
```rust
// ❌ 错误：from_ 用于返回 Vec
pub fn from_context(ctx: &ParagraphEditContext) -> Vec<EditorEditTarget>

// ✅ 正确：build_ 用于返回 Vec
pub fn build_targets(base_id: &str, ctx: &ParagraphEditContext) -> Vec<EditorEditTarget>
```

**严重级别**：🟡 中（影响 API 使用者的直觉判断）

**修复策略**：
1. `from_` 只能用于返回 `Self` 类型（构造函数）
2. 返回 Vec 或其他类型时，应使用 `build_` 或其他合适的前缀

---

### R6 — 名词堆叠检测

**规则**：函数名中包含 3 个以上的名词堆叠时触发。

**检测模式**：
```
collect_target_source_object_ids
       ^     ^      ^       ^
     target source object   ids
→ 4 层名词堆叠，语法关系模糊
```

**严重级别**：🟡 中（影响可读性）

**修复策略**：
1. 利用模块上下文缩短：`object_ids`（模块 `source_identity` 已提供上下文）
2. 如果不够明确，加限定词：`source_object_ids`

---

### R7 — 变量遮蔽检测

**规则**：重命名函数后，局部变量名与新函数名冲突时触发。

**检测模式**：
```rust
// ❌ 局部变量遮蔽函数名
let base_paragraph_id = base_paragraph_id(&patch.region_id).to_string();

// ✅ 变量名与函数名区分
let base_id = base_paragraph_id(&patch.region_id).to_string();
```

**严重级别**：🔴 高（编译错误）

**修复策略**：
1. 重命名函数后，全局搜索旧函数名
2. 搜索新函数名，检查是否有局部变量与之冲突

---

## 开源框架命名基准

### serde — 前缀一致性基准

**核心原则**：同一类操作使用统一前缀。

```rust
// ✅ 统一前缀体系
serialize_u64, serialize_str, serialize_bytes
serialize_struct, serialize_map

// ❌ 混用前缀（应避免）
to_bytes, as_string, convert_u64
```

**扫描检查点**：
- [ ] 同一模块中做"序列化"的函数是否都用了 `serialize_`？
- [ ] 同一模块中做"构建"的函数是否都用了 `build_`？
- [ ] 是否存在同一类操作但前缀不一致的情况？

---

### tokio — 极简命名基准

**核心原则**：模块路径提供上下文，函数名尽量短。

```rust
// tokio::sync::mpsc::Sender
pub async fn send(&self, value: T) -> Result<(), SendError<T>>

// tokio::time::sleep
pub async fn sleep(duration: Duration) -> Sleep
```

**扫描检查点**：
- [ ] 函数名是否重复了模块名已提供的上下文？
- [ ] 是否可以去掉冗余限定词？
- [ ] 参数名是否已提供足够的消歧义信息？

---

### clap — `get_` 使用基准

**核心原则**：`get_` 只在需要区分 getter 和 action 时使用。

```rust
// clap::ArgMatches — 查询操作
pub fn get_one<T>(&self, name: &str) -> Option<&T>
pub fn get_count(&self, name: &str) -> u8
```

**扫描检查点**：
- [ ] 纯字段访问是否使用了 `get_`？（应去掉）
- [ ] 复杂查询是否缺少 `get_`？（需要时加上）

---

### rustc — 谓词命名基准

**核心原则**：返回 `bool` 的函数必须使用 `is_`/`has_` 前缀。

```rust
// rustc internals
fn is_copy(&self) -> bool
fn has_trait(&self, trait: &str) -> bool
fn can_coerce(&self) -> bool
```

**扫描检查点**：
- [ ] 返回 `bool` 的函数是否缺少 `is_`/`has_`/`can_`？
- [ ] 返回 `bool` 的函数是否错误地使用了 `get_`？

---

### std — 转换契约基准

**核心原则**：`as_`/`to_`/`into_`/`from_` 的严格区分。

| Prefix | 成本 | 所有权语义 | 示例 |
|--------|------|-----------|------|
| `as_` | O(1), 免费 | 借入 → 借出 | `str::as_bytes` |
| `to_` | 有开销 | 借入 → 拥有/借出 | `str::to_lowercase` |
| `into_` | O(1) 或重 | 拥有 → 拥有 | `String::into_bytes` |
| `from_` | 有开销 | 参数 → Self | `String::from_utf8` |

**扫描检查点**：
- [ ] 转换方法是否使用了正确的前缀？
- [ ] `from_` 是否只用于返回 `Self` 类型？
- [ ] 返回 `Vec<T>` 的函数是否错误地使用了 `from_`？

---

### R8 — 语义清晰度检查（核心词义污染检测）

**规则**：函数名中的**每个核心词**必须在项目中只有一个精确含义。如果一个词在不同位置指代不同概念，或词义过于抽象模糊，触发此规则。

**严重级别**：🔴 高（语义污染比长度问题更严重，因为它导致调用者根本看不懂函数在做什么）

**检测模式**：

**模式 8a — 一词多义污染**
```
项目中的 "plan" 同时指代：
- EditorDocumentPlan → 编辑配置/上下文
- EditorDraftRenderPlan → 折行+光标计算结果
- CaretPlan → 光标位置列表

→ 语义污染：同一个词 "plan" 承载了 3 个完全不同的概念
```

**模式 8b — 抽象词无具体含义**
```
// ❌ "plan" 太抽象——不知道是什么 plan
pub fn build_caret_plan(...) -> EditorDraftRenderPlan

// 调用者看到 build_caret_plan() 会困惑：
// - 是构建一个"计划/方案"？
// - 还是构建一个"数据结构"？
// - 还是执行一次"计算"？
```

**模式 8c — 类型名与函数名语义不对齐**
```
// ❌ 函数名说 "CaretPlan"，类型名说 "EditorDraftRenderPlan"
// 调用者不知道返回值到底是 "caret 位置" 还是 "完整渲染计划"
pub fn build_caret_plan(...) -> EditorDraftRenderPlan
```

**修复策略**：
1. **语义对齐**：函数名中的核心词必须与返回类型名对齐
2. **消除多义**：如果一个词在项目中有多个含义，统一为一种，其他改为更精确的词
3. **用具体词替代抽象词**：
   - ❌ `plan`（太抽象） → ✅ `context`（配置）/ `layout`（布局结果）/ `positions`（位置列表）
   - ❌ `data`（万能词） → ✅ `runs`（runs 数据）/ `segments`（段数据）
   - ❌ `info`（无含义） → ✅ `summary`（摘要）/ `metadata`（元数据）

---

### R9 — 输入类型命名反模式检测

**规则**：函数名用输入类型（`from_layout`、`from_runs`）作为区分，而非行为动词，触发此规则。优秀开源框架用**行为动词**区分不同路径。

**严重级别**：🔴 高（输入类型命名掩盖了行为差异，导致调用者无法理解函数做什么）

**检测模式**：

**模式 9a — 输入类型命名（from_layout / from_runs）**
```
// ❌ 用输入类型命名，掩盖了行为差异
pub fn build_caret_plan_from_layout(layout: &[LayoutRun]) -> CaretPlan
// 实际做的是：几何计算 → 坐标定位 → 折行判断

pub fn build_caret_plan_from_runs(runs: &[GlyphPaintRun]) -> CaretPlan
// 实际做的是：文本索引映射 → 字符位置查找

→ 问题：调用者无法从函数名得知"做什么"，只能知道"从什么来"
```

**模式 9b — `from_` 前缀滥用**
```
// ❌ from_ 暗示"转换"，但函数实际在做复杂构建
pub fn build_caret_plan_from_layout(layout: &[LayoutRun], measure: &F) -> CaretPlan

// 内部涉及：
// 1. 几何计算
// 2. 折行算法
// 3. 外部 measure 闭包调用
// 这是"构建"不是"转换"
```

**修复策略**：

1. **用行为动词 + 具体返回值区分，而非输入类型**
   - ❌ `build_caret_plan_from_layout` → ✅ `compute_caret_positions`
   - ❌ `build_caret_plan_from_runs` → ✅ `resolve_caret_indices`

2. **输入类型放到参数列表中，名字里不要重复**
   - ✅ 参数类型 `&[LayoutRun]` 已经说明"从 layout 来"
   - ✅ 参数类型 `&[GlyphPaintRun]` 已经说明"从 runs 来"

3. **`from_` 前缀的合理使用场景**
   - ✅ 同类编码的不同变体：`String::from_utf8` / `String::from_utf16`
   - ❌ 不同领域行为的合并：`build_plan_from_layout`

---

### R10 — 类型命名反模式检测

**规则**：struct / enum / type alias 名称存在语义不清、一词多义、抽象词或动词堆砌时触发。**函数名不应为了对齐有问题的类型名而跟着错**——类型名和函数名应该一起重构。

**严重级别**：🔴 高（类型名是项目的"公共语言"，比函数名影响范围更大。坏类型名会污染所有使用它的函数名和参数名）

**检测模式**：

**模式 10a — 抽象词污染（plan / data / info / manager）**
```rust
// ❌ "plan" 是抽象词——项目中同时指"配置"、"计算结果"、"位置列表"
pub struct EditorDraftRenderPlan { ... }

// ❌ 函数名为了对齐类型名而被迫保留 plan
pub fn compute_draft_render_plan(...) -> EditorDraftRenderPlan

// ✅ 类型名和函数名一起重构
pub struct DraftLayout { ... }
pub fn compute_draft_layout(...) -> DraftLayout
```

**模式 10b — 一词多义（同一类型名在不同模块指代不同概念）**
```rust
// module_a.rs
pub struct Plan { ... }  // 指"编辑配置"

// module_b.rs
pub struct Plan { ... }  // 指"渲染结果"

→ 严重语义污染：调用者看到 Plan 不知道指什么
```

**模式 10c — 动词堆砌（类型名不应该包含动词）**
```rust
// ❌ 类型名包含动词
pub struct BuildParagraphResult { ... }
pub struct ComputeLayoutData { ... }

// ✅ 类型名用纯名词
pub struct ParagraphLayout { ... }
pub struct DraftLayout { ... }
```

**模式 10d — 过度描述（类型名比内容更复杂）**
```rust
// ❌ 类型名太长，实际内容简单
pub struct EditorDocumentParagraphEditorMarker {}

// ✅ 简洁、精确
pub struct ParagraphMarker {}
```

**修复策略**：

1. **类型名和函数名一起重构**：不要为了对齐有问题的类型名而保留抽象词
   - ❌ `pub fn compute_draft_render_plan(...) -> EditorDraftRenderPlan`
   - ✅ `pub fn compute_draft_layout(...) -> DraftLayout`

2. **用具体名词替代抽象词**：
   - ❌ `plan` → ✅ `context`（配置）/ `layout`（排版结果）/ `positions`（位置列表）
   - ❌ `data` → ✅ `runs` / `segments` / `patches`
   - ❌ `info` → ✅ `summary` / `metadata` / `details`
   - ❌ `manager` → ✅ `coordinator` / `registry` / `pool`

3. **类型名不应该包含动词**：
   - ❌ `BuildParagraphResult` → ✅ `ParagraphLayout`
   - ❌ `ComputeLayoutData` → ✅ `DraftLayout`

4. **类型名不应该重复模块上下文**：
   - ❌ `EditorDocumentParagraphEditorMarker` → ✅ `ParagraphMarker`

---

## 扫描优先级

扫描时按以下优先级排序问题（高优先级先处理）：

1. **🔴 编译错误**：变量遮蔽函数名（R7）
2. **🔴 语义污染**：一词多义、核心词无具体含义（R8）
3. **🔴 类型命名反模式**：类型名包含抽象词/动词/一词多义，函数名被迫跟随（R10）
4. **🔴 输入类型命名**：用 `from_layout`/`from_runs` 替代行为动词（R9）
5. **🔴 术语漂移**：参数名与类型名不一致（R1）
6. **🟡 动词误用**：前缀与实际语义不匹配（R3）
7. **🟡 转换契约错误**：`as_`/`to_`/`into_`/`from_` 使用不当（R5）
8. **🟡 名词堆叠**：3 层以上名词（R6）
9. **🟡 模块重复**：函数名重复模块上下文（R2）
10. **🟡 `get_` 冗余**：不必要的 `get_` 前缀（R4）

## 扫描输出格式

### 单函数报告

```
🔴 {函数名} ({文件}:{行})
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
签名：{签名}
问题：{问题列表}
建议：{新名称}
理由：{CoT 推理链}
```

### 批量汇总报告

```markdown
## naming-audit 扫描报告

**扫描范围**：{范围}
**扫描文件**：{n} 个
**扫描函数**：{n} 个
**发现问题**：{n} 个

### 按严重程度统计

| 严重程度 | 数量 |
|---------|------|
| 🔴 高 | {n} |
| 🟡 中 | {n} |

### 按问题类型统计

| 问题类型 | 数量 | 典型示例 |
|---------|------|---------|
| 术语漂移 | {n} | ... |
| 模块重复 | {n} | ... |
| 动词误用 | {n} | ... |
| ... | ... | ... |

### 按开源框架对比统计

| 对比框架 | 不符合数量 | 典型问题 |
|---------|-----------|---------|
| serde | {n} | ... |
| tokio | {n} | ... |
| ... | ... | ... |
```
