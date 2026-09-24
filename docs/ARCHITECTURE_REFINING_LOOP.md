# 架构精化闭环与验证分层

> 本文将 SpecDB 论文的 refining agent 闭环转为本项目的工程流程/检查清单，覆盖验证分层决策树、性能预算登记、"三轮无进展"升级规则、借鉴台账格式与分层并行规则。
> 与 `CONTRACTS.md`、`ARCHITECTURE_VARIANTS.md` 分工如下：
>
> - `CONTRACTS.md`：跨模块不变量与修改联动规则。
> - `ARCHITECTURE_VARIANTS.md`：运行时变体、模块特征树、外部参考登记。
> - **本文**：精化闭环流程、验证分层、性能预算、升级规则、借鉴台账、并行规则。

---

## 1. 精化闭环流程

每轮架构改进遵循以下循环：

```
1. 提出可证伪假设
   例："删除 legacy facade X 不会破坏任何生产调用。"
   例："将 tile scheduler 与 manager 拆分后，scheduler 可独立测试。"

2. 搜索调用方/契约
   全局引用搜索、检查 CONTRACTS.md、检查 ARCHITECTURE_VARIANTS.md。

3. 最小改动
   只做验证假设所需的最小变更。

4. 受影响测试
   运行受影响模块的 unit tests、integration tests。

5. Cargo/WASM/TS 验证
   cargo check、cargo test、cargo clippy、cargo fmt、tsc、vitest。

6. 必要时 E2E smoke
   npm run e2e:build && npm run e2e。

7. Trace/构建指标对比
   对比 trace 事件、构建耗时、WASM 体积。

8. 更新文档
   更新 CONTRACTS.md、ARCHITECTURE_VARIANTS.md、本文。

9. 进入下一轮
   提出下一个假设。
```

---

## 2. 验证分层决策树

按变更类型决定验证范围：

### 2.1 Native core 变更

