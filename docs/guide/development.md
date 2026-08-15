# Development Guide -- Build, Test, Debug, Fix

> Practical reference for working on the Sovereignty PDF Viewer.
> All commands verified against the working tree on `refactor/architecture-improvements`.

---

## 1. Prerequisites

- **Rust** (stable, with `wasm32-unknown-unknown` target installed)
- **Node.js** + npm
- **wasm-pack** (`cargo install wasm-pack`)
- **wasm-bindgen-cli 0.2.120** -- MUST match the `wasm-bindgen` version in `Cargo.lock` exactly. Different versions cause a schema mismatch error. Install with:
  ```
  cargo install wasm-bindgen-cli --version 0.2.120
  ```
  Verify: `wasm-bindgen-test-runner -V` should print `0.2.120`.
- **Tauri CLI** (`npx tauri --version`) for desktop app
- **Chrome or Edge** for headless wasm tests and E2E

---

## 2. Build Commands

### WASM (required before any TS work)

```bash
npm run wasm:pdf-viewer-ui
# equivalent to: wasm-pack build ./crates/pdf-viewer-ui --target web
# Output: crates/pdf-viewer-ui/pkg/ (auto-generated, gitignored)
```

### Frontend (TS + CSS)

```bash
npm run build
# tsc + vite build -> dist/
```

### Desktop app

```bash
npm run tauri:dev        # Full Tauri + Vite dev server (opens desktop window)
# OR
npm run dev              # Vite-only, browser at http://127.0.0.1:5000
                         # WARNING: cannot open PDFs (open_pdf requires Tauri backend)
```

### Backend (Tauri Rust)

```bash
cd src-tauri && cargo build
# OR just let tauri:dev handle it
```

---

## 3. Test Commands

### `pdf-viewer-core` (pure Rust, host target)

```bash
cargo test -p pdf-viewer-core
# Result on main: 75 passed, 0 failed (2026-08-15)
```

This always works -- no wasm deps.

### `pdf-viewer-ui` (wasm target ONLY)

```bash
# MUST be on the wasm target. Host target fails to compile by design
# (wasm deps gated to cfg(target_arch = "wasm32")).

set CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
cargo test -p pdf-viewer-ui --target wasm32-unknown-unknown
# Result on main: 9 passed (5 zoom layout + 4 overlay) (2026-08-15)
```

**Gotcha:** `wasm-bindgen-test-runner` (v0.2.120) takes NO flags like `--headless`
or `--node`. Just set it as the runner and let cargo invoke it. If you get:

```
error: unexpected argument '--headless' found
```

You're using a runner version that doesn't support that flag, or passing flags
incorrectly. The correct invocation is just setting the env var.

**Gotcha:** If you get:

```
it looks like the Rust project used to create this Wasm file was linked against
version of wasm-bindgen that uses a different bindgen format than this binary:
  rust Wasm file schema version: 0.2.120
     this binary schema version: 0.2.126
```

Your globally installed `wasm-bindgen-cli` version doesn't match `Cargo.lock`.
Install the matching version (see Prerequisites).

### `src-tauri` tests

```bash
cargo test -p pdf-viewer-standalone  # (workspace name from Cargo.toml)
# 17 integration tests + ~150 unit tests across infrastructure/pdf modules
# Some tests have hard-coded absolute paths (machine-specific, will fail on other machines)
```

### TypeScript

```bash
npx vitest              # vitest (node env, src/**/*.test.ts)
# Currently only: src/__tests__/diagnostics.test.ts (59 lines)
```

### E2E (requires tauri-driver)

```bash
npm run e2e:build       # tauri build --debug --no-bundle
npm run e2e             # wdio run tests/e2e/wdio.conf.ts
# Specs: load_pdf, hello, page_presentation_runtime, editor_bugs (skipped)
# Requires tauri-driver to be installed globally
```

---

## 4. Bug Investigation Decision Tree

When you encounter a bug, use this flow to locate the relevant layer and files.

### Step 1: What symptom do you see?

