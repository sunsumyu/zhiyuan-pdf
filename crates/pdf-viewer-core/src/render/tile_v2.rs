// ─────────────────────────────────────────────────────────────────────────────
// Tile-based rendering system v2
//
// Replaces the base+detail layer architecture with pure tile-based rendering.
// Each tile is 512×512 logical pixels, rendered at device pixel ratio for
// high-resolution output. Uses independent LRU cache with priority-based
// rendering (viewport tiles first).
//
// See docs/adr/0003-tile-based-rendering.md
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fixed tile size in logical pixels
pub const TILE_SIZE: f32 = 512.0;

/// Maximum number of tiles to keep in cache
const MAX_TILE_CACHE_SIZE: usize = 64;

/// Tile key format: `{page}|{zoom}|{dpr}|{x}|{y}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileKey {
    pub page: u16,
    pub zoom: f32,
    pub dpr: f32,
    pub x: i32,
    pub y: i32,
}

// Manual Hash implementation for TileKey since f32 doesn't implement Hash
impl std::hash::Hash for TileKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.page.hash(state);
        // Convert f32 to bits for hashing
        self.zoom.to_bits().hash(state);
        self.dpr.to_bits().hash(state);
        self.x.hash(state);
        self.y.hash(state);
    }
}

// Manual PartialEq implementation for TileKey since f32 doesn't implement Eq
impl PartialEq for TileKey {
    fn eq(&self, other: &Self) -> bool {
        self.page == other.page
            && self.zoom.to_bits() == other.zoom.to_bits()
            && self.dpr.to_bits() == other.dpr.to_bits()
            && self.x == other.x
            && self.y == other.y
    }
}

impl Eq for TileKey {}

impl TileKey {
    pub fn new(page: u16, zoom: f32, dpr: f32, x: i32, y: i32) -> Self {
        Self {
            page,
            zoom,
            dpr,
            x,
            y,
        }
    }

    pub fn to_string_key(&self) -> String {
        format!(
            "{}|{:.4}|{:.4}|{}|{}",
            self.page, self.zoom, self.dpr, self.x, self.y
        )
    }
}

/// Tile state in the rendering pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TileState {
    /// Waiting to be rendered
    Pending,
    /// Currently being rendered
    Rendering,
    /// Successfully rendered
    Ready,
    /// Rendering failed
    Failed,
}

/// A single tile's metadata and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub key: TileKey,
    pub state: TileState,
    pub logical_rect: TileRect,
    pub pixel_rect: TileRect,
    pub last_used: u64,
}

/// Rectangle in logical or pixel coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Tile {
    pub fn new(key: TileKey, logical_rect: TileRect, dpr: f32) -> Self {
        let pixel_rect = TileRect {
            x: logical_rect.x * dpr,
            y: logical_rect.y * dpr,
            width: logical_rect.width * dpr,
            height: logical_rect.height * dpr,
        };

        Self {
            key,
            state: TileState::Pending,
            logical_rect,
            pixel_rect,
            last_used: 0,
        }
    }

    pub fn mark_rendering(&mut self) {
        self.state = TileState::Rendering;
    }

    pub fn mark_ready(&mut self) {
        self.state = TileState::Ready;
    }

    pub fn mark_failed(&mut self) {
        self.state = TileState::Failed;
    }

    pub fn touch(&mut self, timestamp: u64) {
        self.last_used = timestamp;
    }
}

/// LRU cache for tiles
#[derive(Debug)]
pub struct TileCache {
    tiles: HashMap<String, Tile>,
    access_order: Vec<String>,
    max_size: usize,
    current_timestamp: u64,
}