- **触发命令**：
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --exclude pdf-viewer-ui --all-targets -- -D warnings`
  - `cargo test --workspace --exclude pdf-viewer-ui`
- **E2E 触发条件**：影响 PDF 解析、页面计划、渲染抑制。
- **性能 trace 触发条件**：影响启动、打开 PDF、首帧。

### 2.2 WASM/UI 变更

- **触发命令**：
  - `cargo clippy -p pdf-viewer-ui --target wasm32-unknown-unknown --all-targets -- -D warnings`
  - `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown`
  - `npm run wasm:pdf-viewer-ui`
  - `npx tsc --noEmit`
  - `npx vitest run`
- **E2E 触发条件**：影响 wasm ABI、RAF/zoom/tile、editor。
- **性能 trace 触发条件**：影响 RAF 调度、tile 生成、zoom transition。

### 2.3 Tauri/native 变更

- **触发命令**：
  - `cargo clippy -p pdf-viewer-standalone --all-targets -- -D warnings`
  - `cargo test -p pdf-viewer-standalone`
  - `npm run tauri dev`（手动验证）
- **E2E 触发条件**：影响 IPC、asset scheme、save transaction。
- **性能 trace 触发条件**：影响启动、GPU 初始化、PDF 解析。

### 2.4 TS bridge 变更

- **触发命令**：
  - `npx tsc --noEmit`
  - `npx vitest run`
- **E2E 触发条件**：影响 facade 调用、事件处理、DOM 适配。
- **性能 trace 触发条件**：影响 WASM/native bridge、渲染调度。

### 2.5 跨边界变更

- **触发命令**：全部上述命令。
- **E2E 触发条件**：必须触发。
- **性能 trace 触发条件**：必须触发。

---

## 3. 性能预算登记

当前 CI 尚无正式 benchmark 阈值，先复用现有 trace 和 `startup-performance-plan.md`，不伪造外部数值。

### 3.1 已识别性能指标

| 指标 | 当前基线 | 目标 | 验证方式 |
|---|---|---|---|
| 冷编译耗时 | 数十秒到数分钟 | 增量编译 < 30s | `cargo build` 耗时 |
| 启动耗时 | 待测 | 待测 | trace 事件 |
| 打开 PDF 耗时 | 待测 | 待测 | trace 事件 |
| 首帧渲染耗时 | 待测 | 待测 | trace 事件 |
| Page plan 构建耗时 | 待测 | 待测 | trace 事件 |
| Tile 生成耗时 | 待测 | 待测 | trace 事件 |
| RAF frame 耗时 | 待测 | < 16ms | trace 事件 |
| WASM build 体积 | 待测 | 待测 | `npm run wasm:pdf-viewer-ui` 产物大小 |

### 3.2 性能验证流程

1. 识别性能假设。
2. 运行 trace，记录当前基线。
3. 做最小改动。
4. 运行 trace，对比基线。
5. 若改善，更新文档；若恶化，回滚或上报。

---

## 4. "三轮无进展"升级规则

连续三轮同一失败/指标无改善时，停止点修补，转架构复核或上报：

```
轮次 1：尝试修复，记录失败原因。
轮次 2：尝试不同方案，记录失败原因。
轮次 3：尝试第三种方案，记录失败原因。
轮次 4：停止点修补，转架构复核或上报。
```

### 4.1 升级触发条件

- 同一 bug 连续三轮修复失败。
- 同一性能指标连续三轮无改善。
- 同一测试连续三轮失败。

### 4.2 升级动作

- 停止当前修复路径。
- 复核架构设计（检查 `CONTRACTS.md`、`ARCHITECTURE_VARIANTS.md`）。
- 上报团队讨论。
- 记录升级原因和决策。

---

## 5. 借鉴台账格式

每次借鉴外部方案时，记录以下信息：

| 字段 | 说明 |
|---|---|
| **Reference** | 来源（论文、项目、文档）。 |
| **Hypothesis** | 借鉴假设。 |
| **Change** | 实际变更。 |
| **Evidence** | 验证证据（测试、trace、benchmark）。 |
| **Effective?** | 是否有效。 |
| **Rejected?** | 是否拒绝。 |
| **Follow-up** | 后续行动。 |

### 5.1 台账示例

#### 借鉴 1：SpecDB 跨模块不变量

- **Reference**：SpecDB（arXiv 2605.31097）。
- **Hypothesis**：将跨模块不变量显式化为契约节点，可减少多 agent 并行改代码时的冲突。
- **Change**：创建 `docs/CONTRACTS.md`，记录 11 条契约。
- **Evidence**：本文档。
- **Effective?**：是。
- **Rejected?**：否。
- **Follow-up**：无。

#### 借鉴 2：SpecDB 实现变体

- **Reference**：SpecDB（arXiv 2605.31097）。
- **Hypothesis**：用 feature tree / implementation variants 描述合法运行形态，可帮助决定验证范围。
- **Change**：创建 `docs/ARCHITECTURE_VARIANTS.md`，记录 7 个运行时变体。
- **Evidence**：本文档。
- **Effective?**：是。
- **Rejected?**：否。
- **Follow-up**：无。

#### 借鉴 3：SpecDB 精化闭环

- **Reference**：SpecDB（arXiv 2605.31097）。
- **Hypothesis**：将 refining agent 闭环转为工程流程，可提高架构改进的可控性。
- **Change**：创建本文档，记录精化闭环流程。
- **Evidence**：本文档。
- **Effective?**：是。
- **Rejected?**：否。
- **Follow-up**：无。

---

## 6. 分层并行规则

### 6.1 可并行任务

- **CI 层**：
  - native clippy
  - wasm clippy
  - native tests
  - fmt
  - TypeScript typecheck
  - Vitest
- **审计层**：
  - shim/re-export 引用审计
  - E2E spec/fixture 审计
  - trace/performance probe 审计
  - 外部架构参考映射
- **独立代码任务**：
  - 一个 agent 分析 core 目录。
  - 一个 agent 分析 UI/WASM。
  - 一个 agent 分析 Tauri/native。
  - 一个 agent 分析测试和 CI。
  - 一个 agent 分析文档与兼容 shim。

### 6.2 不可并行任务

- 多人同时修改同一个 `mod.rs`。
- 多人同时迁移同一个 WASM API。
- 同时重构 zoom 状态和 RAF 调度。
- 同时删除 facade 并迁移其 TS 调用方。
- 同时改变 render plan 数据结构和其测试 fixture。

### 6.3 并行规则

- 并行任务必须无共享写入模块。
- 并行任务必须通过独立验证。
- 并行任务必须在合并前做集成测试。

---

## 7. 参考

- `docs/CONTRACTS.md`
- `docs/ARCHITECTURE_VARIANTS.md`
- `docs/api-contract.md`
- `docs/architecture-principles.md`
- `docs/CONTEXT.md`
- `docs/architecture-overview.md`
- `docs/startup-performance-plan.md`
- `docs/adr/`
