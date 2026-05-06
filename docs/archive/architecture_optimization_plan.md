# 架构优化方案 — 职责边界重划

> 设计原则：单一职责 (SRP)、依赖倒置 (DIP)、关注点分离 (SoC)

## 一、现状诊断

### 1.1 架构拓扑

```
当前架构（职责混乱）:

┌─────────────────────────────────────────────────┐
│  TypeScript 前端 (41文件, ~350KB)                │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────┐ │
│  │ editor_host   │ │ doc_edit_api │ │ find_ctrl │ │
│  │ (47KB) 光标   │ │ (9KB) 文档   │ │ (18KB)   │ │
│  │ UTF16转换     │ │ 状态管理     │ │ 搜索算法 │ │
│  └──────────────┘ └──────────────┘ └──────────┘ │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────┐ │
│  │ vector_host   │ │ zoom_ctrl    │ │ ai_ctrl   │ │
│  │ (33KB) 渲染   │ │ (18KB) 缩放  │ │ (25KB)   │ │
│  │ 帧调度        │ │ 动画         │ │ AI面板   │ │
│  └──────────────┘ └──────────────┘ └──────────┘ │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────┐ │
│  │ review_ctrl   │ │ comment_ctrl │ │ annot_ctrl│ │
│  │ (29KB) 审阅   │ │ (11KB) 评论  │ │ (9KB)    │ │
│  └──────────────┘ └──────────────┘ └──────────┘ │
└─────────────────────────────────────────────────┘
         ↕ WASM bridge + Tauri IPC
┌─────────────────────────────────────────────────┐
│  Rust 后端 (33文件, ~300KB)                      │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────┐ │
│  │ pdf_read      │ │ pdf_write    │ │ engine    │ │
│  │ (20KB) 解析   │ │ (35KB) 写入  │ │ (27KB)   │ │
│  └──────────────┘ └──────────────┘ └──────────┘ │
└─────────────────────────────────────────────────┘
```

### 1.2 职责越界清单

| 前端文件 | 越界职责 | 应归属层 | 设计原则违反 |
|---------|---------|---------|------------|
| `editor_host.ts` (47KB) | 光标计算、UTF16↔UTF8转换、编辑会话状态机 | Rust Domain | SRP: 前端承担了编辑引擎职责 |
| `document_edit_api.ts` (9KB) | 文档状态管理、patch构建、刷新调度 | Rust Application | DIP: 高层策略依赖低层实现 |
| `pdf_find_controller.ts` (18KB) | 搜索算法、匹配结果管理 | Rust Application | SRP: UI层包含搜索业务逻辑 |
| `pdf_review_controller.ts` (29KB) | 审阅流程状态机、diff计算 | Rust Application | SoC: UI层包含审阅业务逻辑 |
| `pdf_comment_controller.ts` (11KB) | 评论CRUD、状态管理 | Rust Application | SRP: UI层包含评论业务逻辑 |
| `vector_host.ts` (33KB) | 渲染帧调度、缓存策略 | Rust Infrastructure | SoC: UI层包含渲染管线逻辑 |
| `zoom_controller.ts` (18KB) | 缩放动画插值、状态机 | Rust Infrastructure | SRP: UI层包含缩放引擎逻辑 |
| `resume_ai_controller.ts` (25KB) | AI建议管理、diff预览 | Rust Application | SRP: UI层包含AI业务逻辑 |

### 1.3 根因分析

项目从 **纯Web应用** 迁移到 **Tauri** 时，未进行职责重分配：
- 原本全部在JS/WASM中的逻辑，只把PDF解析移到Rust
- 编辑器、搜索、审阅等业务逻辑仍留在前端
- 前端变成了"胖客户端"，违反Tauri的设计哲学

## 二、目标架构

### 2.1 分层原则

