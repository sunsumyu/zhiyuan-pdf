# Sovereignty PDF Viewer -- Architecture Map

> Generated 2026-08-16 from the working tree on `refactor/architecture-improvements`.
> Every claim in this document is traceable to a file path in the current tree.

## 1. Four-Layer Overview

```
index.html / main.ts          UI shell (DOM, event wiring, ?file= params)
       |
   src/bridge/                TS bridge layer (~12,000 lines)
   (pdf_runtime.ts = root)    runtime composition, render loop, zoom, edit
       |
   crates/pdf-viewer-ui/      WASM crate (~5,500 lines Rust + JS glue)
   (pkg/pdf_viewer_ui)        exported structs: DocumentSession, ViewerSession,
       |                       EditorSession, FindSession, ReviewSession,
       |                       PagePresentationRuntime, free functions
       |                       sync_host_layout, handle_wheel_zoom_host, ...
       |
   crates/pdf-viewer-core/    Pure Rust library (~4,000 lines)
                               domain models, text-state, render plans,
                               annotation, edit commands, typography
       |
   src-tauri/                 Desktop backend (~10,700 lines Rust)
   (lib.rs = Tauri entry)     30 IPC commands, PDF parsing (lopdf + pdf-rs),
                               font engine, read/write pipeline, GPU renderer
                               (currently dead), working copies, image cache
```

Data flows **down** (TS calls wasm functions, wasm calls Tauri commands via
`target_invoke`), and **up** (Tauri returns data, wasm processes it, TS renders
to DOM).

---

## 2. Layer 1 -- `src-tauri/` (Desktop Backend)

### 2.1 Command Surface (30 commands)

Registered in `src-tauri/src/lib.rs:82-113`. All live under `src/interfaces/pdf/`
and are re-exported flat via `src/interfaces/pdf/mod.rs:22-29`.

**Document lifecycle:**
| Command | File | Purpose |
|---|---|---|
| `open_pdf` | `interfaces/pdf/document.rs:8` | Load PDF by path, cache `Arc<lopdf::Document>`, return page count |
| `clear_cache` | `interfaces/pdf/document.rs:24` | Release all documents, working copies, caches |
| `save_pdf` | `interfaces/pdf/document.rs:44` | Apply modifications (region patches + text reflows), save to disk |
| `undo` / `redo` | `interfaces/pdf/document.rs:66/78` | Swap document snapshots from history stacks |
| `pick_file` | `interfaces/pdf/system.rs:44` | Native file dialog, returns `Option<String>` |

**Page data:**
| Command | File | Purpose |
|---|---|---|
| `read_preview` | `interfaces/pdf/page.rs:15` | Light page model: dimensions, scanned/text kind, raster preview URL |
| `read_page_asset_bundle` | `interfaces/pdf/render.rs:18` | Combined vector model + glyph paint plan |
| `read_vector` | `interfaces/pdf/render.rs:78` | Full vector page model |
| `read_glyph_plan` | `interfaces/pdf/render.rs:124` | Glyph paint plan for text rendering |
| `read_images` | `interfaces/pdf/render.rs:158` | Embedded images as base64 data URLs |
| `diagnose_page` | `interfaces/pdf/render.rs:163` | Debug JSON with page dict keys, operator counts |

**Edit operations:**
| Command | File | Purpose |
|---|---|---|
| `apply_region_patches` | `interfaces/pdf/replace.rs:9` | Apply text replacement patches |

**Search:**
| Command | File | Purpose |
|---|---|---|
| `find_in_page` | `interfaces/pdf/search.rs:10` | Region-based text search on one page |
| `find_in_document` | `interfaces/pdf/search.rs:35` | Region-based text search across all pages |

