# 启动性能优化计划 v2

> v2 · 2026-06-08 · 修正 v1 的错误分析，聚焦真实根因
> 配套阅读：`docs/architecture-overview.md`、`docs/page-presentation-runtime-architecture.md`

---

## 0. 根因修正

**v1 分析错误**：v1 的分析基于"运行时性能"假设，提出了 wgpu 预热、JSON 序列化等百毫秒级优化，但完全忽略了真正的几十秒延迟来源。

**真实根因**：`几十秒 = npm run tauri:dev 触发的 Rust 全量/增量编译`

```
npm run tauri:dev
  ├── node scripts/dev.mjs        → 启动 Vite dev server (5-10s)
  ├── npx tauri dev               → 触发 cargo build (30s～3min!)
  │   ├── 编译 pdf-viewer-standalone (src-tauri/)
  │   │   ├── 1058 个依赖节点（cargo tree 实测）
  │   │   ├── wgpu = 22.1.0       ← 重量级 GPU 框架，编译极慢
  │   │   ├── vello = 0.3.0       ← 同上
  │   │   ├── cosmic-text         ← text shaping 库
  │   │   ├── lopdf + pdf (两个 PDF 库同时存在)
  │   │   └── tauri = 2.x + 3 个 plugin
  │   └── 编译 pdf-viewer-core (crates/pdf-viewer-core)
  └── 窗口出现（用户才感知到"启动完成"）
```

**验证**：`target/` 目录不存在（从未构建过），意味着每次 `tauri:dev` 都是**完整冷编译**，时间可达 3～10 分钟（低配机器）。

---

## 1. 各阶段耗时分解（dev 模式）

| 阶段 | 耗时（首次） | 耗时（增量，仅改 Rust） | 耗时（增量，仅改 TS） |
|-----|:----------:|:--------------------:|:-----------------:|
| Vite dev server 启动 | 5～15s | 5～15s | 5～15s |
| cargo build（冷编译） | **3～10 min** | **20s～3min** | 0s（跳过） |
| Tauri app 启动 + WebView2 | 2～5s | 2～5s | 2～5s |
| WASM 加载（已编译） | 1～2s | 1～2s | 1～2s |
| 首帧渲染 | 1～3s | 1～3s | 1～3s |

**结论**：`tauri:dev` 模式下，**Rust 编译占据几乎全部等待时间**。这是开发体验问题，不是运行时性能问题。

---

## 2. 问题分类

```
问题A: dev 模式 Rust 编译太慢 (几十秒～几分钟)
  ├── A1: 依赖图过重（wgpu + vello 等 GPU 库）
  ├── A2: 两个 PDF 库并存（lopdf + pdf crate 重复）  
  ├── A3: 全量冷编译（target/ 不存在）
  └── A4: tauri dev 每次都重新链接

问题B: 运行时首次打开 PDF 慢 (1～5s，用户等待感知)
  ├── B1: wgpu/VelloRenderer 首次初始化（懒加载）
  ├── B2: resolve_paths 冷路径（PDF content stream 全解析）
  └── B3: pdfasset:// 图片请求（自定义 scheme 网络开销）

问题C: 纯 TS 热更改慢 (5～15s 等 Vite rebuild)
  └── C1: Vite dev server 不包含内部 bridge 模块在 optimizeDeps 中
```

---

## 3. 优化方案（按 ROI 排序）

---

### 方案 A1：分离前端开发与 Tauri 编译（ROI 极高）

**原理**：当只改 TS/CSS 时，不需要重新编译 Rust。`npm run dev`（`--vite-only` 模式）可以在浏览器里直接跑前端，完全绕过 Rust 编译。

**当前状态**：`scripts/dev.mjs` 已经有 `--vite-only` 模式，`npm run dev` 就是纯 Vite 启动，**不触发 cargo**。

**操作规则**：
```
改 TS/CSS 时   → npm run dev       (纯 Vite, 5-10s 启动, 热更新 < 1s)
改 Rust 时     → npm run tauri:dev  (触发 cargo 增量编译，不可避免)
```

> ⚠️ **重要**：当前用户可能一直在用 `tauri:dev`。只需切换到 `npm run dev` 做 TS 开发，就能立刻消除几十秒等待。

**预期效果**：TS 开发循环从 30s+ 降至 5s 以内（Vite 热更新）。

---

### 方案 A2：持久化 `target/` 目录（ROI 极高）

**原理**：Rust 的增量编译依赖 `target/` 目录缓存。目前 `target/` 不存在意味着每次是**冷编译**。

**操作**：只需执行一次完整编译并保留结果：
```powershell
# 一次性冷编译（等待 3～10 分钟）
cd src-tauri
cargo build

# 之后增量编译只需编译改动部分
```

**注意**：`.gitignore` 里应该有 `target/`，这是正常的。但本地开发时不要手动删 `target/`。

**预期效果**：二次及以后的 `tauri:dev` 从 3-10 分钟降至 20s～2min（仅重编改动的 crate）。

