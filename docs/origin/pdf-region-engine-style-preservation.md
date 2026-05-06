# PDF Region Engine Style Preservation Upgrade

## Overview

This document records the current architectural upgrade of the PDF editor from:

- text-only patches
- segment-oriented editing
- object-covered redraw

to:

- region-oriented editing
- snapshot-driven redraw
- style-run-aware preservation
- edit-intent-driven style strategies

The goal is to prevent editing from collapsing:

- colors
- font weights
- italics
- mixed inline styles
- field key/value visual separation

while keeping the editor aligned with the PDF page model.

The latest refinement also separates:

- `dominant style`
- `full styleRuns`

They are intentionally used for different responsibilities:

- `dominant style`
  - drives single-line editor appearance for paragraph / list-item editing
  - provides a stable visual fallback when the editor must render one style only
- `full styleRuns`
  - preserve mixed inline styles
  - drive snapshot redraw
  - remain the persistence truth for region edits

This prevents the editor from flattening multi-style text just because the inline editor can only display one active style at a time.

---

## Current Problem Statement

Historically, the editor used:

- `patchedSegmentTexts`
- `patchedFieldGroupTexts`
- `patchedParagraphRegionTexts`

as the primary truth source.

That was sufficient for plain text changes, but insufficient for:

- preserving field label/value style separation
- preserving paragraph inline style spans
- repainting edited content without flattening all text into a single style

This caused issues such as:

- color changing after leaving edit mode
- key/value style collapsing into one style
- paragraph lines being redrawn with only one fallback style
- single-line list items inheriting bullet/icon color instead of body text color
- editor appearance drifting toward the first object style instead of the dominant body style

---

## Upgraded Architecture

The upgraded editor now follows this internal pipeline:

```text
RegionProjection
-> RegionSnapshot
-> EditIntent
-> StylePreservationStrategy
-> RegionSnapshotRenderer
-> PersistableRegionPatch
-> PersistableSavePlan
```

This applies to both:

- field rows
- paragraphs / list items

For paragraph / list-item editing, the practical split is now:

```text
Editor appearance
-> dominant style

Snapshot redraw / persistence
-> full styleRuns
```

---

## Core Layers

### 1. Region Model

Main region types:

- `FieldRowRegion`
- `ParagraphRegion`
- `ListItemRegion`

These are inferred in:

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-engine.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-engine.ts)

The engine now resolves list-item and paragraph-line base style from `styleRuns` using dominant-style selection,
instead of blindly using the left-most object as the visual source.
When the dominant-style pass still needs a fallback object, decorative leading glyph objects such as bullets are skipped
so that body text style remains the preferred fallback source.
The interaction layer now follows the same decorative-object rule when selecting editable body objects for single-line
paragraph / list-item editing, so anchor selection, fallback style resolution, and editor targeting stay aligned.

### 2. Projection Layer

Region projections now exist for:

- `FieldGroupProjection`
- `ParagraphRegionProjection`

These define:

- hit boxes
- shell boxes
- line boxes
- value/key boxes

They are used for hit testing and editor positioning, not for persistence.

### 3. Snapshot Layer

The editor now stores structured snapshots instead of only final strings.

#### Paragraph snapshot

- `ParagraphRegionSnapshot`
- `ParagraphRegionSnapshotLine`
- `StyleRunSnapshot`

#### Field snapshot

- `FieldGroupSnapshot`
- `keyRuns`
- `valueRuns`

Snapshots are created in:

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-engine.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-engine.ts)

### 4. Edit Intent Layer

The editor now defines explicit edit intent types:

- `ParagraphEditIntent`
- `FieldEditIntent`

Declared in:

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/edit-intent.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/edit-intent.ts)

This replaces scattered ad-hoc parameters like:

- `selectionStart`
- `selectionEnd`
- `activePart`
- `newText`
- `newKeyText`
- `newValueText`

### 5. Style Preservation Strategy Layer

Two strategy modules now own style-preserving snapshot transformation:

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/paragraph-style-preservation-strategy.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/paragraph-style-preservation-strategy.ts)
- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-style-preservation-strategy.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-style-preservation-strategy.ts)

These strategies now:

- preserve unchanged prefix/suffix style runs
- isolate the changed middle range
- apply the active run style to the changed span
- return a new structured snapshot

### 6. Renderer Layer

Redraw logic is now being moved out of `index.ts` into:

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/region-snapshot-renderer.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/region-snapshot-renderer.ts)

Current exported renderers:

- `renderParagraphSnapshot(...)`
- `renderFieldGroupSnapshot(...)`

They render from structured snapshots, not from raw patched strings.

### 7. Persistence Layer

Region persistence is now being formalized around:

- `PersistableRegionPatch`
- `PersistableSavePlan`

Implemented across:

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-engine.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-engine.ts)
- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/persistable-region-patch.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/persistable-region-patch.ts)

This allows the frontend save path to:

1. collect structured region patches first
2. keep region metadata and snapshots available
3. only then downgrade to legacy `textReflows` for the current backend

This is the transitional step needed before Rust can consume region-aware patches directly.

The frontend now already sends:

- `regionPatches`
- `textReflows`

to `save_pdf(...)`.

The Rust `PdfModifications` payload now accepts:

- `regionPatches` for region-aware intent
- `textReflows` for current lopdf materialization

At the moment, Rust still applies `textReflows` as the execution path and only records `regionPatches` for protocol alignment and future backend materialization.

This has now been upgraded one step further:

- Rust materializes `regionPatches` into effective `TextReflowPatch` values
- then merges them with explicit frontend `textReflows`
- explicit `textReflows` win on conflict
- backend materialization is now split by region source:
  - `field-row`
  - `paragraph-region`
  - `list-item-region`

So the backend execution path is now:

```text
regionPatches
-> materialized effective text reflows
-> lopdf apply_atomic_reflow_to_doc(...)
```

This is still a compatibility materializer, but it means region-aware patches are no longer ignored by the backend runtime.

The materializer is also now ready for further specialization by region kind without changing the save contract again.

Current per-kind behavior:

- `field-row`
  - requires valid `pairId`, `groupId`, and non-empty `targetIndices`
  - prefers rebuilding final text from field snapshot `keyText + valueText`
  - prefers rebuilding final text from `originalText + newValueText`
  - skips invalid patches with empty `targetIndices`
  - falls back to `newText`
- `paragraph-region`
  - prefers `snapshot.text`
  - requires structured snapshot data (`lines` + `styleRuns`) when using snapshot text
  - normalizes line endings
  - skips invalid patches with empty `targetIndices`
  - falls back to `newText`
- `list-item-region`
  - prefers `snapshot.text`
  - requires structured snapshot data (`lines` + `styleRuns`) when using snapshot text
  - normalizes line endings
  - skips invalid patches with empty `targetIndices`
  - falls back to `newText`

The frontend save path has also been tightened one step further:

- `PersistableSavePlan` is now the merge point for save-time text reflows
- region-derived reflows and legacy fallback reflows are now merged in one place
- conflict resolution is now source-aware instead of being hand-assembled inside `index.ts`

Current source priority inside `PersistableSavePlan` is:

- `field-row`
- `paragraph-region`
- `list-item-region`
- `segment`
- `run`
- `object`

This has now been tightened one step further:

- if a region-derived reflow already owns the exact same `pageIndex + targetIndices`
- or if a legacy reflow only targets a subset fully covered by a single region-owned reflow on that page
- generic legacy reflows (`segment` / `run` / `object`) are suppressed before merge

The legacy collector has also been aligned with this rule:

- if a region-owned target set already fully covers a segment / run / object candidate
- that legacy reflow is now skipped before it even enters the save-plan merge stage

This avoids save-time regressions where region-aware paragraph/list-item edits could still be reintroduced into the compatibility path by stale legacy text patches.

This means the semantic region model now explicitly wins over legacy run/segment/object fallbacks when both target the same PDF object indices.

The legacy save collector has also been narrowed further:

- objects that are already semantically classified as `FieldRowRegion`
- objects that belong to `ParagraphRegion`
- objects that belong to `ListItemRegion`

are now skipped by the generic legacy reflow collector even if they do not yet appear in the explicit covered-object sets.

In practice, this means:

- semantic regions are now blocked from leaking back into generic `segment` / `run` / `object` save paths
- the generic collector is increasingly reserved for true non-region compatibility cases
- the engine is closer to a true "region first, legacy last" architecture