**Annotations:**
| Command | File | Purpose |
|---|---|---|
| `read_annotation_targets` | `interfaces/pdf/annotation.rs:10` | Annotatable regions with bounding boxes |
| `read_highlights` / `apply_highlight` / `delete_annotation` | `interfaces/pdf/annotation.rs:22/31/40` | Highlight CRUD |
| `read_comments` / `read_comment_review` / `apply_comment` / `apply_comment_update` | `interfaces/pdf/comment.rs:11/20/29/38` | Comment CRUD + summary |

**System:**
| Command | File | Purpose |
|---|---|---|
| `set_log_level` / `clear_pdf_event_log` / `read_pdf_event_log` | `interfaces/pdf/system.rs:12/17/22` | Logging / event ring buffer |
| `set_page_asset_test_delay_ms` | `interfaces/pdf/system.rs:27` | Debug: artificial asset admission delay |
| `terminal_log` / `resolve_asset_url` | `interfaces/pdf/system.rs:32/37` | Frontend log bridge / filesystem path to asset URL |
| `create_demo_pdf` | `interfaces/pdf/system.rs:7` | Write a hardcoded 1-page PDF |

### 2.2 Module Map (`src-tauri/src/infrastructure/pdf/`)

**Read pipeline:**
| Module | Lines | Role |
|---|---|---|
| `pdf_loader.rs` | 361 | Lenient loader: strict -> load_mem -> trailer repair |
| `pdf_read/content_parser.rs` | 654 | Full PDF operator interpreter (q/Q/cm, colors, paths, BT/ET, XObject Do) |
| `pdf_read/image_builder.rs` | 317 | Image decoding (PNG predictor, JPEG passthrough) |
| `pdf_read/path_resolver.rs` | 186 | Per-page path resolution with double-checked cache + page locks |
| `pdf_read/graphics_state.rs` | 42 | Graphics state stack types |
| `pdf_read/resource_reader.rs` | 61 | Parent-chain resource flattening |
| `vector_engine.rs` | 454 | Display-list -> `NativeVectorPageModel`: line grouping, paragraph inference, palette, occlusion (disabled) |
| `preview_engine.rs` | 392 | Scanned/text classification + largest-image JPEG extraction |
| `layout_engine.rs` | 161 | Spatial semantic region/paragraph inference (moved back from core) |
| `spatial_graph.rs` | 90 | Adjacency graph + connected components |
| `glyph_mapping.rs` | 290 | Glyph-count/positions/glyph-id resolution (extracted from vello) |

**Write pipeline:**
| Module | Lines | Role |
|---|---|---|
| `pdf_write/reflow.rs` | 778 | Content-stream walkers, `PdfTextState`, `ReflowCluster` -- deepest write logic |
| `pdf_write/annotations.rs` | 264 | Highlight/comment annotation dict builders |
| `pdf_write/mod.rs` | 290 | `PdfDocExt` trait on `lopdf::Document`: 14 edit operations |
| `pdf_write/emitters.rs` | 157 | PDF text-op emission for deferred lines |
| `pdf_write/pages.rs` | 60 | Page delete/rotate/insert/metadata |
| `region_materializer.rs` | 577 | Region patches + text reflows -> effective `TextReflowPatch` plan |

**Font engine:**
| Module | Lines | Role |
|---|---|---|
| `font/parse.rs` | 628 | CMap / `ParsedFont` / widths from font dicts |
| `font/match_mod.rs` | 510 | System font substitution incl. CJK, weight matching |
| `font/embed.rs` | 354 | Subset + embed TrueType into PDF |
| `font/mod.rs` | 229 | `PdfTextWriteFont` resolution: reuse original or embed system fallback |
| `font/catalog.rs` | 243 | Windows system font enumeration |
| `font/face.rs` | 165 | Glyph-id encoding |
| `font/ttc.rs` | 118 | TrueType Collection extraction |
| `font/metrics.rs` | 94 | cosmic-text face metrics cache |
| `font/path.rs` | 76 | Douglas-Peucker path simplify |

