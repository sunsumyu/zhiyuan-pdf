# 坐标系统框架级重构设计方案

## 一、问题诊断

### 当前架构的核心债务

经过深度分析，当前坐标系统存在以下根本问题：

**1. `char_origins` 语义混乱**

同一个字段名在不同结构体中有不同的参考原点：

| 结构体 | 参考原点 | 含义 |
|--------|---------|------|
| `StyledRun.char_origins` | `run.tx` | 相对于 run 起点 |
| `StyleRunSnapshot.char_origins` | `line_left` | 相对于行起点 |
| `LayoutRun.char_origins` | `origin_x` | 相对于 run 起点 |
| `NativeTextModel.char_origins` | 无 | 绝对页面坐标 |

阅读代码时极易混淆，debug 时难以还原 glyph 的实际页面位置。

**2. 坐标变换链路过长**

```
PDF绝对位置 → 行内偏移 → split归零 → resolve归零 → 绘制还原
     4次变换，每一步都可能引入误差
```

**3. 冗余字段泛滥**

- `LayoutRun.bbox` vs 从 `origin_x/origin_y + glyphs` 推导
- `LayoutParagraph.origin_x/origin_y` vs `anchor_bbox.left/top`
- `width` vs 从 `glyphs.last() - origin_x` 推导

三处 bbox 计算公式不同，`compute_run_bbox` 甚至不信任存储的 `bbox`。

**4. split 操作散落三处**

- `list_item_region_builder.rs:split_runs_by_body_start`
- `document_plan.rs:split_run_at_char_index`
- `draft_style.rs` 中的类似逻辑

三者对 `origin_x`、`bbox` 的处理不一致，是潜在 bug 源。

## 二、设计原则

### 核心原则

1. **单一数据源**：绝对坐标只在解析层设置，后续不再修改
2. **最小状态**：能推导的就不存储（bbox、width）
3. **一次变换**：只在绘制层做视口变换，中间层不做坐标变换
4. **显式类型**：用 Rust 类型系统区分坐标空间
5. **切割零变换**：split 操作直接切割数组，不重新计算偏移

### 设计模式应用

| 模式 | 是否使用 | 原因 |
|-----|---------|------|
| NewType | ✅ 使用 | `PageCoord` / `ViewportCoord` 区分坐标空间，防止混淆 |
| Method替代Field | ✅ 使用 | `compute_bbox()` 方法替代 `bbox` 字段，消除冗余 |
| Builder | ❌ 不使用 | 数据结构简单，直接构造即可 |
| Strategy | ❌ 不使用 | 绘制逻辑固定，无多策略需求 |
| Factory | ❌ 不使用 | 转换逻辑简单，直接调用 `from_styled` |

**关键决策**：不为用模式而用模式。NewType 和 Method替代Field 是真正能简化代码、增强健壮性的模式。

## 三、新数据结构设计

### 3.1 GlyphPosition - 绝对坐标的基本单元

```rust
/// 单个 glyph 的位置信息
/// 所有坐标都是绝对页面坐标（Y-Down 体系）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlyphPosition {
    /// glyph 原点的绝对页面 X 坐标
    pub x: f32,
    /// glyph 的物理宽度
    pub width: f32,
}

impl GlyphPosition {
    /// glyph 的右边界（绝对坐标）
    pub fn right(&self) -> f32 {
        self.x + self.width
    }
}
```

**设计要点**：
- `x` 是绝对坐标，不是相对于 run 起点
- `width` 是 glyph 物理宽度，用于排版计算
- `right()` 是衍生方法，不存储

### 3.2 TextRun - 使用绝对坐标的文本 run