The frontend also now exposes an internal page-level "effective region patch state":

- effective field-group snapshots
- effective field-group texts
- effective paragraph/list snapshots

This matters because:

- debug rows
- interaction-layer editors
- canvas redraw
- save-plan collection

are now increasingly reading from the same normalized per-page state instead of re-deriving slightly different views of patched data in multiple places.

The intention of this change is to keep:

- `regionPatches`
- snapshot-driven paragraph/field edits

as the primary truth source, while:

- `segment`
- `run`
- `object`

remain compatibility fallbacks only.

This is an important architectural boundary because it prevents the save path from regressing back into a patch-accumulation pipeline.

Paragraph / list snapshot construction has also been strengthened:

- `ParagraphLine` now carries source `styleRuns`
- `ListItemRegion` now carries source `styleRuns`
- initial region snapshots no longer always collapse to "one line = one style run"
- when edited text still matches the original line text, the source run structure is preserved directly
- when edited text changes length, snapshot construction now remaps source runs proportionally instead of flattening immediately

This does not fully solve all paragraph inline-style preservation cases yet, but it significantly reduces the chance that paragraph/list regions fall back to a fully flattened style model before the preservation strategy even runs.

The frontend now also performs one more normalization step before save / redraw:

- legacy `segment` / `run` / `object` edits that land inside a detected `ParagraphRegion` or `ListItemRegion`
- are first reassembled into region text
- then synthesized into a `ParagraphRegionSnapshot`
- and only then enter the region persistence / redraw path

This means paragraph/list content no longer needs an explicit paragraph editor interaction to benefit from the newer region pipeline. Legacy edits can still exist as compatibility input, but they are increasingly normalized into region snapshots before persistence.

Backend materialization has now been extracted into its own Rust module:

- [/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/region_materializer.rs](/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/region_materializer.rs)

This keeps `engine.rs` focused on orchestration and execution while region patch interpretation lives in a dedicated layer.

The backend now also produces an internal materialization plan/report:

- effective text reflows
- per-region decisions
- skipped/materialized counts
- per-source aggregated counts
- explicit skip reasons in logs

This gives us a stable place to inspect region execution quality during real-document validation.

---

## Data Model Summary

### `StyleRunSnapshot`

Represents the smallest style-preserving redraw unit.

Fields:

- `id`
- `text`
- `start`
- `end`
- `style`

### `ParagraphRegionSnapshot`

Contains:

- whole region text
- per-line snapshot
- flattened style runs

### `FieldGroupSnapshot`

Contains:

- `keyText`
- `valueText`
- `keyRuns`
- `valueRuns`
- `keyBox`
- `valueBox`

This is the canonical editable truth for field rows.

---

## Current Execution Flow

### Paragraph / List Item

1. `ParagraphRegionEditorController` collects:
   - `newText`
   - `selectionStart`
   - `selectionEnd`
2. `index.ts` builds a `ParagraphEditIntent`
3. `preserveParagraphRegionStyles(...)` creates the next `ParagraphRegionSnapshot`
4. `ParagraphRegionEditCommand` stores:
   - old text
   - old snapshot
   - new text
   - new snapshot
5. redraw uses `renderParagraphSnapshot(...)`
6. save uses `PersistableRegionPatch -> PersistableSavePlan -> textReflows`

### Field Row

1. `FieldValueEditorController` collects:
   - `activePart`
   - `selectionStart`
   - `selectionEnd`
   - `newKeyText`
   - `newValueText`
2. `index.ts` builds a `FieldEditIntent`
3. `preserveFieldGroupStyles(...)` creates the next `FieldGroupSnapshot`
4. `FieldGroupEditCommand` stores:
   - old text
   - old parts
   - old snapshot
   - new text
   - new parts
   - new snapshot
5. redraw uses `renderFieldGroupSnapshot(...)`
6. save uses `PersistableRegionPatch -> PersistableSavePlan -> textReflows`

---

## What Has Improved

The upgraded architecture now provides:

- structured style-aware redraw
- field key/value style separation
- paragraph inline style preservation
- edit-intent-aware snapshot generation
- reduced dependence on string-only patch logic
- reduced renderer logic inside `index.ts`
- a formal persistence bridge from region snapshots to save payloads