**Shared state:**
| Module | Lines | Role |
|---|---|---|
| `text_state.rs` | 213 | Shared text-state fields (Tf/Tc/Tw/Tz/Tr params) |
| `text_matrix.rs` | 186 | Matrix trio (ctm/tm/tlm) used by both read and write |
| `document_service.rs` | 423 | `open_pdf`/`save_pdf`/`rollback`/`redo`/release; 5 unit tests |
| `document_resolver.rs` | 132 | Working-copy manager (`%TEMP%\working_{md5}.pdf`) |
| `cache.rs` | 79 | 3 lazy_static global caches + invalidation helpers |
| `color.rs` | 120 | strict hex parse (write path), cmyk/gray convert (14 tests) |
| `log_service.rs` | 266 | Level-gated logging, 512-event ring buffer, macros |
| `commands.rs` | 181 | `PdfEditCommand` trait + 10 command types |

**Also at top level:**
| Module | Lines | Role |
|---|---|---|
| `pdf_read/` (sibling dir) | ~780 | pdf-rs fallback backend: `scanned_backend.rs` (514), `classification.rs` (221) |

> Removed 2026-08-15: `vello_renderer.rs` (1130 lines, dead GPU renderer),
> `pdf_read/facade.rs` + `pdf_read/vector_backend.rs` (no callers),
> `page_classifier.rs` (no callers). See §7.

### 2.3 Document Lifecycle (Server-Side)

**Open:** `open_pdf` -> `PdfDocumentService::open_pdf` (`document_service.rs:117`):
cache hit? return page count. Miss? `spawn_blocking(load_pdf_lenient)` -> strict
load -> load_mem -> trailer-repair retry -> insert `Arc<lopdf::Document>` keyed
by path. If lopdf yields 0 pages -> fallback to pdf-rs `ScannedReadBackend`. Only
page count is returned; all page data pulled lazily.

**Render:** Two products:
1. **Raster preview** (`read_preview`): preview cache -> `build_light_page_model`
   (scanned/text classification + largest image decode) -> image cache UUID ->
   `http://pdfasset.localhost/<uuid>` served by custom protocol in `lib.rs:27`.
2. **Vector data** (`read_page_asset_bundle`): revision-keyed 3-tier cache ->
   `resolve_paths` (doc-pointer-address-keyed global cache) -> `parse_content_stream`
   (full operator walk) -> `build_vector_page_model_from_display_list` (grouping /
   palette / flip_y). **Frontend paints this data.** Vello is NOT in the live path.

**Save:** `save_pdf` -> `build_region_materialization_plan` (merge region_patches
+ text_reflows) -> `apply_batch_reflow_to_doc` (content-stream walkers) ->
`doc.save(path)` -> replace cache entry -> invalidate page/layout caches.

### 2.4 Backend State

- **`AppState`** (managed at `lib.rs:81`): `docs` (document cache), `cache`
  (page intermediate / layout / preview), `history` (undo/redo snapshots of
  entire `lopdf::Document`), `renderer` (vello slot -- always `None`).
- **lazy_statics:** `PDF_IMAGE_CACHE`, `PDF_FONT_PROGRAM_CACHE`,
  `PDF_RESOLVE_PATHS_CACHE` (keyed by doc pointer address), `WORKING_COPIES`,
  `PAGE_LOCKS`, `PDF_EVENT_LOG`.

---

## 3. Layer 2 -- `crates/pdf-viewer-core/` (Pure Rust Library)

Zero external deps beyond serde + log. No Tauri, no wasm-bindgen.

| Module | Responsibility |
|---|---|
| `models/` | Core domain types: `NativeVectorPageModel`, marker/overlay types, annotation models |
| `text/` | `TextState`, `TextMatrixCore` -- unified shared fields and operator semantics for read + write paths |
| `render/` | `FramePlanRequest`, `paint_plan`, `effective_page_plan`, path suppression -- render frame planning |
| `edit/` | `document_plan`, paragraph scene, styled runs, text mutation pipeline |
| `annotation/` | Annotation/comment/highlight type definitions |
| `geometry/` | Coordinate/rect primitives |
| `history/` | Document edit history types |
| `common/` | Sanitize functions, shared utilities |
| `persistence/` | Patch/region persistence types |
| `typography/` | Matcher, layout paragraph functions |

