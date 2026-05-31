# Rust PDF Font Engine Refactor Plan

## Goal

Move all PDF font resolution, Windows font matching, glyph planning, and render policy decisions into Rust.

The target user-visible capability is:

- open a text PDF and render text with stable, explainable font decisions
- preserve layout fidelity across static view and edit view
- support a phased path from system-font approximation to embedded-subset exact rendering

## Current Mixed Responsibilities

The current system still mixes responsibilities across layers:

- [src/plugins/pdf-viewer/index.ts](/E:/chain/nushell-enhanced/src/plugins/pdf-viewer/index.ts)
  chooses the active render path and acts as a presentation host
- [src-tauri/src/infrastructure/multimedia/pdf/lopdf_utils.rs](/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/lopdf_utils.rs)
  extracts PDF text, font metadata, and CMap information
- [src-tauri/src/infrastructure/multimedia/pdf/vello_renderer.rs](/E:/chain/nushell-enhanced/src-tauri/src/infrastructure/multimedia/pdf/vello_renderer.rs)
  performs ad-hoc font aliasing and backend-specific text drawing
- [crates/pdf-viewer-core/src/font_resolver.rs](/E:/chain/nushell-enhanced/crates/pdf-viewer-core/src/font_resolver.rs)
  performs only lightweight family-name substitution

This creates a dual-engine problem:

- extraction knows more than rendering consumes
- Rust core and Tauri renderer do not share one font truth
- the static open path and the edit path can diverge visually
- TS still owns render-path selection, while Rust does not fully own text truth

## Target Boundaries

All font logic should converge into Rust with these boundaries:

### Domain: `pdf-viewer-core::typography`

- `PdfFontResolver`
  owns PDF font identity parsing, subset stripping, descriptor normalization, and source selection
- `SystemFontMatcher`
  owns heuristic matching from PDF font identity to Windows-installed fonts
- `GlyphLayoutEngine`
  owns glyph advances, char origins, metrics normalization, and layout-facing font facts
- `RenderPolicyResolver`
  owns the decision between embedded, system-matched, and fallback rendering

### Infrastructure: `src-tauri::infrastructure::multimedia::pdf`

- `WindowsFontCatalogAdapter`
  enumerates Windows fonts and exposes normalized metadata to the domain matcher
- `EmbeddedFontAdapter`
  extracts embedded font bytes from PDF objects
- `GlyphPainterAdapter`
  translates unified paint runs into Vello or other render backends

### Presentation

- TS remains a host only
- TS may open a file, request a render, and display the result
- TS must not decide font family aliases, metrics substitutions, or glyph fallback behavior

## First Extraction Step

The first extraction step that reduces merge risk is:

1. create a Rust typography domain surface in `pdf-viewer-core`
2. move all future font matching rules to that surface
3. stop adding any new font heuristics to `vello_renderer.rs` or TS

This gives the project one seam where new font behavior can land without expanding renderer-specific code.

## What Remains Deferred

These are intentionally deferred after the boundary work:

- full embedded subset exact rendering
- glyph-outline faithful replay for every backend
- cross-platform font catalogs beyond Windows
- metrics-driven visual diff verification between original PDF and reconstructed runs

## Design Ownership Matrix

- PDF font identity parsing: `PdfFontResolver`
- Windows family matching: `SystemFontMatcher`
- glyph metrics normalization: `GlyphLayoutEngine`
- backend draw calls: `GlyphPainterAdapter`
- render-path selection: `RenderPolicyResolver`
- DOM mounting and image display: TS host

## Implementation Phases

### Phase 1: Rust Domain Consolidation

- add typography domain models in `pdf-viewer-core`
- define normalized PDF font identity and system font candidate types
- define render policy types and score explanations
- route all future font heuristics through the new domain surface

### Phase 2: Windows Font Matching

- add a Windows font catalog adapter in Tauri infrastructure
- collect family, full name, PostScript name, style, weight, coverage hints, and mono/serif/symbol traits
- score candidates in Rust domain logic using explainable weighted heuristics

### Phase 3: Shared Paint Truth

- replace backend-local family guessing with resolved Rust font decisions
- carry `ResolvedPdfFont` through extraction, layout, and painting
- make static view and edit view consume the same font facts

### Phase 4: Embedded Subset Exact Rendering

- load embedded font bytes from PDF font streams
- map char codes to glyph ids using PDF font data and ToUnicode where appropriate
- prefer embedded subset rendering when available and valid
- fall back to matched Windows fonts only when embedded rendering is unavailable
- carry `fontSubtype`, `hasEmbeddedProgram`, and `hasToUnicodeCmap` through the unified Rust font model

## Non-Negotiables

- no new font rules in TS
- no new font alias tables inside backend renderers
- no backend may invent its own font fallback policy
- one paint run must carry one resolved font truth
- system matching must be explainable, scored, and cacheable

## Success Criteria

- opening a PDF uses Rust as the single font decision authority
- Windows font matching is deterministic and auditable
- the current Vello path no longer contains unique family-name heuristics
- subset fonts can later plug in without redesigning the host API
