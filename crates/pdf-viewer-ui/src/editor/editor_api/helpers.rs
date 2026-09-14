use super::types::EditorSession;
use crate::editor::editor_types::TextBlockInfo;

// ── Internal helpers (not exported to JS) ───────────────────────

impl EditorSession {
    pub(crate) fn commit_draft_internal(&self) {
        use crate::editor::orchestrator::commit::commit_pending_edit_if_any;
        commit_pending_edit_if_any();
    }
}

pub(crate) fn build_frame_request() -> crate::present::plan_builder::FramePlanRequest {
    let zoom_state = crate::zoom::zoom_store::read_zoom_state();
    let viewer_session = crate::viewer::viewer_store::read_viewer_session();
    crate::present::plan_builder::FramePlanRequest {
        display_zoom: zoom_state.target_zoom.max(0.1),
        render_reason: String::new(),
        page_width: viewer_session.page_width.max(1.0),
        page_height: viewer_session.page_height.max(1.0),
        viewport_width: 0.0,
        viewport_height: 0.0,
        scroll_left: 0.0,
        scroll_top: 0.0,
        device_pixel_ratio: 1.0,
        max_zoom: 8.0,
        max_canvas_dim: 4096.0,
        timestamp_ms: 0.0,
        force_static_render_scale: None,
    }
}

pub(crate) fn resolve_target_at_page_point(
    page_x: f32,
    page_y: f32,
) -> Option<pdf_viewer_core::edit::bridge::ParagraphInteractionTarget> {
    use crate::page::page_store::with_page_state;
    use pdf_viewer_core::edit::bridge::collect_paragraph_interaction_targets;

    let targets = with_page_state(|state| {
        state
            .paint_plan
            .as_ref()
            .map(|plan| collect_paragraph_interaction_targets(plan, state.vector_model.as_ref()))
            .unwrap_or_default()
    });

    if targets.is_empty() {
        return None;
    }

    // Direct hit only (with 4px tolerance).
    // No nearest-neighbor fallback — clicking blank area must NOT open a distant paragraph.
    targets
        .iter()
        .find(|t| {
            page_x >= t.bbox.left - 4.0
                && page_x <= t.bbox.right + 4.0
                && page_y >= t.bbox.top - 4.0
                && page_y <= t.bbox.bottom + 4.0
        })
        .cloned()
}

pub(crate) fn collect_text_blocks() -> Vec<TextBlockInfo> {
    use crate::page::page_store::with_page_state;
    use pdf_viewer_core::edit::bridge::collect_paragraph_interaction_targets;

    with_page_state(|state| {
        state
            .paint_plan
            .as_ref()
            .map(|plan| {
                collect_paragraph_interaction_targets(plan, state.vector_model.as_ref())
                    .into_iter()
                    .map(|t| TextBlockInfo {
                        id: t.paragraph_id,
                        bbox_left: t.bbox.left,
                        bbox_top: t.bbox.top,
                        bbox_right: t.bbox.right,
                        bbox_bottom: t.bbox.bottom,
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}
