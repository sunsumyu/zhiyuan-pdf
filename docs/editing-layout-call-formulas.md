# Editing/Layout Call Formulas

> Current-code quick reference. This document maps the PDF text editing and layout/rendering call chains, plus the coordinate/caret/formula rules that decide whether clicking text can open an editor.
>
> It is not a refactor proposal. For design principles, read [architecture-principles.md](architecture-principles.md), [editor-render-architecture.md](editor-render-architecture.md), [edit-save-architecture.md](edit-save-architecture.md), [edit-state-architecture.md](edit-state-architecture.md), [coordinate-system-refactor-design.md](coordinate-system-refactor-design.md), and [architecture-diagrams.md](architecture-diagrams.md).

---

## 1. Layer map

| Layer | Main files | Responsibility |
| --- | --- | --- |
| HTML/UI | `index.html`, `src/main.ts` | Toolbar buttons, file open, save, edit-mode and format button events. |
| Viewer API/runtime | `src/bridge/viewer/pdf_viewer_api.ts`, `src/bridge/viewer/pdf_runtime.ts`, `src/bridge/viewer/pdf_viewer_dom.ts`, `src/bridge/viewer/pdf_keyboard.ts` | Public UI API, render scheduler wiring, keyboard shortcuts, button state sync. |
| Editor host TS | `src/bridge/editor/index.ts`, `src/bridge/editor/lifecycle.ts`, `src/bridge/editor/input_handler.ts`, `src/bridge/editor/editor_host_view.ts`, `src/bridge/editor/textarea_helper.ts` | DOM overlay, transparent paragraph targets, hidden textarea input/IME, editor shell/canvas, calls into WASM. |
| Editor WASM TS facade | `src/bridge/editor/api.ts`, generated `crates/pdf-viewer-ui/pkg/pdf_viewer_ui.*` | Thin typed calls to `EditorSession`. No layout ownership. |
| WASM UI Rust | `crates/pdf-viewer-ui/src/editor/*`, `crates/pdf-viewer-ui/src/render/*`, `crates/pdf-viewer-ui/src/page/*`, `crates/pdf-viewer-ui/src/document/*` | Editor session, activation, host page state, render transactions, patch persistence. |
| Core Rust | `crates/pdf-viewer-core/src/edit/*`, `crates/pdf-viewer-core/src/geometry/*`, `crates/pdf-viewer-core/src/text/*`, `crates/pdf-viewer-core/src/render/*` | Pure edit/layout/geometry/caret/render-domain logic. |
| Native backend | `src-tauri/src/infrastructure/pdf/*`, `src-tauri/src/interfaces/pdf/*` | PDF read/write, native commands such as region patch persistence. |

Global rule: **TS may orchestrate and host DOM input, but it must not invent PDF text layout or geometry.** Visible edited text is painted through Rust/WASM canvas, not browser text rendering.

---

## 2. Editing entry call chains

### 2.1 Enable Add Text / edit mode

```text
index.html #pdf-add-text-btn
  -> src/main.ts click handler
  -> getPdfViewerAPI().toggleTextEditMode()
  -> PdfViewerAPI.toggleTextEditMode()
       -> ensureWasmInitialized()
       -> editorHost.isTextEditEnabled()
       -> if disabling: editorHost.commitActiveEditor()
       -> editorHost.setTextEditEnabled(nextEnabled)
       -> syncTextEditButton()
       -> editorHost.syncTargets(readTargetZoom())
  -> EditorHost.setTextEditEnabled(true)
       -> api.setEditMode(true)
       -> syncTargets(ctx, cachedDisplayZoom)
  -> src/bridge/editor/api.ts setEditMode()
  -> WASM EditorSession.setEditMode(enabled)
  -> Rust host edit mode + editor_store transition
```

Key files:

- `src/main.ts`
- `src/bridge/viewer/pdf_viewer_api.ts`
- `src/bridge/editor/index.ts`
- `src/bridge/editor/lifecycle.ts`
- `src/bridge/editor/api.ts`
- `crates/pdf-viewer-ui/src/editor/editor_api.rs`

Important behavior:

- The toolbar button must reflect `editorHost.isTextEditEnabled()`, not a manually applied active class.
- `toggleTextEditMode()` must run after WASM initialization.
- If no document is open, edit mode should not create a fake active state.