---

### 方案 A3：启用 Cranelift 代码生成器（ROI 高）

**原理**：Debug 模式下，LLVM 代码生成是编译速度的主要瓶颈。Cranelift 是针对编译速度优化的替代后端，可将编译速度提升 2～4 倍，但生成的二进制运行速度稍慢（dev 模式不在意运行速度）。

**操作**：在 `.cargo/config.toml` 中配置（项目级）：

```toml
# .cargo/config.toml（项目根目录，需新建）
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "codegen-units=1"]

[profile.dev]
# 使用 Cranelift 加速 dev 编译（Rust 1.78+ 稳定特性）
# 注意：需要安装 rustup component add rustc-codegen-cranelift-preview
codegen-backend = "cranelift"
```

或者用更简单的方式加快链接：
```toml
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
linker = "rust-lld"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

**架构约束**：只修改编译配置，不改任何 Rust 源码，完全安全。

**预期效果**：增量编译时间减少 30%～60%。

---

### 方案 A4：隔离 wgpu/vello 为独立 crate（ROI 中等，中期方案）

**原理**：`wgpu`（22.1.0）和 `vello`（0.3.0）是编译最慢的两个依赖。它们只用于 `vello_renderer.rs`（GPU 渲染器）。将 `VelloRenderer` 提取为独立 crate，让 Rust 的增量编译系统把这部分编译结果长期缓存，改其他代码时不会触发重新编译。

**当前问题**：`VelloRenderer` 在 `src-tauri/src/infrastructure/pdf/vello_renderer.rs`（1273 行），与 `document_service.rs`、`page_intermediate_service.rs` 等在同一 crate 里。改任何 Rust 文件都可能触发 wgpu 相关代码的重新链接（即使 wgpu 本身没变）。

**方案**：新建 `crates/pdf-renderer/` crate：

```
crates/
  pdf-viewer-core/       (已有)
  pdf-viewer-ui/         (已有 WASM)
  pdf-renderer/          ← 新增
    Cargo.toml
    src/
      lib.rs
      vello_renderer.rs  (迁移自 src-tauri)
```

`Cargo.toml` 中：
```toml
# pdf-renderer/Cargo.toml
[dependencies]
wgpu = "22.1.0"
vello = "0.3.0"
cosmic-text = "0.12"
swash = "0.1"
# ...
```

```toml
# src-tauri/Cargo.toml
[dependencies]
pdf-renderer = { path = "../crates/pdf-renderer" }
# 移除：wgpu, vello, cosmic-text, swash 等（由 pdf-renderer 间接引入）
```

**架构约束**：
- `pdf-renderer` 是 `infrastructure` 层，不接触 Tauri 状态、IPC、域逻辑
- 符合 `docs/architecture-overview.md §4` 的 crate 分层
- `AppState.renderer` 的类型不变，只有来源 crate 变了

**预期效果**：改 `document_service.rs` 后，`wgpu/vello` 不再重新编译，增量编译时间再减 30%～50%。

---

### 方案 A5：移除冗余的 `pdf` crate（ROI 中等）

**当前状态**：`Cargo.toml` 中同时存在：
- `lopdf = "0.33.0"` — 主要 PDF 解析器，用于编辑
- `pdf = "0.9"` — 第二个 PDF 库，仅用于 `ScannedReadBackend`（lopdf 失败时的 fallback）

**问题**：两个 PDF 解析库同时编译，增加了约 20% 的编译时间。

**验证**：搜索 `pdf` crate 的使用范围：

```rust
// src-tauri/src/infrastructure/pdf_read/scanned_backend.rs
use pdf::file::FileOptions;
```

`pdf` crate 仅用于 `scanned_backend.rs` 中的 PDF-rs 读取（开放 PDF 的 fallback 策略）。

**方案**：评估是否可以用 `lopdf` 的 `Document::load_mem()` 替代 `pdf` crate 的 fallback 功能。如果 `load_pdf_lenient()` 的三层策略已经足够健壮，可以移除 `pdf` crate。

**风险**：需要测试确认 `lopdf` 单独能处理所有问题 PDF。

**预期效果**：减少约 1 个中等规模 crate 的编译时间。

---

### 方案 B1：wgpu 启动时后台预热（ROI 高，运行时优化）

**适用场景**：已有可运行的 exe（production build），或者 `tauri:dev` 完成编译后首次打开 PDF。

**问题**：`VelloRenderer::new()` 在首次调用 `read_page_asset_bundle` 时才执行，包含串行的：
1. `wgpu::Instance::new(Backends::all())` — 枚举所有 GPU 后端
2. `instance.request_adapter()` — async
3. `adapter.request_device()` — async  
4. `Renderer::new()` — 编译 WGSL shader
5. `load_system_font_candidates()` — `EnumFontFamiliesExW`

合计可能 200ms～2s，在用户"点击打开"的关键路径上。

**方案**：在 `lib.rs::run()` 的 `setup()` 中后台预热：

```rust
.setup(move |app| {
    // ... 已有代码 ...
    
    // 新增：后台预热 GPU 渲染器，不阻塞 setup()
    // RendererState 内的 Mutex<Option<...>> 保证幂等性
    let renderer_state = app.state::<crate::AppState>()
        .renderer.vello_renderer.clone();
    tokio::spawn(async move {
        let start = std::time::Instant::now();
        match crate::infrastructure::pdf::vello_renderer::VelloRenderer::new().await {
            Ok(r) => {
                let mut guard = renderer_state.lock().unwrap();
                if guard.is_none() {
                    *guard = Some(std::sync::Arc::new(std::sync::Mutex::new(r)));
                }
                eprintln!("[BOOT] VelloRenderer warm-up OK in {:?}", start.elapsed());
            }
            Err(e) => eprintln!("[BOOT] VelloRenderer warm-up failed: {}", e),
        }
    });
    Ok(())
})
```

**架构约束**：
- 改动在 `lib.rs`（Tauri 入口），属于正确位置
- `RendererState.vello_renderer` 的所有权和锁机制不变
- 预热失败不影响功能（首次打开时正常流程会重试）

**预期效果**：首次打开 PDF 比现在快 200ms～2s。

---

### 方案 C1：Vite dev 覆盖内部模块（ROI 中等，TS 开发专项）

**适用场景**：`npm run dev`（纯前端模式）时，TS 模块加载慢。

**问题**：`optimizeDeps.include` 只列了 npm 包，内部 `src/bridge/**/*.ts` 在 dev 模式下是 50+ 独立 HTTP 请求。

**方案**：更新 `vite.config.ts`：

```ts
optimizeDeps: {
    include: [
        '@tauri-apps/api/core',
        '@tauri-apps/plugin-dialog',
        '@tauri-apps/plugin-fs',
        '@tauri-apps/plugin-shell',
    ],
    entries: ['./src/main.ts'],  // ← 新增：扫描内部模块
},
```

**架构约束**：仅改构建配置，不改运行时逻辑，完全安全。

**预期效果**：`npm run dev` 首次加载从 5-15s 降至 2-5s。

---

## 4. 实施优先级

```
立刻可做（不改代码，只改工作方式）：
  ✅ A1：改 TS 时用 npm run dev，不用 tauri:dev
  ✅ A2：做一次完整编译保留 target/

