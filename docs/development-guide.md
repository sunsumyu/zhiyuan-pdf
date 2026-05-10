# 开发指南

> v1 · 2026-05-06 · 与 `architecture-overview.md` 配套阅读

## 1. 环境准备

### 必须

- Rust ≥ 1.75（带 `wasm32-unknown-unknown` target）
- Node.js ≥ 18 + npm
- `wasm-pack` ≥ 0.12

### 推荐

- VS Code + rust-analyzer
- Tauri CLI：`cargo install tauri-cli`

### 首次设置

```pwsh
rustup target add wasm32-unknown-unknown
npm install
npm run wasm:pdf-viewer-ui   # 首次构建 WASM
```

## 2. 日常开发循环

### 2.1 改 Rust 后

```pwsh
npm run wasm:pdf-viewer-ui   # 重新生成 WASM
npm run dev                   # Vite 自动 reload
```

### 2.2 改 TS 后

```pwsh
npm run dev                   # Vite HMR 即可
```

### 2.3 改 src-tauri 后

```pwsh
npm run tauri dev             # 重启桌面 app
```

### 2.4 类型检查

```pwsh
npx tsc --noEmit              # 仅 TS
cargo check -p pdf-viewer-ui --target wasm32-unknown-unknown
```

## 3. 添加新功能：决策树

```
新增能力？
│
├─ 是 UI 视觉/事件 → 改 src/components/ 或 src/bridge/<existing-controller>
│
├─ 是 PDF 内容/编辑/缩放/渲染逻辑 → 改 Rust WASM
│   │
│   ├─ 现有 facade 已有对应 API？ → 直接调
│   │
│   ├─ 是 Stub API（命名已冻结）？ → 实现它（不要改名）
│   │     例：editor.cut → 在 editor/clipboard_workflow.rs 实现，
│   │            然后改 editor/facade.rs 的 facade_cut 函数体
│   │
│   └─ 全新能力？ → 走 §4「添加新 API」流程
│
└─ 是磁盘/系统/PDF 解析 → 改 src-tauri，新增 #[command]
```

## 4. 添加新 WASM API

### 4.1 选定域

参考 `docs/architecture-overview.md` §3 的 9 个域。如果不确定，提一个 issue 讨论。

### 4.2 决定稳定性

| 等级 | 时机 | 后续 |
|---|---|---|
| **Stub** | 概念已定，实现待做 | 注册命名 + 留 stub 实现 |
| **Experimental** | 在 1-2 个版本内可能调整 | 在文档标注，不放进 facade |
| **Stable** | 立刻可用，命名+签名冻结 | 加进 `<domain>/facade.rs` |

### 4.3 实施步骤

**步骤 1：实现 host workflow**

新代码放在 `<domain>/<topic>.rs`，函数命名 snake_case。

```rust
// crates/pdf-viewer-ui/src/document/metadata.rs
pub fn read_document_metadata() -> DocumentMetadata { ... }
```

**步骤 2：暴露到 facade**

```rust
// crates/pdf-viewer-ui/src/document/facade.rs
use crate::document::metadata::read_document_metadata as host_read_metadata;

#[wasm_bindgen(js_name = "documentFacadeReadMetadata")]
pub fn facade_read_metadata() -> JsValue {
    to_value(&host_read_metadata()).unwrap_or(JsValue::NULL)
}
```

> 如果 API 是 Stub 升级，**不要新增函数**——把现有 `stub("...")` 实现替换为真实逻辑。

**步骤 3：补 TS 端**

```ts
// src/bridge/document_facade.ts
export function facadeDocumentReadMetadata(): unknown {
    return call('documentFacadeReadMetadata');
}
```

**步骤 4：更新文档**

- `docs/api-contract.md` 域表里把 Stub → Stable
- 在表格里写明用途和参数

**步骤 5：构建 + 测试**

```pwsh
npm run wasm:pdf-viewer-ui
npm run build
npm run dev
```

## 5. 添加新 Tauri command

```rust
// src-tauri/src/interfaces/multimedia/pdf.rs
#[command]
pub async fn extract_text_from_page(
    state: tauri::State<'_, crate::AppState>,
    path: String,
    page_index: u16,
) -> Result<String, String> {
    PdfTextService::extract_page(&path, page_index).await
}
```