### 2.2 Generate transparent paragraph targets

```text
EditorHost.syncTargets(displayZoom)
  -> lifecycle.syncTargets(ctx, displayZoom)
       -> ctx.ensureNodes()
       -> getVectorContainer()
       -> readLegacySnapshot(ctx)
            -> api.readLegacySnapshot(displayZoom)
            -> EditorSession.readLegacySnapshot(displayZoom)
       -> if !snapshot.enabled:
            hideInteractionTargets()
            hideEditorShell()
       -> if snapshot.activeTarget:
            positionEditorShell()
            renderActiveEditor()
       -> else:
            renderInteractionTargets(snapshot.targets)
```

Target DOM:

```text
#pdf-page-container
  #pdf-interaction-layer
    #pdf-interaction-root-vector
      #pdf-editor-target-layer-vector
        div[data-paragraph-id="..."]  <-- transparent clickable box
      #pdf-editor-shell-vector
        #pdf-editor-canvas-vector
        #pdf-editor-textarea-vector    <-- off-screen input adapter
```

Key files:

- `src/bridge/editor/lifecycle.ts`
- `src/bridge/editor/editor_host_view.ts`

A visible PDF text run is not itself clicked for editing. The click must hit a transparent editor target or the editor interaction root.

### 2.3 Click paragraph target and open editor

```text
target div pointerdown
  -> editor_host_view.bindPrimaryPress(box)
  -> lifecycle.openEditor(ctx, target, event)
       -> api.begin()
       -> resolveTargetReferenceBox(target, event, nodes.root)
       -> api.hitTest({ clientX, clientY, reference box, page size })
       -> blockId = hitResult.data.blockId ?? target.paragraphId
       -> api.openBlock({ blockId, client/reference/page data, fallbackPageX/Y })
       -> readLegacySnapshot(ctx)
       -> setupActiveEditor(ctx, nodes, activeTarget, draftText, caretIndex)
```

Rust side:

```text
EditorSession.begin()
  -> host_mode::set_edit_mode(true)
  -> editor_store::transition_to_editing()
  -> collect_text_blocks()

EditorSession.hitTest(request)
  -> HostPageTransform::new(...)
  -> HostPageTransform::to_page(client, None)
  -> resolve_target_at_page_point(page_x, page_y)

EditorSession.openBlock(request)
  -> if EditingBlock: commit_draft_internal(); state = Editing
  -> open_editor_tx(...)
  -> activation::activate_from_client(...)
  -> editor_controller::open_at_point(...)
  -> session::open_paragraph_editor(...)
  -> host_snapshot::resolve_snapshot(1.0)
  -> editor_store::transition_editing(block_id)
```

Key files:

- `src/bridge/editor/lifecycle.ts`
- `crates/pdf-viewer-ui/src/editor/editor_api.rs`
- `crates/pdf-viewer-ui/src/editor/activation.rs`
- `crates/pdf-viewer-ui/src/editor/orchestrator/render_transaction.rs`
- `crates/pdf-viewer-ui/src/editor/editor_controller.rs`
- `crates/pdf-viewer-ui/src/editor/session/session.rs`

### 2.4 Click editor root and open by hit test

```text
#pdf-interaction-root-vector pointerdown
  -> input_handler.onRootPointerDown(event)
       -> api.begin()
       -> api.hitTest({ client/reference/page data })
       -> if miss:
            api.discard()
            hideEditorShell()
            syncTargets()
       -> api.openBlock({ blockId, client/reference/page data, fallbackPageX/Y })
       -> readLegacySnapshot()
       -> setupActiveEditor()
```

Target clicks are more robust because `openEditor()` can fall back to `target.paragraphId`. Root clicks depend more directly on hit testing.

### 2.5 Setup active editor

```text
setupActiveEditor(ctx, nodes, activeTarget, draftText, caretIndex)
  -> positionEditorShell(nodes, activeTarget)
  -> textarea.value = draftText
  -> hideInteractionTargets(nodes)
  -> suspendHostOverlays(nodes)
  -> clearDomSelection()
  -> shell.display = block
  -> textarea.focus()
  -> rememberRustCaret(caretIndex)
  -> writeTextareaCaret(textarea, caretIndex)
  -> renderActiveEditor(ctx)
  -> scheduleOpenFocusStabilization()
```

