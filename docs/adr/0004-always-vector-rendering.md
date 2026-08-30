# ADR-0004: Always-Sharp Zoom Rendering (Revised)

## Status

Accepted (Revised 2026-08-30 — supersedes the original "remove CSS scale" decision)

## Context

The zoom pipeline used `css_scale = visual_zoom / last_rendered_zoom` to stretch the
committed bitmap between renders. The user demanded: "去掉中间模糊的过渡，一直都要是高清的矢量渲染"
(no blurry transitions; always high-resolution vector rendering).

### First attempt (REVERTED — caused catastrophic regression)

We pinned the interpolation scale to 1.0 in four places:

- `presentation.rs` `anchored_translate`: `s = 1.0`
- `state.rs` `recompute_css_scale`: always 1.0
- `raf_loop.rs` wheel output: `css_scale = 1.0`
- `plan_builder.rs`: `css_scale = 1.0`

**Result: zoom became violently unstable** (observed in user video): content sizes
jumped between commits, the page rendered as two overlapping surfaces, and content
flew across the viewport.

### Root cause of the regression

`s` is not the source of blur — it is the **bridge between two coordinate spaces**:

- The bitmap lives in `layout_zoom` space (what was last rendered).
- The anchor/translate math lives in `visual_zoom` space (what the user sees).

The translate term `tx = cursor − left + scroll − anchor_page × visual_zoom` assumes
the content is sized `page × visual_zoom`. With `s = 1.0` the content stayed at
`page × layout_zoom`, so every translate displaced content by
`anchor_page × (visual_zoom − layout_zoom)` — unbounded error during animation.
Invariant I1 (visual size = page × visual_zoom for ANY layout_zoom) was broken.

Additionally, `plan_builder`'s ratio compensates the 10240px canvas-dimension clamp;
pinning it to 1.0 collapsed the canvas DOM box whenever clamping was active.

Full-page vector rendering at 60fps is infeasible: a Vello render costs O(100ms),
so "re-render every animation frame" cannot produce smooth animation today.

## Decision (Revised)

**Keep the interpolation scale as the geometry contract. Eliminate blur by making
the render track visual_zoom, not by removing the scale.**

1. **Geometry contract (restored)**: `s = visual_zoom / layout_zoom` everywhere
   (presentation, state, raf_loop, plan_builder). Invariants I1/I2 hold at every
   point of the animation.

2. **Render at visual_zoom** (kept from the original ADR): `resolve_preview_render_zoom`
   returns `visual_zoom`, so every committed frame has `layout_zoom == visual_zoom`
   and `s == 1.0` exactly — **zero stretch at rest, sharp settle**.

3. **Bounded mid-animation stretch**: the RAF reknock (threshold 6% blur, 140ms
   interval) re-renders during the gesture whenever deviation grows, so the
   interpolated stretch never exceeds a small, momentary window.

4. **Canvas-clamp compensation** (restored): `plan_builder`'s
   `display_zoom / render_zoom` ratio only activates when the bitmap exceeds the
   canvas dimension limit — it sizes the DOM box, it is not animation blur.

### Why this satisfies the requirement

- After every gesture: bitmap rendered AT the exact visual zoom → no stretching,
  native vector sharpness.
- During a gesture: deviation is bounded by the 6% reknock threshold and collapses
  to zero at each mid-animation commit.

### Path to zero mid-animation stretch

ADR-0003 tile streaming: 512×512 viewport tiles render in O(10ms) each, making
per-frame re-render at visual_zoom feasible. Once wired into the zoom path, the
interpolation window shrinks to a single frame. This is the follow-up work.

## Consequences

### Positive

1. **Stable zoom restored**: I1/I2 guarantee content stays anchored and sized.
2. **Sharp at rest**: commits render at visual_zoom; s returns to 1.0.
3. **Bounded blur**: mid-animation stretch capped by reknock threshold.

### Negative

1. **Momentary mid-gesture softness**: up to ~6% scale deviation between reknocks.
2. **Interpolation complexity remains** until tile streaming lands.

## Tests guarding the contract

- `presentation.rs::i1_visual_size_independent_of_layout_zoom` — geometry for any layout base
- `presentation.rs::i1_visual_size_continuous_across_committed_frames` — no size jumps at commits
- `presentation.rs::i2_anchor_stays_under_cursor` — anchor stability
- `presentation.rs::adr0004_no_stretch_when_layout_matches_visual` — s == 1.0 at committed zoom
- `animation.rs::tdd_css_scale_matches_visual_over_rendered` — scale formula
- `animation.rs::tdd_full_raf_lifecycle_settles_and_stops` — animation lifecycle

## References

- ADR-0002: presentation state machine (SurfaceOp, I1/I2 invariants)
- ADR-0003: tile-based rendering (the path to zero-stretch animation)
- User report 2026-08-30: "缩放非常不稳定，一直乱跳" (zoom wildly unstable) — video evidence
