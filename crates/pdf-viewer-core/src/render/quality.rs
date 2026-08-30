// ─────────────────────────────────────────────────────────────────────────────
// Progressive Quality Rendering System
//
// ADR-0004: Always vector rendering, no CSS stretching.
// This module manages rendering quality levels for optimal performance:
// - Low: Fast rendering during animation (rough quality)
// - Medium: Balanced quality during过渡
// - High: Sharp rendering on settle (final quality)
//
// See docs/adr/0004-always-vector-rendering.md
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

/// Rendering quality levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderQuality {
    /// Fast rendering during animation (rough quality)
    /// - Lower DPI (0.5x-0.75x)
    /// - Simplified text rendering
    /// - Reduced detail
    Low = 0,

    /// Balanced quality during过渡
    /// - Standard DPI (1.0x)
    /// - Normal text rendering
    /// - Standard detail
    Medium = 1,

    /// Sharp rendering on settle (final quality)
    /// - High DPI (1.5x-2.0x)
    /// - Full text rendering
    /// - Complete detail
    High = 2,
}

impl Default for RenderQuality {
    fn default() -> Self {
        Self::Medium
    }
}

impl RenderQuality {
    /// Get DPI multiplier for this quality level
    pub fn dpi_multiplier(&self) -> f32 {
        match self {
            Self::Low => 0.75,
            Self::Medium => 1.0,
            Self::High => 1.5,
        }
    }

    /// Get text rendering quality (0.0-1.0)
    pub fn text_quality(&self) -> f32 {
        match self {
            Self::Low => 0.5,
            Self::Medium => 0.8,
            Self::High => 1.0,
        }
    }

    /// Get detail level (0.0-1.0)
    pub fn detail_level(&self) -> f32 {
        match self {
            Self::Low => 0.3,
            Self::Medium => 0.7,
            Self::High => 1.0,
        }
    }

    /// Get max render items per frame
    pub fn max_items_per_frame(&self) -> u32 {
        match self {
            Self::Low => 100,
            Self::Medium => 50,
            Self::High => 30,
        }
    }

    /// Get budget in milliseconds per frame
    pub fn budget_ms(&self) -> f64 {
        match self {
            Self::Low => 2.0,   // Very fast
            Self::Medium => 4.0, // Balanced
            Self::High => 8.0,   // Thorough
        }
    }
}

/// Quality state machine for animation
#[derive(Debug, Clone)]
pub struct QualityStateMachine {
    /// Current quality level
    current: RenderQuality,
    /// Target quality level (what we're transitioning to)
    target: RenderQuality,
    /// Frame count at current quality
    frame_count: u32,
    /// Quality transition threshold (frames before upgrade)
    transition_threshold: u32,
}

impl Default for QualityStateMachine {
    fn default() -> Self {
        Self {
            current: RenderQuality::Low,
            target: RenderQuality::High,
            frame_count: 0,
            transition_threshold: 5,
        }
    }
}

impl QualityStateMachine {
    /// Create a new quality state machine
    pub fn new() -> Self {
        Self::default()
    }

    /// Start animation (reset to low quality)
    pub fn start_animation(&mut self) {
        self.current = RenderQuality::Low;
        self.target = RenderQuality::High;
        self.frame_count = 0;
    }

    /// Update quality based on animation state
    pub fn update(&mut self, is_animating: bool, settled: bool) -> RenderQuality {
        self.frame_count += 1;

        if settled {
            // On settle, jump to high quality
            self.current = RenderQuality::High;
            self.target = RenderQuality::High;
            return self.current;
        }

        if is_animating {
            // During animation, progress from low to medium
            if self.frame_count >= self.transition_threshold {
                if self.current == RenderQuality::Low {
                    self.current = RenderQuality::Medium;
                    self.frame_count = 0;
                }
            }
        }

        self.current
    }

    /// Get current quality level
    pub fn current(&self) -> RenderQuality {
        self.current
    }

    /// Set target quality level
    pub fn set_target(&mut self, target: RenderQuality) {
        self.target = target;
    }

    /// Reset to low quality
    pub fn reset(&mut self) {
        self.current = RenderQuality::Low;
        self.frame_count = 0;
    }

