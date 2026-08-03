---
name: pdf-viewer-audit
description: |
  审查/审计纸鸢（Zhiyuan）PDF 阅读器项目（Tauri 2 + Rust + WASM + TS）。
  当用户提到"审查""审计""audit""review""检查项目""代码审查""安全检查""架构审查"或类似关键词时触发，不管中英文。
  对项目进行系统性审查：架构合规性、分层约束、命名约定、Tauri 安全配置（CSP / capabilities）、依赖健康状况、构建检查点、Rust 代码质量与边界洁净度。输出结构化审查报告。
---

# pdf-viewer-audit

审查纸鸢（Zhiyuan）PDF 阅读器项目的 skill。调用本 skill 前请先读 `references/architecture.md` 了解项目结构。

## 工作流

### 1. 确认审查范围

先问用户这次想审查什么维度，再开始。如果用户说"全部审查"或未指定，执行所有维度。

可选维度：
| # | 维度 | 说明 |
|---|------|------|
| 1 | **架构合规性** | 代码是否遵循 4 层架构？职责是否纯净？ |
| 2 | **命名约定** | `*_api.rs`、`*_service.rs`、`*_store.rs`、`interfaces` 包是否正确使用？ |
| 3 | **Tauri 安全配置** | CSP、assetProtocol 范围、capabilities 权限是否过松？ |
| 4 | **依赖审计** | Rust crates / npm 是否有已知漏洞、过时或未使用的依赖？ |
| 5 | **构建健康** | 6 大检查点能否通过？Cargo 有无警告？ |
| 6 | **Rust 质量** | 常见问题：不必要的 `clone()`、`unwrap()`、大锁竞争、unsafe 代码 |
| 7 | **边界洁净度** | Interface 层是否混入业务逻辑？Infrastructure 层是否接触了 UI 状态？ |

### 2. 按维度执行审查

每个维度独立审查，一个维度完成后再进行下一个。审查时注意：

#### 2.1 架构合规性
- **Presentation 层** (`src/bridge/` + HTML)：只做防抖/节流/DOM 刷新/aborted 信号处理。确保没有业务逻辑混入。
- **Interface 层** (`src-tauri/src/interfaces/`)：只做反序列化 + 委派 + Early-Abort。用 `grep` 或 `Agent` 搜索该目录下的 `.rs` 文件，检查是否包含了 `lopdf`、PDF 解析、几何计算等业务逻辑代码。
- **Infrastructure 层** (`src-tauri/src/infrastructure/`、`crates/`)：搜索是否有对 `app_state`、`page_index`、当前页面等前端交互状态的引用。该层必须纯净。
- **Application 层** (`src-tauri/src/application/`)：业务编排层，不应直接暴露给 Tauri Command（应该通过 Interface 层中转）。

#### 2.2 命名约定
- 搜索 `crates/pdf-viewer-ui/src/` 下是否存在 `*_api.rs` 文件（WASM 边界应使用该后缀）。
- 搜索 `src-tauri/src/interfaces/` 下文件名和模块名是否按 `interfaces` 包组织。
- 搜索全局的 `*_store.rs`（有状态模块）和 `*_service.rs`（纯业务模块），检查它们是否混用了职责。

#### 2.3 Tauri 安全配置
- 读取 `src-tauri/tauri.conf.json` 检查 CSP，注意：
  - `unsafe-inline` + `unsafe-eval` 的风险。
  - 是否真的需要 `generativelanguage.googleapis.com`（Agent 用途？）。
  - `assetProtocol.scope: ["**"]` 是否全放开。
- 读取 `src-tauri/capabilities/default.json`：
  - `fs:scope` 中 `$DESKTOP/**` 是否过宽（允许读写整个桌面）。
  - `fs:allow-write-text-file` 是否必要。

