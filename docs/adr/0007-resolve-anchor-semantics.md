# ADR-0007: Resolve Anchor Semantics — Delete Dead Anchor Code

## Status

Accepted

## Context

The zoom architecture has two `compute_anchor_viewport_layout_result` functions:

1. **`crates/pdf-viewer-core/src/render/plan_builder.rs`** — Actually uses anchor parameters to compute `content_left`, `content_top`, and `scroll_left/right`. Called from `present/plan_builder.rs::resolve_anchor_layout_from_zoom_state`.

2. **`crates/pdf-viewer-core/src/render/zoom/animation.rs`** — Accepts all the same anchor parameters but **ignores every one of them** (prefixed with `_`): the function body always centers content via `(viewport_width - display_width).max(0.0) * 0.5`, sets `scroll_left: 0.0`, `scroll_top: 0.0`, and discards anchor state entirely.

The `pending_anchor` field in `HostZoomState` is computed by `resolve_wheel_zoom_request` (storing `anchor_page_x`, `anchor_page_y`, `viewport_x`, `viewport_y`) but is only consumed by functions that either:
- Call the `animation.rs` `compute_anchor_viewport_layout_result` (which ignores it), or
- Are themselves dead code (no callers).

### Dead code inventory

- `compute_anchor_viewport_layout_result` in `animation.rs` — all params ignored, always centers
- `compute_anchor_scroll_result` in `animation.rs` — called only by `zoom_anchor.rs` functions (all dead)
- `resolve_anchor_from_visible_preview_state` in `animation.rs` — called only by `resolve_wheel_zoom_request`
- `pending_anchor` field in `HostZoomState` — only consumed by dead functions
- `zoom_anchor.rs` module — `take_pending_anchor_scroll`, `peek_pending_anchor_scroll`, `peek_pending_anchor_layout`, `take_pending_anchor_layout`, `resolve_anchor_scroll` — **zero callers** from TypeScript or any other module
- `ZoomAnchorState` struct — only used to populate `pending_anchor`
- `AnchorScrollRequest`, `AnchorScrollResult` structs — only used by dead `zoom_anchor.rs` functions
- `AnchorViewportLayoutResult` in `plan_builder.rs` — structurally identical to `ViewportLayoutResult` but with two extra zero fields (`scroll_left`, `scroll_top`)

### Contradiction with CONTEXT.md

The glossary in `docs/CONTEXT.md` states:

> scroll_left = content_left + anchor_x - viewport_x

This formula exists only in the dead `plan_builder.rs::compute_anchor_viewport_layout_result`. The actual runtime code in `animation.rs` always centers, making `scroll_left` always 0. CONTEXT.md documents the *intended* behavior, not the *actual* behavior.

### Design decision

The actual zoom behavior is: **always center content during zoom** (matching PDF.js behavior, as noted in the `animation.rs` function comment — "Centered zoom matches PDF.js behavior and user expectation"). No cursor-anchored scroll is needed because:

1. The canvas is resized directly via SetBox (ADR-0006) — no CSS transform compensation
2. The browser's native wheel zoom + centering provides a smooth experience
3. Anchor-to-cursor scroll would require scroll mutation during gesture, which conflicts with the RAF loop's exclusive write ownership (ADR-0002)

## Decision

**Delete the dead anchor code.** The system always centers content; there is no cursor-anchored zoom. This is a deliberate architectural decision, not a bug.

### Changes

1. Delete `crates/pdf-viewer-ui/src/zoom/zoom_anchor.rs` entirely
2. Remove `zoom_anchor` module declaration from `crates/pdf-viewer-ui/src/zoom/mod.rs`
3. Remove dead re-exports from `crates/pdf-viewer-ui/src/zoom/zoom_controller.rs`
4. Replace `compute_anchor_viewport_layout_result` calls in `raf_loop.rs` with `compute_viewport_layout_result` (the centered version in `plan_builder.rs`)
5. Remove `pending_anchor` field from `HostZoomState`
6. Remove all anchor-related functions from `animation.rs`:
   - `compute_anchor_viewport_layout_result`
   - `compute_anchor_scroll_result`
   - `resolve_anchor_from_visible_preview_state`
   - `AnchorScrollRequest`, `AnchorScrollResult` structs
7. Remove dead `AnchorViewportLayoutResult` from `plan_builder.rs` (functionally identical to `ViewportLayoutResult`)
8. Simplify `present/plan_builder.rs` — remove `consume_anchor` parameter and anchor resolution path
9. Update `docs/CONTEXT.md` — remove the incorrect `scroll_left = content_left + anchor_x - viewport_x` formula

## Consequences

### Positive

1. **Eliminates 220+ lines of dead code** — `zoom_anchor.rs` (115 lines), `compute_anchor_scroll_result` (40 lines), `compute_anchor_viewport_layout_result` in animation.rs (30 lines), struct definitions, test code
2. **No API mismatch risk** — the two `compute_anchor_viewport_layout_result` functions had divergent behavior (plan_builder used anchors, animation.rs ignored them); one was dead code
3. **Clearer intent** — code that always centers says so explicitly; no more "anchor params ignored" confusion
4. **Simpler state machine** — `HostZoomState` no longer carries a `pending_anchor` field that's never meaningfully consumed

### Negative

None. No runtime behavior changes — the current code already always centers. The deleted code was never executed.