```rust
/// 文本 run，所有坐标都是绝对页面坐标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRun {
    pub id: String,
    pub text: String,
    /// run 起始 glyph 的 X 坐标（绝对页面坐标）
    pub origin_x: f32,
    /// run 基线的 Y 坐标（绝对页面坐标）
    pub baseline_y: f32,
    /// 每个 glyph 的绝对位置
    pub glyphs: Vec<GlyphPosition>,
    /// 样式
    pub style: TextStyle,
    /// 来源 PDF object IDs
    pub object_ids: Vec<String>,
}

impl TextRun {
    /// 计算 run 的包围盒（不存储，按需计算）
    pub fn compute_bbox(&self) -> BoundingBox {
        if self.glyphs.is_empty() {
            return BoundingBox::default();
        }
        let left = self.origin_x;
        let right = self.glyphs.iter().map(|g| g.right()).max().unwrap_or(left);
        let top = self.baseline_y - self.style.font_size;
        let bottom = self.baseline_y;
        BoundingBox { left, top, right, bottom }
    }
    
    /// run 的物理宽度（从 glyphs 推导）
    pub fn physical_width(&self) -> f32 {
        if self.glyphs.is_empty() {
            return 0.0;
        }
        self.glyphs.iter().map(|g| g.right()).max().unwrap_or(self.origin_x) - self.origin_x
    }
    
    /// 获取第 i 个 glyph 的绝对 X 坐标
    pub fn glyph_x(&self, index: usize) -> Option<f32> {
        self.glyphs.get(index).map(|g| g.x)
    }
}
```

**移除的字段**：
- `bbox` → 改为 `compute_bbox()` 方法
- `width` → 改为 `physical_width()` 方法
- `char_origins` → 改名为 `glyphs`，语义更清晰
- `char_widths` → 合并到 `GlyphPosition.width`

**新增的方法**：
- `compute_bbox()` - 单一的 bbox 计算逻辑
- `physical_width()` - 从 glyphs 推导宽度
- `glyph_x()` - 直接获取绝对坐标，无需 `origin_x + char_origins[i]`

### 3.3 TextParagraph - 段落结构

```rust
/// 文本段落
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextParagraph {
    pub id: String,
    pub runs: Vec<TextRun>,
}

impl TextParagraph {
    /// 计算段落的包围盒（所有 run 的并集）
    pub fn compute_bbox(&self) -> BoundingBox {
        self.runs.iter()
            .filter(|run| !run.text.is_empty())
            .map(|run| run.compute_bbox())
            .reduce(|acc, bbox| BoundingBox {
                left: acc.left.min(bbox.left),
                top: acc.top.min(bbox.top),
                right: acc.right.max(bbox.right),
                bottom: acc.bottom.max(bbox.bottom),
            })
            .unwrap_or_default()
    }
}
```

**移除的字段**：
- `bbox` → 改为方法
- `origin_x/origin_y` → 删除，与 `anchor_bbox.left/top` 重复
- `wrap_width` → 删除，可从 bbox 推导

### 3.4 EditorSession - 编辑器会话

```rust
/// 编辑器会话上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSession {
    /// 编辑区域的绝对页面坐标
    pub anchor_bbox: BoundingBox,
    /// 段落数据
    pub paragraph: TextParagraph,
}

impl EditorSession {
    /// 计算相对于 anchor 的 glyph 位置（仅用于 caret 计算）
    pub fn glyph_local_x(&self, run_index: usize, glyph_index: usize) -> Option<f32> {
        let run = self.paragraph.runs.get(run_index)?;
        let glyph = run.glyphs.get(glyph_index)?;
        Some(glyph.x - self.anchor_bbox.left)
    }
}
```

**设计要点**：
- `glyph_local_x()` 只在编辑器内部使用，用于 caret 定位
- 外部渲染使用绝对坐标，不做此变换

## 四、坐标空间显式类型

### 4.1 PageCoord / ViewportCoord

```rust
/// 页面坐标（绝对，Y-Down）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageCoord(pub f32);

/// 视口坐标（相对于 canvas）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportCoord(pub f32);

/// 视口偏移
#[derive(Debug, Clone, Copy)]
pub struct ViewportOffset {
    pub x: f32,
    pub y: f32,
}

impl PageCoord {
    /// 转换为视口坐标（唯一的一次变换）
    pub fn to_viewport(self, offset: f32, scale: f32) -> ViewportCoord {
        ViewportCoord((self.0 + offset) * scale)
    }
}
```

**设计要点**：
- 编译器层面防止混淆：`PageCoord` 不能直接赋给 `ViewportCoord`
- 变换只在 `to_viewport()` 一处发生
- Rust 的 newtype 模式，零成本抽象

## 五、关键算法流程

### 5.1 Split 操作 - 切割零变换

