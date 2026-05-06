use crate::render::progressive::{resolve_progressive_render_policy, ProgressiveRenderPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProgressiveRenderPolicyRequest {
    pub use_viewport_tile: bool,
    pub prefer_progressive_layer: bool,
    pub total_items: u32,
}

pub fn resolve_progressive_render_policy_request(
    request: ProgressiveRenderPolicyRequest,
) -> ProgressiveRenderPolicy {
    resolve_progressive_render_policy(
        request.use_viewport_tile,
        request.prefer_progressive_layer,
        request.total_items as usize,
    )
}
