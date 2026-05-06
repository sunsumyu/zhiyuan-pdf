# 架构铁律（Architecture Principles）

> 综合自 `origin/pdf-blue-bar-investigation-2026-04-24.md`、`origin/pdf-viewer-architecture-audit-2026-04-19.md`、`origin/pdf-engine-naming-guide.md` 以及 2026-05-06 的分叉修复经验。

---

## 1. 单一渲染链原则（No Chain Fork）

**所有可见像素必须经由同一条渲染链输出。** 这是项目的最高原则——每次违反都会产生"飘移、tofu、颜色错"类肉眼可见缺陷。

### 视觉链定义

```
PDF 文件
  → pdf-viewer-core 解析（字符 + 字形 + 字体 + 坐标）
    → pdf-viewer-ui 生成 paint plan（含 overlays）
      → effective_page_plan 决定每对象是否抑制
        → canvas painter（Rust） 用 PDF 原字体绘制
          → DOM canvas 像素
```

### 三种禁止行为

1. **不要让浏览器字体承担显示职责。** HTML/CSS 字体回退会破坏字形一致性，PUA bullet 会变 tofu，CJK 字体替换会造成位置和粗细差异。
2. **不要在 TS 端做"二次绘制"。** TS 不能用 `<canvas>.fillText` 或 DOM textarea 的可见样式补绘任何 PDF 对象。
3. **不要让编辑层画白底遮罩去"修复"页面层未抑制的对象。** 这会制造视觉补丁链。

### 可见 textarea 是反模式

历史教训：standalone 在分叉时把编辑器改成"可见 textarea + display:none canvas"，这是错误方向。正确架构是**屏外不可见 textarea + 可见 canvas（由 Rust 绘制）**。详见 [editor-render-architecture.md](editor-render-architecture.md)。

---

## 2. 单一所有者原则（Single Owner）

每项能力**只允许一个 owner**，源自 `pdf-blue-bar-investigation`：

| 能力 | 唯一 owner |
|------|-----------|
| 进入编辑态 | `editor::activation`（Rust 单一用例）|
| 原 PDF 对象抑制 | `render::effective_page_plan` + `render::canvas` |
| Draft body 文字绘制 | 编辑层（不画装饰 path） |
| Caret 渲染 | 编辑层 |
| Marker（项目符号）重绘 | 编辑层 |
| 坐标转换 | `core::coordinate_transform`（统一 mapper） |
| 渲染事务 | `RenderTransaction` 单一通道 |
| 编辑命令（手动 + AI） | `EditorEngine::apply_command/commit` |
| 保存写回 | `PdfPersistencePort` |
| 日志 | `DiagnosticsApi::emit` |

**违反信号：** 修一处 bug 后另一条路径出现新 bug。这是分叉链的典型征兆。

---

## 3. Rust core / UI / TS 三层边界

源自 `origin/pdf-viewer-architecture-audit-2026-04-19.md`：

### Rust core (`crates/pdf-viewer-core`)

负责：领域模型与规则。

```
document/    page, region, paragraph, list
geometry/    coordinate_space, mapper, hit_test, viewport
text/        glyph_layout, editable_text, text_index, list_semantics
typography/  font_resolver, font_matcher, glyph_encoding
render/      paint_plan, snapshot_plan
persistence/ patch, history, write_plan
```

铁律：**core 不知道 DOM，不知道 Tauri。**

### Rust UI/WASM (`crates/pdf-viewer-ui`)

负责：应用服务 + WASM 适配。WASM API 是薄边界，应用服务组合 core 能力。

铁律：**不要在 WASM 函数名里塞完整流程。**

### TypeScript 插件 (`src/`)

负责：宿主适配。采集 DOM 事件 + 尺寸，执行 Rust 返回的 host 指令，调用 WASM。

铁律：**TS 不做 PDF 坐标、不做文本布局、不做字体解析、不做编辑语义、不做保存策略。**

---

## 4. 命名规范

源自 `origin/pdf-engine-naming-guide.md`：

### Rust 函数前缀

| 前缀 | 含义 |
|------|------|
| `build_*` | 创建无副作用的值 |
| `find_*` | 返回 `Option<T>` |
| `resolve_*` | 由输入 + 回退规则导出决定 |
| `sync_*` | 跨边界拷贝状态 |
| `set_*` | 单字段变更 |
| `open_*` / `close_*` / `save_*` / `commit_*` / `undo_*` / `redo_*` | 用例动作 |

### 禁止模式

- 边界词堆叠：`runtime_workflow_action_host` ❌
- 历史标签污染：`v19`、`audit`、`sovereign`（除临时日志 tag）❌
- 描述重构过程而非用途：`migrated_open_editor` ❌
- 仅以 `runtime`/`workflow`/`host` 区分的一次性包装函数 ❌
- `utils`/`helper`/`manager`/`misc` 模块（除明确临时）❌

### TS 命名

- 文件名按宿主能力（`dom_*`、`canvas_*`、`wasm_client`、`panel_*`），不按历史补丁
- TS 中**不出现** `layout engine`、`glyph`、`font resolver` 等领域词

---

## 5. 链分叉的诊断信号

如果出现以下症状，**先怀疑链分叉，不要在症状端修补**：

- 编辑态视觉与原 PDF 不一致（位置、字体、颜色、粗细）
- "□" tofu 字符（说明文字走了浏览器字体而非 PDF-Glyph）
- 修一处后另一处出新 bug
- 同一能力存在多个相似函数（如 `commit_v2` / `commit_v3` / `commit_workflow`）
- TS 在做坐标计算、字体判断、文本切分

**根治方法：找到那条没参与的"主链"，把分叉合回去，删除补丁链。**

---

## 6. 暂缓事项

下面这些事**不要在第一轮重构中做**（来自 audit 文档）：

- 大规模移动 `src-tauri` PDF 基础设施到 crates
- 一次性删除所有旧 WASM API
- 直接重写 PDF 字体写回
- 同时实现 Word 高级功能

原因：当前最大风险是链分叉与边界不清。先统一 API 与诊断，再扩展功能。

---

## 7. 验收标准

每完成一阶段重构，都应满足：

- [ ] 同一能力只有一个 Rust 入口
- [ ] TS 只做宿主适配，不持有领域规则
- [ ] 手动编辑、AI 编辑、撤销重做、保存使用同一条编辑/写回/渲染链
- [ ] 坐标转换只通过统一 mapper
- [ ] 日志一屏内能看到关键链路，不输出全量 DOM JSON
- [ ] 文件名和模块名能直接反映功能
- [ ] 新功能不进入大杂烩文件
- [ ] `cargo check --target wasm32-unknown-unknown` 通过
- [ ] `npx tsc --noEmit` 通过
- [ ] `npm run build` 通过
