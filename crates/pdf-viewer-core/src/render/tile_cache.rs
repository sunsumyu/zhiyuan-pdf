// ─────────────────────────────────────────────────────────────────────────────
// Tile cache — backward-compatible re-exports from tile_cache_legacy.
//
// All existing consumers (plan_builder, present_store, workflow) continue to
// use `tile_cache::*` without changes. New code should use `tile_v2::*` and
// `tile_manager::*` directly.
// ─────────────────────────────────────────────────────────────────────────────

pub use super::tile_cache_legacy::*;