```
目标架构（清晰边界）:

┌─────────────────────────────────────────────┐
│  Presentation Layer (TS) — 纯UI             │
│  职责：DOM事件 → Tauri命令 → 渲染结果        │
│  原则：不包含任何业务逻辑                     │
│                                              │
│  main.ts (事件绑定)                          │
│  ui/ (DOM操作、样式、动画触发)                │
└──────────────────┬──────────────────────────┘
                   │ Tauri IPC (序列化边界)
┌──────────────────┴──────────────────────────┐
│  Application Layer (Rust) — 业务用例         │
│  职责：编排领域对象，实现用户用例             │
│  原则：不依赖前端，不依赖基础设施细节         │
│                                              │
│  EditorUseCase (编辑会话、光标、格式)         │
│  SearchUseCase (搜索、替换)                   │
│  ReviewUseCase (审阅流程)                     │
│  CommentUseCase (评论CRUD)                    │
│  AnnotationUseCase (标注管理)                 │
│  AiUseCase (AI建议管理)                       │
└──────────────────┬──────────────────────────┘
┌──────────────────┴──────────────────────────┐
│  Domain Layer (Rust) — 核心模型              │
│  职责：纯业务规则，无I/O                     │
│                                              │
│  Document, Page, Paragraph, TextPatch        │
│  EditorSession, CaretPosition, FormatState   │
│  SearchQuery, ReviewChange, Comment          │
└──────────────────┬──────────────────────────┘
┌──────────────────┴──────────────────────────┐
│  Infrastructure Layer (Rust) — 技术实现      │
│  职责：PDF解析、文件I/O、渲染、缓存          │
│                                              │
│  PdfReadService, PdfWriteService             │
│  RenderPipeline, ZoomEngine, CacheManager    │
└─────────────────────────────────────────────┘
```

### 2.2 设计模式应用

| 模式 | 应用场景 | 解决问题 |
|------|---------|---------|
| **Facade** | `EditorUseCase` 封装编辑器复杂交互 | 前端只需调用一个方法，不需要知道光标/UTF16/会话的内部协作 |
| **Command** | Tauri IPC天然是Command模式 | 每个用户操作是一个Tauri command，序列化边界清晰 |
| **Observer** | Rust → 前端的事件推送 | 编辑状态变更通过Tauri event推送，前端只做响应式渲染 |
| **State** | 编辑器状态机移到Rust | `EditorSession` 在Rust侧管理，前端只读快照 |
| **Strategy** | 渲染策略（Canvas/Vello/Preview） | 前端选择策略，Rust执行 |
| **Repository** | PDF文档仓库 | `PdfDocumentService` 封装存储细节，Application层不感知 |

## 三、迁移计划

### Phase 1: 编辑器引擎迁移 (最高优先级)

**目标**：`editor_host.ts` (47KB) → Rust `EditorUseCase`

#### 3.1.1 当前问题

```typescript
// editor_host.ts — 前端承担了编辑引擎
function writeTextareaCaret(textarea, caretIndex) {
    // UTF16转换 — 这是编码层逻辑，不属于UI
    const converted = editorApi.charIndexToUtf16Offset(textarea.value, charIndex);
}
// 光标位置计算、编辑会话状态机、格式应用 — 全在前端
```

#### 3.1.2 目标结构

```rust
// Rust Application Layer
pub struct EditorUseCase {
    session: Option<EditorSession>,
    format_state: FormatState,
}

impl EditorUseCase {
    // 前端只需调用这些方法，不需要知道内部细节
    pub fn open_editor(&mut self, target: EditorTarget) -> EditorSnapshot;
    pub fn sync_input(&mut self, text: &str, caret: usize) -> EditorSyncResult;
    pub fn apply_format(&mut self, action: FormatAction) -> FormatState;
    pub fn commit(&mut self) -> CommitResult;
    pub fn close(&mut self) -> CloseResult;
    pub fn read_snapshot(&self) -> EditorSnapshot;
}

// 前端只做这个：
// textarea.oninput = () => invoke('sync_editor_input', { text, caret })
```

#### 3.1.3 迁移步骤

1. 在Rust `application/pdf/` 下创建 `editor_usecase.rs`
2. 将 `editor_wasm_api.ts` 中的WASM调用替换为Tauri command
3. 将 `editor_host.ts` 中的状态管理逻辑移到 `EditorUseCase`
4. 前端 `editor_host.ts` 精简为纯DOM操作（定位textarea、更新样式）
5. UTF16转换移到Rust侧（WASM模块已有此能力，只需暴露为Tauri command）

### Phase 2: 搜索/审阅/评论迁移

**目标**：3个controller (58KB) → Rust Application Layer

