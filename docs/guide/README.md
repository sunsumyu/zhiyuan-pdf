# Sovereignty PDF Viewer -- Developer Guide

> **Start here.** This folder contains the authoritative documentation for
> working on this codebase. Everything here is generated from the current working
> tree (2026-08-16) and reflects the code as it actually is today.

## Quick Start

1. **Read `architecture-map.md`** -- a layer-by-layer map of every module, every
   registered Tauri command, every WASM export, and every end-to-end flow
   (open, render, zoom, edit), with file:line anchors.

2. **Read `development.md`** -- build/test commands with exact gotchas (wasm
   target requirements, bindgen version matching, tauri:dev vs vite-only), a
   bug investigation decision tree, and key invariants to preserve.

3. **When you need a specific module**, look it up in the architecture map's
   module tables (layer 2-4). Every module has a 1-2 line responsibility
   statement and line count.

---

## Architecture Map (`architecture-map.md`)

Covers the full four-layer stack:

| Layer | Directory | Lines | What it does |
|---|---|---|---|
| UI shell | `index.html`, `src/main.ts` | ~300 | DOM events, ?file= params, button wiring |
| TS bridge | `src/bridge/` | ~12,000 | Runtime composition, render loop, zoom, edit |
| WASM crate | `crates/pdf-viewer-ui/` | ~5,500 | wasm-bindgen exports, layout contract, zoom host |
| Pure Rust lib | `crates/pdf-viewer-core/` | ~4,000 | Domain models, text-state, render plans |
| Desktop backend | `src-tauri/` | ~10,700 | 30 IPC commands, PDF parsing, font engine, read/write |

Plus: dead code inventory, duplicated logic, naming hazards.

## Development Guide (`development.md`)

- **Build commands** with gotchas (wasm-pack, bindgen version, vite-only limitations)
- **Test commands** for each layer with verified output
- **Bug investigation decision tree** (symptom -> layer -> call chain)
- **Common fix patterns** (zoom contract, fonts, edit patches, cache invalidation)
- **Branch guide** (main vs architecture-improvements vs fix branches)

---

## Documentation Inventory -- What's Current vs Stale

The `docs/` folder has ~45 files accumulated over the project's life. Most were
written before the August 2026 salvage refactoring and don't reflect the current
codebase. Below is an honest assessment.

### Current (use these)

| File | Date | Status | Notes |
|---|---|---|---|
| `docs/guide/README.md` | 2026-08-16 | **CURRENT** | This file |
| `docs/guide/architecture-map.md` | 2026-08-16 | **CURRENT** | Full module map from working tree |
| `docs/guide/development.md` | 2026-08-16 | **CURRENT** | Build/test/debug reference |
| `CONTEXT.md` | 2026-08-15 | **CURRENT** | Domain glossary (started on this branch) |
| `docs/superpowers/specs/2026-08-04-zoom-bug-fix-via-merge.md` | 2026-08-15 | **CURRENT** (closed) | Zoom spec, marked as implemented via salvage |
| `.scratch/UBIQUITOUS_LANGUAGE.md` | 2026-08-15 | **CURRENT** | Domain term definitions |
| `docs/runbooks/manual-zoom-e2e-verification.md` | 2026-08-16 | **CURRENT** | E2E runbook (on `fix/zoom-layout-tests-wasm-runnable`) |
| `docs/bug-postmortems/blue-block-overlay-artifact.md` | ? | **CURRENT** | Specific bug postmortem |
| `docs/bug-postmortems/vector-text-rendering-blank-issue.md` | ? | **CURRENT** | Specific bug postmortem |

### Partially stale (read for context, don't trust details)

| File | Date | Staleness |
|---|---|---|
| `docs/architecture-overview.md` | 2026-05-06 | Pre-salvage. High-level flow is broadly correct but module names
and relationships have changed (shallow modules deleted, TextState added). |
| `docs/architecture-principles.md` | 2026-05-06 | Principles still valid; examples reference old module names. |
| `docs/page-presentation-runtime-architecture.md` | 2026-06-03 | Detailed but predates zoom fix and module restructuring. |
| `docs/edit-save-architecture.md` | ? | Edit flow description still broadly correct; missing post-salvage
changes to `edit_commands.rs` and `region_materializer.rs`. |
| `docs/editor-api-architecture-proposal.md` | ? | 125KB -- a design proposal. Parts implemented, parts abandoned.
Read for historical context only. |
| `docs/route-b-core-redesign.md` | ? | 84KB -- design document for core extraction. Partially realized
(`pdf-viewer-core` exists). Still useful for understanding design intent. |
| `docs/development-guide.md` | 2026-05-06 | Old dev guide. Superseded by `docs/guide/development.md` above. |
| `docs/origin/` (10 files) | Various | Original design documents from project inception. Historical. |
| `docs/naming-and-architecture-refactor-plan.md` | ? | Refactor plan. Mostly executed; some items superseded by salvage. |
| `docs/naming-refactor-review-plan.md` | 27KB | Naming audit. Partially executed. |

### Stale (do not use for current development)

| File | Date | Why stale |
|---|---|---|
| `docs/architecture-audit.md` | 2026-05-09 | Pre-salvage audit. Module structure has changed significantly. |
| `docs/architecture-diagrams.md` | ? | Diagrams reference old module layout. |
| `docs/architecture-review.md` | ? | 22KB review of pre-salvage architecture. |
| `docs/api-audit.md` | ? | API audit of pre-salvage surface. |
| `docs/api-contract.md` | ? | API contract of pre-salvage surface. |
| `docs/method-inventory.md` | 268KB | Massive method inventory -- generated, entirely pre-salvage. |
| `docs/method-constraint-audit.md` | 106KB | Generated, pre-salvage. |
| `docs/structure-flow-audit.md` | ? | Pre-salvage structure audit. |
| `docs/framework-refactor-completion-plan.md` | ? | Refactor plan, mostly executed or superseded. |
| `docs/startup-performance-plan.md` | ? | Performance plan from early project. |
| `docs/ts-to-rust-migration-plan.md` | ? | Migration plan from TS-first era. No longer relevant. |
| `docs/nutrient-comparison.md` | ? | PDF library comparison from project inception. |
| `docs/nushell-divergence-report-2026-05-06.md` | 2026-05-06 | One-off shell report. |

### Archive (historical reference only)

| Directory | Contents |
|---|---|
| `docs/archive/` (10 files, 650KB+) | Early architecture plans, method mappings, naming conventions. All pre-salvage. |
| `docs/origin/` (10 files) | Original design documents. Historical context. |
| `docs/images/` (4 PNGs) | Architecture/editing/rendering/UI diagrams. May show old layout. |

### Scratch (working artifacts, not documentation)

| Directory | Contents |
|---|---|
| `.scratch/tickets/` (5 files) | Active ticket queue (01-05) |
| `.scratch/zoom-layout-refactor-verification/` | Zoom verification sub-tickets |
| `.scratch/unify-dispatch/` | TextState unification sub-tickets |
| `.scratch/wayfinder-map.md` + `wayfinder-README.md` | Wayfinder exploration from early architecture work |
| `.scratch/*.diff`, `.scratch/*.rs`, `.scratch/*.log` | Temporary working files |
