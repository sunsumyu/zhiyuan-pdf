// ─────────────────────────────────────────────────────────────────────────────
// Tile Manager — coordinates tile rendering across viewport and animation
//
// Responsibilities:
// - Viewport tile priority rendering
// - Async queue management for tile rendering
// - Zoom animation incremental rendering
// - Integration with FrameToken concurrency control
//
// See docs/adr/0003-tile-based-rendering.md
// ─────────────────────────────────────────────────────────────────────────────

use super::tile_v2::{Tile, TileCache, TileKey, TileRect, TileState, TILE_SIZE};
use serde::{Deserialize, Serialize};

/// Rendering priority for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TilePriority {
    /// Viewport tiles (highest priority)
    Viewport = 0,
    /// Near-viewport tiles (medium priority)
    NearViewport = 1,
    /// Far-viewport tiles (lowest priority)
    FarViewport = 2,
}

/// A tile rendering request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileRenderRequest {
    pub tile_key: TileKey,
    pub priority: TilePriority,
    pub frame_token: u32,
}

/// Tile manager state
#[derive(Debug)]
pub struct TileManager {
    /// Tile cache for storing rendered tiles
    pub cache: TileCache,
    /// Queue of pending render requests
    render_queue: Vec<TileRenderRequest>,
    /// Current frame token for concurrency control
    current_frame_token: u32,
    /// Viewport state for priority calculation
    viewport: ViewportState,
    /// Animation state for incremental rendering
    animation: AnimationState,
}

/// Viewport state for tile priority calculation
#[derive(Debug, Clone)]
pub struct ViewportState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub page: u16,
    pub zoom: f32,
    pub dpr: f32,
}

/// Animation state for incremental rendering
#[derive(Debug, Clone)]
pub struct AnimationState {
    pub is_animating: bool,
    pub current_visual_zoom: f32,
    pub target_zoom: f32,
    pub render_interval: u32,
    pub frame_count: u32,
}

impl TileManager {
    pub fn new() -> Self {
        Self {
            cache: TileCache::new(),
            render_queue: Vec::new(),
            current_frame_token: 0,
            viewport: ViewportState {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                page: 0,
                zoom: 1.0,
                dpr: 1.0,
            },
            animation: AnimationState {
                is_animating: false,
                current_visual_zoom: 1.0,
                target_zoom: 1.0,
                render_interval: 3,
                frame_count: 0,
            },
        }
    }

    /// Update viewport state and schedule tile rendering
    pub fn update_viewport(
        &mut self,
        page: u16,
        zoom: f32,
        dpr: f32,
        viewport_x: f32,
        viewport_y: f32,
        viewport_width: f32,
        viewport_height: f32,
        frame_token: u32,
    ) {
        self.viewport = ViewportState {
            x: viewport_x,
            y: viewport_y,
            width: viewport_width,
            height: viewport_height,
            page,
            zoom,
            dpr,
        };

        self.current_frame_token = frame_token;
        self.schedule_viewport_tiles();
    }

    /// Start zoom animation
    pub fn start_animation(&mut self, target_zoom: f32) {
        self.animation.is_animating = true;
        self.animation.target_zoom = target_zoom;
        self.animation.current_visual_zoom = self.viewport.zoom;
        self.animation.frame_count = 0;

        // Mark all tiles as eligible for eviction during animation
        self.cache.mark_all_eligible_for_eviction();
    }

    /// Update animation state (called each frame)
    pub fn update_animation(&mut self, visual_zoom: f32, frame_token: u32) {
        if !self.animation.is_animating {
            return;
        }

        self.animation.current_visual_zoom = visual_zoom;
        self.animation.frame_count += 1;
        self.current_frame_token = frame_token;

        // Incremental rendering during animation
        if self.animation.frame_count % self.animation.render_interval == 0 {
            self.schedule_incremental_tiles();
        }
    }

    /// End zoom animation
    pub fn end_animation(&mut self, frame_token: u32) {
        self.animation.is_animating = false;
        self.current_frame_token = frame_token;

        // Schedule final high-resolution tiles
        self.schedule_viewport_tiles();
    }

    /// Get next render request from queue
    pub fn next_render_request(&mut self) -> Option<TileRenderRequest> {
        // Sort by priority (viewport first)
        self.render_queue.sort_by_key(|r| r.priority);

        // Find first request with valid frame token that still needs rendering
        while let Some(request) = self.render_queue.first() {
            let stale_token = request.frame_token != self.current_frame_token;
            let already_done = self
                .cache
                .peek(&request.tile_key)
                .map(|t| matches!(t.state, TileState::Ready | TileState::Rendering))
                .unwrap_or(false);
            if stale_token || already_done {
                // Stale or duplicate request, remove it
                self.render_queue.remove(0);
            } else {
                return Some(self.render_queue.remove(0));
            }
        }

        None
    }

    /// Mark a tile as rendering
    pub fn mark_rendering(&mut self, key: &TileKey) -> bool {
        if let Some(tile) = self.cache.get_mut(key) {
            tile.mark_rendering();
            true
        } else {
            false
        }
    }

