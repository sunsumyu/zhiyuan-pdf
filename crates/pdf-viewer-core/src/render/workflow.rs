use serde::{Deserialize, Serialize};

use crate::render::plan_builder::FramePlanResult;
use crate::render::progressive::{ProgressiveRenderStart, ProgressiveRenderStep};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderFrameEnvelope {
    pub frame_token: u32,
    pub frame_plan: FramePlanResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderFrameTransition {
    pub accepted: bool,
    pub next_frame: Option<RenderFrameEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProgressiveRenderStartResult {
    pub started: bool,
    pub total_items: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProgressiveRenderStepResult {
    pub active: bool,
    pub completed: bool,
    pub processed_items: u32,
    pub remaining_items: u32,
}

pub fn build_render_frame_envelope(
    frame_token: u32,
    frame_plan: FramePlanResult,
) -> RenderFrameEnvelope {
    RenderFrameEnvelope {
        frame_token,
        frame_plan,
    }
}

pub fn frame_plan_requires_render(frame_plan: &FramePlanResult) -> bool {
    frame_plan.render_base_layer || frame_plan.render_detail_layer
}

pub fn frame_plan_needs_viewport_refresh(frame_plan: &FramePlanResult) -> bool {
    frame_plan.use_viewport_tile && frame_plan.render_detail_layer
}

pub fn frame_plans_share_render_work(left: &FramePlanResult, right: &FramePlanResult) -> bool {
    left.render_reason == right.render_reason
        && left.render_base_layer == right.render_base_layer
        && left.render_detail_layer == right.render_detail_layer
        && left.use_viewport_tile == right.use_viewport_tile
        && left.base_cache_key == right.base_cache_key
        && left.detail_cache_key == right.detail_cache_key
}

pub fn progressive_start_result(start: ProgressiveRenderStart) -> ProgressiveRenderStartResult {
    ProgressiveRenderStartResult {
        started: start.started,
        total_items: start.total_items.min(u32::MAX as usize) as u32,
    }
}

pub fn progressive_step_result(step: ProgressiveRenderStep) -> ProgressiveRenderStepResult {
    ProgressiveRenderStepResult {
        active: step.active,
        completed: step.completed,
        processed_items: step.processed_items.min(u32::MAX as usize) as u32,
        remaining_items: step.remaining_items.min(u32::MAX as usize) as u32,
    }
}