---

## 4. Layer 3 -- `crates/pdf-viewer-ui/` (WASM Crate)

Depends on `pdf-viewer-core` with feature `"wasm"`. Exports to JS via
`wasm-bindgen` (target: web). **Cannot be test-compiled on host target**
(wasm-gated deps: `web-sys`, `js-sys`, `wasm-bindgen-futures`).

### 4.1 Exported WASM Structs (singleton session handles)

| Struct | File | JS class | Purpose |
|---|---|---|---|
| `DocumentSession` | `document/document_api.rs` | `api.DocumentSession` | Open/close PDF, undo/redo/rotate, region patches |
| `ViewerSession` | `viewer/viewer_api.rs` | `api.ViewerSession` | Page navigation, zoom, current-page tracking |
| `EditorSession` | `editor/editor_api.rs` | `api.EditorSession` | Text editing: begin/hitTest/openBlock/commit/save |
| `FindSession` | `find/find_api.rs` | `api.FindSession` | Search orchestration |
| `ReviewSession` | `review/review_api.rs` | `api.ReviewSession` | Review panel state |
| `PagePresentationRuntime` | `presentation/presentation_api.rs` | `api.PagePresentationRuntime` | Page-turn admission, prefetch decisions |

### 4.2 Key Free Functions (exported to JS)

| Function | File | Purpose |
|---|---|---|
| `sync_host_layout` | `host/layout.rs:44` | **Zoom contract**: compute domWidth/domHeight/cssScale from display+render zooms |
| `handle_wheel_zoom_host` | `zoom/wheel_host.rs` | Wheel-zoom decision: target zoom, render decision, preview transforms |
| `step_preview_host` | `zoom/preview_step.rs` | RAF-driven smooth zoom preview interpolation |
| `build_frame_plan` / related | `present/plan_builder.rs` | Frame plan lifecycle (peek/take/schedule/commit) |
| `build_glyph_paint_plan` | `render/paint_plan.rs` | Glyph rendering plan for text painting |

### 4.3 Module Map

| Module | Responsibility |
|---|---|
| `bridge.rs` | JS FFI bindings (`target_invoke` -> `window.__TAURI__.core.invoke`) |
| `host/` | `layout.rs` -- sync_host_layout (the zoom cancellation contract: `dom_width * css_scale == display_width`) |
| `zoom/` | Wheel zoom host, preview stepping, zoom controller state |
| `document/` | Session API, open/close pipeline, edit mutation pipeline, patch persistence |
| `editor/` | Text editing session, block hit-test, caret sync |
| `render/` | Canvas overlay rendering, marker/paragraph overlay drawing |
| `present/` | Frame plan builder, viewport layout |
| `presentation/` | Page-turn admission, prefetch decisions, cancel-gate |
| `viewer/` | Viewer session state, scroll/viewport refresh |
| `find/` | Search session |
| `review/` | Review session |
| `annotation/` | Annotation session |
| `comment/` | Comment session |
| `common/` | Sanitize functions, shared utils |
| `models.rs` | Re-exports from core + UI-local types |

### 4.4 Tests (wasm target only)

| Module | Count | Coverage |
|---|---|---|
| `host/layout.rs` | 5 `#[wasm_bindgen_test]` | Zoom cancellation guarantee, no-flash, fallback, sanitization |
| `editor/overlay/paragraph_overlay.rs` | 4 `#[wasm_bindgen_test]` | Overlay patch/commit/marker/carries |
| **Total** | **9** | |

---

## 5. Layer 4 -- `src/bridge/` (TypeScript Bridge)