    /// Mark a tile as ready
    pub fn mark_ready(&mut self, key: &TileKey) -> bool {
        if let Some(tile) = self.cache.get_mut(key) {
            tile.mark_ready();
            true
        } else {
            false
        }
    }

    /// Mark a tile as failed
    pub fn mark_failed(&mut self, key: &TileKey) -> bool {
        if let Some(tile) = self.cache.get_mut(key) {
            tile.mark_failed();
            true
        } else {
            false
        }
    }

    /// Flip a Rendering tile back to Pending after its render was dropped as
    /// stale (page/zoom moved on mid-flight) or failed, so cache state stays
    /// honest and the tile can be re-queued by a later viewport update.
    pub fn reset_stale_rendering(&mut self, key: &TileKey) -> bool {
        if let Some(tile) = self.cache.get_mut(key) {
            if matches!(tile.state, TileState::Rendering) {
                tile.state = TileState::Pending;
                return true;
            }
        }
        false
    }

    /// Check if a tile is ready for display (read-only, no LRU touch)
    pub fn is_tile_ready(&self, key: &TileKey) -> bool {
        self.cache
            .tile_state(key)
            .map(|s| matches!(s, TileState::Ready))
            .unwrap_or(false)
    }

    /// Get all ready tiles for the current viewport
    pub fn get_ready_viewport_tiles(&self) -> Vec<&Tile> {
        self.cache.get_viewport_tiles(
            self.viewport.page,
            self.viewport.zoom,
            self.viewport.dpr,
            self.viewport.x,
            self.viewport.y,
            self.viewport.width,
            self.viewport.height,
        )
    }

    /// Clear cache for a specific page
    pub fn clear_page(&mut self, page: u16) {
        self.cache.clear_page(page);
    }

    /// Get cache statistics
    pub fn stats(&self) -> TileManagerStats {
        let cache_stats = self.cache.stats();
        TileManagerStats {
            cache: cache_stats,
            queue_size: self.render_queue.len(),
            current_frame_token: self.current_frame_token,
            is_animating: self.animation.is_animating,
        }
    }

    fn schedule_viewport_tiles(&mut self) {
        let page = self.viewport.page;
        let zoom = self.viewport.zoom;
        let dpr = self.viewport.dpr;

        // Calculate which tiles cover the viewport
        let start_tile_x = (self.viewport.x / TILE_SIZE).floor() as i32;
        let start_tile_y = (self.viewport.y / TILE_SIZE).floor() as i32;
        let end_tile_x = ((self.viewport.x + self.viewport.width) / TILE_SIZE).ceil() as i32;
        let end_tile_y = ((self.viewport.y + self.viewport.height) / TILE_SIZE).ceil() as i32;

        // Schedule viewport tiles with high priority
        for y in start_tile_y..=end_tile_y {
            for x in start_tile_x..=end_tile_x {
                let key = TileKey::new(page, zoom, dpr, x, y);
                self.schedule_tile(key, TilePriority::Viewport);
            }
        }

        // Schedule near-viewport tiles with medium priority
        let margin = 1;
        for y in (start_tile_y - margin)..=(end_tile_y + margin) {
            for x in (start_tile_x - margin)..=(end_tile_x + margin) {
                if x < start_tile_x
                    || x > end_tile_x
                    || y < start_tile_y
                    || y > end_tile_y
                {
                    let key = TileKey::new(page, zoom, dpr, x, y);
                    self.schedule_tile(key, TilePriority::NearViewport);
                }
            }
        }
    }

    fn schedule_incremental_tiles(&mut self) {
        let page = self.viewport.page;
        let zoom = self.animation.current_visual_zoom;
        let dpr = self.viewport.dpr;

        // Calculate viewport tiles at current visual zoom
        let start_tile_x = (self.viewport.x / TILE_SIZE).floor() as i32;
        let start_tile_y = (self.viewport.y / TILE_SIZE).floor() as i32;
        let end_tile_x = ((self.viewport.x + self.viewport.width) / TILE_SIZE).ceil() as i32;
        let end_tile_y = ((self.viewport.y + self.viewport.height) / TILE_SIZE).ceil() as i32;

        // Schedule only viewport tiles during animation
        for y in start_tile_y..=end_tile_y {
            for x in start_tile_x..=end_tile_x {
                let key = TileKey::new(page, zoom, dpr, x, y);
                self.schedule_tile(key, TilePriority::Viewport);
            }
        }
    }

    fn schedule_tile(&mut self, key: TileKey, priority: TilePriority) {
        // Re-queue tiles that still need rendering. A Pending tile whose
        // request was dropped as stale (frame token moved on) must be
        // re-enqueued here — contains() alone would strand it forever, since
        // dropped queue entries are never re-examined. Failed tiles get one
        // retry per viewport update. Ready/Rendering tiles are never duplicated.
        let needs_render = match self.cache.peek(&key) {
            None => true,
            Some(tile) => matches!(tile.state, TileState::Pending | TileState::Failed),
        };
        if !needs_render {
            return;
        }
        if !self.cache.contains(&key) {
            let logical_rect = TileRect {
                x: key.x as f32 * TILE_SIZE,
                y: key.y as f32 * TILE_SIZE,
                width: TILE_SIZE,
                height: TILE_SIZE,
            };
            let tile = Tile::new(key.clone(), logical_rect, key.dpr);
            self.cache.insert(tile);
        }

        // Add to render queue
        let request = TileRenderRequest {
            tile_key: key,
            priority,
            frame_token: self.current_frame_token,
        };
        self.render_queue.push(request);
    }
}

