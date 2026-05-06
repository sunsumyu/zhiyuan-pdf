use serde::{Deserialize, Serialize};

use crate::present::runtime::settle_render_frame;
use crate::render::workflow::RenderFrameTransition;
use crate::viewer::runtime::set_page_size;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RenderCommitResult {
    pub accepted: bool,
    pub next_frame: Option<crate::render::workflow::RenderFrameEnvelope>,
    pub page_width: f32,
    pub page_height: f32,
}

pub fn commit_render_result(
    frame_token: u32,
    rendered_zoom: f32,
    page_width: f32,
    page_height: f32,
) -> RenderCommitResult {
    let transition: RenderFrameTransition =
        settle_render_frame(frame_token, Some(rendered_zoom));
    if transition.accepted {
        set_page_size(page_width, page_height);
    }
    RenderCommitResult {
        accepted: transition.accepted,
        next_frame: transition.next_frame,
        page_width,
        page_height,
    }
}
