# PDF 布局引擎标准数据契约 (V3.0 - DDD & 异构图驱动版)

## 1. 核心设计哲学
将 PDF 编辑从“改 DOM”升级为“改领域模型”。建立区域级排版引擎，实现“该流的流，不该流的坚决不流”。

### 三大硬规则
- **FlowRegion 局部流转**: 允许换行重排，但物理影响仅限本区域。
- **Fixed/Anchored 绝对锁定**: 标题、图标、页码不参与普通文本流，防止被正文挤走。
- **样式跟随 Run**: 字体、字号、颜色等属性永远跟随 `Run/Block`，不随 Region 或 Paragraph 的属性变化而丢失。

---

## 2. DDD 限界上下文 (Bounded Contexts)

| 上下文 | 职责 |
| :--- | :--- |
| **PdfIngestion** | 从原生 PDF/OCR 提取原子对象，通过 ACL 隔离原始复杂性。 |
| **LayoutContext** | 负责区域划分、行布局、对齐、制表位、锚定计算。 |
| **EditingContext** | 负责 Command 派发、撤销/重做、Run 的拆分与合并规则。 |
| **PersistenceContext** | 负责快照生成、Overlay 物化及最终写入 PDF patch。 |
| **PresentationContext** | 负责 UI 渲染投影、坐标变换、交互事件捕获。 |

---

## 3. 聚合根与领域对象 (Aggregates & Entities)

### 3.1 RegionAggregate (聚合根)
编辑与排版的边界。每个 Region 拥有独立的状态机和布局策略。

```rust
pub struct RegionAggregate {
    pub id: String,
    pub kind: RegionKind,
    pub layout_mode: LayoutMode, // FLOW | FIXED | ANCHORED
    pub bbox: BoundingBox,
    pub paragraphs: Vec<LayoutParagraph>,
}
```

### 3.2 LayoutParagraph (实体)
段落承载排版属性（对齐、缩进），但其内容由多个样式各异的 Run 组成。

```rust
pub struct LayoutParagraph {
    pub id: String,
    pub style: ParagraphStyle, // 值对象：对齐、行高、缩进
    pub runs: Vec<LayoutRun>,
}
```

### 3.3 LayoutRun (实体) & StyleToken (值对象)
Run 是样式的最小载体。

```rust
pub struct StyleToken {
    pub font_name: String,
    pub font_size: f32,
    pub color: String,
    pub is_bold: bool,
    pub is_italic: bool,
}

pub struct LayoutRun {
    pub id: String,
    pub text: String,
    pub style: StyleToken,
    pub metrics: RunMetrics, // 物理宽度、基线位置
}
```

---

## 4. 排版策略与模式 (Strategies & Patterns)

### 4.1 布局策略 (Strategy Pattern)
- **FlowLayoutStrategy**: 负责处理段落换行、两端对齐、首行缩进。
- **FixedLayoutStrategy**: 保持原位。
- **AnchoredLayoutStrategy**: 计算相对锚点的偏移（如项目符号圆点）。

### 4.2 保存规格 (Specification Pattern)
- **CanPatchNativeSpec**: 判断改动是否足够简单，允许直接修改原 PDF。
- **NeedsOverlaySpec**: 判断是否发生了换行或样式拆分，必须启用覆盖层。

### 4.3 编辑命令 (Command Pattern)
所有操作（Insert, Delete, Split, StyleChange）均为命令，支持完整的撤销/重做栈。

---

## 5. 标准字段命名总结 (The Unified Schema)
- `id`: 唯一标识符。
- `bbox`: `[left, top, right, bottom]` 包围盒。
- `origin_x / origin_y`: 物理坐标基准（基于 Baseline）。
- `visual_x / visual_y`: 渲染偏移量。
