# Nutrient (PSPDFKit) Web SDK 对标（Batch 3 / 3）

> **参考**：Nutrient Web SDK `NutrientViewer.Instance` 官方 API 文档（原 PSPDFKit for Web）。资料抓取日期 2026-05-10。
> **范围**：只对照 **架构模式 / API 设计范式 / 事件模型 / 状态管理 / 可扩展性**，不追产品功能覆盖度——本项目规模 ~1/10，功能对齐非目标。

## 总览：两种截然不同的 API 哲学

| 维度 | Nutrient | 本项目 | 取向判定 |
|------|---------|-------|:---:|
| **顶层 handle 数量** | 1 个（`Instance`）~130 方法 | 10 个（Session/Manager）~206 方法 | 分域（本项目）更清晰 |
| **多文档支持** | N 个 Instance 并存 | 0（thread_local 单例） | **⚠️ 重大差距** |
| **状态更新模式** | 不可变对象 + 函数式更新 | 可变 thread_local | Nutrient 更严谨 |
| **事件模型** | DOM-style addEventListener | Rust callback（单 observer） | **⚠️ 显著差距** |
| **创建/删除** | 多态 `create()`/`delete()` | 每域独立方法 | Nutrient 更统一 |
| **保存语义** | `AutoSaveMode` 枚举 + `hasUnsavedChanges` | 每次 commit 即写 | **⚠️ 缺位** |
| **坐标变换** | 6 个显式 transform 方法 | 散落在 geometry 模块 | Nutrient 更规范 |

---

## 1. Instance vs 10-Session — API 组织哲学

### Nutrient：God Object 的平衡术

```typescript
// 单一入口
NutrientViewer.load(config).then((instance) => {
    instance.getAnnotations(0);         // 标注
    instance.setViewState(s => ...);    // 视图状态
    instance.create(annotation);        // 创建（多态）
    instance.save();                    // 保存
    instance.search("hello");           // 搜索
    instance.history.undo();            // 撤销（子对象）
    instance.addEventListener(...);     // 事件
});
```

**优势**：
- JS 学习曲线平缓：类型提示一览全部能力
- 容器化 API：`instance.xxx` 一个 namespace 管所有
- 文档组织简单：就一个类

**代价**：
- Instance 类 130+ 方法是实打实的"god object"
- TypeScript 编译时一次加载全部类型定义（代价由 SDK 吸收）
- 难以 tree-shake：不用的功能也在包里

### 本项目：Session 分域

```typescript
editor.begin(page, x, y);
editor.commit();
viewerSession.setPage(2);
zoomController.setTarget(1.5);
findSession.search("hello");
annotationManager.add(...);
historyController.undo();
```

**优势**：
- 每个 Session 独立发展（体现在 Batch 1 数据：206 个方法分散在 10 文件，平均 6.9/文件）
- 未来可独立 tree-shake（把 `find` 整个 feature-gate 掉，包体积降几百 KB）
- 边界清晰，测试易隔离

**代价**：
- 跨域编排要在 TS 侧手写装配（见 Batch 2 §5.3 的 openTextPdfFlow 问题）
- 新人需要先理解 10 个 Session 的职责边界

**判定**：**本项目的分域是对的方向**。Nutrient 的 god object 是商业 SDK 对 DX 的妥协，适用于「外部开发者」场景；本项目是应用型代码，内部域拆分收益更大。

---

## 2. 状态管理：immutable ViewState vs mutable thread_local

### Nutrient 的 ViewState 模式

```typescript
// 单一不可变对象承载所有 UI 状态
instance.viewState   // 读取（只读）

// 原子化、函数式更新
instance.setViewState(state => state.set("currentPageIndex", 2));

// 一次多字段变更 → 只触发一次 viewState.change 事件
instance.setViewState(state => state
    .set("currentPageIndex", 2)
    .set("zoom", 1.5)
    .set("sidebarMode", "ANNOTATIONS")
);
```

**三个关键设计点**：

1. **Immutable.js Record**：state 本身不可变，只能通过 `.set()` 生成新实例
2. **函数式更新接受闭包**：`(currentState) => newState` 保证读写原子
3. **事件去抖合并**：一次 setViewState 只触发一次 change，即使改了 5 个字段

### 本项目的 thread_local 模式

```rust
// 每个域一个 thread_local（Batch 2 审计：13 个）
HOST_VIEWER_SESSION.with(|c| c.borrow_mut().page = 2);
HOST_ZOOM_STATE.with(|c| c.borrow_mut().target = 1.5);
// 没有"这一组改动是一次 transaction"的语义
// 没有"订阅我所有状态改变"的入口
```