| Symptom | Likely Layer | Start here |
|---|---|---|
| Page renders blank / white | Tauri backend (read pipeline) or TS render loop | Check `vector_engine.rs`, `path_resolver.rs`, `render_flow.ts` |
| Page renders garbled / wrong colors | Backend content parser | `pdf_read/content_parser.rs` |
| Text is blurry at certain zoom levels | Zoom layout contract | `host/layout.rs`, `pdf_layout_sync.ts` |
| Zoom jumps / flashes | Zoom layout contract (solved 2026-08-15) | `host/layout.rs`, `zoom_controller.ts`, `vector_canvas_host.ts` |
| Edit doesn't save | Edit pipeline | `editor/editor_api.rs`, `document_edit_api.ts`, `edit_commands.rs` |
| Fonts look wrong (weight, bold) | Font engine | `font/match_mod.rs`, `font/embed.rs`, `font/parse.rs` |
| Annotations / highlights missing | Annotation pipeline | `annotation_store.rs`, `page_annotation.rs`, `pdf_annotation_controller.ts` |
| Search finds nothing | Search pipeline | `page_search.rs`, `find_facade.ts`, `pdf_find_controller.ts` |
| PDF won't open | Loader / Tauri open path | `pdf_loader.rs`, `document_service.rs:117` |
| Crash / panic in Rust | Backend | Check `src-tauri/src/` stack trace |
| UI button does nothing | TS wiring | `main.ts`, `pdf_viewer_api.ts`, check if handler exists |
| Slow page turn / render | Prefetch / cache | `page_asset.rs`, `vector_page_bundle.ts`, `raster_image_cache.ts` |

### Step 2: Trace the call chain

Once you know the layer, trace from the entry point:

**Rendering bug:**
```
render_scheduler.ts:175 (requestRender)
  -> pdf_runtime.ts:483 (executeRender)
  -> render_flow.ts:509 (executeActualRender / runRenderLoop)
  -> vector_host.ts:238 (renderVectorPageWithPlan)
    -> vector_page_bundle.ts:267 (resolveVectorPageBundle)
      -> Tauri read_page_asset_bundle
        -> render.rs:18 -> page_intermediate_service.rs -> vector_engine.rs
          -> path_resolver.rs -> content_parser.rs
    -> vector_canvas_host.ts:233 (applyViewportCanvasFrame)
  -> pdf_layout_sync.ts:27 (syncLayoutBox -> wasm syncHostLayout)
```

**Zoom bug:**
```
zoom_controller.ts:397 (bindWheelZoom)
  -> frame_plan.ts:342 (handleWheelZoomHost -> wasm)
  -> zoom_controller.ts:221 (startSmoothZoomPreview -> RAF loop)
  -> zoom_controller.ts:289 (commitRenderedFrame -> syncLayoutBox)
  -> pdf_layout_sync.ts:27 (syncLayoutBox -> wasm syncHostLayout)
    -> host/layout.rs:44 (SyncHostLayoutRequest -> SyncHostLayoutResult)
       Contract: dom_width * css_scale == display_width
```

**Edit/Save bug:**
```
editor/index.ts:465 (commitEditor -> api.commit)
  -> editor/api.ts:168 (saveSession -> wasm EditorSession)
  -> document_edit_api.ts:79 (refreshDocument)
  -> pdf_runtime.ts:161 (invalidateVectorRenderCache)
  -> render cycle
  OR
save_pdf button -> pdf_viewer_api.ts:180 (save)
  -> editor/index.ts:662 (saveEdits)
  -> editor_wasm_api.ts:205 (saveSession -> wasm)
  -> Tauri save_pdf -> document_service.rs:199
    -> region_materializer.rs -> pdf_write/reflow.rs
```

### Step 3: Add diagnostic logging

The project has a built-in diagnostic system:

- **Rust side:** `pdf_log!()` macro (`log_service.rs`) with levels 0-3.
  Set with Tauri command `set_log_level(level)`.
- **TS side:** `emitPdfDiagnostic()` (`shared/diagnostics.ts`) logs to console +
  `window.__PDF_DIAGNOSTICS_HISTORY` + Tauri `terminal_log`.
- **Event log:** `read_pdf_event_log` / `clear_pdf_event_log` commands expose a
  512-entry ring buffer.
- **Layout trace:** `src/bridge/render/layout_trace.ts` logs DOM geometry on
  mismatch/transform/verbose.
