# PDF Viewer 架构审计与收拢计划

日期：2026-04-19

## 目标

P0 功能已经接近可用，但代码组织仍然存在明显的架构债：能力边界不清、坐标变换散落、多条渲染/编辑/保存链路并存、命名不统一、日志噪声过大。这些问题会导致同一个 bug 在不同链路里反复出现，也会让后续 Word 类能力扩展变得不可控。

本文先记录问题，不直接改业务代码。后续重构应以本文为检查清单，按能力收拢，而不是继续做局部补丁。

## 当前结论

当前系统已经不是单纯“某个函数写错”的问题，而是典型的边界腐化：

- `pdf-viewer-core`、`pdf-viewer-ui`、`src/plugins/pdf-viewer` 三层都在不同程度参与坐标、缩放、编辑、渲染、保存调度。
- Rust 内部模块名与文件路径大量不一致，`#[path]` 映射让读者无法通过目录判断能力边界。
- TS 层仍然有不少布局、坐标、日志、渲染调度和编辑桥接逻辑，已经超过“宿主适配器”的合理范围。
- 同一能力存在多个入口：手动编辑、AI 编辑、保存、撤销重做、渲染刷新没有完全通过统一 API。
- 诊断日志没有统一策略，关键信息和全量 JSON 混在一起，反而降低定位效率。

## 高风险文件

这些文件超过或接近职责膨胀阈值，需要优先拆分或收拢：

| 文件 | 行数 | 问题 |
| --- | ---: | --- |
| `crates/pdf-viewer-core/src/document/page_region_context.rs` | 1210 | 页面区域、文本段、列表、持久化上下文混在一起，已经超过单文件合理边界。 |
| `src/plugins/pdf-viewer/ai/resume_ai_controller.ts` | 961 | AI 面板、请求、建议、应用、日志和错误展示混合，TS 侧业务过重。 |
| `src/plugins/pdf-viewer/editor_host.ts` | 936 | DOM 事件、textarea、WASM 调用、编辑提交、坐标 fallback、日志混合。 |
| `crates/pdf-viewer-core/src/models.rs` | 911 | 全局模型仓库化，模型边界不清。 |
| `crates/pdf-viewer-ui/src/render/canvas.rs` | 898 | Canvas painter、编辑 overlay、调试摘要、完整 paint plan 混合。 |
| `crates/pdf-viewer-ui/src/editor/document_plan.rs` | 766 | 编辑文档计划过大，可能同时承担目标解析、布局输入、段落计划。 |
| `crates/pdf-viewer-core/src/text/glyph_layout.rs` | 654 | 字形布局核心较大，应检查是否混入编辑态策略。 |
| `crates/pdf-viewer-ui/src/wasm_api/viewer.rs` | 642 | WASM API 过宽，暴露了很多内部步骤。 |
| `src/plugins/pdf-viewer/vector_host.ts` | 642 | TS 侧仍承担 progressive render、cache、present 决策协调。 |
| `crates/pdf-viewer-ui/src/present/plan_builder.rs` | 591 | present plan 构建复杂，应与 render/zoom 状态边界重新梳理。 |
| `crates/pdf-viewer-ui/src/editor/draft_layout.rs` | 587 | 编辑草稿布局核心较大，需要和 core glyph/layout 能力确认边界。 |
| `crates/pdf-viewer-ui/src/zoom/interaction.rs` | 518 | 缩放交互核心合理偏大，但需要成为唯一缩放数学入口。 |

## 主要架构问题

### 1. 模块名和文件路径不一致

`crates/pdf-viewer-ui/src/lib.rs` 大量使用：

```rust
#[path = "editor/draft_layout.rs"]
pub mod editor_draft_layout_workflow;
```

这会造成三个问题：

- 目录名说的是 `editor/draft_layout`，公开模块名却是 `editor_draft_layout_workflow`。
- 读调用点时无法直接定位文件。
- 重构时很容易出现“旧名字继续保留，新文件又新增”的假统一。

目标：

- 逐步取消内部 `#[path]` 别名，改成正常 `mod.rs` 或清晰目录模块。
- 文件名、模块名、能力名保持一致。
- `*_workflow`、`*_runtime`、`*_facade` 只在确实表达架构层级时保留，不作为默认后缀。

### 2. 坐标变换没有唯一所有者

现在坐标/viewport/zoom 相关逻辑分散在：

