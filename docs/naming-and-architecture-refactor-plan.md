# 命名与架构重构计划

> 基于 `docs/method-inventory.md`、`docs/method-constraint-audit.md`、`docs/architecture-principles.md`、`docs/architecture-overview.md`、`docs/development-guide.md`、`docs/architecture-review.md` 汇总。
> 详细 Before/After 对照见 `docs/naming-refactor-review-plan.md`。

## 基线

- 扫描源码文件：362。
- 提取方法/函数：2894。
- 提取类型/类：730。
- Tauri commands：30，全部符合 `snake_case`。
- 显式 WASM `js_name` 导出：144，全部符合 camelCase/PascalCase。
- 裸 WASM 推断 JS 名：68。
- 裸 WASM 推断名违规：57。
- 长/句子式方法名：78。
- 生产代码长/句子式方法名：13。
- 测试代码长/句子式方法名：65。
- 类型/类名违规：0。
- raw invoke 命令字符串：23，命令字符串本身符合小写规则。

## 约束摘要

- 命名质量是硬架构约束，影响可读性、编码速度、重构安全和项目后续可维护性。
- 方法、函数、类、类型名必须短、稳定、可搜索、语义聚焦，并和项目统一前缀/后缀习惯一致。
- Rust 函数保持 `snake_case`。
- Tauri command 必须保持 `snake_case`。
- WASM JS-facing 名必须是 camelCase/PascalCase，并尽量通过 domain facade/session API 暴露。
- `docs/api-contract.md` 中 Stable API 已冻结；破坏性改名必须走 v2 命名空间和兼容周期。
- 公开 API 不应泄漏 `_workflow`、`_runtime`、`_host`、`_tx`、`pipeline`、历史标签、版本标签、`helper/manager/utils/misc`。
- 前缀/后缀词汇必须稳定：`read/get/resolve/build/find/sync/set/open/close/save/commit/undo/redo` 保持文档含义，不随意引入同义词。
- TS 只做宿主适配；PDF 决策、布局、字体、glyph、渲染准入、编辑语义归 Rust/WASM。

## 命名问题

### P0：长/句子式方法名

这是第一优先级。问题不在于这些名字是不是合法 `snake_case`，而在于它们把完整测试场景、条件、实现细节塞进函数名，破坏阅读和重构。

当前数量：

- 总数：78。
- 生产代码：13。
- 测试代码：65。

典型例子：

- `source_layout_sanitizes_partial_underlines_for_editor_canvas`
- `draft_layout_renders_compact_pdf_text_when_runs_have_no_spaces`
- `changed_active_draft_layout_preserves_source_geometry_for_unchanged_parts`
- `runs_to_source_index_map_clamps_when_runs_has_chars_missing_in_source`
- `resolve_vector_page_model_from_app_state_with_revision`
- `execute_pdf_commands_with_app_state`

处理方向：

- 测试名缩短，场景放到测试数据、注释或子模块名。
- 大型 `#[cfg(test)] mod tests` 视情况迁出生产模块。
- 生产函数名用模块路径表达领域上下文，函数名只保留聚焦动作。
- 含 `when/with_policy/with_revision/from_app_state` 的名字优先审查。

### P0：裸 WASM 推断 snake_case 导出

显式 `js_name` 表面没问题，真正问题是裸 `#[wasm_bindgen]` 函数会把 Rust snake_case 直接暴露到 JS。

影响范围：

- `crates/pdf-viewer-ui/src/document/free_api.rs`
- `crates/pdf-viewer-ui/src/render/free_api.rs`
- `crates/pdf-viewer-ui/src/viewer/free_api.rs`
- `crates/pdf-viewer-ui/src/zoom/free_api.rs`
- `crates/pdf-viewer-ui/src/render/canvas.rs`

处理方向：

- 仍被 TS 使用的 API，加显式 camelCase `js_name` 或迁移到 session/facade。
- 兼容 API 加 `Compat` 或废弃说明。
- 未使用导出删除。
- 审查脚本禁止新增裸 snake_case WASM 导出。

### P1：历史标签污染

当前命中：

- `targetInvokeV3`
- `__targetInvokeV3`
- `host.wasmv3`
- `[V3-Sovereign]`
- `[Sovereignty]`
- `v3_y`
- `v3_model`
- `backend-sovereign`

处理方向：