/// Tile manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileManagerStats {
    pub cache: super::tile_v2::CacheStats,
    pub queue_size: usize,
    pub current_frame_token: u32,
    pub is_animating: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_manager_creation() {
        let manager = TileManager::new();
        let stats = manager.stats();
        assert_eq!(stats.cache.total, 0);
        assert_eq!(stats.queue_size, 0);
        assert!(!stats.is_animating);
    }

    #[test]
    fn test_viewport_update_schedules_tiles() {
        let mut manager = TileManager::new();
        manager.update_viewport(1, 1.0, 1.0, 0.0, 0.0, 1024.0, 768.0, 1);

        let stats = manager.stats();
        assert!(stats.queue_size > 0);
    }

    #[test]
    fn test_animation_state() {
        let mut manager = TileManager::new();
        manager.start_animation(2.0);

        let stats = manager.stats();
        assert!(stats.is_animating);

        manager.end_animation(2);
        let stats = manager.stats();
        assert!(!stats.is_animating);
    }

    #[test]
    fn test_tile_priority_ordering() {
        let viewport = TileRenderRequest {
            tile_key: TileKey::new(1, 1.0, 1.0, 0, 0),
            priority: TilePriority::Viewport,
            frame_token: 1,
        };

        let far = TileRenderRequest {
            tile_key: TileKey::new(1, 1.0, 1.0, 10, 10),
            priority: TilePriority::FarViewport,
            frame_token: 1,
        };

        assert!(viewport.priority < far.priority);
    }

    #[test]
    fn test_frame_token_concurrency() {
        let mut manager = TileManager::new();
        manager.update_viewport(1, 1.0, 1.0, 0.0, 0.0, 1024.0, 768.0, 1);

        // Get request with current token
        let request = manager.next_render_request();
        assert!(request.is_some());
        assert_eq!(request.unwrap().frame_token, 1);

        // Update token, old requests should be skipped
        manager.update_viewport(1, 1.0, 1.0, 0.0, 0.0, 1024.0, 768.0, 2);
        let mut found_stale = false;
        while let Some(request) = manager.next_render_request() {
            if request.frame_token != 2 {
                found_stale = true;
                break;
            }
        }
        // All remaining requests should have current token
        assert!(!found_stale);
    }

    #[test]
    fn test_pending_tile_not_stranded_across_viewport_updates() {
        // Regression: a Pending tile whose request was dropped as stale must be
        // re-enqueued by the next viewport update, never stranded forever.
        let mut manager = TileManager::new();
        manager.update_viewport(1, 1.0, 1.0, 0.0, 0.0, 512.0, 512.0, 1);

        // Drain one request but do NOT render it; then move the frame token.
        // The old-token request is dropped; the tile stays Pending in cache.
        let _ = manager.next_render_request();
        manager.update_viewport(1, 1.0, 1.0, 0.0, 0.0, 512.0, 512.0, 2);

        // The tile must be re-queued with the NEW token (not stranded).
        let request = manager.next_render_request();
        assert!(request.is_some());
        let request = request.unwrap();
        assert_eq!(request.frame_token, 2);
        assert_eq!(request.tile_key.x, 0);
        assert_eq!(request.tile_key.y, 0);
    }

    #[test]
    fn test_next_request_skips_ready_and_rendering_duplicates() {
        let mut manager = TileManager::new();
        manager.update_viewport(1, 1.0, 1.0, 0.0, 0.0, 512.0, 512.0, 1);

        // Render tile (0,0) to Ready.
        let key = TileKey::new(1, 1.0, 1.0, 0, 0);
        assert!(manager.next_render_request().is_some());
        manager.mark_rendering(&key);
        manager.mark_ready(&key);

        // Re-schedule at the same token must not hand out (0,0) again.
        manager.update_viewport(1, 1.0, 1.0, 0.0, 0.0, 512.0, 512.0, 1);
        while let Some(request) = manager.next_render_request() {
            assert!(
                !(request.tile_key.x == 0 && request.tile_key.y == 0),
                "Ready tile must not be re-queued"
            );
        }
    }

    #[test]
    fn test_reset_stale_rendering_flips_only_rendering() {
        let mut manager = TileManager::new();
        manager.update_viewport(1, 1.0, 1.0, 0.0, 0.0, 512.0, 512.0, 1);
        let key = TileKey::new(1, 1.0, 1.0, 0, 0);

        assert!(!manager.reset_stale_rendering(&key), "Pending stays Pending");

        assert!(manager.next_render_request().is_some());
        manager.mark_rendering(&key);
        assert!(manager.reset_stale_rendering(&key));
        assert!(!manager.is_tile_ready(&key));
        assert_eq!(manager.stats().cache.ready, 0);
    }
}