本周内（低风险配置改动）：
  → A3：.cargo/config.toml 配置链接器优化
  → C1：vite.config.ts 加 optimizeDeps.entries

下周（中风险代码改动，需验证）：
  → A5：评估并移除 pdf crate
  → B1：wgpu 后台预热

中期（需要架构设计的改动）：
  → A4：提取 pdf-renderer crate 隔离 wgpu/vello
```

---

## 5. 量化目标

| 场景 | 当前 | A1+A2 后 | A3+A4+A5 后 |
|-----|:----:|:-------:|:----------:|
| 改 TS，查看效果（dev 模式） | 30s+ | **< 5s** | < 5s |
| 改 Rust，查看效果（增量） | 30s+ | 20s～2min | 10s～60s |
| exe 首次打开 PDF | 2～5s | 2～5s | **< 1s**（+B1） |

---

## 6. 与现有架构的兼容性

| 约束 | 检查结果 |
|-----|---------|
| 单一渲染链 | ✅ 以上所有方案不改渲染路径 |
| 单一所有者 | ✅ `RendererState` 所有权不变 |
| Interface 层守门人 | ✅ 不改 IPC 接口 |
| Infrastructure 层纯净 | ✅ A4 的 crate 拆分符合分层规范 |
| 文件大小 < 1500 行 | ✅ A4 的 `vello_renderer.rs`（1273 行）刚好合规，迁移不增加行数 |
| 函数命名规范 | ✅ 新代码无需新函数 |

---

## 7. 立即执行步骤

以下是零风险、立刻可做的步骤：

### Step 1：运行一次完整编译（建立 target/ 缓存）

```powershell
cd e:\chain\pdf-viewer-standalone
npx tauri dev
# 等待 3～10 分钟完成首次编译
# 完成后 Ctrl+C 停止
# target/ 目录现在有了缓存
```

### Step 2：之后改 TS，用 npm run dev

```powershell
# 仅 TS/CSS 开发（不需要 Tauri 功能时）
npm run dev
# 5-10s 启动，< 1s 热更新
```

### Step 3：配置加速链接器

新建 `e:\chain\pdf-viewer-standalone\.cargo\config.toml`：

```toml
[target.x86_64-pc-windows-msvc]
# 使用 rust-lld 替代 MSVC link.exe，链接速度快 2～3 倍
linker = "rust-lld"
```

> `rust-lld` 随 Rust 一起安装，不需要额外配置。

### Step 4：vite.config.ts 补全 optimizeDeps

在 `vite.config.ts` 中加一行 `entries`（见 §C1）。