    /// Upgrade quality (after successful render)
    pub fn upgrade(&mut self) {
        self.current = match self.current {
            RenderQuality::Low => RenderQuality::Medium,
            RenderQuality::Medium => RenderQuality::High,
            RenderQuality::High => RenderQuality::High,
        };
    }
}

/// Quality-aware render request
#[derive(Debug, Clone)]
pub struct QualityRenderRequest {
    /// Page index
    pub page: u16,
    /// Zoom level
    pub zoom: f32,
    /// Device pixel ratio
    pub dpr: f32,
    /// Quality level
    pub quality: RenderQuality,
    /// Frame token for concurrency control
    pub frame_token: u32,
    /// Viewport bounds
    pub viewport: ViewportBounds,
}

/// Viewport bounds for tile selection
#[derive(Debug, Clone)]
pub struct ViewportBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl QualityRenderRequest {
    /// Get effective DPI for this request
    pub fn effective_dpi(&self) -> f32 {
        self.dpr * self.quality.dpi_multiplier()
    }

    /// Get render budget in milliseconds
    pub fn budget_ms(&self) -> f64 {
        self.quality.budget_ms()
    }

    /// Get max items to render
    pub fn max_items(&self) -> u32 {
        self.quality.max_items_per_frame()
    }
}

/// Quality-aware tile key (extends existing TileKey)
#[derive(Debug, Clone)]
pub struct QualityTileKey {
    /// Base tile key
    pub page: u16,
    pub zoom: f32,
    pub x: i32,
    pub y: i32,
    /// Quality level
    pub quality: RenderQuality,
}

impl QualityTileKey {
    /// Create a cache key string
    pub fn cache_key(&self) -> String {
        format!(
            "{}|{:.4}|{}|{}|{:?}",
            self.page, self.zoom, self.x, self.y, self.quality
        )
    }

    /// Check if this tile can be reused for a different quality
    pub fn can_reuse_for(&self, target_quality: RenderQuality) -> bool {
        // Can reuse lower quality tiles as fallback
        // But same quality is not "reuse" - it's exact match
        self.quality < target_quality
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_levels_ordering() {
        assert!(RenderQuality::Low < RenderQuality::Medium);
        assert!(RenderQuality::Medium < RenderQuality::High);
    }

    #[test]
    fn test_quality_dpi_multipliers() {
        assert!((RenderQuality::Low.dpi_multiplier() - 0.75).abs() < 0.001);
        assert!((RenderQuality::Medium.dpi_multiplier() - 1.0).abs() < 0.001);
        assert!((RenderQuality::High.dpi_multiplier() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_quality_state_machine_animation() {
        let mut sm = QualityStateMachine::new();
        sm.start_animation();

        assert_eq!(sm.current(), RenderQuality::Low);

        // Simulate animation frames
        for _ in 0..10 {
            let quality = sm.update(true, false);
            if sm.frame_count >= 5 {
                assert_eq!(quality, RenderQuality::Medium);
            }
        }
    }

    #[test]
    fn test_quality_state_machine_settle() {
        let mut sm = QualityStateMachine::new();
        sm.start_animation();

        // Simulate animation then settle
        for _ in 0..10 {
            sm.update(true, false);
        }

        let quality = sm.update(false, true);
        assert_eq!(quality, RenderQuality::High);
    }

    #[test]
    fn test_quality_tile_key_reuse() {
        let low_key = QualityTileKey {
            page: 1,
            zoom: 1.0,
            x: 0,
            y: 0,
            quality: RenderQuality::Low,
        };

        assert!(low_key.can_reuse_for(RenderQuality::Medium));
        assert!(low_key.can_reuse_for(RenderQuality::High));
        assert!(!low_key.can_reuse_for(RenderQuality::Low));
    }

    #[test]
    fn test_quality_render_request() {
        let request = QualityRenderRequest {
            page: 1,
            zoom: 1.5,
            dpr: 2.0,
            quality: RenderQuality::High,
            frame_token: 1,
            viewport: ViewportBounds {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
        };

        assert!((request.effective_dpi() - 3.0).abs() < 0.001); // 2.0 * 1.5
        assert_eq!(request.budget_ms(), 8.0);
        assert_eq!(request.max_items(), 30);
    }
}