- `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs`
- `crates/pdf-viewer-core/src/geometry/layout_engine.rs`
- `crates/pdf-viewer-ui/src/editor/host_workflow.rs`
- `crates/pdf-viewer-ui/src/editor/text_geometry.rs`
- `crates/pdf-viewer-ui/src/editor/projection.rs`
- `crates/pdf-viewer-ui/src/projection_workflow.rs`
- `crates/pdf-viewer-ui/src/zoom/interaction.rs`
- `src/plugins/pdf-viewer/frame_plan.ts`
- `src/plugins/pdf-viewer/index.ts`
- `src/plugins/pdf-viewer/editor_host.ts`
- `src/plugins/pdf-viewer/viewer_geometry_probe.ts`
- `src/plugins/pdf-viewer/layout_trace.ts`

这违反了“一个坐标空间一个转换入口”的原则。后果是：

- 编辑命中、保存后重绘、撤销重做、缩放预览可能各自算一遍坐标。
- 一处修复后，另一条链仍然使用旧算法。
- TS fallback 坐标可能覆盖 Rust 侧真实投影。

目标：

- `pdf-viewer-core::geometry` 定义唯一坐标模型和值对象。
- `pdf-viewer-ui::geometry` 或 `projection` 只做 WASM 场景适配，不重新发明规则。
- TS 只能采集 DOM rect、scroll、client point，然后传入 Rust；不得计算 PDF/page/editor 坐标。

建议收拢接口：

```rust
pub struct ViewportMetrics { ... }
pub struct ClientPoint { ... }
pub struct PagePoint { ... }
pub struct PageRect { ... }

pub trait CoordinateMapper {
    fn client_to_page(&self, metrics: &ViewportMetrics, point: ClientPoint) -> PagePoint;
    fn page_to_client(&self, metrics: &ViewportMetrics, point: PagePoint) -> ClientPoint;
    fn page_rect_to_host(&self, metrics: &ViewportMetrics, rect: PageRect) -> HostRect;
}
```

### 3. 渲染链路仍然分叉

当前渲染相关能力分布在：

- Rust UI：`render/canvas.rs`、`render/workflow.rs`、`render/loop_workflow.rs`、`render/layer.rs`、`render/tile_cache.rs`、`present/*`、`zoom/*`
- TS：`vector_host.ts`、`vector_canvas_host.ts`、`render_flow.ts`、`frame_plan.ts`、`layout_trace.ts`

问题不是“文件多”，而是职责边界不稳定：

- Rust 生成 frame/render/present 决策。
- TS 也在做 cache、present、canvas box、progressive render 调度。
- 保存、撤销重做、编辑提交都会触发 refresh，但不是都经过同一个 render transaction。

目标：

- Rust 统一产出 `RenderTransaction`。
- TS 只执行 `RenderTransaction` 中的 DOM/canvas 指令，不参与决策。
- 保存、撤销、重做、AI 应用、手动编辑提交都调用同一个 `request_render(reason)`。

建议统一接口：

```rust
pub enum RenderReason {
    OpenDocument,
    EditPreview,
    EditCommit,
    Save,
    Undo,
    Redo,
    Zoom,
    PageNavigation,
}

pub struct RenderTransaction { ... }

pub fn request_render(reason: RenderReason, host: HostMetrics) -> RenderTransaction;
pub fn commit_present(result: PresentResult) -> RenderState;
```

### 4. 编辑链路仍然过散

编辑相关逻辑分散在：

- `editor/session.rs`
- `editor/command.rs`
- `editor/text_index.rs`
- `editor/text_model.rs`
- `editor/text_geometry.rs`
- `editor/draft_layout.rs`
- `editor/document_plan.rs`
- `editor/bridge.rs`
- `editor/host_workflow.rs`
- `src/plugins/pdf-viewer/editor_host.ts`
- `src/plugins/pdf-viewer/document_edit_api.ts`

典型症状：

- WASM API 名称过长，例如 `wasm_sync_and_commit_active_editor_text_and_schedule_render_v3`。
- 一个 API 同时表达同步文本、提交编辑、调度渲染三个动作。
- TS 侧仍有 caret、selection、fallback page point、textarea 同步等复杂逻辑。

目标：

- Rust 中形成 `EditorService` 或 `EditorEngine` 统一入口。
- 文本命令、光标、选择、布局、提交、撤销重做都走同一条编辑命令链。
- TS 只转发 DOM 输入事件，不维护编辑语义。

建议统一接口：

```rust
pub enum EditCommand {
    OpenAt(ClientPoint),
    InsertText(String),
    DeleteBackward,
    DeleteForward,
    MoveCaret(CaretMove),
    Commit,
    Cancel,
}

pub struct EditResult {
    pub editor_state: EditorState,
    pub render: Option<RenderTransaction>,
}

pub fn apply_edit(command: EditCommand, host: HostMetrics) -> EditResult;
```

