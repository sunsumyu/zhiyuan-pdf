// ─────────────────────────────────────────────────────────────────────────────
// Tile Scheduler — viewport and animation tile scheduling
//
// Responsibilities:
// - Viewport tile priority rendering
// - Zoom animation incremental rendering
// - Tile grid calculation and priority assignment
//
// Pure logic — no cache management, no render queue.
// See docs/adr/0003-tile-based-rendering.md
// ─────────────────────────────────────────────────────────────────────────────

use super::tile_v2::{TileKey, TILE_SIZE};
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

/// A tile rendering request with priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileRenderRequest {
    pub tile_key: TileKey,
    pub priority: TilePriority,
    pub frame_token: u32,
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

/// Tile scheduler state
#[derive(Debug)]
pub struct TileScheduler {
    /// Viewport state for priority calculation
    viewport: ViewportState,
    /// Animation state for incremental rendering
    animation: AnimationState,
    /// Current frame token for concurrency control
    current_frame_token: u32,
}

impl TileScheduler {
    pub fn new() -> Self {
        Self {
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
            current_frame_token: 0,
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
    }

    /// Start zoom animation
    pub fn start_animation(&mut self, target_zoom: f32) {
        self.animation.is_animating = true;
        self.animation.target_zoom = target_zoom;
        self.animation.current_visual_zoom = self.viewport.zoom;
        self.animation.frame_count = 0;
    }

    /// Update animation state (called each frame)
    pub fn update_animation(&mut self, visual_zoom: f32, frame_token: u32) {
        if !self.animation.is_animating {
            return;
        }

        self.animation.current_visual_zoom = visual_zoom;
        self.animation.frame_count += 1;
        self.current_frame_token = frame_token;
    }

    /// End zoom animation
    pub fn end_animation(&mut self, frame_token: u32) {
        self.animation.is_animating = false;
        self.current_frame_token = frame_token;
    }

    /// Get current frame token
    pub fn current_frame_token(&self) -> u32 {
        self.current_frame_token
    }

    /// Check if currently animating
    pub fn is_animating(&self) -> bool {
        self.animation.is_animating
    }

    /// Get current animation state
    pub fn animation_state(&self) -> &AnimationState {
        &self.animation
    }

    /// Get current viewport state
    pub fn viewport_state(&self) -> &ViewportState {
        &self.viewport
    }

    /// Schedule viewport tiles (called on viewport update or animation end)
    pub fn schedule_viewport_tiles(&self) -> Vec<TileRenderRequest> {
        let page = self.viewport.page;
        let zoom = self.viewport.zoom;
        let dpr = self.viewport.dpr;

        // Calculate which tiles cover the viewport
        let start_tile_x = (self.viewport.x / TILE_SIZE).floor() as i32;
        let start_tile_y = (self.viewport.y / TILE_SIZE).floor() as i32;
        let end_tile_x = ((self.viewport.x + self.viewport.width) / TILE_SIZE).ceil() as i32;
        let end_tile_y = ((self.viewport.y + self.viewport.height) / TILE_SIZE).ceil() as i32;

        let mut requests = Vec::new();

        // Schedule viewport tiles with high priority
        for y in start_tile_y..=end_tile_y {
            for x in start_tile_x..=end_tile_x {
                let key = TileKey::new(page, zoom, dpr, x, y);
                requests.push(TileRenderRequest {
                    tile_key: key,
                    priority: TilePriority::Viewport,
                    frame_token: self.current_frame_token,
                });
            }
        }

        // Schedule near-viewport tiles with medium priority
        let margin = 1;
        for y in (start_tile_y - margin)..=(end_tile_y + margin) {
            for x in (start_tile_x - margin)..=(end_tile_x + margin) {
                if x < start_tile_x || x > end_tile_x || y < start_tile_y || y > end_tile_y {
                    let key = TileKey::new(page, zoom, dpr, x, y);
                    requests.push(TileRenderRequest {
                        tile_key: key,
                        priority: TilePriority::NearViewport,
                        frame_token: self.current_frame_token,
                    });
                }
            }
        }

        requests
    }

    /// Schedule incremental tiles during animation (called every N frames)
    pub fn schedule_incremental_tiles(&self) -> Vec<TileRenderRequest> {
        let page = self.viewport.page;
        let zoom = self.animation.current_visual_zoom;
        let dpr = self.viewport.dpr;

        // Calculate viewport tiles at current visual zoom
        let start_tile_x = (self.viewport.x / TILE_SIZE).floor() as i32;
        let start_tile_y = (self.viewport.y / TILE_SIZE).floor() as i32;
        let end_tile_x = ((self.viewport.x + self.viewport.width) / TILE_SIZE).ceil() as i32;
        let end_tile_y = ((self.viewport.y + self.viewport.height) / TILE_SIZE).ceil() as i32;

        let mut requests = Vec::new();

        // Schedule only viewport tiles during animation
        for y in start_tile_y..=end_tile_y {
            for x in start_tile_x..=end_tile_x {
                let key = TileKey::new(page, zoom, dpr, x, y);
                requests.push(TileRenderRequest {
                    tile_key: key,
                    priority: TilePriority::Viewport,
                    frame_token: self.current_frame_token,
                });
            }
        }

        requests
    }

    /// Check if animation frame should trigger incremental render
    pub fn should_render_incremental(&self) -> bool {
        self.animation.is_animating
            && self.animation.frame_count % self.animation.render_interval == 0
    }
}
