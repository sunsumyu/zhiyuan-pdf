// ─────────────────────────────────────────────────────────────────────────────
// Tile Manager — coordinates tile rendering across viewport and animation
//
// Responsibilities:
// - Render queue management with priority ordering
// - FrameToken concurrency control
// - Cache coordination (delegates to TileCache)
//
// Delegates viewport/animation scheduling to TileScheduler.
// See docs/adr/0003-tile-based-rendering.md
// ─────────────────────────────────────────────────────────────────────────────

use super::tile_scheduler::{TilePriority, TileRenderRequest, TileScheduler};
use super::tile_v2::{Tile, TileCache, TileKey, TileRect, TileState, TILE_SIZE};
use serde::{Deserialize, Serialize};

/// Tile manager state
#[derive(Debug)]
pub struct TileManager {
    /// Tile cache for storing rendered tiles
    pub cache: TileCache,
    /// Viewport and animation scheduler
    scheduler: TileScheduler,
    /// Queue of pending render requests
    render_queue: Vec<TileRenderRequest>,
}

impl TileManager {
    pub fn new() -> Self {
        Self {
            cache: TileCache::new(),
            scheduler: TileScheduler::new(),
            render_queue: Vec::new(),
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
        self.scheduler.update_viewport(
            page,
            zoom,
            dpr,
            viewport_x,
            viewport_y,
            viewport_width,
            viewport_height,
            frame_token,
        );
        self.schedule_viewport_tiles();
    }

    /// Start zoom animation
    pub fn start_animation(&mut self, target_zoom: f32) {
        self.scheduler.start_animation(target_zoom);
        // Mark all tiles as eligible for eviction during animation
        self.cache.mark_all_eligible_for_eviction();
    }

    /// Update animation state (called each frame)
    pub fn update_animation(&mut self, visual_zoom: f32, frame_token: u32) {
        self.scheduler.update_animation(visual_zoom, frame_token);

        // Incremental rendering during animation
        if self.scheduler.should_render_incremental() {
            self.schedule_incremental_tiles();
        }
    }

    /// End zoom animation
    pub fn end_animation(&mut self, frame_token: u32) {
        self.scheduler.end_animation(frame_token);
        // Schedule final high-resolution tiles
        self.schedule_viewport_tiles();
    }

    /// Get next render request from queue
    pub fn next_render_request(&mut self) -> Option<TileRenderRequest> {
        // Sort by priority (viewport first)
        self.render_queue.sort_by_key(|r| r.priority);

        let current_frame_token = self.scheduler.current_frame_token();

        // Find first request with valid frame token that still needs rendering
        while let Some(request) = self.render_queue.first() {
            let stale_token = request.frame_token != current_frame_token;
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
        let vp = self.scheduler.viewport_state();
        self.cache.get_viewport_tiles(
            vp.page,
            vp.zoom,
            vp.dpr,
            vp.x,
            vp.y,
            vp.width,
            vp.height,
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
            current_frame_token: self.scheduler.current_frame_token(),
            is_animating: self.scheduler.is_animating(),
        }
    }

    fn schedule_viewport_tiles(&mut self) {
        let requests = self.scheduler.schedule_viewport_tiles();
        for request in requests {
            self.schedule_tile(request.tile_key, request.priority);
        }
    }

    fn schedule_incremental_tiles(&mut self) {
        let requests = self.scheduler.schedule_incremental_tiles();
        for request in requests {
            self.schedule_tile(request.tile_key, request.priority);
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
            frame_token: self.scheduler.current_frame_token(),
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