### 5. 手动编辑和 AI 编辑必须共用写回能力

历史问题说明：AI 写回曾经绕开或半绕开手动编辑能力，导致字体、列表符号、fallback、撤销重做、保存状态不一致。

目标：

- AI 不直接写 PDF。
- AI 输出的是 `EditIntent` 或 `DocumentPatchIntent`。
- `EditIntent` 进入与手动编辑相同的 `EditorEngine` / `PersistenceEngine`。
- 字体解析、列表语义、可编码检查、fallback 字体、undo/redo 都在统一写回链处理。

建议统一接口：

```rust
pub enum EditSource {
    Manual,
    AiSuggestion { suggestion_id: String },
}

pub struct TextReplacementIntent {
    pub target: EditTarget,
    pub replacement: String,
    pub source: EditSource,
}

pub fn apply_text_replacement(intent: TextReplacementIntent) -> EditResult;
```

### 6. 日志没有诊断策略

当前日志入口包括：

- `src/plugins/pdf-viewer/layout_trace.ts`
- `src/plugins/pdf-viewer/editor_host.ts`
- `src/plugins/pdf-viewer/document_edit_api.ts`
- `src/plugins/pdf-viewer/render_flow.ts`
- `src/plugins/pdf-viewer/vector_host.ts`
- `src/plugins/pdf-viewer/vector_page_bundle.ts`
- `src/plugins/pdf-viewer/viewer_geometry_probe.ts`
- `src/plugins/pdf-viewer/ai/resume_ai_controller.ts`
- Rust `editor_debug_trace_workflow`

问题：

- 大 JSON 和关键摘要混在一起。
- 诊断点没有统一命名。
- TS 和 Rust 都可以随意打 terminal 日志。

目标：

- 建一个统一 `DiagnosticsApi`。
- 默认只输出一行摘要。
- 详细快照只在明确打开 debug flag 时输出。
- 日志必须包含 `chain`、`event`、`reason`、`page`、`zoom`、`size`、`revision` 等固定字段。

建议格式：

```text
[pdf.layout] save.before page=0 zoom=1 wrapper=976x842 page=595x842 canvas=595x842 rev=21
[pdf.layout] save.after  page=0 zoom=1 wrapper=976x842 page=595x842 canvas=595x842 rev=22 delta=stable
```

### 7. Dioxus 依赖和生成物需要清理

当前 `crates/pdf-viewer-ui/Cargo.toml` 已没有 `dioxus` 依赖，说明 Rust 源码层面不再依赖 Dioxus。

但 `src/plugins/pdf-viewer/wasm/` 生成物仍包含：

- `snippets/dioxus-cli-config-*`
- `snippets/dioxus-interpreter-js-*`
- `snippets/dioxus-web-*`
- `pdf_viewer_ui.js` 内仍 import Dioxus snippets

这说明生成物可能是旧 wasm 包残留，或者构建产物没有完全刷新。它不一定是当前源码问题，但会造成误判和运行时包污染。

目标：

- 确认当前构建产物是否仍引用 Dioxus。
- 若不需要，重新构建 wasm 包并删除旧 snippets。
- 若删除后功能正常，将 Dioxus 残留列入清理提交。

### 8. PDF 写回能力仍在 Tauri infrastructure 中

PDF 保存/写回相关能力集中在：

- `src-tauri/src/infrastructure/multimedia/pdf/lopdf_utils.rs`
- `src-tauri/src/infrastructure/multimedia/pdf/save_text_write_plan.rs`
- `src-tauri/src/infrastructure/multimedia/pdf/pdf_write_font_resolver.rs`
- `src-tauri/src/infrastructure/multimedia/pdf/save_engine.rs`
- `src-tauri/src/interfaces/multimedia/pdf.rs`

这部分目前还没有和 `pdf-viewer-core` / `pdf-viewer-ui` 形成清晰的应用服务边界。它是基础设施能力，但手动编辑和 AI 编辑都需要稳定调用它。

目标：

- 把 PDF 写回定义成 `PdfPersistencePort`。
- `src-tauri` 是 port adapter，不让 UI/AI 直接知道 lopdf 细节。
- `pdf-viewer-core` 保留写回意图、文本布局、字体语义，不直接依赖 Tauri。

## 目标架构

### Rust core：领域模型和规则

建议目录：