The textarea is invisible and off-screen. It captures input/IME and selection state only. Visible edited text is painted by `paintCanvas()`.

---

## 3. Text input, caret, format, commit and save

### 3.1 Hidden textarea input

```text
textarea beforeinput insertText/insertLineBreak/delete...
  -> preventDefault()
  -> input_handler.onBeforeInputRequested(command, text, textarea)
       -> readTextareaCaret(textarea)  // DOM UTF-16 -> Rust char index
       -> api.syncInput({ text: textarea.value, caretIndex })
       -> api.applyCommand({ command, insertedText: text })
          // applyCommand does not pass host text/caret as command state;
          // Rust command reads stored LiveEditorParagraphState after syncInput.
       -> writeTextareaCaret(textarea, result.caretIndex)
       -> renderActiveEditor(ctx)
       -> if changed: renderCurrentPage('editorVisibility')
```

Navigation keys:

```text
textarea keydown ArrowLeft/Right/Up/Down/Home/End
  -> onNavigationRequested(command, textarea)
       -> api.syncInput(...)
       -> api.applyCommand({ command, insertedText: null })
       -> writeTextareaCaret(...)
       -> renderActiveEditor(ctx)
```

Backspace/Delete:

```text
textarea keydown Backspace/Delete
  -> preventDefault()
  -> onBeforeInputRequested('backspace' | 'delete', null, textarea)
```

IME:

```text
compositionstart -> composing = true
compositionend   -> composing = false
                 -> api.syncInput({ text, caretIndex })
                 -> renderActiveEditor(ctx)
```

Key files:

- `src/bridge/editor/editor_host_view.ts`
- `src/bridge/editor/input_handler.ts`
- `src/bridge/editor/textarea_helper.ts`
- `crates/pdf-viewer-ui/src/editor/command.rs`
- `crates/pdf-viewer-core/src/edit/document_edit_ops.rs`

### 3.2 Shell click moves caret

```text
#pdf-editor-shell-vector pointerdown
  -> input_handler.onShellPointerDown(event)
       -> readHostReferenceBox(nodes.root)
       -> api.moveCaret({ client/reference/page data })
       -> EditorSession.moveCaret(...)
       -> activation::move_caret_to_client(...)
       -> text_geometry::caret_at_shell_point(...)
       -> editor_controller::set_editor_caret(caret_index)
       -> writeTextareaCaret(textarea, caret_index)
       -> renderActiveEditor(ctx)
```

### 3.3 Render active editor

```text
renderActiveEditor(ctx)
  -> readLegacySnapshot(ctx)
  -> positionEditorShell(nodes, snapshot.activeTarget)
  -> sync textarea.value
  -> writeTextareaCaret(textarea, caretIndex)
  -> api.paintCanvas(nodes.canvas, displayZoom, draftText, caretIndex)
  -> syncFormatButtons()

EditorSession.paintCanvas(...)
  -> Rust editor visual canvas rendering
```

### 3.4 Apply format

```text
format button or keyboard shortcut
  -> PdfViewerAPI.toggleBold/toggleItalic/toggleUnderline/setColor
  -> editorHost.applyFormatAction(action)
  -> api.applyFormat(action)
  -> EditorSession.applyFormat(action)
  -> render_transaction::apply_format_tx(...)
  -> editor_controller::apply_format(action)
  -> editor_format::apply_format(action)
  -> LiveEditorParagraphState style mutation
  -> syncTargets()
  -> syncFormatButtons()
```

Relevant style mutations include bold, italic, underline, color, alignment, list kind, line height and paragraph controls.

### 3.5 Commit active editor

```text
Escape / Ctrl+Enter / blur / disabling edit / save
  -> lifecycle.commitEditor(ctx) or commitForSave(ctx)
       -> readLegacySnapshot(ctx)
       -> draftText = snapshot.draftText
       -> caretIndex = lastRustCaretIndex ?? snapshot.caretIndex ?? readTextareaCaret(textarea)
       -> api.commit({ draftText, caretIndex })
       -> hideEditorShell(ctx)
       -> syncTargets(ctx, displayZoom)
```

Rust commit:

```text
EditorSession.commit(request)
  -> platform_bridge::begin_commit()
  -> render_transaction::commit_editor_tx(draft_text, caret_index, frame_request)
       -> if session has changes:
            sync_editor_input(new_text, caret_index)
       -> commit::commit_text(text_to_commit)
            -> editor_controller::build_patch(new_text)
            -> apply_document_patch_direct(patch)
            -> close_active_editor()
  -> platform_bridge::finish_commit()
  -> editor_store::transition_to_viewing()
  -> host_mode::set_edit_mode(false)
```

### 3.6 Save edits to PDF backend

```text
#pdf-save-btn click
  -> PdfViewerAPI.save()
  -> editorHost.saveEdits()
  -> lifecycle.saveEdits(ctx)
       -> commitForSave(ctx)
       -> deps.saveEditorSession()
  -> DocumentEditApi.saveEdits('manual-save')
  -> editorApi.saveSession(path, currentPage)
  -> EditorSession.saveSession(path, pageIndex)
  -> activation::save_editor_session(path, pageIndex)
  -> patch_persistence::save_persistable_patches(path, pageIndex)
  -> raw_invoke('apply_region_patches', { path, pageIndex, patches })
  -> clear_persistable_patches(true)
  -> refreshDocument(source)
```

### 3.7 Undo/redo

There are two systems:

| System | Entry | Scope |
| --- | --- | --- |
| Active editor local history | `EditorSession.undo/redo`, `session::undo_active_editor`, `session::redo_active_editor` | Draft text/style while editor is active. |
| Document patch history | `PdfViewerAPI.undo/redo` -> `undoDocumentPipeline/redoDocumentPipeline` -> `ui_state_store::undo/redo` | Persistable document patch stack. |

Viewer toolbar/keyboard undo currently commits active editor first, then applies document-level undo/redo.

---

## 4. Layout/rendering call chains related to editing

### 4.1 Open/render document to current page state

```text
PdfViewerAPI.openPdfFile(path)
  -> ensureWasmInitialized()
  -> documentRuntime.openTextPdfFlow(path)
  -> DocumentSession.open({ path, initialZoom, default dimensions })
  -> renderCurrentPage()
  -> renderScheduler / renderFlow
  -> framePlanAdapter build/take/commit frame
  -> page_store/current paint plan updated in WASM UI layer
```

Editor target generation depends on current page state and paint plan. If current visible surface is preview/raster/scanned without vector text paint plan, `readLegacySnapshot()` can be enabled but return no targets.

### 4.2 Vector render -> editor overlay sync

```text
renderFlow commits vector frame
  -> pdf_runtime syncEditorOverlay(displayZoom)
  -> editorHost.syncTargets(displayZoom)
  -> readLegacySnapshot(displayZoom)
  -> renderInteractionTargets(snapshot.targets)
```

Preview/raster paths may clear editor overlay:

```text
renderFlow preview/raster visible path
  -> clearEditorOverlay()
  -> editorHost.clear()
  -> hideInteractionTargets()
  -> commitEditor(ctx)
```

Key files:

- `src/bridge/viewer/pdf_runtime.ts`
- `src/bridge/render/render_flow.ts`
- `src/bridge/editor/lifecycle.ts`

### 4.3 Source suppression and replacement overlay

Conceptual chain:

```text
commit_text(new_text)
  -> build rich patch + replacement snapshot
  -> apply_document_patch_direct(patch)
  -> ui_state_store patch maps + patch revision
  -> requestDocumentRefresh(source)
  -> effective page/render plan suppresses/replaces source text
  -> canvas/vector renderer draws resulting page state
```

Relevant files:

- `crates/pdf-viewer-ui/src/document/patch_persistence.rs`
- `crates/pdf-viewer-ui/src/render/source_suppression.rs`
- `crates/pdf-viewer-ui/src/render/canvas_overlay.rs`
- `crates/pdf-viewer-core/src/render/source_suppression.rs`
- `crates/pdf-viewer-core/src/render/text_suppression.rs`
- `crates/pdf-viewer-core/src/edit/replacement_region.rs`
- `crates/pdf-viewer-core/src/edit/replacement_snapshot.rs`

---

## 5. Formula and data transformation reference

### 5.1 Client coordinate -> page coordinate

Source: `crates/pdf-viewer-core/src/geometry/coordinate_transform.rs`.

Definitions:

```text
scale.x = positive_ratio(reference.width, page.width)
scale.y = positive_ratio(reference.height, page.height)
```

where:

