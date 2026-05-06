use serde::{Deserialize, Serialize};

use crate::document::mutation_pipeline::request_document_refresh;
use crate::editor::runtime::build_region_text_patch;
use crate::present::plan_builder::FramePlanRequest;
use crate::render::workflow::RenderFrameEnvelope;
use crate::state_manager::apply_patch_with_history;
use pdf_viewer_core::text::search_replace::{replace_query_matches, SearchReplaceOptions};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegionTextReplaceRequest {
    pub page_index: u16,
    pub region_id: String,
    pub kind: String,
    pub original_text: String,
    pub query: String,
    pub replacement: String,
    #[serde(default)]
    pub replace_all_occurrences: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegionTextReplaceResult {
    pub applied_count: usize,
    pub skipped_count: usize,
    pub render_frame: Option<RenderFrameEnvelope>,
}

pub fn apply_region_text_replacements_tx(
    requests: Vec<RegionTextReplaceRequest>,
    frame_request: FramePlanRequest,
) -> RegionTextReplaceResult {
    let mut applied_count = 0usize;
    let mut skipped_count = 0usize;

    for request in requests {
        let Some(new_text) = replace_query_matches(
            &request.original_text,
            &request.query,
            &request.replacement,
            SearchReplaceOptions {
                case_sensitive: false,
                replace_all_occurrences: request.replace_all_occurrences,
            },
        ) else {
            skipped_count += 1;
            continue;
        };

        let Some(patch) = build_region_text_patch(
            request.page_index,
            &request.region_id,
            &request.kind,
            &request.original_text,
            new_text,
        ) else {
            skipped_count += 1;
            continue;
        };

        apply_patch_with_history(patch);
        applied_count += 1;
    }

    let render_frame = if applied_count > 0 {
        request_document_refresh("find-replace", frame_request).render_frame
    } else {
        None
    };

    RegionTextReplaceResult {
        applied_count,
        skipped_count,
        render_frame,
    }
}