impl TileCache {
    pub fn new() -> Self {
        Self {
            tiles: HashMap::new(),
            access_order: Vec::new(),
            max_size: MAX_TILE_CACHE_SIZE,
            current_timestamp: 0,
        }
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            tiles: HashMap::new(),
            access_order: Vec::new(),
            max_size,
            current_timestamp: 0,
        }
    }

    /// Insert a tile into the cache
    pub fn insert(&mut self, tile: Tile) {
        let key = tile.key.to_string_key();

        // If tile already exists, update it
        if self.tiles.contains_key(&key) {
            self.tiles.insert(key.clone(), tile);
            self.touch_tile(&key);
            return;
        }

        // Evict if at capacity
        while self.tiles.len() >= self.max_size {
            self.evict_lru();
        }

        // Insert new tile
        self.tiles.insert(key.clone(), tile);
        self.access_order.push(key);
    }

    /// Get a tile from the cache
    pub fn get(&mut self, key: &TileKey) -> Option<&Tile> {
        let key_str = key.to_string_key();
        if self.tiles.contains_key(&key_str) {
            self.touch_tile(&key_str);
            self.tiles.get(&key_str)
        } else {
            None
        }
    }

    /// Get a mutable reference to a tile
    pub fn get_mut(&mut self, key: &TileKey) -> Option<&mut Tile> {
        let key_str = key.to_string_key();
        if self.tiles.contains_key(&key_str) {
            self.touch_tile(&key_str);
            self.tiles.get_mut(&key_str)
        } else {
            None
        }
    }

    /// Read-only peek without touching LRU order (for hot-path queries)
    pub fn peek(&self, key: &TileKey) -> Option<&Tile> {
        let key_str = key.to_string_key();
        self.tiles.get(&key_str)
    }

    /// Read-only state check without touching LRU order
    pub fn tile_state(&self, key: &TileKey) -> Option<TileState> {
        self.peek(key).map(|t| t.state.clone())
    }

    /// Check if a tile exists in the cache
    pub fn contains(&self, key: &TileKey) -> bool {
        let key_str = key.to_string_key();
        self.tiles.contains_key(&key_str)
    }

    /// Remove a tile from the cache
    pub fn remove(&mut self, key: &TileKey) -> Option<Tile> {
        let key_str = key.to_string_key();
        if let Some(tile) = self.tiles.remove(&key_str) {
            self.access_order.retain(|k| k != &key_str);
            Some(tile)
        } else {
            None
        }
    }

    /// Clear all tiles for a specific page
    pub fn clear_page(&mut self, page: u16) {
        let keys_to_remove: Vec<String> = self
            .tiles
            .keys()
            .filter(|key| {
                // Parse page from key format: `{page}|{zoom}|{dpr}|{x}|{y}`
                key.split('|').next().and_then(|p| p.parse::<u16>().ok()) == Some(page)
            })
            .cloned()
            .collect();

        for key in keys_to_remove {
            self.tiles.remove(&key);
            self.access_order.retain(|k| k != &key);
        }
    }

    /// Mark all tiles as eligible for eviction (used when zoom changes)
    pub fn mark_all_eligible_for_eviction(&mut self) {
        // Reset all timestamps to 0, making them all eligible for LRU eviction
        for tile in self.tiles.values_mut() {
            tile.last_used = 0;
        }
    }

    /// Get all pending tiles for a page, sorted by priority (viewport first)
    pub fn get_pending_tiles(&self, page: u16) -> Vec<&Tile> {
        self.tiles
            .values()
            .filter(|tile| tile.key.page == page && matches!(tile.state, TileState::Pending))
            .collect()
    }

    /// Get all tiles for a specific viewport at a given zoom level
    pub fn get_viewport_tiles(
        &self,
        page: u16,
        zoom: f32,
        dpr: f32,
        viewport_x: f32,
        viewport_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Vec<&Tile> {
        self.tiles
            .values()
            .filter(|tile| {
                tile.key.page == page
                    && (tile.key.zoom - zoom).abs() < 0.0001
                    && (tile.key.dpr - dpr).abs() < 0.0001
                    && tile.rect_intersects_viewport(
                        viewport_x,
                        viewport_y,
                        viewport_width,
                        viewport_height,
                    )
            })
            .collect()
    }

    /// Calculate tile grid for a page at given zoom and DPR
    pub fn calculate_tile_grid(
        page_width: f32,
        page_height: f32,
        zoom: f32,
        dpr: f32,
    ) -> Vec<TileKey> {
        let scaled_width = page_width * zoom;
        let scaled_height = page_height * zoom;
        let tiles_x = (scaled_width / TILE_SIZE).ceil() as i32;
        let tiles_y = (scaled_height / TILE_SIZE).ceil() as i32;

        let mut tiles = Vec::new();
        for y in 0..tiles_y {
            for x in 0..tiles_x {
                tiles.push(TileKey::new(0, zoom, dpr, x, y));
            }
        }
        tiles
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let mut pending = 0;
        let mut rendering = 0;
        let mut ready = 0;
        let mut failed = 0;

        for tile in self.tiles.values() {
            match tile.state {
                TileState::Pending => pending += 1,
                TileState::Rendering => rendering += 1,
                TileState::Ready => ready += 1,
                TileState::Failed => failed += 1,
            }
        }

        CacheStats {
            total: self.tiles.len(),
            pending,
            rendering,
            ready,
            failed,
            max_size: self.max_size,
        }
    }

    fn touch_tile(&mut self, key: &str) {
        self.current_timestamp += 1;
        if let Some(tile) = self.tiles.get_mut(key) {
            tile.touch(self.current_timestamp);
        }
        self.access_order.retain(|k| k != key);
        self.access_order.push(key.to_string());
    }

    fn evict_lru(&mut self) {
        if let Some(oldest_key) = self.access_order.first().cloned() {
            self.tiles.remove(&oldest_key);
            self.access_order.remove(0);
        }
    }
}