```rust
// application/pdf/search_usecase.rs
pub struct SearchUseCase { ... }
impl SearchUseCase {
    pub fn search(&self, query: &str, scope: SearchScope) -> SearchResult;
    pub fn replace(&mut self, query: &str, replacement: &str) -> ReplaceResult;
    pub fn navigate_next(&mut self) -> Option<SearchMatch>;
    pub fn navigate_prev(&mut self) -> Option<SearchMatch>;
}

// application/pdf/review_usecase.rs
pub struct ReviewUseCase { ... }

// application/pdf/comment_usecase.rs  
pub struct CommentUseCase { ... }
```

前端精简为：
```typescript
// 前端只做UI
async function onFindInput(query: string) {
    const result = await invoke('search_document', { query });
    renderSearchHighlights(result.matches);
}
```

### Phase 3: 渲染管线迁移

**目标**：`vector_host.ts` (33KB) + `zoom_controller.ts` (18KB) → Rust Infrastructure

```rust
// infrastructure/multimedia/pdf/render_pipeline.rs
pub struct RenderPipeline { ... }
impl RenderPipeline {
    pub fn render_frame(&mut self, plan: RenderPlan) -> RenderResult;
    pub fn update_zoom(&mut self, target: f32) -> ZoomTransition;
}
```

前端精简为：
```typescript
// 前端只接收渲染结果
listen('render-frame', (event) => {
    canvas.drawImage(event.payload.imageData);
});
```

### Phase 4: AI模块迁移

**目标**：`resume_ai_controller.ts` (25KB) → Rust Application Layer

```rust
// application/pdf/ai_usecase.rs
pub struct AiUseCase { ... }
```

## 四、迁移前后对比

### 4.1 代码量预估

| 层 | 迁移前 | 迁移后 | 变化 |
|----|-------|-------|------|
| TS前端 | ~350KB (41文件) | ~80KB (10文件) | **-77%** |
| Rust Application | ~30KB (7文件) | ~120KB (12文件) | +300% |
| Rust Domain | ~30KB (5文件) | ~50KB (8文件) | +67% |
| Rust Infrastructure | ~300KB (33文件) | ~350KB (38文件) | +17% |

### 4.2 前端文件精简

| 迁移前 (41文件) | 迁移后 (10文件) |
|----------------|----------------|
| editor_host.ts (47KB) | main.ts (事件绑定) |
| editor_host_view.ts (17KB) | ui/editor_view.ts (DOM定位) |
| editor_host_diagnostics.ts (14KB) | ui/annotation_view.ts (标注UI) |
| document_edit_api.ts (9KB) | ui/comment_view.ts (评论UI) |
| editor_wasm_api.ts (12KB) | ui/review_view.ts (审阅UI) |
| pdf_find_controller.ts (18KB) | ui/find_view.ts (搜索UI) |
| pdf_review_controller.ts (29KB) | ui/ai_view.ts (AI面板UI) |
| pdf_comment_controller.ts (11KB) | ui/zoom_view.ts (缩放UI) |
| pdf_annotation_controller.ts (9KB) | pdf_viewer_api.ts (API门面) |
| vector_host.ts (33KB) | | 
| zoom_controller.ts (18KB) | |
| ... (31 more files) | |

### 4.3 调用路径对比

**迁移前**（跨语言多跳）：
```
用户点击 → TS事件 → TS状态管理 → WASM调用 → Rust处理 → 
WASM返回 → TS状态更新 → DOM操作 → TS格式同步 → WASM调用 → ...
```

**迁移后**（单次IPC）：
```
用户点击 → TS事件 → Tauri invoke → Rust UseCase编排 → 
Tauri event推送 → TS DOM更新
```

## 五、执行优先级

| 优先级 | 任务 | 原因 |
|-------|------|------|
| **P0** | 编辑器引擎迁移 | 职责越界最严重(47KB)，跨语言调用最频繁 |
| **P1** | 搜索/审阅/评论迁移 | 业务逻辑不应在前端 |
| **P2** | 渲染管线迁移 | 性能敏感，但当前WASM方案可工作 |
| **P3** | AI模块迁移 | 独立模块，影响范围小 |

## 六、风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| Tauri IPC序列化开销 | 批量操作合并为单次invoke，使用二进制传输 |
| WASM→Tauri迁移期间功能中断 | 渐进迁移：先添加Rust command，再切换前端调用 |
| 前端动画/实时交互延迟 | 保留前端轻量状态（如缩放动画），仅将决策逻辑移到Rust |
| 编辑器光标响应延迟 | 使用Tauri event推送快照，前端只做渲染 |