注册：

```rust
// src-tauri/src/lib.rs
.invoke_handler(tauri::generate_handler![
    ...,
    interfaces::multimedia::pdf::extract_text_from_page,
])
```

前端调用：

```ts
import { invoke } from '@tauri-apps/api/core';
const text = await invoke<string>('extract_text_from_page', { path, pageIndex });
```

> Tauri command 必须是 `snake_case`，参数在 TS 端用 `camelCase`（Tauri 自动转换）。

## 6. 测试策略

### 当前状态

- WASM 单元测试：`crates/pdf-viewer-ui/tests/`（部分覆盖）
- Rust 后端：`src-tauri/src/**/*tests*`
- 前端 E2E：暂无（待 Phase 7）

### 推荐

1. **新 host workflow** → 加 `#[cfg(test)] mod tests` 单测
2. **新 facade API** → 加一条 wasm-bindgen-test（运行在浏览器）
3. **新 Tauri command** → 加 mock IO 的 unit test

## 7. 编码规范

### Rust

- 公共 API 必须有 doc comment
- 没有 `unwrap()` 在 wasm 入口（用 `unwrap_or(JsValue::NULL)`）
- thread_local 状态必须在单一文件管理（不要散落）
- 从不在 `<domain>/facade.rs` 写业务逻辑——只做 serde 桥接 + 转发

### TypeScript

- `strict: true` 已开启
- 不允许 `any`，请用 `unknown` + 类型 guard
- facade 调用 `getWasmApi()` 必须 null-check（防止 wasm 未加载）

### 命名

- WASM js_name → `<domain>Facade<Verb>`（camelCase）
- Tauri command → `<verb>_<noun>` （snake_case）
- TS facade 函数 → `facade<Domain><Verb>`

## 8. 提交前 checklist

- [ ] `npm run wasm:pdf-viewer-ui` 通过（0 errors）
- [ ] `npm run build` 通过
- [ ] 改了 facade？同步更新 `docs/api-contract.md`
- [ ] 改了架构？同步更新 `docs/architecture-overview.md`
- [ ] 加了 Stub？在对应 Session 模块里用 `XxxError::NotImplemented { method }` 占位
- [ ] commit message 描述哪个域 + 哪个 API

## 9. 常见任务

### 9.1 把 Stub 升级为 Stable

参考 §4.3 步骤 2，**不要改 js_name**。

### 9.2 弃用一个 Stable API

```rust
#[wasm_bindgen(js_name = "editorFacadeOpen")]
#[deprecated(since = "0.2.0", note = "use editorFacadeOpenV2")]
pub fn facade_open(...) -> ... { ... }
```

并在 `docs/api-contract.md` 弃用清单中登记。最少保留 2 个 release 周期再删除。

### 9.3 添加一个新域

1. `crates/pdf-viewer-ui/src/<domain>/mod.rs` + `facade.rs`
2. 在 `lib.rs` 加 `pub mod <domain>;`
3. 创建 `src/bridge/<domain>_facade.ts`
4. 在 `docs/api-contract.md` 增加 §3.x 节
5. 在 `docs/architecture-overview.md` §3 表格加一行

## 10. 排错

| 症状 | 可能原因 | 排查 |
|---|---|---|
| WASM 函数 undefined | wasm 包未重新生成 | `npm run wasm:pdf-viewer-ui` |
| 修改 .rs 不生效 | wasm-pack 缓存 | 删 `crates/pdf-viewer-ui/pkg`, 重建 |
| 前端报 panic | Rust 端 unwrap on None | 看 console，定位 panic！的栈 |
| Tauri invoke 超时 | command 未注册 | 检查 `src-tauri/src/lib.rs` invoke_handler |
| Vite HMR 失效 | 改了 wasm 但没 reload | 手动 F5 |

## 11. 进一步阅读

- `docs/architecture-overview.md` — 三层架构总览
- `docs/api-contract.md` — 9 域 API 清单
- `docs/architecture-principles.md` — 设计原则
- `docs/architecture-review.md` — 当前架构审查与 phase checkboxes
- `docs/archive/progress-2026-05-06-facade-era.txt` — 历史阶段进度（facade 时期，已被 Session API 取代）
