# PDF Viewer Zoom & Layout Architecture Refactor Design

## Background & Problem Statement

During page zoom operations (dropdown selection, zoom in/out buttons, and Ctrl + Mouse Wheel), the viewer experiences severe visual flickering ("flashing a larger version first, then snapping back to normal").

### Root Cause Analysis

1. **Double-Scaling Explosion**:
   - `syncLayoutBox` on TS side immediately updates the container DOM width to target `displayZoom` ($W_{\text{page}} \cdot Z_{\text{display}}$).
   - Concurrently, `applyVisualZoomPreview` / `applyRenderPlan` applies CSS `transform: scale(cssScale)` where $\text{cssScale} = Z_{\text{display}} / Z_{\text{rendered}}$.
   - Resulting visual width during zoom preview:
     $$\text{Visual Width} = (W_{\text{page}} \cdot Z_{\text{display}}) \cdot \frac{Z_{\text{display}}}{Z_{\text{rendered}}} = W_{\text{page}} \cdot \frac{Z_{\text{display}}^2}{Z_{\text{rendered}}}$$
     For $Z_{\text{rendered}} = 1.0$ and $Z_{\text{display}} = 1.25$, visual width becomes $W_{\text{page}} \cdot 1.5625$ (1.5625x size, flashing bigger).
   - Once background re-rendering completes, $Z_{\text{rendered}}$ updates to $1.25$, $\text{cssScale}$ becomes $1.0$, and visual width snaps back to $W_{\text{page}} \cdot 1.25$ (1.25x size, flashing back).

2. **Split Responsibility & State Mismatch**:
   - TS bridge tried to infer whether layout boxes should use `displayZoom` vs `baseZoom`.
   - Rust layout engine calculated `display_width` in `sync_host_layout` without accounting for current rendered scale.

---

## Architectural Goals

1. **Single Source of Truth**: Move all layout box sizing decisions into the Rust state engine (`HostZoomState` & `FramePlanResult`).
2. **Mathematical Cancellation Guarantee**: Ensure container DOM width is strictly tied to $Z_{\text{rendered}}$ during preview transitions, so CSS `scale(Z_{\text{display}} / Z_{\text{rendered}})` cancels out perfectly:
   $$\text{Visual Width} = (W_{\text{page}} \cdot Z_{\text{rendered}}) \cdot \frac{Z_{\text{display}}}{Z_{\text{rendered}}} = W_{\text{page}} \cdot Z_{\text{display}}$$
3. **Declarative TS View Layer**: TS Bridge (`pdf_layout_sync.ts` & `zoom_controller.ts`) becomes a pure declarative renderer that directly applies DOM styles dictated by Rust.

---

## Detailed Technical Changes

### 1. Rust Core & UI Contract Updates

#### A. `crates/pdf-viewer-ui/src/platform/layout.rs`
- Update `SyncHostLayoutRequest` to accept `rendered_zoom: Option<f32>`.
- Update `SyncHostLayoutResult` to return:
  - `dom_width`: Base DOM width (`page_width * render_zoom`).
  - `dom_height`: Base DOM height (`page_height * render_zoom`).
  - `display_width`: Final target display width (`page_width * display_zoom`).
  - `display_height`: Final target display height (`page_height * display_zoom`).
  - `css_scale`: Scale factor (`display_zoom / render_zoom`).
  - `content_left` / `content_top`: Layout offsets calculated against `dom_width` / `dom_height`.

#### B. `crates/pdf-viewer-core/src/render/plan_builder.rs` & `zoom_interaction.rs`
- Ensure `build_frame_plan_result` and `build_zoom_preview_frame` populate layout bounds matching the active `rendered_base_zoom`.

---

### 2. TS Bridge Updates

#### A. `src/bridge/viewer/pdf_layout_sync.ts` (`syncLayoutBox`)
- Receive `domWidth`, `domHeight`, `contentLeft`, `contentTop`, `cssScale` from Rust.
- Set `container.style.width = ${domWidth}px`.
- Set `container.style.height = `${domHeight}px`.
- Set `container.style.transform = cssScale === 1.0 ? '' : scale(${cssScale})`.

#### B. `src/bridge/zoom/zoom_controller.ts`
- Simplify `applyVisualZoomPreview`, `applyPreviewFrame`, and `applyCommittedFrame`.
- Eliminate duplicate DOM scale calculations in TS.

#### C. `src/bridge/viewer/viewer_geometry_probe.ts`
- Align `applyRenderPlan` with the updated `syncLayoutBox` contract.

---

## Verification & Testing Plan

1. **Unit Tests**:
   - Rust tests for `sync_host_layout` under preview vs committed states.
   - Verify `dom_width * css_scale == display_width`.

2. **Manual & UI Integration Tests**:
   - Zoom via dropdown selection (100% -> 125% -> 150%). Verify no visual size pop/flash.
   - Zoom via toolbar buttons (+ / -).
   - Wheel Zoom (Ctrl + Scroll). Verify smooth interpolation and layout stability.
