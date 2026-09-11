# ADR-0006: 移除 CSS Transform Zoom，改用直接尺寸设置 (SetBox)

## Status

Accepted

## Context

原有的缩放系统使用 CSS transform 实现缩放：
1. **容器尺寸**: 设置为 `page × render_zoom`（滞后值）
2. **CSS transform**: `scale(display_zoom / render_zoom)` 补偿差异
3. **CSS transform string**: 由 `resolve_css_transform_string()` 计算

这导致了多个问题：
1. **双重缩放闪烁**: 程序化缩放（下拉框/自适应宽度）时，容器先以旧尺寸渲染，再被 transform 拉伸
2. **瓦片定位错误**: 瓦片除以 `cssScale` 期望容器 transform 补偿，但 CSS transform 已移除
3. **代码复杂度**: CSS transform 计算分散在多个模块（`zoom_css.rs`, `raf_transform.rs`, `pdf_layout_sync.ts`）

## Decision

完全移除 CSS transform zoom 机制，改用直接容器尺寸设置（SetBox）：

### 1. 移除 CSS transform 逻辑

**删除的代码**:
- `zoom_css.rs`: `resolve_css_transform()`, `resolve_css_transform_string()`, `resolve_settled_transform()`
- `raf_transform.rs`: 空 stub 模块
- `free_api.rs`: `resolveCssTransform`, `resolveCssTransformString` WASM 导出
- TS 类型: `WheelEventOutput.cssScale`, `CssTransformRequest` 等

### 2. 容器尺寸直接设置

**`layout.rs`** 改为:
```rust
// CSS transform removed: DOM dimensions track display_zoom directly
let dom_width = page_w * display_zoom;
let dom_height = page_h * display_zoom;
let display_width = dom_width;
let display_height = dom_height;
let css_scale = 1.0; // No transform needed
```

### 3. 瓦片定位直接使用 display 坐标

**`tile_layer.ts`** 改为:
```typescript
// No CSS transform — tile coordinates are display-space coordinates directly.
canvas.style.left = `${rect.left}px`;
canvas.style.top = `${rect.top}px`;
canvas.style.width = `${rect.width}px`;
canvas.style.height = `${rect.height}px`;
```

### 4. 模块重命名

`zoom_css.rs` → `zoom_layout.rs`，因为模块内容已不再与 CSS 相关，剩余内容为布局 fallback、fit-to-width 与渲染原因分类（评审中由暂定的 `zoom_utils.rs` 更名而来，以符合 architecture-principles.md §4 对 utils 模块的禁令）。

## Consequences

### Positive

1. **消除闪烁**: 容器尺寸直接设置为 display_zoom，无中间状态
2. **简化架构**: 移除 ~260 行死代码，减少 3 个模块
3. **提升可维护性**: 不再有 CSS transform 补偿逻辑
4. **测试简化**: 159 测试通过，无需 mock CSS transform

### Negative

1. **缩放精度**: 直接尺寸设置可能导致 subpixel 渲染差异（已验证无影响）

### Mitigations

1. **渐进测试**: 159 个单元测试覆盖所有缩放路径
2. **视觉验证**: 用户报告的闪烁问题已解决

## Alternatives Considered

1. **修复 CSS transform 补偿**: Rejected — 补偿逻辑复杂，容易出现双重缩放
2. **保留 CSS transform 但隐藏**: Rejected — 增加维护成本，无实际收益
3. **使用 WebGL transform**: Rejected — 增加复杂度，现有 canvas 足够

## Implementation

### Phase 1: 移除死代码

1. 删除 `presentation.rs` 模块
2. 清理 `zoom_css.rs` 中的死函数
3. 移除 `free_api.rs` 中的死 WASM 导出
4. 删除 `raf_transform.rs` stub 模块

### Phase 2: 修复容器尺寸

1. 修改 `layout.rs`: `dom_width/dom_height` 改用 `display_zoom`
2. 修改 `tile_layer.ts`: 移除 `/s` 除法
3. 修改 `tile_geometry.ts`: `tileElementBox()` 参数改为 `_cssScale`

### Phase 3: 模块重命名

1. `zoom_css.rs` → `zoom_utils.rs`
2. 更新所有导入引用

## References

- CONTEXT.md: Zoom State, Visual Zoom, Target Zoom, Rendered Zoom
- Grilling Session: CSS transform removal design decisions
- User Bug Report: "selecting 48% zoom causes page to double in size"
- Commit: `fix(zoom): snap visual_zoom on programmatic zoom and cull off-page tiles`