```text
positive_ratio(numerator, denominator) =
  numerator / denominator, if both are finite and > 0
  1.0 otherwise
```

Client to page:

```text
page.x = (client.x - reference.left) / scale.x
page.y = (client.y - reference.top) / scale.y
```

No PDF Y-up inversion is applied here; this viewer uses a top-left, Y-down page coordinate chain for editor hit testing.

### 5.2 Client coordinate -> clamped point inside shell bbox

Source: `crates/pdf-viewer-ui/src/editor/activation.rs`.

```text
page_point = HostPageTransform::to_page(client, None)

x = clamp(page_point.x, shell_bbox.left, shell_bbox.right)
y = clamp(page_point.y, shell_bbox.top, shell_bbox.bottom)
```

Fallback open point when client point cannot be used:

```text
fallback.x = shell_bbox.left + max(shell_bbox.right - shell_bbox.left, 0) * 0.5
fallback.y = shell_bbox.top  + max(shell_bbox.bottom - shell_bbox.top, 0) * 0.5
```

### 5.3 Hit-test bbox formula

Source: `crates/pdf-viewer-ui/src/editor/activation.rs` and `editor_api.rs`.

```text
point_in_bbox(x, y, bbox, tolerance) =
  x >= bbox.left - tolerance
  && x <= bbox.right + tolerance
  && y >= bbox.top - tolerance
  && y <= bbox.bottom + tolerance
```

Current tolerance:

```text
tolerance = 4.0
```

### 5.4 Shell point -> page point -> caret

Source: `crates/pdf-viewer-ui/src/editor/format/text_geometry.rs`.

Shell to page:

```text
shell_left = active_target.scene.shell_bbox.left
shell_top  = active_target.scene.shell_bbox.top
body_left  = active_target.scene.body_session().anchor_bbox.left
body_top   = active_target.scene.body_session().anchor_bbox.top

body_offset_x = max(body_left - shell_left, 0)
body_offset_y = max(body_top  - shell_top, 0)

page_x = body_left + max(shell_x - body_offset_x, 0)
page_y = body_top  + max(shell_y - body_offset_y, 0)
```

Body text left after marker:

```text
marker_advance = active_target.scene.marker().map(|m| m.advance).unwrap_or(0)
body_text_left = active_target.scene.body_session().anchor_bbox.left + marker_advance

if page_x <= body_text_left:
    caret_index = 0
```

Draft local click point:

```text
local_click_x = max(page_x - (session.anchor_bbox.left + marker_advance), 0)
local_click_y = max(page_y - session.anchor_bbox.top, 0)
```

### 5.5 Caret line-stop scoring

Source: `crates/pdf-viewer-core/src/text/caret_geometry.rs`.

For each candidate caret stop:

```text
dx = local_click_x - stop.left
dy = local_click_y - line.baseline_y
score = dx * dx + dy * dy * 4.0
```

The caret index is the stop with minimum score. Vertical distance is weighted by `4.0`.

Navigation rules:

```text
Home      -> first stop index of current line
End       -> last stop index of current line
ArrowUp   -> previous line stop minimizing abs(stop.left - current_left)
ArrowDown -> next line stop minimizing abs(stop.left - current_left)
ArrowLeft -> caret_index.saturating_sub(1)
ArrowRight-> min(caret_index + 1, draft_text.chars().count())
```

### 5.6 UTF-16 DOM offset <-> Rust char index

Source files:

- `src/bridge/editor/textarea_helper.ts`
- `crates/pdf-viewer-ui/src/editor/editor_api.rs`
- `crates/pdf-viewer-ui/src/editor/format/text_index.rs`
- `crates/pdf-viewer-core/src/text/index_convert.rs`

Rule:

```text
DOM textarea selection offsets are UTF-16 offsets.
Rust editor caret indices are Unicode scalar char indices.
```

TS read:

```text
utf16Offset = max(0, textarea.selectionStart ?? textarea.value.length)
charIndex = api.utf16ToCharIndex(textarea.value, utf16Offset)
```

TS write:

```text
charIndex = max(0, caretIndex)
utf16Offset = api.charToUtf16Offset(textarea.value, charIndex)
textarea.selectionStart = utf16Offset
textarea.selectionEnd = utf16Offset
```

### 5.7 Insert/delete text formulas

