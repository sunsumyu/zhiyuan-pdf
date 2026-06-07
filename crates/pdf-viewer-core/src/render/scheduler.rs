/// Pure data structures for the render frame scheduler.
/// The thread_local state and scheduling logic remain in the UI crate.

#[derive(Debug, Clone)]
pub struct HostRenderState<TPlan> {
    pub next_frame_token: u32,
    pub active_frame_token: u32,
    pub committed_frame_token: u32,
    pub in_flight_frame_token: u32,
    pub queued_frame_token: u32,
    pub in_flight_frame_plan: Option<TPlan>,
    pub queued_frame_plan: Option<TPlan>,
}

impl<TPlan> Default for HostRenderState<TPlan> {
    fn default() -> Self {
        Self {
            next_frame_token: 1,
            active_frame_token: 0,
            committed_frame_token: 0,
            in_flight_frame_token: 0,
            queued_frame_token: 0,
            in_flight_frame_plan: None,
            queued_frame_plan: None,
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderFrameEnvelope<TPlan> {
    pub frame_token: u32,
    pub frame_plan: TPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFrameTransition<TPlan> {
    pub accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settled_frame_plan: Option<TPlan>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub next_frame: Option<RenderFrameEnvelope<TPlan>>,
}

impl<TPlan> Default for RenderFrameTransition<TPlan> {
    fn default() -> Self {
        Self {
            accepted: false,
            settled_frame_plan: None,
            next_frame: None,
        }
    }
}

pub fn allocate_render_frame_token<TPlan>(state: &mut HostRenderState<TPlan>) -> u32 {
    let token = state.next_frame_token.max(1);
    state.next_frame_token = token.wrapping_add(1).max(1);
    token
}