```text
crates/pdf-viewer-core/src/
  document/
    page.rs
    region.rs
    paragraph.rs
    list.rs
  geometry/
    coordinate_space.rs
    mapper.rs
    hit_test.rs
    viewport.rs
  text/
    glyph_layout.rs
    editable_text.rs
    text_index.rs
    list_semantics.rs
  typography/
    font_resolver.rs
    font_matcher.rs
    glyph_encoding.rs
  render/
    paint_plan.rs
    snapshot_plan.rs
  persistence/
    patch.rs
    history.rs
    write_plan.rs
```

原则：

- core 不知道 DOM。
- core 不知道 Tauri。
- core 定义文本、字体、坐标、编辑、渲染计划的领域规则。

### Rust UI/WASM：应用服务和适配

建议目录：

```text
crates/pdf-viewer-ui/src/
  api/
    viewer.rs
    editor.rs
    render.rs
  app/
    viewer_engine.rs
    editor_engine.rs
    render_engine.rs
  host/
    metrics.rs
    commands.rs
  render/
    canvas_painter.rs
    render_transaction.rs
  editor/
    editor_session.rs
    editor_command.rs
    editor_projection.rs
```

原则：

- WASM API 是薄边界。
- 应用服务组合 core 能力。
- 不在 WASM API 函数名里塞完整流程。

### TS 插件：宿主适配器

建议目录：

```text
src/plugins/pdf-viewer/
  bootstrap/
  dom/
  wasm/
  canvas/
  ai-panel/
  diagnostics/
```

原则：

- TS 采集 DOM 事件和尺寸。
- TS 执行 Rust 返回的 canvas/DOM host 指令。
- TS 不做 PDF 坐标、文本布局、字体、编辑语义、保存策略。

## 统一 API 收拢清单

| 能力 | 当前问题 | 目标 API |
| --- | --- | --- |
| 打开 PDF | TS、WASM、Tauri 多段 pipeline | `ViewerEngine::open_document` |
| 页面/缩放 | TS 构造 frame request，Rust 也维护 zoom state | `ViewerEngine::set_view` / `ZoomEngine::apply` |
| 坐标转换 | 多处 `client/page/scale` 计算 | `CoordinateMapper` |
| 编辑打开 | TS fallback + Rust target 解析 | `EditorEngine::open_at` |
| 文本输入 | textarea 与 Rust 同步逻辑复杂 | `EditorEngine::apply_command` |
| 编辑提交 | 同步、commit、render 绑在长 API 名里 | `EditorEngine::commit` |
| AI 应用 | 曾绕开手动编辑链 | `EditorEngine::apply_text_replacement` |
| 保存 | Tauri persistence 和 UI state 边界不清 | `DocumentService::save` |
| 撤销重做 | 可能绕过 render transaction | `DocumentService::undo/redo` |
| 渲染刷新 | 多入口触发 refresh | `RenderEngine::request_render` |
| 日志 | 分散 terminal_log | `DiagnosticsApi::emit` |

## 命名规范

### Rust

- 内部函数使用短而准确的 snake_case。
- `v3` 只允许保留在外部兼容层，不进入内部领域函数。
- 避免 `workflow`、`runtime`、`facade` 泛滥。
- 函数名不要同时描述多个动作。

示例：

| 当前倾向 | 建议 |
| --- | --- |
| `wasm_sync_and_commit_active_editor_text_and_schedule_render_v3` | WASM 边界：`wasm_commit_editor_v3`；内部：`commit_editor` |
| `wasm_open_paragraph_editor_at_client_point_and_schedule_render_v3` | WASM 边界：`wasm_open_editor_v3`；内部：`open_editor_at` |
| `editor_draft_layout_workflow` | `editor::draft_layout` |
| `render_facade_workflow` | `render::engine` 或 `render::transaction` |
| `state_manager` | 按具体状态命名，例如 `document_state`、`history_state` |

### TypeScript

- 文件名按宿主能力命名，不按历史补丁命名。
- TS 中不出现 `layout engine`、`glyph`、`font resolver` 等领域语义。
- TS 中保留 `dom_*`、`canvas_*`、`wasm_client`、`panel_*` 这类适配器命名。

## 分阶段重构计划

### Phase 0：建立审计基线

目标：

- 保留本文档作为后续重构的检查清单。
- 不改业务行为。
- 后续每次重构都在文档中标记完成项。

### Phase 1：日志收口

目标：

- 新建统一诊断模块。
- 禁止随意 `terminal_log` 输出大 JSON。
- 所有链路日志统一短格式。

收益：

- 先让后续定位可靠，避免日志把关键信息冲掉。

### Phase 2：坐标变换收口

