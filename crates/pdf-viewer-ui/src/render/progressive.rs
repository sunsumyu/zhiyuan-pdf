use pdf_viewer_core::models::BoundingBox;
use serde::{Deserialize, Serialize};

use crate::render::effective_page_plan::{
    build_effective_vector_render_plan, EffectiveVectorRenderEntry,
};
use crate::editor::paragraph_overlay::ParagraphRenderOverlay;
use crate::render::prepared_scene::PreparedPageScene;

const BASE_PROGRESSIVE_BUDGET_MS: f64 = 1.0;
const DETAIL_PROGRESSIVE_BUDGET_MS: f64 = 2.2;
const BASE_PROGRESSIVE_MAX_ITEMS: u32 = 6;
const DETAIL_PROGRESSIVE_MAX_ITEMS: u32 = 10;
const BASE_ONE_SHOT_THRESHOLD: u32 = 18;
const DETAIL_ONE_SHOT_THRESHOLD: u32 = 16;

#[derive(Debug, Clone, Default)]
pub struct ProgressiveRenderStart {
    pub started: bool,
    pub total_items: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ProgressiveRenderStep {
    pub active: bool,
    pub completed: bool,
    pub processed_items: usize,
    pub remaining_items: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressiveRenderPolicy {
    pub use_progressive: bool,
    pub budget_ms: f64,
    pub max_items: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ProgressiveVectorRenderTask {
    pub entries: Vec<EffectiveVectorRenderEntry>,
    pub next_index: usize,
    pub viewport_bbox: BoundingBox,
}

impl ProgressiveVectorRenderTask {
    pub fn build(
        vector_model: &pdf_viewer_core::models::VectorPageModel,
        prepared_scene: Option<&PreparedPageScene>,
        viewport_bbox: BoundingBox,
        overlays: &[ParagraphRenderOverlay],
    ) -> Option<Self> {
        let entries = build_effective_vector_render_plan(
            vector_model,
            prepared_scene,
            &viewport_bbox,
            overlays,
        );
        if entries.is_empty() {
            return None;
        }

        Some(Self {
            entries,
            next_index: 0,
            viewport_bbox,
        })
    }

    pub fn is_complete(&self) -> bool {
        self.next_index >= self.entries.len()
    }

    pub fn total_items(&self) -> usize {
        self.entries.len()
    }
}

pub fn resolve_progressive_render_policy(
    use_viewport_tile: bool,
    prefer_progressive_layer: bool,
    total_items: usize,
) -> ProgressiveRenderPolicy {
    if !prefer_progressive_layer {
        return ProgressiveRenderPolicy::default();
    }

    let total_items = total_items.min(u32::MAX as usize) as u32;
    let (budget_ms, max_items, one_shot_threshold) = if use_viewport_tile {
        (
            DETAIL_PROGRESSIVE_BUDGET_MS,
            DETAIL_PROGRESSIVE_MAX_ITEMS,
            DETAIL_ONE_SHOT_THRESHOLD,
        )
    } else {
        (
            BASE_PROGRESSIVE_BUDGET_MS,
            BASE_PROGRESSIVE_MAX_ITEMS,
            BASE_ONE_SHOT_THRESHOLD,
        )
    };

    ProgressiveRenderPolicy {
        use_progressive: total_items > one_shot_threshold,
        budget_ms,
        max_items,
    }
}
