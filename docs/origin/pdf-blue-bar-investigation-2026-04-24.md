# PDF 编辑态蓝横条全局分析与修复计划

## 问题定义

当用户点击列表段落中的正文行进入编辑态时，页面会出现一条原 PDF 中不存在的蓝色横向粗线。

当前稳定复现特征：

- 点击列表段落正文，例如“专业：计算机科学与技术”
- 进入编辑态后立即出现蓝条
- 蓝条常常横跨正文区域，有时会从正文右侧“漏出”
- 退出编辑态后，蓝条状态会变化或消失

这说明问题不是单纯的文本样式问题，而是“进入编辑态时页面层与编辑层的渲染 owner 不一致”。

## 当前架构现状

### Rust 侧相关模块

- 激活入口：[crates/pdf-viewer-ui/src/editor/activation.rs](E:/chain/nushell-enhanced/crates/pdf-viewer-ui/src/editor/activation.rs)
- 事务入口：[crates/pdf-viewer-ui/src/editor/render_transaction.rs](E:/chain/nushell-enhanced/crates/pdf-viewer-ui/src/editor/render_transaction.rs)
- 编辑运行态：[crates/pdf-viewer-ui/src/editor/runtime.rs](E:/chain/nushell-enhanced/crates/pdf-viewer-ui/src/editor/runtime.rs)
- 文档计划：[crates/pdf-viewer-ui/src/editor/document_plan.rs](E:/chain/nushell-enhanced/crates/pdf-viewer-ui/src/editor/document_plan.rs)
- 编辑可视层：[crates/pdf-viewer-ui/src/editor/visual.rs](E:/chain/nushell-enhanced/crates/pdf-viewer-ui/src/editor/visual.rs)
- 页面 overlay 组装：[crates/pdf-viewer-ui/src/editor/paragraph_overlay.rs](E:/chain/nushell-enhanced/crates/pdf-viewer-ui/src/editor/paragraph_overlay.rs)
- 页面有效绘制计划：[crates/pdf-viewer-ui/src/render/effective_page_plan.rs](E:/chain/nushell-enhanced/crates/pdf-viewer-ui/src/render/effective_page_plan.rs)
- 页面实际绘制：[crates/pdf-viewer-ui/src/render/canvas.rs](E:/chain/nushell-enhanced/crates/pdf-viewer-ui/src/render/canvas.rs)

### TS 宿主侧相关模块

- 编辑宿主控制：[src/plugins/pdf-viewer/editor_host.ts](E:/chain/nushell-enhanced/src/plugins/pdf-viewer/editor_host.ts)
- 编辑宿主视图：[src/plugins/pdf-viewer/editor_host_view.ts](E:/chain/nushell-enhanced/src/plugins/pdf-viewer/editor_host_view.ts)
- wasm 适配：[src/plugins/pdf-viewer/editor_wasm_api.ts](E:/chain/nushell-enhanced/src/plugins/pdf-viewer/editor_wasm_api.ts)
- 终端诊断：[src/plugins/pdf-viewer/editor_host_diagnostics.ts](E:/chain/nushell-enhanced/src/plugins/pdf-viewer/editor_host_diagnostics.ts)

## 当前根因假设

### H1：原 PDF Path 未被正确 suppress

最高优先级假设。

表现符合：

- 蓝条更像原页面 Path，而不是编辑态文字或 caret
- 蓝条位置和列表段落所在区域强相关
- 日志已经证明当前目标行不是正文 underline 继承

### H2：shell / body / marker 边界不一致

如果页面 suppress 依据的 shell bbox 和编辑层遮罩使用的 bbox 不一致，就会出现：

- 页面层没有 suppress 掉整条 path
- 编辑层只遮掉与 shell 相交的一部分
- 视觉上表现为“剩余半截蓝条”

### H3：列表语义仍未完全对象化

如果 marker、body、装饰 line/path 仍然靠多处推断，而不是统一进入 `EditorActivationResult`，就会出现某些对象被错误归属到 body 或外部装饰对象。

## 单一观察面

后续分析只保留下面 6 个关键事件，不再输出大面积噪声：

1. `activation.client.resolved-open-point`
2. `open.runtime.target-built`
3. `visual.paint.style-flags`
4. `paint.overlay.active-shell-occlusion`
5. `effective-plan.overlay-path-summary`
6. `canvas.draw.vector-path`

它们分别回答 6 个问题：

1. 这次点击最终打开了哪一个 paragraph，shell bbox 是什么
2. 运行态最终构建出的 active target 是什么
3. 编辑层正文是不是 underline
4. 页面层当前到底遮了哪块 shell
5. 页面层对于相交 path 的 suppress 统计是什么
6. 最终真正画出来的可疑 path 是哪个对象

## 全局修复目标

### 目标 1：单一激活入口

进入编辑态的行为必须收敛为单一 Rust 用例：

- 输入：点击请求或区域请求
- 输出：统一的激活结果

不允许 TS、事务层、运行态各自再补半段逻辑。

### 目标 2：单一页面 owner

原 PDF 对象是否继续绘制，只允许页面层决定：

- `effective_page_plan`
- `canvas.draw_vector_object`

编辑层不再靠白底遮罩去“补救”页面层漏掉的对象。

### 目标 3：单一编辑 owner

编辑层只负责：

- draft body text
- caret
- marker 重绘

不负责列表装饰 path，不负责 suppress，不负责页面级补偿。

### 目标 4：TS 退回宿主适配

TS 只负责：

- DOM 点击与键盘桥接
- canvas 挂载
- 调用 wasm 单一入口
- 显示摘要日志

不再持有任何“蓝条该不该显示”的判断。

## 修复阶段

### 阶段 A：观察面统一

- 收紧终端日志为 6 个事件
- 固定复现样本
- 固定同一行点击
- 输出 shell bbox、body bbox、first path summary、drawn path summary

### 阶段 B：激活结果对象化

将进入编辑态时真正需要的结果对象化为：

- `EditorActivationRequest`
- `EditorActivationResult`
- `ParagraphShellPlan`
- `ParagraphSuppressionPlan`
- `EditorVisualPlan`

### 阶段 C：页面 suppress 收口

由页面层统一消费 `ParagraphSuppressionPlan`：

- 决定哪些 path/text/image 继续画
- 决定哪些 path 整体 suppress

### 阶段 D：编辑层收口

由编辑层统一消费 `EditorVisualPlan`：

- 只画 draft body
- 只画 caret
- 只画 marker

### 阶段 E：删除旧分叉

删除所有仍然在：

- TS 宿主
- 事务层
- runtime 辅助链

里残留的重复判定与视觉补偿。

## 验收标准

1. 点击任意列表段落正文进入编辑态，不再出现蓝横条
2. 退出编辑态后页面与静态视图一致
3. Rust 页面层能明确记录被 suppress 的目标 path 或证明不存在该 path
4. 编辑层日志能明确证明当前正文没有额外 underline 或装饰线
5. TS 不再承担蓝条隐藏逻辑
6. `cargo check --manifest-path crates/pdf-viewer-ui/Cargo.toml --target wasm32-unknown-unknown` 通过
7. `npx tsc --noEmit` 通过
