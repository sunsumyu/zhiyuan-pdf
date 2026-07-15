---
name: fn-analyzer
description: |
  分析 Rust 函数的功能并验证命名一致性。
  当用户提到"分析函数"、"函数功能"、"方法功能"、"命名审查"、"方法命名"、"函数审查"、"命名规范"、"函数名是否准确"或类似关键词时触发。
  对指定目录或文件中的函数逐个分析：读取完整源码、总结功能、评估命名是否精准匹配功能。
  输出结构化摘要 + 命名一致性评分。这是一个只读的审查工具，不修改任何文件。
---

# fn-analyzer

分析 Rust 函数功能并验证命名的 skill。调用本 skill 前请先读 `references/naming_principles.md` 了解命名原则体系。

## 工作流

### 1. 确认分析范围

先问用户想分析哪个目录/文件的函数。如果用户已指定，直接进入分析。

可选范围示例：
- 单个文件：`crates/pdf-viewer-core/src/edit/document_plan.rs`
- 模块目录：`crates/pdf-viewer-core/src/edit/`
- 当前项目所有 pub fn：项目根路径
- 指定函数列表：逐个分析用户列出的函数

### 2. 获取函数列表

使用 Bash 运行以下命令获取指定范围内的所有 pub fn：

```bash
cd <project-root>
grep -rn "^pub fn\|^pub async fn\|^pub(crate) fn\|^pub(super) fn" <target-path> --include="*.rs"
```

如果用户要求分析所有函数（含私有 fn），改为：
```bash
grep -rn "^pub fn\|^pub async fn\|^fn " <target-path> --include="*.rs" | grep -v "#\[" | grep -v "cfg(test)"
```

### 3. 逐函数分析

对每个函数执行以下步骤：

#### 3.1 读取源码

用 Read 读取函数所在的文件，定位到函数定义处（包含完整的 doc comments 和签名）。

#### 3.2 功能摘要

提取以下信息：
- **函数签名**：完整签名（含泛型、where 约束）
- **参数列表**：每个参数的类型和语义角色
- **返回值**：返回类型和语义
- **核心逻辑摘要**：函数内部做了什么（用一两句话概括）
- **副作用**：是否有 IO、全局状态修改、日志等

#### 3.3 命名一致性验证

对照 `references/naming_principles.md` 中的六条原则，逐条检查：

| 原则 | 检查点 | 分数 |
|------|--------|------|
| P1 动词单层 | 名称是否只有一层动词？有无叠词（build_create, get_fetch）？ | /5 |
| P2 Prefix 准确 | 动词前缀是否匹配实际语义（build vs from, resolve vs compute）？ | /5 |
| P3 模块不重复 | 函数名是否重复了模块名已提供的上下文？ | /5 |
| P4 动词+宾语 | 语序是否是"动词+宾语"？有无中间状语句式？ | /5 |
| P5 转换契约 | 如果是转换方法，as_/to_/into_ 是否符合成本约定？ | /5 |
| P6 术语一致性 | 参数名中的术语与类型名是否一致？有无概念别名？ | /5 |

**总分：30 分**
- 🟢 **优秀 (28-30)**：命名精准，无歧义
- 🟡 **良好 (22-27)**：基本准确，有小瑕疵
- 🟠 **可改进 (15-21)**：存在明显问题
- 🔴 **需重命名 (<15)**：命名与功能严重不匹配

### 4. 输出格式

对每个函数输出如下结构化摘要：

```
────────────────────────────────────────────────────────────────
函数：build_editor_document_plan_from_session
文件：crates/pdf-viewer-core/src/edit/document_plan.rs:381
────────────────────────────────────────────────────────────────

【签名】
pub fn build_editor_document_plan_from_session(session: &ParagraphEditContext) -> EditContext

【功能摘要】
从 ParagraphEditContext 构建一个基本的 EditContext，执行三个步骤：
1. 调用 build_editor_session_text_plan(session) 重建语义文本
2. 调用 build_body_line_plans(session, &text_plan) 按行拆分
3. 调用 select_draft_template_run(session, &lines) 选取样式模板
最终组装为一个 EditContext。不处理 marker 拆分或 graphic marker 检测。

【命名评估】
| 原则 | 分数 | 说明 |
|------|------|------|
| P1 动词单层 | 2/5 | 三层叠词：build + editor_document + plan |
| P2 Prefix 准确 | 3/5 | build 是构建，准确；但 plan 在返回 EditContext 时产生歧义 |
| P3 模块不重复 | 1/5 | 模块是 document_plan，函数名又出现 editor_document_plan |
| P4 动词+宾语 | 2/5 | 宾语过长，中间有多个修饰语 |
| P5 转换契约 | 3/5 | 不涉及转换方法 |
| P6 术语一致性 | 2/5 | session 与类型 ParagraphEditContext 不一致 |

【总分】13/30 🔴 需重命名

【建议】
改为关联函数 EditContext::build(ctx)，其中 ctx: &ParagraphEditContext

【调用影响】
- 仅 3 个测试调用，无生产代码调用
```

### 5. 批量输出

分析完一个文件/目录的所有函数后，输出汇总表格：

```
## 汇总：{文件名/模块名}

| 函数名 | 当前长度 | 总分 | 评级 | 主要问题 |
|--------|---------|------|------|---------|
| build_editor_document_plan_from_session | 38 | 13/30 | 🔴 | 叠词、模块重复、术语不一致 |
| from_paragraph | 13 | 26/30 | 🟡 | 模块限定不够明确 |
| ... | ... | ... | ... | ... |
```

## 注意事项

- **不要修改任何文件**。本 skill 是只读分析工具，发现问题记录在报告中即可。
- 分析前务必读取 `references/naming_principles.md`，确保评分标准一致。
- 引用代码使用精确路径 + 行号格式（如 `crates/pdf-viewer-core/src/edit/document_plan.rs:381`）。
- 功能摘要部分只描述函数实际做了什么，不要根据命名推测功能。
- **命名一致性比对必须严格**：函数名中的每个词都要和实际逻辑对应，不能有任何"大致对得上"的情况。
- 如果函数有 `#[deprecated]` 标注，在摘要中注明。
- 最终输出直接以 Markdown 消息发送给用户，不写入文件。