Source: `crates/pdf-viewer-core/src/edit/document_edit_ops.rs`.

Insert:

```text
current_caret = resolved.caret_index
next_chars.splice(current_caret..current_caret, inserted_text.chars())
caret_after = current_caret + inserted_text.chars().count()
```

Delete backward:

```text
if current_caret == 0:
    no-op
else:
    remove index current_caret - 1
    caret_after = current_caret - 1
```

Delete forward:

```text
if current_caret >= char_count:
    no-op
else:
    remove index current_caret
    caret_after = current_caret
```

### 5.8 Sync input and command-state rules

Source:

- `crates/pdf-viewer-ui/src/editor/session/session.rs`
- `crates/pdf-viewer-ui/src/editor/command.rs`
- `crates/pdf-viewer-ui/src/editor/editor_api.rs`

Sync input:

```text
before_text = live_state.current_text()
if new_text != before_text:
    mode.history.push_snapshot(live_state)

text_changed = live_state.set_draft_text(new_text)
normalized_caret = min(caret_index, live_state.text_char_count())
caret_changed = live_state.set_caret_index(normalized_caret)

scene_changed = text_changed
request_visibility_render = text_changed
```

Apply command:

```text
EditorSession.applyCommand({ command, insertedText })
  -> apply_input_tx(command, None, None, frame_request)
  -> command.effective-state reads stored LiveEditorParagraphState
  -> insert/delete/navigation mutates Rust state
```

The hidden textarea is an event/IME adapter. It may provide a sync snapshot before command execution, but `applyCommand` itself does not let TS overwrite command text/caret. Diagnostics should therefore compare:

```text
TS caret.beforeinput.before.hostCaret
session.sync.requestedCaretIndex / afterCaretIndex
command.effective-state.storedCaretIndex / effectiveCaretIndex
mutation.*.caretBefore / removeIndex / caretAfter
TS caret.beforeinput.afterWrite.hostCaret
```

Local history coalescing source: `crates/pdf-viewer-ui/src/editor/session/history.rs`.

```text
push if last snapshot older than 1000ms
else push only if text length diff > 2 or style-change status differs
max undo stack length = 100
```

### 5.9 Commit patch no-op rule

Source: `crates/pdf-viewer-ui/src/editor/editor_controller.rs` and related patch builders.

A patch may be suppressed as no-op when all are true:

```text
text_unchanged
&& style_unchanged
&& alignment_unchanged
&& line_height_unchanged
&& marker_unchanged
```

If not no-op:

```text
build_patch(new_text)
  -> maybe new rich runs
  -> alignment / line_height / marker fields
  -> replacement snapshot
  -> apply_document_patch_direct(patch)
  -> ui_state_store::record_patch(patch)
```

### 5.10 Target bbox and zoom relationship

`readLegacySnapshot(displayZoom)` returns target/editor bboxes already suitable for the current display overlay. TS uses those values directly:

```text
target div left   = target.left px
target div top    = target.top px
target div width  = target.width px
target div height = target.height px
```

The editor root reference box is measured from DOM:

```text
reference = nodes.root.getBoundingClientRect()
```

Hit-test requests include both the DOM reference box and Rust page dimensions:

```text
{ clientX, clientY,
  referenceLeft, referenceTop, referenceWidth, referenceHeight,
  pageWidth, pageHeight }
```

Therefore, target mismatch usually means one of these is stale or inconsistent:

- `displayZoom`
- root DOM rect
- current page width/height
- current page paint plan
- preview/raster/vector surface state

---

## 6. Diagnostics for "click text does nothing"

Recent editor diagnostics are emitted through `emitPdfDiagnostic('editor', ...)`. Check DevTools console, terminal logs, or `window.__PDF_DIAGNOSTICS_HISTORY`.