```rust
/// 在字符边界分割 TextRun
/// 返回 (左侧部分, 右侧部分)，可能为 None
pub fn split_run_at(run: &TextRun, char_index: usize) -> (Option<TextRun>, Option<TextRun>) {
    let glyph_count = run.glyphs.len();
    
    if char_index == 0 {
        return (None, Some(run.clone()));
    }
    if char_index >= glyph_count {
        return (Some(run.clone()), None);
    }
    
    // 左侧部分
    let left_text: String = run.text.chars().take(char_index).collect();
    let left_glyphs = run.glyphs[..char_index].to_vec();
    let left_run = if left_glyphs.is_empty() {
        None
    } else {
        Some(TextRun {
            id: format!("{}::split::{}", run.id, char_index),
            text: left_text,
            origin_x: run.origin_x,  // 保持原起点
            baseline_y: run.baseline_y,
            glyphs: left_glyphs,  // 直接切割，零变换
            style: run.style.clone(),
            object_ids: run.object_ids.clone(),
        })
    };
    
    // 右侧部分
    let right_text: String = run.text.chars().skip(char_index).collect();
    let right_glyphs = run.glyphs[char_index..].to_vec();
    let right_run = if right_glyphs.is_empty() {
        None
    } else {
        let right_origin_x = right_glyphs[0].x;  // 直接取第一个 glyph 的绝对坐标
        Some(TextRun {
            id: format!("{}::split::{}", run.id, char_index),
            text: right_text,
            origin_x: right_origin_x,  // 新起点
            baseline_y: run.baseline_y,
            glyphs: right_glyphs,  // 直接切割，零变换
            style: run.style.clone(),
            object_ids: run.object_ids.clone(),
        })
    };
    
    (left_run, right_run)
}
```

**关键变化**：
- glyphs 数组直接切割，无需 `value - first_origin` 的归一化
- origin_x 直接取第一个 glyph 的绝对坐标
- 一处实现替代三处散落的逻辑

### 5.2 绘制流程 - 一次变换

```rust
/// 绘制段落到 canvas
pub fn draw_paragraph(
    ctx: &mut CanvasContext,
    paragraph: &TextParagraph,
    viewport_offset: ViewportOffset,
    scale: f32,
) {
    // 1. 一次性视口变换
    ctx.translate(viewport_offset.x, viewport_offset.y);
    ctx.scale(scale, scale);
    
    // 2. 每个 glyph 直接使用绝对坐标
    for run in &paragraph.runs {
        for (char, glyph) in run.text.chars().zip(run.glyphs.iter()) {
            ctx.draw_glyph(char, glyph.x, run.baseline_y, &run.style);
        }
    }
}
```

**对比原有流程**：

| 原有 | 新设计 |
|-----|-------|
| `resolve_run_layout` 归零 | 删除 |
| `line_left + first_origin` 还原 | 删除 |
| `ctx.translate(origin_x) + char_origins[i]` | `ctx.translate(viewport_offset) + glyph.x` |

## 六、迁移计划

### Phase 1: 数据结构定义（1天）

1. 定义 `GlyphPosition`、`TextRun`、`TextParagraph` 新结构
2. 保留旧结构，两者并存
3. 编写单元测试验证新结构

### Phase 2: 解析层适配（2天）

1. `StyledRun::to_text_run()` 转换方法
2. `NativeTextModel::to_text_paragraph()` 转换方法
3. 验证 Y-Down 翻转正确性

### Phase 3: 渲染层迁移（3天）

1. 替换 `GlyphPaintRun` 为 `TextRun`
2. 替换 `build_paint_run` 为直接使用 `TextRun`
3. 简化 `resolve_run_layout` 或删除

### Phase 4: Split 操作统一（2天）

1. 删除 `split_runs_by_body_start`
2. 删除 `split_run_at_char_index`
3. 统一使用 `split_run_at`
4. 修复 marker/body 分割逻辑

### Phase 5: 编辑器迁移（3天）

1. 替换 `LayoutRun` 为 `TextRun`
2. 简化 `compute_run_bbox` 为 `TextRun::compute_bbox`
3. 简化 caret 计算逻辑

### Phase 6: 清理旧代码（2天）

1. 删除旧结构定义
2. 删除冗余字段
3. 删除冗余方法
4. 全量测试

**总时长：约 12-15 天**

## 七、风险评估