目标：

- 把 `client/page/viewport/zoom/scroll` 转换集中到 Rust 坐标模块。
- TS 只传 DOM metrics。
- 删除 TS fallback page 坐标计算。

收益：

- 解决编辑命中、缩放锚点、保存后闪动、撤销重做重绘的共同根因之一。

### Phase 3：RenderTransaction 收口

目标：

- 保存、撤销、重做、编辑提交、AI 应用全部产出同一种 `RenderTransaction`。
- TS 只执行 transaction，不决定 render/present 策略。

收益：

- 避免“保存时放大再恢复”和“某些操作触发另一条刷新链”。

### Phase 4：编辑 API 收口

目标：

- 建立 `EditorEngine` 作为唯一编辑入口。
- 手动编辑和 AI 编辑共用 `apply_text_replacement`。
- caret、selection、delete、commit、cancel 统一由 Rust 管。

收益：

- 防止一条链修好了，另一条链继续压缩 gap 或错位。

### Phase 5：保存/写回 Port 化

目标：

- `src-tauri` 的 lopdf 写回能力作为 `PdfPersistencePort`。
- UI/AI 不直接依赖基础设施细节。
- 字体匹配、fallback、子集编码、历史记录走统一服务。

收益：

- 避免 AI 写回和手动写回不一致。

### Phase 6：模块命名和目录整理

目标：

- 消除内部 `#[path]` 别名。
- 按 capability 重命名文件。
- 拆分超过 800 行且职责混杂的文件。

收益：

- 项目能通过目录和函数名直接看出功能边界。

### Phase 7：清理 Dioxus 生成残留

目标：

- 确认当前 wasm 构建不需要 Dioxus。
- 删除或重新生成 `src/plugins/pdf-viewer/wasm/snippets/dioxus-*`。
- 更新 wasm package 描述，避免误导。

收益：

- 降低依赖误判和运行产物污染。

## 重构执行顺序建议

优先顺序不能从“看起来最乱的文件”开始，而应该从最容易制造跨链 bug 的能力开始：

1. 日志策略统一。
2. 坐标变换统一。
3. RenderTransaction 统一。
4. EditorEngine 统一。
5. AI/manual edit 统一。
6. persistence port 统一。
7. 文件和模块命名整理。
8. 删除旧生成物和无效工具。

## 明确暂缓

这些不应该在第一轮重构中做：

- 大规模移动 `src-tauri` PDF 基础设施到 crates。
- 一次性删除所有旧 WASM API。
- 直接重写 PDF 字体写回。
- 同时实现 Word 高级功能。

原因：当前最危险的是链路分叉和边界不清，必须先统一 API 和诊断，再扩展功能。

## 验收标准

后续每完成一阶段，应满足：

- 同一能力只有一个 Rust 入口。
- TS 只做宿主适配，不持有领域规则。
- 手动编辑、AI 编辑、撤销重做、保存使用同一条编辑/写回/渲染链。
- 坐标转换只通过统一 mapper。
- 日志一屏内能看到关键链路，不输出全量 DOM JSON。
- 文件名和模块名能直接反映功能。
- 新功能不进入大杂烩文件。

## 2026-04-19 执行记录

已开始按本文档做第一轮低风险收口：

- 新增 `src/plugins/pdf-viewer/diagnostics.ts`，作为 TS 宿主层唯一 terminal 日志适配器。
- 将 layout、document edit、render flow、vector host、vector page bundle、geometry probe、editor host、AI controller 的散落 `terminal_log` 调用接入统一诊断出口。
- 默认保留关键 layout 异常日志，render/editor/AI 细节日志改为 `__PDF_DIAGNOSTICS_VERBOSE === true` 时输出。
- 在 Rust core 的 `coordinate_transform` 中补充 `HostPageTransform`、`HostReferenceRect`、`ClientPoint`、`PageSize`、`PageScale`，作为后续统一 client/page/shell 坐标转换的核心入口。
- `dom_projection`、`projection_workflow`、`editor/host_workflow` 已开始改为调用 core 坐标入口，不再各自手写 scale 计算。

验证：

- `cargo check --manifest-path crates/pdf-viewer-ui/Cargo.toml --target wasm32-unknown-unknown` 通过。
- `npm run build` 通过。

遗留：

- `src/plugins/pdf-viewer/wasm` 仍有旧 Dioxus 生成物，但当前源码入口使用 `crates/pdf-viewer-ui/pkg`；需要单独确认后清理。
- `docs/` 当前被 `.gitignore` 忽略，本文档不会自动进入 git。
