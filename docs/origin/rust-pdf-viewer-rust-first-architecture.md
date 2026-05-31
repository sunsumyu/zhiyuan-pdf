# Rust-First PDF Viewer Architecture

## Goal

Move the PDF viewer toward a Rust-first interaction and rendering architecture where:

- TS owns DOM event capture and host element assignment only.
- Rust owns viewer session, zoom state, anchor geometry, frame planning, and render policy.
- High-frequency input passes small event/state payloads instead of page models.
- Preview and final render consume one shared `FramePlan`.

## Current Mixed Responsibilities

Today the viewer still mixes concerns across layers:

- `index.ts`
  - plugin bootstrap
  - host layout computation
  - viewport sizing
  - some render-policy composition
- `zoom-controller.ts`
  - wheel event capture
  - preview animation
  - anchor restore
  - render scheduling triggers
- `render-flow.ts`
  - page render orchestration
  - stale render chasing
  - host timing decisions
- `v3_exports.rs`
  - zoom state
  - partial anchor geometry
  - partial render zoom policy

This is still a dual-control system: Rust computes pieces of the truth, then TS reassembles another part of the truth.

## Target Module Boundaries

### Presentation

- `index.ts`
  - composition root only
  - dependency wiring
  - window action registration
- `viewer-dom-host.ts`
  - DOM accessors
  - canvas mount
  - scroll assignment
  - host measurements
- `host-event-adapter.ts`
  - normalize wheel / scroll / resize / pointer events
  - invoke Rust facade

### Application

- `viewer-session-adapter.ts`
  - TS facade over Rust viewer session exports
- `frame-plan-adapter.ts`
  - TS facade over Rust frame-plan exports
- `render-flow.ts`
  - invoke rendering backends using a Rust-produced `FramePlan`

### Domain in Rust

- `ViewerSession`
  - current document, page, page dimensions, zoom bounds
- `ZoomStateMachine`
  - `target_zoom`
  - `visual_zoom`
  - `last_rendered_zoom`
  - animation progression
- `AnchorLayoutSolver`
  - preserve cursor-point invariants
  - solve `content_left/top` and `scroll_left/top`
- `RenderCoordinator`
  - choose preview vs commit
  - choose full-page vs viewport tile
  - reject stale frame tokens
- `FramePlanBuilder`
  - produce one immutable `FramePlan`
- `ViewportTileRenderer`
  - derive tile rectangles
  - cooperate with cache

### Infrastructure in Rust

- `PageSceneRepository`
  - hold vector model, paint plan, image refs for current page
- `TileCache`
  - reuse rendered tiles for nearby scroll / zoom updates
- PDF/Tauri bridge
  - low-frequency document/page fetch

## Design Patterns

- `Adapter`
  - TS host event bridge and DOM host bridge
- `Facade`
  - Rust exports expose a small viewer API instead of many piecemeal helpers
- `State`
  - zoom animation and viewer interaction state
- `Strategy`
  - render policy selection such as `FullPage` vs `ViewportTile`
- `Builder`
  - build a stable `FramePlan` from session, zoom, anchor, viewport, and cache state
- `Bridge`
  - keep render plan separate from concrete canvas/Vello backends

## Event Protocol

High-frequency events should pass signal-sized payloads only.

### `WheelZoomInput`

- `delta_y`
- `viewport_x`
- `viewport_y`
- `viewport_width`
- `viewport_height`
- `scroll_left`
- `scroll_top`
- `timestamp_ms`
- `device_pixel_ratio`
- `ctrl_key`

### `ScrollChanged`

- `scroll_left`
- `scroll_top`
- `viewport_width`
- `viewport_height`

### `ViewportChanged`

- `viewport_width`
- `viewport_height`
- `device_pixel_ratio`

### `PointerMoved`

- `viewport_x`
- `viewport_y`

## Low-Frequency Protocol

These may cross the bridge with heavier state:

- `OpenDocument(path)`
- `SetPage(page_index)`
- `RefreshPageScene(page_index)`
- `ApplyDocumentPatch(...)`

## Frame Protocol

Rust should return a single `FramePlan` object for both preview and commit flows.

### `FramePlan`

- `frame_token`
- `present_mode`
  - `Preview`
  - `Commit`
- `display_zoom`
- `render_zoom`
- `css_scale`
- `host_width`
- `host_height`
- `content_left`
- `content_top`
- `scroll_left`
- `scroll_top`
- `use_viewport_tile`
- `tile_left`
- `tile_top`
- `tile_width`
- `tile_height`

TS must apply this object directly and avoid recomputing geometry.

## First Extraction Step

The first safe extraction is a Rust-owned `FramePlanBuilder`.

Why this seam first:

- it reduces future merge risk
- it removes repeated layout/render-policy recomposition from TS
- it lets preview and final render share one geometry truth
- it shrinks `index.ts` and `zoom-controller.ts` without requiring a full rewrite

## What Remains Deferred

- full Rust-owned render scheduler with frame-token cancellation
- tile cache reuse across high-frequency zoom/scroll
- automated GUI zoom probe harness
- complete migration of preview animation timing into Rust

## Dual-Engine Status

The system still has a dual-control problem today. TS still owns visual rules that should be in Rust, especially around frame assembly and preview application. The purpose of this architecture is to remove that split incrementally without regressing the working viewer.