| Diagnostic | Meaning | Likely area |
| --- | --- | --- |
| `toggleTextEditMode.noDocument` | Add Text clicked without an open PDF. | UI state / user flow. |
| `toggleTextEditMode.failed` | Edit mode toggle threw. | WASM init, `EditorSession`, host API. |
| `host.isTextEditEnabled.failed` | Reading editor state threw. | `readLegacySnapshot`, WASM session. |
| `host.setTextEditEnabled.failed` | `setEditMode` or target sync threw. | WASM/session or snapshot generation. |
| `targets.disabled` | Snapshot says edit mode is not enabled. | Button state mismatch or failed toggle. |
| `targets.empty` | Edit mode is enabled, but no clickable paragraph targets exist. | Scanned/preview/raster page, no paint plan, no editable text regions. |
| `targets.ready` | Targets were generated. | DOM/pointer/hit-test should be checked next. |
| `openEditor.hitTestMiss` | Target click happened but hit-test did not find block; target fallback may still be used. | Coordinate transform / stale page geometry. |
| `openEditor.openBlockFailed` | Rust refused to open the block. | Activation, shell bbox, target id, page store. |
| `openEditor.missingActiveTarget` | `openBlock` returned ok but snapshot lacks active target. | Editor session/snapshot inconsistency. |
| `rootPointerDown.hitTestMiss` | Root click did not hit a paragraph. | Pointer hit layer or coordinate mismatch. |
| `rootPointerDown.openBlockFailed` | Root hit succeeded but block open failed. | Rust activation/session state. |

DOM checks after clicking Add Text:

```js
[
  'pdf-page-container',
  'pdf-interaction-layer',
  'pdf-interaction-root-vector',
  'pdf-editor-target-layer-vector',
  'pdf-text-layer',
].map(id => {
  const el = document.getElementById(id);
  return {
    id,
    exists: !!el,
    display: el && getComputedStyle(el).display,
    visibility: el && getComputedStyle(el).visibility,
    pointerEvents: el && getComputedStyle(el).pointerEvents,
    zIndex: el && getComputedStyle(el).zIndex,
    children: el?.childElementCount,
  };
});
```

Expected on editable vector text page:

```text
#pdf-interaction-root-vector pointer-events = auto
#pdf-editor-target-layer-vector display = block
#pdf-editor-target-layer-vector childElementCount > 0
```

If `document.elementFromPoint(x, y)` over text returns a `#pdf-text-layer` child while Add Text is enabled, editor targets are missing/disabled/under the text layer. The DOM text layer supports selection only; it does not open editors.

## 7. Diagnostics for caret/delete, marker, overlay

Recent caret/delete diagnostics are emitted on both TS and Rust sides. For a suspected Backspace/Delete issue, collect one contiguous event sequence from the first DOM event through write-back:

| Stage | Event/action | Required fields |
| --- | --- | --- |
| TS beforeinput | `caret.beforeinput.before` | command, insertedText, hostCaret, selectionStart, selectionEnd, utf16Length, charCount, lastRustCaretIndex |
| Rust sync | `session.sync` / `sync_input_tx` | requestedText, requestedCaretIndex, normalizedCaretIndex, afterText, afterCaretIndex, textChanged, caretChanged |
| Rust command | `command.apply`, `command.effective-state` | command, storedText, storedCaretIndex, hostTextIgnored, hostCaretIndex, effectiveText, effectiveCaretIndex |
| Core mutation | `mutation.backspace` / `mutation.delete` / `mutation.insert` | beforeText, removedText/insertedText, removeIndex, caretBefore, caretAfter, afterText, isPristine, isSlotBacked |
| TS write-back | `caret.beforeinput.afterCommand`, `caret.beforeinput.afterWrite` | resultCaretIndex, resultDraftUtf16Length, resultDraftCharCount, resultChanged, selectionStart/selectionEnd after write |

Decision rules:

1. If `mutation.backspace.removeIndex == caretBefore - 1` and `mutation.delete.removeIndex == caretBefore`, core deletion is behaving correctly.
2. If `session.sync.afterCaretIndex` differs from `caret.beforeinput.before.hostCaret`, inspect UTF-16↔char conversion and textarea selection preservation.
3. If `command.effective-state.storedCaretIndex` differs from `session.sync.afterCaretIndex`, inspect render refresh / active editor state reset between sync and command.
4. If Rust result caret is correct but TS `afterWrite.selectionStart` is wrong, inspect `charToUtf16Offset` and textarea value write-back.

Marker/overlay follow-up diagnostics:

| Symptom | Capture |
| --- | --- |
| marker position differs by line | marker source path, `anchor_bbox.left`, `marker.advance`, computed `body_text_left`, `inject_fixed_marker` marker/body origins |
| marker width too large | marker text, source bbox width, `char_widths`, measured width path (`char_origins` vs canvas `measureText`); persisted overlay now prefers source `char_widths`/bbox before canvas fallback |
| original text remains / double paint | overlay source object indices, replacement region bbox, `text_suppression` decision, effective page plan entries |