**差距**：
- ❌ 无状态快照不可变性（维护者可绕过 API 直接改字段）
- ❌ 无原子批量更新（改 page+zoom 要触发两次渲染）
- ❌ 无统一事件订阅（每个 Session 各自有 `on_xxx_change` 回调，需订阅 N 个）

### 建议：引入 `ViewerState` immutable snapshot

在 `viewer/viewer_api.rs` 增加：

```rust
#[wasm_bindgen]
impl ViewerSession {
    #[wasm_bindgen(js_name = "getState")]
    pub fn get_state(&self) -> JsValue { /* 返回 snapshot */ }

    /// 函数式更新：JS 传 updater(currentState) -> newState
    #[wasm_bindgen(js_name = "setState")]
    pub fn set_state(&self, updater: &js_sys::Function) -> JsValue {
        // 1. 读当前 snapshot
        // 2. 调 JS updater 得到 new state
        // 3. 批量应用所有字段变更
        // 4. 单次广播 "viewState.change" 事件
    }
}
```

**对应 Batch 2 行动项 §4**（给 Session 加 enum State）的升级版——不仅是状态，而是整体 state snapshot + 原子更新语义。

---

## 3. 事件模型：DOM-style vs 回调 observer

### Nutrient 的事件系统

```typescript
// 事件名 = namespaced 字符串
instance.addEventListener("viewState.change", (state) => ...);
instance.addEventListener("annotations.create", (annotation) => ...);
instance.addEventListener("document.change", () => ...);
instance.addEventListener("page.press", (event) => ...);

// DOM 风格：同一事件可多订阅
instance.addEventListener("viewState.change", handler1);
instance.addEventListener("viewState.change", handler2);

// 移除必须用同一函数引用
instance.removeEventListener("viewState.change", handler1);
```

**事件名清单**（节选，Nutrient `EventName` 枚举共 ~50 个）：
- `viewState.change` / `viewState.currentPageIndex.change` / `viewState.zoom.change`
- `annotations.create` / `annotations.update` / `annotations.delete` / `annotations.press`
- `document.change` / `document.saveStateChange`
- `history.change` / `history.undo` / `history.redo`
- `search.stateChange`
- `formFieldValues.update`

### 本项目的回调 observer

```rust
// EditorSession 目前的事件订阅方式
#[wasm_bindgen(js_name = "onStateChange")]
pub fn on_state_change(&self, callback: js_sys::Function) {
    // 注册到 thread_local 单订阅者
}
```

- ✅ 有 `on_state_change` / `on_active_block_change` 两个 editor 侧回调
- ❌ 其他 8 个 Session 几乎没有事件出口
- ❌ 单订阅者（后注册的覆盖先注册的）
- ❌ 没有命名空间化事件

### 建议：引入统一的 EventEmitter

在 `crate::events` 新建：

```rust
pub trait Event: 'static + Clone {}

pub struct EventBus {
    listeners: RefCell<HashMap<&'static str, Vec<js_sys::Function>>>,
}

impl EventBus {
    pub fn add_listener(&self, event: &str, cb: js_sys::Function);
    pub fn remove_listener(&self, event: &str, cb: js_sys::Function);
    pub fn emit(&self, event: &str, payload: JsValue);
}

// 全局单例
thread_local! {
    pub static EVENTS: EventBus = EventBus::default();
}
```

然后在所有 Session 上提供：
```rust
#[wasm_bindgen(js_name = "addEventListener")]
pub fn add_event_listener(&self, event: String, listener: js_sys::Function);
```

**估算工时**：2 天（1 天实现 + 1 天迁移所有 Session 的 on_* 回调）。

---

## 4. 保存语义：AutoSaveMode vs 同步写回

### Nutrient 的三档 AutoSaveMode

```typescript
enum AutoSaveMode {
    IMMEDIATE,      // 每次 create/update/delete 立即保存
    INTELLIGENT,    // 批量合并 + 延迟保存（默认）
    DISABLED,       // 完全手动，需调 instance.save()
}

instance.hasUnsavedChanges()    // bool
await instance.save()           // 手动保存
await instance.ensureChangesSaved() // 等待 pending 的 autosave 完成
```

**对用户的价值**：
- 长篇编辑不会反复写盘（INTELLIGENT 合并）
- 可暂存撤销栈，Ctrl+Z 跨越多次编辑不丢数据
- 关闭前一次性 flush（ensureChangesSaved）

### 本项目的同步写回

