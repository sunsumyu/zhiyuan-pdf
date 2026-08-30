// ─────────────────────────────────────────────────────────────────────────────
// Tile manager host state — single shared TileManager instance for all tile
// facade entrypoints.
//
// IMPORTANT: the thread_local MUST live at module scope. Declaring it inside
// each facade function body creates *separate* statics per function, so
// update_viewport would write one instance while next_tile_request reads
// another (silently empty queue).
// ─────────────────────────────────────────────────────────────────────────────

use std::cell::RefCell;

use pdf_viewer_core::render::tile_manager::TileManager;

thread_local! {
    pub static TILE_MANAGER_HOST: RefCell<TileManager> = RefCell::new(TileManager::new());
}

/// Run `f` with the shared TileManager.
pub fn with_tile_manager<R>(f: impl FnOnce(&mut TileManager) -> R) -> R {
    TILE_MANAGER_HOST.with(|mgr| f(&mut mgr.borrow_mut()))
}
