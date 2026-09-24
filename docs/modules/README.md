# PDF Viewer 模块与方法索引

> 本目录用于记录当前代码树中“每文件每方法”的功能梳理。方法功能以源码当前工作区为准；函数签名和文件路径可能随重构变化，文档不替代源码。

## 阅读顺序

1. 先读 [`architecture-map.md`](../guide/architecture-map.md)：四层边界和数据流总览（其中部分统计已滞后）。
2. 再读 `core-*`：纯 Rust 领域模型、几何、编辑和渲染决策。
3. 再读 `tauri-*`：桌面后端的应用命令、PDF 读写、字体和服务。
4. 最后读 `ui-*`：WASM 会话域、编辑、缩放和渲染编排。
5. 跨层追踪时，优先沿每份文档顶部的“上游 / 下游”字段跳转。

## 已完成的模块文档

| 文档 | 覆盖范围 | 主要职责 |
|---|---|---|
| [`core-foundation.md`](core-foundation.md) | `pdf-viewer-core/src/models*`, `geometry/`, `text/`, `typography/`, `common/`, `lib.rs` | DTO、坐标空间、排版、文本状态、字体匹配 |
| [`core-document-persistence.md`](core-document-persistence.md) | `pdf-viewer-core/src/document/`, `annotation/`, `history/`, `persistence/` | 文档区域、批注类型、历史和补丁持久化 |
| [`core-edit.md`](core-edit.md) | `pdf-viewer-core/src/edit/` | 编辑目标、源文本、草稿布局、替换区域 |
| [`core-render.md`](core-render.md) | `pdf-viewer-core/src/render/` | 帧计划、有效渲染计划、瓦片、质量、缩放决策 |
| [`tauri-application-interfaces.md`](tauri-application-interfaces.md) | `src-tauri/src/application/`, `interfaces/`、根入口 | Tauri 入口、30 个 IPC 命令、应用服务 |
| [`tauri-infra-readwrite.md`](tauri-infra-readwrite.md) | `src-tauri/src/infrastructure/pdf/pdf_read/`, `pdf_write/` 及布局/预览 | PDF 解析、内容流修补、布局和预览 |
| [`tauri-infra-services-font.md`](tauri-infra-services-font.md) | `src-tauri/src/infrastructure/pdf/font/` 及文档/缓存/日志服务 | 字体解析嵌入、文档服务、工作副本、缓存 |
| [`ui-editor.md`](ui-editor.md) | `pdf-viewer-ui/src/editor/` | 编辑会话、覆盖层、格式化、替换事务 |
| [`ui-zoom.md`](ui-zoom.md) | `pdf-viewer-ui/src/zoom/` | 缩放权威、RAF 循环、settle 信封、呈现提交 |
| [`ui-render.md`](ui-render.md) | `pdf-viewer-ui/src/render/` | Canvas、帧缓存、渐进渲染、瓦片宿主、WASM 入口 |
| [`ui-domains.md`](ui-domains.md) | `pdf-viewer-ui/src/document/`, `viewer/`, `page/`, `present/`, `presentation/`, `find/`, `comment/`, `annotation/`, `review/`, `host/` | 各会话域、平台布局/滚动桥接 |

## 尚未覆盖

以下范围没有在本轮生成逐方法文档，不能据此声称项目已完成全量方法梳理：

- `crates/pdf-viewer-ui/src/` 根与胶水层：`api.rs`、`application.rs`、`app_controller.rs`、`runtime.rs`、`ui_state_store.rs`、`projection_workflow.rs` 等。
- `src/` TypeScript 前端，尤其 `src/bridge/` 的运行时组装、编辑、缩放、渲染和 Tauri 调用桥。
- `src-tauri/src/infrastructure/pdf/` 中未被三份 Tauri 文档覆盖的外围文件（如若存在新增文件，应以 Glob 复核）。
- `tests/`、`src/__tests__/` 和构建脚本的测试方法清单。

## 已发现的跨模块结构问题

这些不是推测，而是各模块文档在源码中发现并记录的候选项：

- 缩放状态存在多套历史接口：`ZOOM_STATE`、`zoom_authority`、旧 preview host 和提交帧队列并存；详见 `ui-zoom.md`。
- UI 会话状态存在双轨：`editor_store` 与 `session/EDITOR_MODE_STATE` 并存；详见 `ui-editor.md`。
- `free_api`、`facade`、`host_runtime` 等迁移期命名仍未完全符合重构公约；详见各文档“疑点”。
- Core 的 persistence/history 与 UI 的 `ui_state_store` 各自持有历史语义，可能形成两套撤销模型；详见 `core-document-persistence.md`。
- Tauri 图像资产存在 `asset.localhost` 与 `pdfasset://` 两条协议；详见 `tauri-application-interfaces.md` 与 `tauri-infra-readwrite.md`。
- 字体查找在 Tauri `infrastructure/pdf/font/` 与 core `typography/` 各有实现；详见 `tauri-infra-services-font.md` 与 `core-foundation.md`。

## 领域词汇表的规范来源

目前仓库有两份词汇表：根目录 [`CONTEXT.md`](../../CONTEXT.md) 和 [`docs/CONTEXT.md`](../CONTEXT.md)。本轮只读未合并，后续应由 domain-modeling 逐条裁决。已确认的冲突包括：

- 缩放唯一权威：根文档使用 `ZOOM_STATE` / Zoom Authority；docs 文档使用 `HostZoomState`。
- 字体目录：根文档沿用 `pdf_write_font/` 的旧名；实际代码为 `src-tauri/src/infrastructure/pdf/font/`。
- Tile Manager：根文档强调调度器与缓存协作；docs 文档还保留旧的 `render_queue` 描述。
- `Anchor`：docs 文档称 ADR-0007 后已是死代码；根文档仍把锚点布局作为 `CommittedLayout` 的一部分描述。

在这些冲突裁决前，新增文档应同时引用源码路径与对应 ADR，不要擅自选择术语。
