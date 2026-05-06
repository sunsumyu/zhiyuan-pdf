# Sovereignty PDF Viewer — 全量方法架构图

> 基于 methods_comparison_full.md 提取的项目自有代码（排除 
ode_modules/ 与 	arget/）。
> 总计约 **1,912+** 个方法分布在 **167+** 个源文件中。

---

## 1. 总体模块层次架构图

`mermaid
graph TB
    subgraph Frontend[Frontend (TypeScript / Vite) — 472 methods]
        F1[src/main.ts - 3]
        F2[src/core/ - 8 files, 21]
        F3[src/bridge/ - 33 files, 407]
        F4[src/bridge/ai/ - 5 files, 41]
    end

    subgraph TauriHost[Tauri Host (Rust) — 387 methods]
        I1[interfaces/multimedia/pdf.rs - 51 commands]
        A1[application/pdf/ - 6 files, 45]
        INF1[infrastructure/multimedia/pdf/ - 22 files, 263]
        INF2[infrastructure/multimedia/pdf_read/ - 4 files, 28]
    end

    subgraph WasmUI[WASM UI (Rust to WASM) — 855 methods]
        W1[wasm_api/ - 3 files, 137]
        W2[editor/ - 38 files, 412]
        W3[render/ - 16 files, 164]
        W4[document/ - 7 files, 40]
        W5[present/ - 5 files, 44]
        W6[viewer/ - 4 files, 33]
        W7[zoom/ - 7 files, 51]
        W8[host/page/bridge/ - 14 files, 21]
    end

    subgraph Kernel[PDF Kernel (Rust Core) — 198 methods]
        K1[algorithms/ - 2 files, 11]
        K2[analysis/ - 1 file, 5]
        K3[document/ - 3 files, 21]
        K4[geometry/ - 4 files, 32]
        K5[persistence/ - 4 files, 23]
        K6[render/ - 3 files, 18]
        K7[text/ - 6 files, 62]
        K8[typography/ - 3 files, 26]
    end

    F1 --> F3
    F3 --> I1
    F3 --> W1
    I1 --> A1
    I1 --> INF1
    I1 --> INF2
    A1 --> INF1
    W1 --> W2
    W1 --> W3
    W2 --> W3
    W4 --> W5
    W5 --> W3
    W6 --> W3
    W7 --> W3
    INF1 --> K7
    INF1 --> K4
    INF1 --> K8
    K7 --> K6
    K4 --> K3
    K5 --> K3
`

---

## 2. 方法分布统计

| 层级 | 模块数 | 文件数 | 方法数 |
|------|--------|--------|--------|
| Frontend (TS) | 4 | 42 | 472 |
| Tauri Host (Rust) | 4 | 33 | 387 |
| WASM UI (Rust/WASM) | 8 | 68 | 855 |
| PDF Kernel (Rust) | 8 | 24 | 198 |
| **合计** | **~24** | **~167** | **~1,912** |

---

## 3. 核心调用链路 (时序图)

`mermaid
sequenceDiagram
    participant TS as Frontend (TS)
    participant Host as Tauri Host (Rust)
    participant WASM as WASM UI (Rust/WASM)
    participant Core as PDF Kernel (Core)

    TS->>Host: invoke('open_document')
    Host->>Core: pdf_read::open
    Core-->>Host: Document handle
    Host-->>TS: Document metadata

    TS->>WASM: wasm_open_document_pipeline()
    WASM->>Core: build_page_region_context()
    Core-->>WASM: PageRegionContext

    TS->>WASM: wasm_init_page_context()
    WASM->>WASM: build_render_plan()
    WASM-->>TS: RenderToken

    TS->>WASM: wasm_resolve_render_zoom()
    WASM->>Core: resolve_pdf_font()
    Core-->>WASM: FontFace

    WASM->>WASM: render_page()
    WASM-->>TS: Canvas bitmap / instructions

    TS->>Host: invoke('commit_document_edits')
    Host->>Core: apply_text_patch()
    Core-->>Host: Save result
    Host-->>TS: Ok()
`

---

## 4. 完整方法名附录

详见 [methods_comparison_full.md](./methods_comparison_full.md) — 包含每个源文件的逐文件方法清单。