- 内部统一改成 `invokeTauriCommand`、`pdfViewerWasm`、`[PDF-WASM]`、`[PDF-RUNTIME]`。
- window alias 暂时保留兼容。
- 局部参数改为 `page_y_down`、`layout_model`。
- 语义标签改为 `backend-owned-region`。

### P1/P2：helper / manager / utils

问题点：

- `crates/pdf-viewer-core/src/utils/*`
- `crates/pdf-viewer-ui/src/utils/chain_trace.rs`
- `src/bridge/comment/pdf_comment_wasm_bridge.ts::getCommentManager`
- `utils/ai-settings.ts`

处理方向：

- 通用调试类迁到 `diagnostics::*`。
- 几何/数值 sanitize 迁到领域模块。
- `getCommentManager` 改成 `getCommentSessionApi`。
- `loadAiSettings` 改成 `readAiSettings`。

## 其它既有问题

### API 出口仍然过宽

`free_api.rs`、历史 WASM 出口、裸导出仍然太多。目标是让 `api.rs` 和 domain session/facade 成为唯一文档化出口，旧 free API 分批废弃。

### raw invoke 仍是弱类型边界

当前有 23 处 raw invoke 字符串。命令名格式正确，但边界仍是 stringly typed。目标是引入 `pdfCommands.*` typed wrapper。

### TS 仍需防止越界做 PDF 决策

重点复查：

- `src/bridge/render/*`
- `src/bridge/viewer/page_presentation_runtime.ts`
- `src/bridge/viewer/pdf_runtime.ts`

TS 可以持有 DOM/canvas/timer/event，但不应持有 PDF 语义、布局、字体、glyph、渲染准入决策。

### 全局状态和端到端数据流仍不够清晰

需要文档化并逐步收口：

- 打开 PDF -> preview/vector/detail render -> 页面可见。
- 编辑文本 -> commit patch -> save PDF -> refresh render。

### 打包 exe 回归必须进入命名重构验收

之前出现过 dev 正常、release exe 打开/渲染异常。每批重构必须验证：

- 文本 PDF。
- 扫描 PDF。
- 矢量复杂 PDF。
- 翻页和 preview-to-vector。
- 涉及文档命令时验证保存/撤销/重做。

## 阶段计划

### Phase 0：固化审查规则

- 保留并维护 `scripts/generate-method-inventory.mjs`。
- 审查项包括：长/句子式方法名、Tauri command snake_case、显式 WASM `js_name`、裸 WASM snake_case、历史标签。
- 验证：`node scripts/generate-method-inventory.mjs`。

### Phase 1：缩短长方法名并整理测试

- 先处理 `crates/pdf-viewer-core/src/edit/draft_layout.rs`。
- 再处理其它 P0 长测试名。
- 最后处理 13 个生产代码长命名。
- 验证：相关 Rust 单测 + inventory。

### Phase 2：清理历史标签

- `targetInvokeV3` 内部改为 `invokeTauriCommand`。
- `Sovereignty/V3` 日志改为中性标签。
- `v3_*` 参数和 `backend-sovereign` 标签改名。
- 验证：`npm run build`。

### Phase 3：收拢公开 WASM 导出

- 先处理较小的 `document/free_api.rs` 和 `zoom/free_api.rs`。
- 再分组处理 `render/free_api.rs`。
- 加兼容 wrapper 或迁移到 session/facade。
- 验证：`npm run wasm:pdf-viewer-ui`、`npm run build`、相关 E2E。

### Phase 4：typed boundary

- 新建 typed Tauri PDF command facade。
- 替换 `targetInvokeV3('command')` raw string。
- 保留单一底层 invoke 函数。
- 验证：raw invoke 调用数下降。

### Phase 5：架构命名债

- `utils` 迁到领域或 diagnostics。
- 清理死 free API。
- 审查 `workflow/runtime/host`，只保留内部且准确的名字。
- 同步更新架构文档。

### Phase 6：打包应用回归门禁

- dev smoke。
- packaged build。
- release exe 验证文本/扫描/vector PDF。
- 补充或更新对应 E2E fixture。

## 验收标准

- 不新增长/句子式方法名。
- 不新增裸 WASM snake_case 导出。
- Tauri command 保持 `snake_case`。
- 显式 WASM `js_name` 保持 camelCase/PascalCase。
- 类型/类名保持 PascalCase、短语义。
- raw invoke 调用被 typed wrapper 包住。
- 历史标签不出现在活跃运行时命名和日志中，兼容 alias 除外。
- release exe 能打开并渲染文本、扫描、矢量 PDF。
