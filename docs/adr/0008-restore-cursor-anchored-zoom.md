# ADR-0008: Restore Cursor-Anchored Zoom

## Status

Accepted

## Supersedes

ADR-0007 (zoom always centers content). ADR-0007 is preserved as a historical record
of the decision and the dead-code cleanup that accompanied it; its "always centers"
decision is no longer followed.

## Context

ADR-0007 deleted the cursor-anchored zoom code on three grounds:

1. The canvas is resized directly via SetBox — no CSS transform compensation.
2. The browser's native wheel zoom + centering provides a smooth experience.
3. Anchor-to-cursor scroll would require scroll mutation during gesture, which
   conflicts with the RAF loop's exclusive write ownership (ADR-0002).

Reason 2 has been superseded by the smooth-canvas-scale layer (raf_loop
`apply_canvas_visual_scale`) that now provides continuous visual feedback: the
centered layout produces an instantaneous jump at gesture start that the smooth
layer then scales, making the initial jump more visible, not less. Reason 3 is
partially superseded: the anchor is now a single container-offset write on the
wheel event (not continuous scroll mutation during the gesture), and the RAF
loop remains the only writer between wheel events.

The visual problem: zooming out by ~50% on a document that fits the viewport
caused the page to jump horizontally and vertically by ~30 px at the first wheel
event — the centered offset for the new zoom differs from the old one by
`(display_new − display_old)/2`. Zoom-in has the same effect in the opposite
direction. The cursor-anchored formula eliminates this.

## Decision

Restore cursor-anchored zoom using a compact formula, implemented in Rust:

```
cursor_page_x = (cursor_viewport - old_content_left) / old_zoom
new_content_left = cursor_viewport - cursor_page_x * new_zoom
clamped to [0, display_width - viewport_width]
```

- When the page is smaller than the viewport, the formula falls back to
  centered offset (same as the ADR-0007 behavior for fits-in-viewport).
- When there is no prior `visual_layout` (first wheel event of the session),
  the formula falls back to centered offset.
- The cursor page point is clamped to `[0, page_width]` to keep clicks in the
  margin from producing out-of-range offsets.

The wheel path (`on_wheel_event`) now stores the anchored offset in
`HostZoomState.visual_layout`, and `build_frame_plan_result` reads it back when
building the committed frame at settle — the frame geometry matches what the
wheel event put on screen.

## Changes

- `crates/pdf-viewer-core/src/render/zoom/animation.rs`:
  - `WheelZoomResult` gains `anchor_content_left/top` and
    `anchor_scroll_left/top`.
  - New `anchor_content_offset` (content offset only) and `anchor_layout`
    (content offset + scroll compensation) helpers.
  - `resolve_wheel_zoom_request` reads the prior `visual_layout` and produces
    anchored offsets (falls back to `centered_offset`).
- `crates/pdf-viewer-ui/src/zoom/raf_loop.rs`:
  - `on_wheel_event` writes `result.anchor_content_left/top` into `visual_layout`
    and the DOM, falling back to centered for the first gesture.
  - It also assigns the scroller's `scrollLeft`/`scrollTop` from
    `result.anchor_scroll_left/top`, so the cursor's page point is preserved
    when `content_left` is clamped (page overflowing the viewport).
- `crates/pdf-viewer-ui/src/present/plan_builder.rs`:
  - `build_frame_plan_result` uses `zoom_state.visual_layout` content offsets
    when `display_zoom` matches the plan's render `display_zoom`, so the
    committed frame at settle writes exactly the anchor geometry.
- `src/index.css`: `#pdf-scroll-container` changed from `display:flex;
  justify-content:center` to `display:block` — flex centering of overflowing
  content makes the overflow portion unreachable by scroll, defeating the
  anchor scroll compensation. Small-page centering is handled by the
  container's explicit `left` offset.
- `docs/CONTEXT.md`: anchor entry updated with the restored formula.
- `docs/adr/0007-resolve-anchor-semantics.md`: kept as-is; this ADR supersedes
  the "always centers" decision documented there.

## Consequences

### Positive

- Zoom gestures no longer jump the content horizontally or vertically at the
  first wheel event. The cursor's page point stays fixed across the zoom
  change, matching user expectation (PDF.js, Adobe Reader, SumatraPDF).
- The committed frame at settle preserves the same geometry the wheel event
  wrote, so the page does not shift when the settle frame applies.
- When the page overflows the viewport (zoom > fit-width), the anchor is
  preserved via `scrollLeft`/`scrollTop` adjustment — without this, the
  cursor's page point would drift by whatever `content_left` clamping ate.
- `#pdf-scroll-container` is now `display:block` instead of `display:flex;
  justify-content:center`. Flex centering of an overflowing child pushes it
  symmetrically off-screen and makes the left/top portion unreachable by
  scroll; the new layout uses the container's explicit `left`/`top` offset
  for centering (still correct when display < viewport) and keeps overflow
  fully reachable.

### Negative

- `HostZoomState.visual_layout` now carries the cursor-anchored offset;
  callers that read it must accept that it is not necessarily centered. The
  `plan_builder` branch handles this by only using it when `display_zoom`
  matches; non-matching callers still compute the centered offset.
- The `syncHostLayout` path (resize handling) still writes the centered offset
  into `visual_layout`, so a resize during a zoom gesture would reset the
  anchor to centered. That's the current behavior and matches the prior
  ADR-0007 state; a follow-up can preserve the anchor across resizes if
  needed.