### 5.1 Runtime Composition Root

`src/bridge/viewer/pdf_runtime.ts` (`createPdfViewerRuntime`, line 81) is the
composition root. It builds and wires every sub-runtime via late-bound `let ... !`
assignments + dep-injection closures (lines 147-152, 278, 370, 458, 480):

```
viewerSession (singleton)
pagePresentationRuntime (adapter over wasm PagePresentationRuntime)
framePlanAdapter (adapter over wasm frame-plan free functions)
layoutSync (syncLayoutBox -> wasm syncHostLayout)
documentEditApi (mutation surface)
editorHost (paragraph editor lifecycle)
zoomController (wheel zoom host + smooth preview + commit)
renderFlow (the render loop)
renderScheduler (serializes render requests)
documentRuntime (open/reset/render)
resumeAiController, findController, commentController,
reviewController, annotationController
```

### 5.2 End-to-End Flows

#### Open (button / URL param / drag)

```
index.html:242-264 (inline script)
  -> window.__pdfOpenHandler (main.ts:49-63)
  -> Tauri invoke('pick_file') or hidden input click
  -> handleFileOpen(path) (main.ts:38-46)
  -> api().openPdfFile(path) (pdf_viewer_api.ts:72)
  -> deps.openTextPdfFlow(path) (pdf_runtime.ts:577)
  -> documentRuntime.openTextPdfFlow(path) (pdf_document_runtime.ts:85)
    -> clearVectorHost() + clearEditorHost()  [cancel in-flight renders]
    -> session.open({path, ...})  [wasm DocumentSession]
      -> target_invoke("open_pdf") -> window.__TAURI__.core.invoke -> src-tauri
    -> renderCurrentPage() (pdf_document_runtime.ts:123)
```

#### Render

```
renderScheduler.requestRender(source, reason, ctx) (render_scheduler.ts:175)
  -> executeRender (pdf_runtime.ts:483-570)
    -> documentRuntime.renderCurrentPage(reason) (line 498)
      -> renderFlow.renderCurrentPage (render_flow.ts:509)
        -> framePlanAdapter.scheduleRender -> runRenderLoop
          -> scanned preview fast path: presentRaster -> #pdf-render-target
          -> vector path: renderVectorPageWithPlan (vector_host.ts:238)
            -> resolveVectorPageBundle (Tauri read_page_asset_bundle)
            -> applyViewportCanvasFrame (re-box canvases in render-zoom units)
            -> per-layer renderLayer (worker or main thread)
            -> deferred presents queued
          -> commitVectorRenderResult
            -> beforePresent: zoomController.commitRenderedFrame -> syncLayoutBox
            -> copy pixels, make visible
    -> markPageVisible + prefetch adjacent pages
```

#### Zoom (post-578c058 fix)

The **core contract**: container is always sized in *render-zoom* units; the
display-vs-render gap is expressed solely as CSS `scale()` computed by wasm
`syncHostLayout`.

```
Ctrl+wheel on #pdf-scroll-container
  -> zoomController.bindWheelZoom (zoom_controller.ts:397)
  -> build full request (viewport point, scroll, bounds)
  -> wasm handleWheelZoomHost (Rust owns target zoom + render decision)
  -> syncZoomSelect() [updates dropdown]
  -> startSmoothZoomPreview() [RAF loop]
    -> wasm stepPreviewHost returns {previewPresent: {translateX/Y, cssScale}}
    -> applyPreviewFrame writes transient transform on container
  -> scheduleWheelZoomRender [debounce -> either render now or keep previewing]

Layout sync (the fix):
  pdf_layout_sync.ts:27-131
  -> syncLayoutBox(displayZoom, renderedZoom, layoutOverride)
  -> wasm syncHostLayout returns {domWidth, domHeight, cssScale, hostWidth, ...}
  -> wrapper sized to hostWidth x hostHeight
  -> #pdf-page-container: positioned at contentLeft/Top, sized domWidth x domHeight
     with transform: scale(cssScale)  [displayWidth = domWidth * cssScale]
  -> applyViewportCanvasFrame re-boxes canvases in render-zoom units only
     (no longer writes displayWidth into container)

Commit:
  zoomController.commitRenderedFrame (line 289)
  -> clear transform -> syncLayoutBox(displayZoom, renderZoom) -> restore scroll
```