当前 EditorSession.commit() → smart_invoke("apply_region_patches") → save 是**一体化同步链**。每次编辑提交都立即写盘。

- ❌ 无 dirty-tracking（不知道有没有未保存）
- ❌ 无延迟/合并（频繁编辑 = 频繁磁盘 I/O）
- ❌ 无"关闭前确认保存"能力

### 建议

在 `DocumentSession` 增加：

```rust
#[wasm_bindgen]
impl DocumentSession {
    #[wasm_bindgen(js_name = "hasUnsavedChanges")]
    pub fn has_unsaved_changes(&self) -> bool;

    #[wasm_bindgen(js_name = "save")]
    pub async fn save(&self) -> JsValue;

    #[wasm_bindgen(js_name = "setAutoSaveMode")]
    pub fn set_auto_save_mode(&self, mode: u8); // 0=Immediate, 1=Intelligent, 2=Disabled
}
```

**优先级**：P2（非阻塞，但长编辑会话体验明显提升）。

---

## 5. 多文档支持：Instance 复数 vs 单例

### Nutrient

每次调 `NutrientViewer.load(config)` 得到独立 Instance。N 个 Instance 可同时存在（对应 N 个 tab / N 个 DOM 容器），状态完全隔离。

### 本项目

所有 Session 都基于 thread_local（WASM 单线程只有一个 thread），意味着：
- ❌ 同一页面无法打开两个 PDF（共享 VIEWER_SESSION）
- ❌ 无法做 diff view（两文档并排对比）
- ❌ Tab 模式打开多文档必须 worker-per-tab

**根本原因**：Session struct 是 0-sized unit，状态在 thread_local 全局变量里（Batch 2 §3.2）。

### 迁移路径

**方案 A（激进）**：所有状态从 thread_local 搬到 Session 的 `Box<Cell<State>>` 字段
- 破坏性大，需要所有 JS 端重新 `new Session()` 后才能用
- 但换来真正的多文档能力

**方案 B（折中）**：thread_local 从 `Cell<State>` 改为 `HashMap<DocumentId, State>`
- 每个 Session 方法额外接收 `document_id: String`
- 向后兼容（省略 id 用默认 "active" 文档）

**方案 C（按需）**：先不支持多文档，明确写进架构约束
- 文档化："本 SDK 单页面单文档"
- 未来要多文档时再做 A/B

**建议**：**先选 C + 文档化**，真有多文档需求时再上 B。上 A 成本过高不划算。

---

## 6. 多态 CRUD：create/delete vs 按域方法

### Nutrient

```typescript
instance.create(annotation1);            // 单个标注
instance.create([ann1, ann2, bookmark1]); // 混合批量
instance.delete(annotationIds);
```

- 单一入口，polymorphic
- 类型系统负责分流（TS 重载签名）

### 本项目

```typescript
annotationManager.add(highlight);
annotationManager.delete(id);
commentManager.addRegionComment(req);
// 各 Manager 独立实现 CRUD
```

- 域内自洽，域间不一致（命名 add vs addRegionComment）
- 无跨域批量（不能一次创建"1 高亮 + 1 评论"）

**判定**：本项目的分域 CRUD 更**类型安全**（TS 编译器能区分 Highlight/Comment），但缺少**批量原子性**。

**建议**：不抄 polymorphic create，只在需要批量时在 DocumentSession 加 `applyOperations(ops: Op[])`——接收异质操作列表（Nutrient 也有 `applyOperations` 方法，见 `exportPDFWithOperations`）。

---

## 7. 坐标空间变换：6 个 transform 方法

### Nutrient

```typescript
instance.transformClientToPageSpace(rect, pageIndex)
instance.transformPageToClientSpace(rect, pageIndex)
instance.transformContentClientToPageSpace(rect, pageIndex)
instance.transformContentPageToClientSpace(rect, pageIndex)
instance.transformPageToRawSpace(rect, pageIndex)
instance.transformRawToPageSpace(rect, pageIndex)
```

**三个坐标空间**：
- **Client**：浏览器 DOM 像素（鼠标事件坐标）
- **Page**：PDF 页面 CSS 像素（zoom 已应用）
- **Raw**：PDF 原始单位（72 DPI，不受 zoom 影响）

本项目有 `crates/pdf-viewer-core/src/geometry/*` 做 viewport/layer/page 的相互投影，但：
- ❌ 没暴露为统一的 WASM API（散落在各 Session 的内部函数里）
- ❌ 没有 Raw Space（始终用 "PDF 点" = 72 DPI 隐含）

### 建议

