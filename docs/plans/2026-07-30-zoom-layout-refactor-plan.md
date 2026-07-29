# PDF Viewer Zoom & Layout Architecture Refactor Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate PDF viewer zoom flickering ("flashing a larger version first, then snapping back") by making Rust the single source of truth for DOM layout bounds and CSS scale factors.

**Architecture:** Rust's `HostZoomState` & `FramePlanResult` compute the base DOM dimensions (`domContentWidth`/`domContentHeight`) based on `rendered_base_zoom` during preview, and target `displayZoom` when settled. TS Bridge declaratively applies these exact dimensions and `cssScale` to DOM styles without inferring state or double-scaling.

**Tech Stack:** Rust (`pdf-viewer-core`, `pdf-viewer-ui`), TypeScript (Vite/Tauri frontend bridge).

---

### Task 1: Update Rust Layout & Frame Plan Contracts

**Files:**
- Modify: `crates/pdf-viewer-ui/src/platform/layout.rs`
- Modify: `crates/pdf-viewer-core/src/render/plan_builder.rs`

**Interfaces:**
- Consumes: Existing `SyncHostLayoutRequest` / `FramePlanResult`.
- Produces: `SyncHostLayoutResult` with `dom_width`, `dom_height`, `css_scale`. `FramePlanResult` with consistent layout fields.