- **Editor self-test:** `window.verifyEditorBugs()` in DevTools console
  (`src/dev/verify_editor_bugs.ts`).

### Step 4: Write a test before fixing

**For Rust (core/ui crate):** Write a `#[test]` or `#[wasm_bindgen_test]` that
reproduces the bug. See `host/layout.rs` tests (lines 108-200) for the pattern:
pure-function call with `assert_close`.

**For src-tauri:** Write a `#[cfg(test)]` unit test. Many modules already have
them (`color.rs` 22 tests, `glyph_mapping.rs` 18, `pdf_write/reflow.rs` 15,
etc.).

**For TS:** Write a vitest in `src/__tests__/`. The harness exists
(`vitest.config.ts`) but coverage is minimal.

**For E2E:** Add a spec in `tests/e2e/specs/` (requires tauri-driver).

### Step 5: Common fix patterns

**Rendering misalignment at zoom:**
The zoom layout contract in `host/layout.rs` (`dom_width * css_scale == display_width`)
is the single source of truth. If the visual doesn't match expected, dump the
`SyncHostLayoutResult` fields and check which invariant breaks.

**Font rendering wrong:**
Check `font/match_mod.rs` for system font substitution -- it does weight matching
and CJK fallback. The font parse chain is `parse.rs` -> `face.rs` -> `embed.rs`.

**Edit patch not applied:**
Trace `region_materializer.rs::build_region_materialization_plan` -- it merges
region_patches + text_reflows into effective `TextReflowPatch` entries. Check
the materialization report in `cache.pdf_materialization_reports`.

**Page data stale after edit:**
Invalidate caches explicitly: `invalidate_pdf_page_cache` (prefix-based) in
`cache.rs`. The edit path in `edit_commands.rs` does this automatically; manual
editors must call `requestRefresh`.

---

## 5. Working on `main` vs `refactor/architecture-improvements`

**`main`** is the production branch. It contains all salvaged bug fixes (marker,
font, zoom, dialog) plus deep module refactoring (TextState, TextMatrixCore,
module deletion). Always PR against main.

**`refactor/architecture-improvements`** is the active architecture branch.
Recent commits:
- Domain glossary (`CONTEXT.md`) and zoom spec (`2026-08-04-zoom-bug-fix-via-merge.md`)
- Vitest frontend test harness (`diagnostics.test.ts`)
- Shallow module removal, delegating to `pdf-viewer-core`
- Dependency inversion fixes

**`fix/zoom-layout-tests-wasm-runnable`** is a small branch (2 commits) that
fixes the zoom cancellation tests to be wasm-runnable and ports the manual E2E
runbook. PR candidate for main.

**`codex/refactor-split`** is historical. DO NOT merge into main -- the zoom fixes
and font/marker/dialog fixes have already been salvaged into main via batch
salvage. A full merge would import a parallel architecture through 33 conflicts.

---

## 6. Key Invariants to Preserve

1. **Zoom contract:** `dom_width * css_scale == display_width` (enforced by 5 wasm
   tests in `host/layout.rs`). Never write `displayWidth` into the container
   directly; always go through `syncHostLayout`.

2. **Edit ordering:** `clearVectorHost()` MUST be called BEFORE `session.open()` to
   cancel in-flight renders. This is enforced only by call order in
   `pdf_document_runtime.ts:96-100`, not by the API shape.

3. **Cache invalidation:** After any document mutation, invalidate:
   `pdf_page_cache`, `pdf_page_intermediate_cache`, `pdf_layout_cache`,
   `PDF_RESOLVE_PATHS_CACHE` (prefix-matched by doc pointer).

4. **Wasm singletons:** `DocumentSession`, `ViewerSession`, `EditorSession`,
   `ReviewSession`, `PagePresentationRuntime` are lazily constructed singletons
   backed by wasm thread_local state. The TS objects are just handles -- the
   real state lives in wasm.

5. **Working copy lifecycle:** `resolve_working_path` copies to
   `%TEMP%\working_{md5}.pdf`. ALL saves write to the ORIGINAL path. The
   working copy goes stale after edits and is only re-created when absent.
