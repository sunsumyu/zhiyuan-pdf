# ADR-0005: TileManager 拆分为调度器 + 协调器

## Status

Accepted

## Context

TileManager 最初设计为单一模块（360 行），承担三个职责：
1. **Viewport 调度** — 计算哪些瓦片需要渲染
2. **Animation 跟踪** — 缩放动画期间的增量渲染
3. **渲染队列管理** — 优先级排序和帧令牌并发控制

随着 tile-based rendering 系统成熟，这三个职责在修改时相互干扰：
- 修改 viewport 调度逻辑时，需要理解渲染队列和缓存协调
- 动画增量渲染逻辑与 viewport 调度共享代码，但触发时机不同
- 单元测试需要 mock 缓存行为才能测试调度逻辑

## Decision

将 `TileManager` 拆分为两个模块：

### 1. `tile_scheduler.rs` — 纯调度逻辑

**职责**:
- Viewport tile 优先级计算
- Animation incremental tile 调度
- Tile grid calculation
- 不持有 TileCache 引用，不管理渲染队列

**接口**:
```rust
pub struct TileScheduler {
    viewport: ViewportState,
    animation: AnimationState,
    current_frame_token: u32,
}

impl TileScheduler {
    pub fn update_viewport(&mut self, ...);
    pub fn start_animation(&mut self, target_zoom: f32);
    pub fn update_animation(&mut self, visual_zoom: f32, frame_token: u32);
    pub fn end_animation(&mut self, frame_token: u32);
    pub fn schedule_viewport_tiles(&self) -> Vec<TileRenderRequest>;
    pub fn schedule_incremental_tiles(&self) -> Vec<TileRenderRequest>;
    pub fn should_render_incremental(&self) -> bool;
}
```

### 2. `tile_manager.rs` — 协调器

**职责**:
- 渲染队列管理（优先级排序 + 帧令牌并发控制）
- 缓存协调（委托给 TileCache）
- 委托 viewport/animation 调度给 TileScheduler

**接口变化**:
- `TileManager` 内部持有 `scheduler: TileScheduler`
- `update_viewport()` 等方法委托给 scheduler 后调用 `schedule_viewport_tiles()`
- 缓存操作（`mark_rendering`, `mark_ready`, `reset_stale_rendering`）保持不变

## Consequences

### Positive

1. **Locality**: 调度逻辑集中在 `tile_scheduler.rs`，修改调度时无需理解缓存协调
2. **Testability**: `TileScheduler` 可独立测试，无需 mock TileCache
3. **深度**: `tile_scheduler.rs` (~170 行) 是深模块，接口简单实现复杂
4. **可维护性**: `TileManager` 从 360 行精简至 ~200 行，职责清晰

### Negative

1. **模块数量**: 新增一个文件，增加导航成本
2. **委托开销**: 方法调用链增加一层（可忽略）

### Mitigations

1. **模块命名**: `tile_scheduler` 明确表达调度职责
2. **文档**: 本 ADR 记录拆分决策和职责边界

## Alternatives Considered

1. **保持单模块**: Rejected — 职责混合导致修改困难
2. **拆分为三个模块** (scheduler + animation + manager): Rejected — animation 逻辑与 scheduler 紧密耦合，拆分过度
3. **提取为 trait**: Rejected — 当前无多态需求，增加抽象成本

## Implementation

### Phase 1: 创建 tile_scheduler.rs

1. 提取 `ViewportState`, `AnimationState` 结构体
2. 提取 `schedule_viewport_tiles()`, `schedule_incremental_tiles()`
3. 提取 `update_viewport()`, `start_animation()`, `update_animation()`, `end_animation()`

### Phase 2: 重构 tile_manager.rs

1. `TileManager` 持有 `scheduler: TileScheduler` 字段
2. 方法委托给 scheduler 后调用缓存协调逻辑
3. 移除重复的 viewport/animation 字段

### Phase 3: 更新模块导出

1. `render/mod.rs` 添加 `pub mod tile_scheduler`
2. 验证所有测试通过

## References

- ADR-0003: Tile-Based Rendering Architecture
- Commit: `feat(tile-core): tile-based rendering system with manager scheduler and LRU cache`
- Grilling Session: TileManager split design decisions
