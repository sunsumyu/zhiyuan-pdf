# Triage 标签词汇

> 由 setup-matt-pocock-skills 于 2026-08-04 生成 · 五个 canonical 角色 → 实际标签字符串的映射

## 映射

| Canonical 角色 | 实际标签字符串 | 用途 |
|---|---|---|
| needs-triage | `needs-triage` | 新条目待分类 |
| needs-info | `needs-info` | 缺信息，等待补全 |
| ready-for-agent | `ready-for-agent` | 可交给 agent 抓取执行 |
| ready-for-human | `ready-for-human` | 需要人类处理 |
| wontfix | `wontfix` | 明确不做 |

## 使用说明

- 当前 tracker 为 **Local markdown**：这些"标签"以票文件顶部的 `Status:` 行体现（值取上表实际标签字符串），不依赖 GitHub 等外部 label。
- 迁移到 GitHub/GitLab 时，改为实际 repo labels；本表同步更新。
