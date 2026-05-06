// ─────────────────────────────────────────────────────────────────────────────
// LEGACY: this file used to expose 26 raw snake_case wasm_bindgen entrypoints
// for the editor / document / review domains. Those entrypoints have been
// fully superseded by the canonical facades:
//   • document.* → `crate::document::facade`
//   • editor.*   → `crate::editor::facade`
//   • review.*   → `crate::review::facade`
//
// All TS callers were migrated in Phase 2D (see `progress.txt`). Removed.
// ─────────────────────────────────────────────────────────────────────────────