This is a major step away from patch-based UI logic and toward a proper editor core.

---

## Remaining Gaps

Although the architecture is now substantially stronger, there are still important follow-up areas:

### 1. Renderer decomposition

`region-snapshot-renderer.ts` is the first extraction step, but rendering can still be split further into:

- `paragraph-region-renderer.ts`
- `field-row-renderer.ts`

### 2. Persistable region patch formalization

The next architectural step should be to formalize:

- `PersistableRegionPatch`

so the backend can eventually consume:

- region snapshots
- edit intent / semantic patch metadata

instead of only flattened text materialization.

### 3. More precise run-splitting

Current strategies preserve style boundaries using:

- common prefix
- common suffix
- active middle span

This is already much better than string-only redraw, but can be further improved with:

- true run-level edit range mapping
- stronger line-local offset tracking
- multilingual inline run heuristics

### 4. Icon field region support

The engine still lacks a formal:

- `IconFieldRegion`

for content such as:

- phone icon + phone number
- email icon + email address

---

## Files Involved

### Core engine

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-engine.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-engine.ts)

### Controllers

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-value-editor-controller.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-value-editor-controller.ts)
- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/paragraph-region-editor-controller.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/paragraph-region-editor-controller.ts)

### Hit test

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-hit-test-controller.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-hit-test-controller.ts)
- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/paragraph-hit-test-controller.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/paragraph-hit-test-controller.ts)
- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/page-hit-test-facade.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/page-hit-test-facade.ts)

### Strategies

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/paragraph-style-preservation-strategy.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/paragraph-style-preservation-strategy.ts)
- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-style-preservation-strategy.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/field-row-style-preservation-strategy.ts)

### Rendering

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/region-snapshot-renderer.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/region-snapshot-renderer.ts)

### Protocol

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/edit-intent.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/edit-intent.ts)

### Main façade / orchestration

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/index.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/index.ts)

### Persistence bridge

- [/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/persistable-region-patch.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/persistable-region-patch.ts)

### Rust materialization

- [/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/region_materializer.rs](/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/region_materializer.rs)
- [/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/engine.rs](/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/engine.rs)
- [/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/models.rs](/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/models.rs)
- [/E:/chain/nushell-enhanced/src-tauri/src/interfaces/multimedia/pdf.rs](/E:/chain/nushell-enhanced/src-tauri/src/interfaces/multimedia/pdf.rs)

---

## Materialization Report

The save pipeline now exposes a structured materialization report end to end.

### Backend

After `save_pdf(...)` builds the region materialization plan, Rust now:

- stores the latest `PdfMaterializationReport` in `AppState`
- keeps:
  - `regionPatchCount`
  - `explicitTextReflowCount`
  - `effectiveTextReflowCount`
  - `decisions[]`
- exposes it through the Tauri command:
  - `get_last_pdf_materialization_report(path)`

Each decision includes:

- `regionId`
- `source`
- `status`
- `reason`

The report also now includes aggregated summary fields:

- `materializedCount`
- `skippedCount`
- `bySource[]`

Each `bySource` row includes:

- `source`
- `materialized`
- `skipped`

### Frontend

After a save completes, the PDF viewer now fetches the latest materialization report and exposes:

- `window.__pdfLastMaterializationReport`
- `window.__pdfMaterializationReport()`

The helper prints:

- a summary object
- a per-source summary table
- a `console.table(...)` of all materialization decisions

The frontend also now exposes a combined audit helper:

- `window.__pdfRegionAudit()`

It joins:

- the current region debug rows
- the latest materialization report

and prints a combined table showing, for each visible block:

- `regionId`
- `regionKind`
- `blockKind`
- `materializationStatus`
- `materializationReason`
- `materializationSource`

This makes save-path validation possible without reading Rust logs directly.

## Final Summary

The editor is no longer organized around:

- patched strings
- ad-hoc redraw rules
- segment-centric editing only

It is now increasingly organized around:

- region semantics
- projection
- snapshot truth
- explicit edit intent
- style preservation strategies
- renderer isolation

This is the correct direction for industrial-grade PDF region editing.