- [ ] **Step 1: Update `SyncHostLayoutRequest` and `SyncHostLayoutResult` structs in Rust**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncHostLayoutRequest {
    pub display_zoom: f32,
    pub render_zoom: Option<f32>,
    pub page_width: f32,
    pub page_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub layout_override: Option<HostLayoutOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncHostLayoutResult {
    pub display_zoom: f32,
    pub render_zoom: f32,
    pub dom_width: f32,
    pub dom_height: f32,
    pub display_width: f32,
    pub display_height: f32,
    pub css_scale: f32,
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
}
```

- [ ] **Step 2: Implement `sync_host_layout` in `crates/pdf-viewer-ui/src/platform/layout.rs`**

```rust
pub fn sync_host_layout(request: SyncHostLayoutRequest) -> SyncHostLayoutResult {
    let display_zoom = sanitize_positive(request.display_zoom, 1.0);
    let render_zoom = sanitize_positive(request.render_zoom.unwrap_or(display_zoom), display_zoom);
    let page_w = sanitize_positive(request.page_width, 1.0);
    let page_h = sanitize_positive(request.page_height, 1.0);

    let dom_width = page_w * render_zoom;
    let dom_height = page_h * render_zoom;
    let display_width = page_w * display_zoom;
    let display_height = page_h * display_zoom;
    let css_scale = if render_zoom > 0.0001 {
        display_zoom / render_zoom
    } else {
        1.0
    };

    let layout = request
        .layout_override
        .map(|layout| HostLayoutOverride {
            host_width: sanitize_positive(
                layout.host_width,
                dom_width.max(request.viewport_width),
            ),
            host_height: sanitize_positive(
                layout.host_height,
                dom_height.max(request.viewport_height),
            ),
            content_left: sanitize_non_negative(layout.content_left, 0.0),
            content_top: sanitize_non_negative(layout.content_top, 0.0),
        })
        .unwrap_or_else(|| {
            let computed = compute_viewport_layout_result(
                dom_width,
                dom_height,
                sanitize_non_negative(request.viewport_width, 0.0),
                sanitize_non_negative(request.viewport_height, 0.0),
            );
            HostLayoutOverride {
                host_width: computed.host_width,
                host_height: computed.host_height,
                content_left: computed.content_left,
                content_top: computed.content_top,
            }
        });

    set_visual_layout(display_zoom, layout.content_left, layout.content_top);

    SyncHostLayoutResult {
        display_zoom,
        render_zoom,
        dom_width,
        dom_height,
        display_width,
        display_height,
        css_scale,
        host_width: layout.host_width,
        host_height: layout.host_height,
        content_left: layout.content_left,
        content_top: layout.content_top,
    }
}
```

- [ ] **Step 3: Run Rust tests to verify layout compilation**

Run: `cargo test --package pdf-viewer-ui`
Expected: PASS

- [ ] **Step 4: Commit Task 1**

```bash
git add crates/pdf-viewer-ui/src/platform/layout.rs crates/pdf-viewer-core/src/render/plan_builder.rs
git commit -m "feat(zoom): update Rust layout sync contract for preview-aware DOM sizing"
```

---

### Task 2: Refactor TS Layout Sync & Zoom Controller

**Files:**
- Modify: `src/bridge/viewer/pdf_layout_sync.ts`
- Modify: `src/bridge/zoom/zoom_controller.ts`
- Modify: `src/bridge/viewer/viewer_geometry_probe.ts`

**Interfaces:**
- Consumes: Updated `syncHostLayout` output (`domWidth`, `domHeight`, `cssScale`, `contentLeft`, `contentTop`).
- Produces: Declarative DOM updates on `getVectorContainer()` and `getWrapper()`.

- [ ] **Step 1: Refactor `syncLayoutBox` in `src/bridge/viewer/pdf_layout_sync.ts`**

```typescript
        const layout = wasm.syncHostLayout?.({
            displayZoom: safeDisplayZoom,
            renderZoom: renderedZoom > 0 ? renderedZoom : safeDisplayZoom,
            pageWidth: deps.getPageWidth(),
            pageHeight: deps.getPageHeight(),
            viewportWidth: scrollContainer.clientWidth || rect.width || 0,
            viewportHeight: scrollContainer.clientHeight || rect.height || 0,
            layoutOverride: layoutOverride ? {
                hostWidth: layoutOverride.hostWidth,
                hostHeight: layoutOverride.hostHeight,
                contentLeft: layoutOverride.contentLeft,
                contentTop: layoutOverride.contentTop,
            } : null,
        }) ?? null;

        const domWidth = Number.isFinite(layout?.domWidth) ? layout.domWidth : deps.getPageWidth() * safeDisplayZoom;
        const domHeight = Number.isFinite(layout?.domHeight) ? layout.domHeight : deps.getPageHeight() * safeDisplayZoom;
        const hostWidth = Number.isFinite(layout?.hostWidth) ? layout.hostWidth : domWidth;
        const hostHeight = Number.isFinite(layout?.hostHeight) ? layout.hostHeight : domHeight;
        const contentLeft = Number.isFinite(layout?.contentLeft) ? layout.contentLeft : 0;
        const contentTop = Number.isFinite(layout?.contentTop) ? layout.contentTop : 0;
        const cssScale = Number.isFinite(layout?.cssScale) ? layout.cssScale : 1.0;

        wrapper.style.display = 'block';
        wrapper.style.position = 'relative';
        wrapper.style.width = `${hostWidth}px`;
        wrapper.style.height = `${hostHeight}px`;
        wrapper.style.margin = '0';
        wrapper.style.textAlign = 'left';
        wrapper.style.transform = '';
        wrapper.style.transformOrigin = '0 0';

        if (container) {
            container.style.position = 'absolute';
            container.style.left = `${contentLeft}px`;
            container.style.top = `${contentTop}px`;
            container.style.width = `${domWidth}px`;
            container.style.height = `${domHeight}px`;
            container.style.margin = '0';
            container.style.transformOrigin = '0 0';
            container.style.transform = Math.abs(cssScale - 1.0) < 0.001 ? '' : `scale(${cssScale})`;
        }
```

- [ ] **Step 2: Update `applyVisualZoomPreview` in `src/bridge/zoom/zoom_controller.ts`**

Simplify `applyVisualZoomPreview` so it delegates DOM width/height/transform sizing directly to `syncLayoutBox`:

```typescript
    function applyVisualZoomPreview(previewZoom: number): void {
        const container = deps.getVectorContainer();
        const scrollContainer = deps.getScrollContainer();
        if (!container) return;

        const baseZoom = deps.getZoomState().lastRenderedZoom > 0 ? deps.getZoomState().lastRenderedZoom : 1.0;
        const anchorLayout = deps.peekFramePlan(previewZoom);
        deps.syncLayoutBox(previewZoom, baseZoom, anchorLayout);

        if (scrollContainer && anchorLayout) {
            scrollContainer.scrollLeft = anchorLayout.scrollLeft;
            scrollContainer.scrollTop = anchorLayout.scrollTop;
        }
    }
```

- [ ] **Step 3: Run frontend build check**

Run: `npm run build` or `npx tsc --noEmit`
Expected: PASS with 0 type errors.

- [ ] **Step 4: Commit Task 2**

```bash
git add src/bridge/viewer/pdf_layout_sync.ts src/bridge/zoom/zoom_controller.ts src/bridge/viewer/viewer_geometry_probe.ts
git commit -m "refactor(zoom): simplify TS layout sync to use Rust domWidth and cssScale"
```

---

### Task 3: End-to-End Verification & Validation

**Files:**
- Test: Build project and launch standalone PDF viewer

- [ ] **Step 1: Build Wasm/Rust UI crates**

Run: `npm run build:wasm` or `cargo build --package pdf-viewer-ui`
Expected: Build succeeds cleanly.

- [ ] **Step 2: Verify Zoom via Dropdown & Buttons**
- Change zoom dropdown from 100% -> 125% -> 150%.
- Observe: No transient double-scaling flash occurs. Smooth transition.

- [ ] **Step 3: Verify Wheel Zoom (Ctrl + Mouse Wheel)**
- Perform Ctrl + Wheel zoom.
- Observe: Continuous smooth scaling without pop/flash.

- [ ] **Step 4: Final Git Commit**

```bash
git commit --allow-empty -m "test(zoom): verify smooth zoom without double-scaling flash"
```
