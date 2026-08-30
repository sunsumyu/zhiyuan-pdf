//! Anchor management — scroll/layout computation from pending zoom anchors.
//!
//! Anchors capture the cursor page point during a wheel gesture so that
//! zoom can be centered on the user's focus point.

use crate::present::plan_builder::AnchorViewportLayoutResult;
use pdf_viewer_core::render::zoom::animation::{
    compute_anchor_scroll_result, compute_anchor_viewport_layout_result, AnchorScrollRequest,
    AnchorScrollResult,
};
use crate::zoom::zoom_store::ZOOM_STATE;

pub fn take_pending_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorScrollResult> {
    ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let anchor = state.pending_anchor.take()?;
        Some(compute_anchor_scroll_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor.anchor_page_x,
            anchor.anchor_page_y,
            anchor.page_width,
            anchor.page_height,
            anchor.viewport_x,
            anchor.viewport_y,
        ))
    })
}

pub fn peek_pending_anchor_scroll(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorScrollResult> {
    ZOOM_STATE.with(|state| {
        let state = state.borrow();
        let anchor = state.pending_anchor.as_ref()?;
        Some(compute_anchor_scroll_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor.anchor_page_x,
            anchor.anchor_page_y,
            anchor.page_width,
            anchor.page_height,
            anchor.viewport_x,
            anchor.viewport_y,
        ))
    })
}

pub fn peek_pending_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorViewportLayoutResult> {
    ZOOM_STATE.with(|state| {
        let state = state.borrow();
        let anchor = state.pending_anchor.as_ref()?;
        Some(compute_anchor_viewport_layout_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor.anchor_page_x,
            anchor.anchor_page_y,
            anchor.page_width,
            anchor.page_height,
            anchor.viewport_x,
            anchor.viewport_y,
        ))
    })
}

pub fn take_pending_anchor_layout(
    display_width: f32,
    display_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> Option<AnchorViewportLayoutResult> {
    ZOOM_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let anchor = state.pending_anchor.take()?;
        Some(compute_anchor_viewport_layout_result(
            display_width,
            display_height,
            viewport_width,
            viewport_height,
            anchor.anchor_page_x,
            anchor.anchor_page_y,
            anchor.page_width,
            anchor.page_height,
            anchor.viewport_x,
            anchor.viewport_y,
        ))
    })
}

pub fn clear_pending_anchor() {
    ZOOM_STATE.with(|state| {
        state.borrow_mut().pending_anchor = None;
    });
}

pub fn resolve_anchor_scroll(request: &AnchorScrollRequest) -> AnchorScrollResult {
    compute_anchor_scroll_result(
        request.display_width,
        request.display_height,
        request.viewport_width,
        request.viewport_height,
        request.anchor_pdf_x,
        request.anchor_pdf_y,
        1.0,
        1.0,
        request.viewport_x,
        request.viewport_y,
    )
}
