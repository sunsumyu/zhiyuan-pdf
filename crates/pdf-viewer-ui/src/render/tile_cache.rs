// ─────────────────────────────────────────────────────────────────────────────
// Tile cache re-exports — bridges core tile modules into the UI layer.
//
// Legacy (base+detail) types re-exported for backward compatibility.
// New tile_v2 and tile_manager types available for new code.
// ─────────────────────────────────────────────────────────────────────────────

// Legacy base+detail tile system (backward compatible)
pub use pdf_viewer_core::render::tile_cache::*;

// New tile-based rendering system
pub use pdf_viewer_core::render::tile_v2::*;
pub use pdf_viewer_core::render::tile_manager::*;
