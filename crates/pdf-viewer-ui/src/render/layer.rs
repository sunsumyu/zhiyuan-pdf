use serde::{Deserialize, Serialize};

use crate::present::plan_builder::FramePlanResult;

fn allow_detail_overlay_retention(frame_plan: &FramePlanResult) -> bool {
    frame_plan.use_viewport_tile
        && !matches!(
            frame_plan.render_reason.trim(),
            "editorVisibility" | "documentMutation"
        )
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LayerExecutionPlan {
    pub skip_render: bool,
    pub render_base_layer: bool,
    pub render_detail_layer: bool,
    pub show_detail_overlay: bool,
    pub retain_detail_overlay_during_base: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LayerPresentDecision {
    pub show_detail_overlay: bool,
    pub retain_detail_overlay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderLayerRuntimePlan {
    pub use_detail_layer: bool,
    pub cache_key: String,
    pub render_zoom: f32,
    pub prefer_progressive: bool,
    pub show_detail_overlay: bool,
    pub retain_detail_overlay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderExecutionPlan {
    pub skip_render: bool,
    pub base_layer: Option<RenderLayerRuntimePlan>,
    pub detail_layer: Option<RenderLayerRuntimePlan>,
}

pub fn resolve_layer_execution_plan(
    bundle_changed: bool,
    frame_plan: &FramePlanResult,
) -> LayerExecutionPlan {
    let render_base_layer = bundle_changed || frame_plan.render_base_layer;
    let render_detail_layer = frame_plan.show_detail_overlay && frame_plan.render_detail_layer;
    let allow_detail_overlay_retention = allow_detail_overlay_retention(frame_plan);
    LayerExecutionPlan {
        skip_render: !bundle_changed && !frame_plan.render_base_layer && !render_detail_layer,
        render_base_layer,
        render_detail_layer,
        show_detail_overlay: frame_plan.show_detail_overlay,
        retain_detail_overlay_during_base: !frame_plan.show_detail_overlay
            && allow_detail_overlay_retention,
    }
}

pub fn resolve_layer_present_decision(
    use_detail_layer: bool,
    frame_plan: &FramePlanResult,
) -> LayerPresentDecision {
    let allow_detail_overlay_retention = allow_detail_overlay_retention(frame_plan);
    LayerPresentDecision {
        show_detail_overlay: use_detail_layer && frame_plan.show_detail_overlay,
        retain_detail_overlay: !use_detail_layer && allow_detail_overlay_retention,
    }
}

pub fn resolve_render_execution_plan(
    bundle_changed: bool,
    frame_plan: &FramePlanResult,
) -> RenderExecutionPlan {
    let execution = resolve_layer_execution_plan(bundle_changed, frame_plan);
    if execution.skip_render {
        return RenderExecutionPlan {
            skip_render: true,
            base_layer: None,
            detail_layer: None,
        };
    }

    let base_layer = if execution.render_base_layer {
        let present = resolve_layer_present_decision(false, frame_plan);
        Some(RenderLayerRuntimePlan {
            use_detail_layer: false,
            cache_key: frame_plan.base_cache_key.clone(),
            render_zoom: frame_plan.base_render_zoom,
            prefer_progressive: frame_plan.prefer_progressive_base,
            show_detail_overlay: present.show_detail_overlay,
            retain_detail_overlay: present.retain_detail_overlay,
        })
    } else {
        None
    };

    let detail_layer = if execution.render_detail_layer {
        let present = resolve_layer_present_decision(true, frame_plan);
        Some(RenderLayerRuntimePlan {
            use_detail_layer: true,
            cache_key: frame_plan.detail_cache_key.clone(),
            render_zoom: frame_plan.render_zoom,
            prefer_progressive: frame_plan.prefer_progressive_detail,
            show_detail_overlay: present.show_detail_overlay,
            retain_detail_overlay: present.retain_detail_overlay,
        })
    } else {
        None
    };

    RenderExecutionPlan {
        skip_render: false,
        base_layer,
        detail_layer,
    }
}