---

## 8. Latest local validation snapshot (2026-06-28)

Commands run from `E:\chain\pdf-viewer-standalone`:

| Command | Result | Notes |
| --- | --- | --- |
| `cargo test -p pdf-viewer-core document_edit_ops -- --nocapture` | pass: 4 tests | char-index insert/delete, Unicode scalars, boundaries, semantic text vs raw slot text. |
| `cargo test -p pdf-viewer-core draft_text_diff -- --nocapture` | pass: 10 tests | source/runs synthetic gap mapping and caret remap. |
| `cargo test -p pdf-viewer-core draft_layout -- --nocapture` | pass: 23 tests | draft/persisted overlay geometry; includes single-character marker source-width and bbox-fallback contract. |
| `cargo test -p pdf-viewer-core document_plan -- --nocapture` | pass: 9 tests, 0 ignored | marker/vector geometry covered; `keeps_overlay_source` 已恢复为常规测试并通过。 |
| `cargo test -p pdf-viewer-ui` | fail on native target | Not a valid WASM-path check: `web-sys`/`js-sys`/`wasm_bindgen_futures` are wasm32-gated. |
| `npm run wasm:pdf-viewer-ui` | pass | WASM package generated under `crates/pdf-viewer-ui/pkg`. |
| `npm run build` | pass | Vite reports dynamic/static import chunk warnings only. |
| `npm run e2e:build` | pass | Built debug app at `target/debug/pdf-viewer-standalone.exe`; same Vite warnings plus Tauri bundle identifier warning. |

---

## 9. Maintenance invariants

1. **Single visual chain.** Visible PDF/editor text must be painted by Rust/WASM canvas painter. Browser text nodes and textarea rendering must not become the source of visible glyphs.
2. **Textarea is input only.** It captures keyboard, IME and selection offsets off-screen; it does not own layout or visual text.
3. **TS does not own PDF geometry.** TS may pass DOM reference boxes and client coordinates, but page/caret/layout decisions belong to Rust.
4. **Editor targets come from current page state.** If page store/paint plan/vector surface is unavailable, target generation can be empty even when the PDF visually shows pixels.
5. **DOM text layer is selection-only.** Clicking text for editing must hit editor interaction targets/root, not the selectable text layer.
6. **Suppression/replacement belongs to page/render pipeline.** Editing code builds patches and replacement snapshots; source clearing and persisted overlay rendering are page/render responsibilities.
7. **Commit before document-level undo/save.** Active editor draft state must become a patch before save or document-level history operations.
8. **All offsets crossing DOM/Rust boundary must convert UTF-16 <-> char index.** Never pass raw DOM `selectionStart` as a Rust char index without conversion.

---

## 10. Quick end-to-end recipes

### Enable edit and open text

```text
Add Text button
  -> toggleTextEditMode()
  -> setEditMode(true)
  -> readLegacySnapshot(displayZoom)
  -> renderInteractionTargets(targets)
  -> target pointerdown
  -> begin + hitTest + openBlock
  -> setupActiveEditor
  -> paintCanvas
```

### Type one character

```text
beforeinput insertText
  -> readTextareaCaret UTF-16 -> char
  -> syncInput(text, caret)
  -> applyCommand(InsertText) using Rust stored state
  -> Rust splice inserted chars
  -> writeTextareaCaret char -> UTF-16
  -> paintCanvas
  -> optional renderCurrentPage('editorVisibility')
```

### Commit and save

```text
commitEditor/commitForSave
  -> api.commit(draftText, caretIndex)
  -> build_patch(new_text)
  -> apply_document_patch_direct
  -> saveSession(path, pageIndex)
  -> save_persistable_patches
  -> raw_invoke('apply_region_patches')
  -> refreshDocument
  -> render scheduled frame
```

### Diagnose no reaction

```text
1. Check if toggle failed: toggleTextEditMode.failed
2. Check if edit enabled: targets.disabled
3. Check if targets exist: targets.empty vs targets.ready
4. If targets ready, check DOM pointer-events and elementFromPoint
5. If click reaches editor, check hitTest/openBlock diagnostics
6. If targets empty, verify vector text page, not scanned/preview/raster
```