#### Edit

```
#pdf-add-text-btn click -> toggleTextEditMode()
  -> EditorSession.setEditMode (wasm)
  -> syncTargets: render interaction targets

pointerdown on target -> api.begin() -> api.hitTest -> api.openBlock
  -> setupActiveEditor: position shell, focus textarea, paint caret

beforeinput -> onBeforeInputRequested
  -> api.syncInput (caret sync Rust <-> JS)
  -> api.applyCommand
  -> write back draftText + caret, repaint
  -> renderCurrentPage('editorVisibility') [immediate frame, not queued]

blur/Escape -> commitEditor -> api.commit({draftText, caretIndex})
  -> Rust builds patch

Save (#pdf-save-btn):
  -> api().save() -> editorHost.saveEdits
  -> documentEditApi.saveEdits('manual-save')
  -> invalidate caches -> wasm requestRefresh -> render frame
```

### 5.3 WASM Loading

`shared/wasm_loader.ts:1` imports `crates/pdf-viewer-ui/pkg/pdf_viewer_ui`
(built by `npm run wasm:pdf-viewer-ui`). Initialization:

1. `installTargetInvokeBridge()` (line 14) -- install JS shim for Rust's
   `target_invoke` BEFORE wasm init (Rust calls back into JS during init).
2. `await init()` (line 58) -- wasm module loads.
3. Install bridge again (line 64) -- ensure latest binding.
4. Fingerprint check: `'pdf-viewer-rust-single-chain-20260429'`.

The TS bridge never calls Tauri commands directly; wasm calls
`window.__TAURI__.core.invoke` via the `targetInvokeV3` bridge.

### 5.4 UI Element -> Bridge Mapping

| HTML element | Handler | Bridge API |
|---|---|---|
| `#open-btn` | inline script -> `__pdfOpenHandler` | `invoke('pick_file')` -> `openPdfFile` |
| `#pdf-save-btn` | `main.ts:89` | `api().save()` -> `editorHost.saveEdits` |
| `#pdf-undo/redo-btn` | `main.ts:99-100` | `undo()/redo()` |
| `#pdf-prev/next-page-btn` | `main.ts:103-108` | `prevPage()/nextPage()` |
| `#pdf-zoom-select` | `main.ts:112-115` | `setZoom(val)` |
| `#pdf-zoom-in/out-btn` | `main.ts:117-129` | `setZoom(...)` |
| Ctrl+wheel over `#pdf-scroll-container` | `zoomController.bindWheelZoom` | wasm `handleWheelZoomHost` |
| `#pdf-add-text-btn` | `main.ts:167-171` | `toggleTextEditMode()` |
| `#pdf-search-btn` / Ctrl+F | `findController.open()` | wasm `FindSession` |
| Arrows/PageUp/Down | `handlePdfViewerKeydown` | `prevPage()/nextPage()` |

---

## 6. Domain Types That Flow Between Layers

