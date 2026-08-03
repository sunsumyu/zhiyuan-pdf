# Issue Tracker 配置

> 由 setup-matt-pocock-skills 于 2026-08-04 生成 · 供 to-spec / to-tickets / wayfinder / triage 技能读取

## A. Tracker 选择

**Local markdown**（`.scratch/<feature-slug>/`）

选择理由：项目无 .github/、无 CI、无 gh CLI；零外部依赖，与 GitHub 仓库（sunsumyu/zhiyuan-pdf）不冲突；文档即代码，随仓库走。

## 约定（Local markdown 形态）

| 条目 | 位置 | 说明 |
|---|---|---|
| spec / PRD | `.scratch/<feature-slug>/spec.md` | 综合对话与代码库后合成 |
| tickets | `.scratch/<feature-slug>/issues/<NN>-<slug>.md` | 从 `01` 起按依赖顺序编号 |
| blocking 边 | 票内 `Blocked by: NN, NN` 行 | 依赖其它票时声明 |
| triage 状态 | 票顶部 `Status:` 行 | 取值见 `triage-labels.md` |
| 评论 | 追加到 `## Comments` 节 | 追加记录，不覆盖历史 |

### 发布规范

- spec 与 tickets 发布时打 `ready-for-agent` 标签（标签词汇见 `docs/agents/triage-labels.md`）。
- 每张票必须是一个可独立 demo 的垂直切片（切穿 schema / API / UI / tests 全层），单个新上下文窗口可装下。
- 宽重构使用 expand–contract 序列。

## 其它选项备忘（当前未采用）

- **GitHub（gh CLI）**：需 gh 可用并已认证，未来如启用，本文档切换为 GitHub，标签词汇改为实际 repo labels。
- **GitLab（glab CLI）**：同上，切换为 glab。
- **Other**：Jira / Linear 等，记录为一段自由文本描述。
