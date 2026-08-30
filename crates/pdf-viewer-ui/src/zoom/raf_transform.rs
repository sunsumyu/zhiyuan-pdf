//! CSS transform application for the zoom RAF loop.
//!
//! Computes and applies the cursor-anchored transform for the current
//! animation state. Delegates math to the core presentation state machine.

use std::cell::RefCell;

use super::raf_dom_cache::with_dom_cache;
use crate::zoom::zoom_store::ZOOM_STATE;

thread_local! {
    /// Last applied (css_scale, (cursor_x, cursor_y)) to skip redundant DOM writes.
    pub(super) static LAST_APPLIED_SCALE: RefCell<(f32, (f32, f32))> = RefCell::new((f32::NAN, (0.0, 0.0)));
}

/// Apply the cursor-anchored transform for the current animation state.
///
/// Delegates the math to the core presentation state machine (ADR-0002):
/// `tx = cursor − left + scroll − anchor_page × visual_zoom`, which keeps the
/// anchor page point exactly under the cursor for ANY layout base.
pub(super) fn apply_css_transform() {
    let (anchor_page, cursor, visual_zoom, layout_zoom) = ZOOM_STATE.with(|state| {
        let s = state.borrow();
        let (ap, cur) = match s.pending_anchor.as_ref() {
            Some(a) => ((a.anchor_page_x, a.anchor_page_y), (a.viewport_x, a.viewport_y)),
            None => ((0.0, 0.0), (0.0, 0.0)),
        };
        let base = if s.last_rendered_zoom > 0.0 { s.last_rendered_zoom } else { 1.0 };
        (ap, cur, s.visual_zoom, base)
    });

    with_dom_cache(|dom| {
        let dom = match dom {
            Some(d) => d,
            None => {
                web_sys::console::warn_1(
                    &"[ZOOM-RAF] DOM cache empty — transform skipped".into(),
                );
                return;
            }
        };

        let style = dom.container.style();

        let layout = pdf_viewer_core::render::zoom::presentation::SurfaceLayout {
            layout_zoom,
            left: style
                .get_property_value("left")
                .ok()
                .and_then(|v| v.trim_end_matches("px").parse().ok())
                .unwrap_or(0.0),
            top: style
                .get_property_value("top")
                .ok()
                .and_then(|v| v.trim_end_matches("px").parse().ok())
                .unwrap_or(0.0),
            width: 0.0,
            height: 0.0,
        };
        let scroll = (
            dom.scroll_container.scroll_left() as f32,
            dom.scroll_container.scroll_top() as f32,
        );

        let op = pdf_viewer_core::render::zoom::presentation::transform_op(
            &layout, anchor_page, visual_zoom, cursor, scroll,
        );
        match op {
            pdf_viewer_core::render::zoom::presentation::SurfaceOp::SetTransform { transform, .. } => {
                let _ = style.set_property("transform", &transform);
            }
            _ => {}
        }
    });
}