| 风险 | 影响 | 缓解措施 |
|-----|------|---------|
| 现有功能回归 | 高 | 每个 Phase 完成后全量测试 |
| 数据迁移遗漏 | 中 | 新旧结构并存期间，双路径验证 |
| 绘制位置偏差 | 高 | 可视化对比测试（截图 diff） |
| 编辑器 caret 定位错误 | 中 | 单元测试覆盖 caret 边界场景 |

## 八、预期收益

1. **代码量减少约 30%**：删除冗余字段和方法
2. **Bug 减少**：单一 bbox 计算逻辑，统一 split 实现
3. **可维护性提升**：坐标语义清晰，新人易于理解
4. **扩展性增强**：新坐标空间只需添加 newtype，不改动现有代码

## 九、命名改进记录

### 介词禁用规则

方法名中**禁止使用介词**（`at`、`by`、`with`、`from`、`into`），除非是 Rust 标准库已有惯例且语义精确。

理由：
1. 介词让方法名变长但不增加信息量——参数名已说明"在哪/用什么"
2. 主流框架极少用介词命名（Rust std、Swift、Go、Java 均无此惯例）
3. 介词暗示的语义（位置、方式、来源）应由参数类型承载

Rust std 仅有的介词例外：
- `slice.split_at(n)` —— 区分 `split_off`，编码所有权语义
- `Vec.split_off(n)` —— 表达"分离并转移所有权"

### 本次重构中的命名变更

| 旧名 | 新名 | 理由 |
|------|------|------|
| `split_runs_by_body_start` | `split_runs_at` | 对齐 Rust std 的 split_at |
| `split_run_at_char_index` | `split_run` | 简洁，无冗余介词 |
| `layout_run_from_glyph_paint` | `GlyphPaintRun::to_text_run()` | 构造方法归入 impl 块 |

### 正反例对照

```rust
// ❌ 介词冗余
split_run_at_char_index(run, char_index)
find_node_with_id(id)
create_session_from_config(config)

// ✅ 去掉介词
split_run(run, index)
find_node(id)
impl From<Config> for Session
```

### Rust std 命名参考

| 方法 | 介词 | 语义 |
|------|------|------|
| `slice.split_at(n)` | `at` | 借用分割，区分所有权 |
| `Vec.split_off(n)` | `off` | 所有权转移 |
| `Option.ok_or(err)` | `or` | 回退值 |
| `Result.unwrap_or(default)` | `or` | 回退值 |

只有 `_at` 和 `_off` 用于区分所有权语义，`_or` 用于回退值。其他场景不用介词。

## 十、重构完成状态

### 已完成

| Phase | 内容 | 状态 |
|-------|------|------|
| Phase 1 | 数据结构定义 | ✅ 完成 |
| Phase 2 | 解析层适配 | ✅ 完成 |
| Phase 3 | 渲染层迁移 | ✅ 完成 |
| Phase 4 | Split 操作统一 | ✅ 完成 |
| Phase 5 | 编辑器迁移 | ✅ 完成 |
| Phase 6 | 清理旧代码 | ✅ 完成 |

### 新数据结构

- `GlyphPosition` - 绝对坐标的 glyph 位置
- `TextRun` - 使用绝对坐标的文本 run
- `TextParagraph` - 段落结构
- `EditorSession` - 编辑器会话

### 转换方法

- `StyledRun → TextRun` via `TextRun::from_styled()`
- `GlyphPaintRun → TextRun` via `GlyphPaintRun::to_text_run()`
- `LayoutRun → TextRun` via `LayoutRun::to_text_run()`
- `TextRun → LayoutRun` via `TextRun::to_layout_run()`
- `LayoutParagraph → TextParagraph` via `LayoutParagraph::to_text_paragraph()`
- `ParagraphEditContext → EditorSession` via `ParagraphEditContext::to_editor_session()`

### 核心改进

1. **绝对坐标贯穿**：glyphs 数组直接存储绝对坐标，无需 origin_x + char_origins[i] 计算
2. **单一 bbox 计算**：`TextRun::compute_bbox()` 是唯一的 bbox 计算实现
3. **split 零变换**：直接切割 glyphs 数组，无需归零-还原循环
4. **渲染管线简化**：`resolve_run_layout` 直接输出绝对坐标