impl Tile {
    fn rect_intersects_viewport(
        &self,
        viewport_x: f32,
        viewport_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let tile_right = self.logical_rect.x + self.logical_rect.width;
        let tile_bottom = self.logical_rect.y + self.logical_rect.height;
        let viewport_right = viewport_x + viewport_width;
        let viewport_bottom = viewport_y + viewport_height;

        self.logical_rect.x < viewport_right
            && tile_right > viewport_x
            && self.logical_rect.y < viewport_bottom
            && tile_bottom > viewport_y
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total: usize,
    pub pending: usize,
    pub rendering: usize,
    pub ready: usize,
    pub failed: usize,
    pub max_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tile(page: u16, zoom: f32, dpr: f32, x: i32, y: i32) -> Tile {
        let key = TileKey::new(page, zoom, dpr, x, y);
        let logical_rect = TileRect {
            x: x as f32 * TILE_SIZE,
            y: y as f32 * TILE_SIZE,
            width: TILE_SIZE,
            height: TILE_SIZE,
        };
        Tile::new(key, logical_rect, dpr)
    }

    #[test]
    fn test_tile_key_serialization() {
        let key = TileKey::new(1, 1.5, 2.0, 0, 1);
        let serialized = key.to_string_key();
        assert_eq!(serialized, "1|1.5000|2.0000|0|1");
    }

    #[test]
    fn test_tile_cache_insert_and_get() {
        let mut cache = TileCache::new();
        let tile = create_test_tile(1, 1.0, 1.0, 0, 0);

        cache.insert(tile);
        assert!(cache.contains(&TileKey::new(1, 1.0, 1.0, 0, 0)));
        assert!(!cache.contains(&TileKey::new(1, 1.0, 1.0, 1, 0)));
    }

    #[test]
    fn test_tile_cache_lru_eviction() {
        let mut cache = TileCache::with_capacity(2);

        cache.insert(create_test_tile(1, 1.0, 1.0, 0, 0));
        cache.insert(create_test_tile(1, 1.0, 1.0, 1, 0));
        assert_eq!(cache.stats().total, 2);

        // Insert third tile, should evict first
        cache.insert(create_test_tile(1, 1.0, 1.0, 2, 0));
        assert_eq!(cache.stats().total, 2);
        assert!(!cache.contains(&TileKey::new(1, 1.0, 1.0, 0, 0)));
    }

    #[test]
    fn test_tile_cache_clear_page() {
        let mut cache = TileCache::new();

        cache.insert(create_test_tile(1, 1.0, 1.0, 0, 0));
        cache.insert(create_test_tile(1, 1.0, 1.0, 1, 0));
        cache.insert(create_test_tile(2, 1.0, 1.0, 0, 0));

        cache.clear_page(1);
        assert_eq!(cache.stats().total, 1);
        assert!(cache.contains(&TileKey::new(2, 1.0, 1.0, 0, 0)));
    }

    #[test]
    fn test_tile_state_transitions() {
        let mut tile = create_test_tile(1, 1.0, 1.0, 0, 0);

        assert!(matches!(tile.state, TileState::Pending));

        tile.mark_rendering();
        assert!(matches!(tile.state, TileState::Rendering));

        tile.mark_ready();
        assert!(matches!(tile.state, TileState::Ready));
    }

    #[test]
    fn test_viewport_tiles_query() {
        let mut cache = TileCache::new();

        // Create a 2x2 grid of tiles
        for y in 0..2 {
            for x in 0..2 {
                cache.insert(create_test_tile(1, 1.0, 1.0, x, y));
            }
        }

        // Query viewport that should intersect with tiles (0,0) and (1,0)
        let viewport_tiles = cache.get_viewport_tiles(1, 1.0, 1.0, 100.0, 100.0, 600.0, 200.0);
        assert_eq!(viewport_tiles.len(), 2);
    }

    #[test]
    fn test_tile_grid_calculation() {
        let tiles = TileCache::calculate_tile_grid(1024.0, 1024.0, 1.0, 1.0);
        assert_eq!(tiles.len(), 4); // 2x2 grid

        let tiles = TileCache::calculate_tile_grid(2048.0, 2048.0, 1.0, 1.0);
        assert_eq!(tiles.len(), 16); // 4x4 grid
    }
}