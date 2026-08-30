# ADR-0003: Tile-Based Rendering Architecture

## Status

Accepted

## Context

The current PDF rendering system uses a base+detail layer approach:
- Base layer: Full page raster at rendered zoom level
- Detail layer: Incrementally refined tiles for high-frequency content

This approach has several issues:
1. **CSS Stretching Blur**: `cssScale = visualZoom / lastRenderedZoom` causes blur during zoom animations
2. **Full Page Re-render**: Changing zoom level requires re-rendering the entire base layer
3. **Resolution Limitation**: Canvas size capped at 10240 pixels limits maximum zoom level

The user explicitly stated: "不要用css这种方式，css这种拉伸方式不可能有高分辨率" (don't use CSS approach, CSS stretching cannot produce high-resolution).

## Decision

Replace the base+detail layer architecture with a pure tile-based rendering system:

### Core Design

1. **Fixed Tile Size**: 512×512 logical pixels per tile
2. **Independent TileCache**: New LRU cache separate from existing FrameCache
3. **Priority Rendering**: Render viewport tiles first, then surrounding tiles
4. **DPR-Aware Rendering**: Render tiles at device pixel ratio for high-resolution output

### Rendering Strategy

- **Zoom Animation**: Incremental rendering of current visualZoom tiles (every N frames)
- **Post-Animation**: Full viewport tile rendering at target zoom level
- **Scroll/Pan**: Render only newly visible tiles, reuse cached tiles

### Cache Management

- **Page Switch**: Clear all tile caches
- **Zoom Change**: Mark tiles as eligible for LRU eviction
- **LRU Eviction**: Automatic cleanup based on memory pressure

### Integration

- **Editor Overlay**: Rendered on separate canvas above tile layer (existing mechanism)
- **FrameToken**: Reused for optimistic concurrency control
- **SurfaceOp**: Tile layer becomes the primary presentation surface

## Consequences

### Positive

1. **High-Resolution Output**: No CSS stretching, tiles rendered at native resolution
2. **Progressive Quality**: Viewport tiles render first, surrounding tiles render later
3. **Memory Efficiency**: LRU cache evicts unused tiles automatically
4. **Zoom Performance**: No full page re-render on zoom change

### Negative

1. **Complexity**: New TileManager and TileCache modules increase codebase size
2. **Tile Boundaries**: Potential visible seams between tiles (mitigated by overlap or anti-aliasing)
3. **Memory Usage**: More tiles may consume more memory than single base layer

### Mitigations

1. **Tile Overlap**: Render 1px overlap between tiles to prevent seams
2. **Memory Budget**: Configurable maximum tile cache size
3. **Progressive Fallback**: Fall back to base+detail if tile rendering fails

## Alternatives Considered

1. **Base+Detail with Improved CSS**: Rejected - CSS stretching cannot produce high-resolution
2. **WebGL Tiling**: Rejected - adds complexity, existing canvas approach sufficient
3. **Vector Tiles**: Rejected - PDF content is raster-heavy, vector tiles not beneficial

## Implementation Plan

### Phase 1: Core Infrastructure

1. Create `TileCache` module (独立 LRU 缓存)
2. Create `TileManager` module (调度器)
3. Define `TileKey`, `TileState` data structures

### Phase 2: Rendering Integration

1. Integrate with existing FrameToken mechanism
2. Add viewport tile priority rendering
3. Implement DPR-aware tile rendering

### Phase 3: Cache Management

1. Implement page switch cache clearing
2. Implement zoom change eviction marking
3. Add memory budget configuration

### Phase 4: Remove Old Architecture

1. Remove base layer rendering logic
2. Remove detail layer rendering logic
3. Update VisibleSurface to use tile layer

## References

- CONTEXT.md: Tile Cache, Tile Manager, Tile Key, Tile State definitions
- Grilling Session: Design decisions Q1-Q15
- User Requirement: High-resolution zoom without CSS stretching