把 geometry 下的 transform 函数集合暴露为独立 `GeometryApi` WASM handle（参考 Batch 1 · API → 功能 §3.2 缺位）：

```rust
#[wasm_bindgen]
pub struct GeometryApi;

#[wasm_bindgen]
impl GeometryApi {
    pub fn client_to_page(&self, rect: JsValue, page_index: u16) -> JsValue;
    pub fn page_to_client(&self, rect: JsValue, page_index: u16) -> JsValue;
    pub fn page_to_raw(&self, rect: JsValue, page_index: u16) -> JsValue;
    pub fn raw_to_page(&self, rect: JsValue, page_index: u16) -> JsValue;
}
```

**估算工时**：4h（实现集中 + 测试）。**收益**：消除 TS bridge 里手写的坐标转换代码。

---

## 8. 其他值得借鉴的设计点（摘要）

| Nutrient API | 用途 | 本项目当前 | 借鉴优先级 |
|-------------|------|----------|:--:|
| `hasUnsavedChanges()` | Dirty tracking | ❌ 缺位 | **P1** |
| `exportInstantJSON()` / `exportXFDF()` | 标注导入导出标准格式 | ❌ 缺位 | P2 |
| `renderPageAsArrayBuffer()` / `renderPageAsImageURL()` | 程序化导出页面为图片 | ❌ 缺位 | P2 |
| `setCustomRenderers()` | 插入自定义渲染层 | ❌ 缺位 | P3 |
| `setCustomUIConfiguration()` | 自定义工具栏/UI | 部分（TS 侧） | P3 |
| `groupAnnotations()` / `setGroup()` | 标注分组 | ❌ 缺位 | P3 |
| `calculateFittingTextAnnotationBoundingBox()` | 自适应文本框尺寸 | 部分在 editor 里 | P2 |
| `beginContentEditingSession()` / `saveContentEditingSession()` | 正文编辑会话（类似我们的 EditorSession） | ✅ 已有 EditorSession | 对齐成功 ✅ |
| `compareDocuments()` | 两 PDF 差异比较 | ❌ 缺位 | P3 |
| `applyRedactions()` | 不可逆涂黑 | ❌ 缺位 | P2 |

**对齐成功 ✅**：Nutrient 的 `beginContentEditingSession` / `saveContentEditingSession` 是 2023 才加的商业功能，与本项目 EditorSession 的 begin/commit 语义几乎一致——**本项目的 editor 设计思路和行业前沿对齐**。

---

## 9. 最终建议清单

按收益/成本排序：

| # | 借鉴项 | 工时 | 收益 |
|---|-------|:---:|------|
| 1 | 引入统一 `EventBus` + `addEventListener` API | 2d | 事件模型一致化；JS 侧订阅更自然 |
| 2 | `DocumentSession.hasUnsavedChanges` + `save()` + `AutoSaveMode` | 1d | 避免频繁写盘；符合主流 SDK 预期 |
| 3 | `ViewerSession.getState` / `setState(updater)` 不可变模式 | 1d | 原子更新；合并重渲染 |
| 4 | 独立 `GeometryApi` 暴露坐标变换 | 4h | 消除 TS 手写转换 |
| 5 | 文档化"单页面单文档"约束；或实施 thread_local → HashMap 重构 | 文档 1h / 重构 3d | 清晰预期 / 多文档能力 |
| 6 | `exportInstantJSON` / `exportXFDF` 标注导入导出 | 2d | 跨工具互操作 |
| 7 | `renderPageAsImageURL` 程序化导出 | 1d | 缩略图 / 分享链路 |

**总估**：~2 周（不含方案 B 的多文档重构）可把 1-4 全做完，项目 DX 和 Nutrient 对齐 70%。

---

## 10. 三批交付物清单

| 文件 | 篇幅 | 聚焦 |
|------|----:|------|
| `docs/architecture-diagrams.md` | 471 行 | 重构后架构图册（10 图） |
| `docs/api-audit.md` | 273 行 | API 表面 + 20 Tauri / 106 WASM 死码清单 |
| `docs/structure-flow-audit.md` | 308 行 | 结构 / 流程 / 状态机 / 8 个行动项 |
| **`docs/nutrient-comparison.md`** | 本文 | Nutrient 对标 + 7 条借鉴建议 |

> **本文不改任何代码**。您可按需选：
> - 回复 **"做 Batch A"** → 触发 Batch 1 · 整目录删除
> - 回复 **"做 §X"** → 触发 Batch 2 的某个行动项
> - 回复 **"做借鉴 #X"** → 触发本文的某条 Nutrient 借鉴