#### 2.4 依赖审计
- 运行 `cd "F:/chain/pdf-viewer-standalone" && cargo audit 2>/dev/null || echo "cargo-audit 未安装"`。
- 运行 `npm audit --omit=dev 2>/dev/null || npm audit`（从项目目录）。
- 检查 `Cargo.toml` / `package.json` 中是否有明显过时或不兼容的版本。

#### 2.5 构建健康
- 尝试 `cargo check -p pdf-viewer-core` 并记录结果。
- 尝试 `cargo check -p pdf-viewer-standalone` 并记录结果。
- 注意：WASM 检查需要 wasm32 target，如果用户环境不支持（如 Git Bash on Windows）则注明。
- 检查 `src/` TypeScript 是否可通过 `tsc --noEmit` 类型检查。

#### 2.6 Rust 质量 (快速扫描)
- 搜索 `unwrap()`、`expect()`、`panic!`、`todo!()` 在生产代码中的出现频率。
- 搜索 `.clone()` 在大数据路径上是否过多（尤其是 pdf-viewer-core）。
- 检查 `use std::sync::Mutex` 或 `RwLock` 的粒度是否过大。
- 搜索 `#![allow(...)]` 全局属性，查看是否有不必要的宽放。

#### 2.7 边界洁净度
- Interface 层 `src-tauri/src/interfaces/pdf/` 是否导入了 `crates/pdf-viewer-core` 的计算 API？是否有业务逻辑？正确做法是只调用 Application 层的方法。
- Infrastructure 层是否引用了 `tauri::State`、`app_state` 或任何 UI 交互变量。

### 3. 输出结构化报告

审查完毕后，按以下模板输出结果。使用 Markdown 表格和代码引用（格式 `路径:行号`）。

## 报告模板

```markdown
# 纸鸢 PDF 阅读器 — 审查报告

**审查日期**：{日期}
**审查维度**：{用户指定的维度 / 全部}
**审查方式**：自动化脚本 + 源码静态分析
**环境**：{OS, Rust 版本, Node 版本}

---

## 1. 架构合规性
{每个层级逐一列出，格式：}
| 层级 | 状态 | 发现问题 |
|------|------|----------|
| Presentation | ✅ / ⚠️ / ❌ | {问题描述 + 引用} |
| Interface | ✅ / ⚠️ / ❌ | ... |
| ... | ... | ... |

{详细说明每个问题}

## 2. 命名约定
{列出不符合约定的文件，列出正确做法}

## 3. Tauri 安全配置
| 配置项 | 评级 | 建议 |
|--------|------|------|
| CSP script-src | 🟢/🟡/🔴 | ... |
| assetProtocol.scope | 🟢/🟡/🔴 | ... |
| fs:scope | 🟢/🟡/🔴 | ... |

## 4. 依赖审计
| 源 | 结果 |
|----|------|
| cargo audit | {输出/无法运行} |
| npm audit | {输出/无法运行} |
| 人工检查 | {异常依赖} |

## 5. 构建健康
| 检查点 | 结果 |
|--------|------|
| cargo check -p pdf-viewer-core | ✅ / ❌ {输出摘要} |
| cargo check -p pdf-viewer-standalone | ✅ / ❌ {输出摘要} |
| tsc --noEmit | ✅ / ❌ {输出摘要} |
| ... | ... |

## 6. Rust 代码质量
- unwrap/expect 数量：{n}
- clone 热点：{路径}
- 全局 allow：{列表}
- 锁粒度建议：{如有}

## 7. 边界洁净度
{问题列表}

---

## 总结与优先级建议
{按严重性排序的改进建议}
```

## 注意事项

- **不要修改任何文件**。本 skill 是只读审计，发现的问题记录在报告中即可。
- 如果 cargo check 等命令因环境问题无法运行，在报告中注明原因而非跳过。
- 引用代码使用精确路径 + 行号格式（如 `src-tauri/src/foo.rs:42`）。
- 如果某个维度用户未要求审查，在报告中跳过或注明"未审查"。
- 最终报告直接以 Markdown 消息发送给用户，不写入文件。