| Type | Defined | Consumed by | Purpose |
|---|---|---|---|
| `SyncHostLayoutRequest/Result` | `pdf-viewer-ui/host/layout.rs` | TS `pdf_layout_sync.ts` | Zoom cancellation contract |
| `FramePlanRequest` | `pdf-viewer-core/render` | `pdf-viewer-ui/present/plan_builder.rs` | Render frame planning |
| `TextState` / `TextMatrixCore` | `pdf-viewer-core/text/` | Both read (content_parser) and write (reflow) in src-tauri | Shared PDF text operator state |
| `NativeVectorPageModel` | `src-tauri/infrastructure/pdf/models.rs` (re-export of core) | TS `vector_page_bundle.ts` | Full vector page for rendering |
| `NativePageModel` / annotation types | `pdf-viewer-core/models/` | `src-tauri/application/pdf/page_annotation.rs` | Page region context |
| `PersistableRegionPatch` | `pdf-viewer-core/persistence/` | `src-tauri/infrastructure/pdf/region_materializer.rs` | Edit patch materialization |
| `PdfEditCommand` | `src-tauri/infrastructure/pdf/commands.rs` | `edit_commands.rs` | Edit transaction abstraction |

---

## 7. Known Issues / Dead Code

### Removed in the 2026-08-15 dead-code sweep (branch `refactor/architecture-improvements`)
- `vello_renderer.rs` + `RendererState` (1130 lines) -- `VelloRenderer::new()` had zero call sites.
  Also dropped the `vello` + `wgpu` deps (Cargo.lock -642 lines); vello/wgpu were never called by
  the frontend in this project *or* in the origin project (`nushell-enhanced`'s
  `render_vector_tile` was registered but never invoked from TS). Real rendering is the
  wasm Canvas 2D path (`pdf-viewer-ui/src/render/canvas.rs`), GPU-accelerated via WebView2/Skia.
  Recover with `git show fdde982^:src-tauri/src/infrastructure/pdf/vello_renderer.rs`.
- `pdf_read/facade.rs`, `pdf_read/vector_backend.rs` -- no callers
- `page_classifier.rs` -- no callers (preview_engine has its own inline version)
- `interfaces/pdf/ipc_converters.rs` -- re-export shim; call sites now use
  `application::pdf::edit_commands` directly (also fixed an Application -> Interfaces
  dependency inversion in `page_annotation.rs`)
- `PDF_OPS_LOCK`, `read_document_meta_cache`, legacy `PageModel`/`PageTextInfo`/`TextObjectInfo`
- `color.rs` orphan helpers `blend`/`parse_rgb`/`parse_vello` (orphaned with the vello renderer;
  the strict `parse_pdf` write path remains)
- Occlusion culling block in `vector_engine.rs` -- scanned and logged but the drain was
  commented out (pure dead computation)
- `#vector-render-container` div in `index.html` -- never read by TS
- `window.pdfSetToolMode()` calls in `main.ts` -- defined nowhere (were no-ops via `?.`)

### Duplicated logic
- ~~`truncate_for_log` in `pdf_utils.rs` and `edit_commands.rs`~~ -- deduped 2026-08-15
  (`edit_commands.rs` now imports from `pdf_utils`)
- ~~`/Rotate` parent-chain walking~~ -- the one true duplicate (`path_resolver.rs`) now calls
  `pdf_utils::read_page_rotation`. `resource_reader.rs` / `preview_engine.rs` walk the parent
  chain for `/Resources`-inherited XObject/Font lookups -- pattern-similar but semantically
  distinct; a generic "inherited attribute walker" would be needed to unify them (low value).
- ~~`DocumentSession` / `ReviewSession` TS singletons~~ -- now constructed only via
  `src/bridge/shared/session_singletons.ts` (was 2 / 3 separate module-level copies)
- ~~undo/redo history cap `HISTORY_LIMIT=20` vs hardcoded `20`~~ -- single constant at
  `app_state::HISTORY_LIMIT`, used by both `document_service.rs` and `edit_commands.rs`

### Naming hazards
- Two `pdf_read/` dirs: `src-tauri/src/infrastructure/pdf/pdf_read/` (lopdf parsing)
  vs `src-tauri/src/infrastructure/pdf_read/` (pdf-rs backends)
- `font/layout.rs` shadows std `layout` name
- Many files contain GBK-mojibake Chinese comments (UTF-8/GBK round-trip damage)
