# pdf-viewer-standalone (纸鸢) 已知架构

## 项目范围
Tauri 2 + Rust + WASM + TypeScript 的 PDF 阅读器，项目目录 `F:\chain\pdf-viewer-standalone`。

## 工作空间
- `crates/pdf-viewer-core` — PDF 数据引擎（纯 Rust，lopdf 解析、Vello 渲染、几何、文本、历史）
- `crates/pdf-viewer-ui` — WASM 前端代理（通过 wasm-pack + wasm-bindgen → web 靶目标）
- `src-tauri` — Tauri 后端主程序（standalone）
- `src/` — TypeScript 前端（Vite 构建）

## 分层架构

| 层 | 路径 | 职责 |
|---|---|---|
| **Presentation** | `src/bridge/` + HTML | 防抖/节流、DOM 刷新、aborted 信号优雅处理 |
| **Interface** | `src-tauri/src/interfaces/` | 入参反序列化 + 委派分发 + Early-Abort 拦截 |
| **Infrastructure** | `src-tauri/src/infrastructure/` + `crates/` | 纯净计算：PDF 读取、渲染、几何计算，禁止读取 UI 交互状态 |
| **Application** | `src-tauri/src/application/` | 存储、分类、分析（业务编排） |
| **Verification** | 构建流程 | 6 大构建检查点 |

## 命名约定
- WASM/JS 边界文件：`*_api.rs`（UI crate 侧）
- 后端接口层：`interfaces` 包
- 有状态模块：`*_store.rs`
- 纯业务逻辑：`*_service.rs`

## 构建检查点
1. `cargo check -p pdf-viewer-core`
2. `cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown`
3. `cargo check -p pdf-viewer-standalone`
4. `cargo test -p pdf-viewer-core`
5. 前端修改 → `tsc && vite build`
6. 完整 `cargo build`

## 安全相关
- CSP 允许 `unsafe-inline` `unsafe-eval`，大量外部域名白名单
- `assetProtocol.scope: ["**"]` 全放开
- 本地方可读路径：桌面 `$DESKTOP/英语/**`、`$DESKTOP/**`、`$APPDATA/**`
- 文件写入允许：`fs:allow-write-text-file`

## 依赖
- Rust: serde, serde_json, log, wasm-bindgen, lopdf (implied by core), vello (implied by render)
- TS: @tauri-apps/api ^2.0.0, plugin-dialog, plugin-fs, plugin-shell, vite ^5
- Agent: Google Gemini API (generativelanguage.googleapis.com 在 CSP 中)
