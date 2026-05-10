use serde::{Deserialize, Serialize};

use crate::present::plan_builder::FramePlanRequest;
use crate::present::present_store::schedule_render_frame_request;
use crate::render::workflow::RenderFrameEnvelope;
use crate::viewer::viewer_controller::note_document_mutation;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRefreshPipelineResult {
    pub revision: u64,
    pub render_frame: Option<RenderFrameEnvelope>,
}

pub fn request_document_refresh(
    reason: &str,
    frame_request: FramePlanRequest,
) -> DocumentRefreshPipelineResult {
    let revision = note_document_mutation(reason);
    let render_frame = schedule_render_frame_request(&frame_request);
    DocumentRefreshPipelineResult {
        revision,
        render_frame,
    }
}
