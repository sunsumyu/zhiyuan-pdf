//! Committed frame management — queue and application of render pipeline frames.
//!
//! When the render pipeline commits a frame (new layout zoom + scroll),
//! this module queues it for the RAF loop to apply atomically. If the
//! RAF loop is not running (post-settle), the frame is applied immediately.

use std::cell::RefCell;

use crate::zoom::zoom_store::ZOOM_STATE;
use super::raf_dom_cache::{with_dom_cache, init_dom_cache};
use super::raf_settle::{cancel_settle_cleanup, schedule_settle_cleanup};
use super::raf_transform::LAST_APPLIED_SCALE;

/// Committed frame from the render pipeline.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedFrame {
    pub display_zoom: f32,
    pub render_zoom: f32,
    pub host_width: f32,
    pub host_height: f32,
    pub content_left: f32,
    pub content_top: f32,
    pub scroll_left: f32,
    pub scroll_top: f32,
}

// Queue of committed frames waiting to be applied.
thread_local! {
    static COMMITTED_FRAME_QUEUE: RefCell<Vec<CommittedFrame>> = RefCell::new(Vec::new());
}

/// Push a committed frame into the queue. Called from the render pipeline.
///
/// If the RAF loop is not running (it stops itself after settle), the frame
/// would otherwise sit in the queue until the next wheel gesture — apply it
/// immediately instead so final renders land on screen right away.
pub fn commit_rendered_frame(frame: CommittedFrame) {
    if !super::raf_loop::is_raf_loop_running() {
        // Make sure DOM refs exist before touching them.
        init_dom_cache();
        let visual_zoom = ZOOM_STATE.with(|s| s.borrow().visual_zoom);
        apply_committed_frame(frame, visual_zoom);
        return;
    }
    COMMITTED_FRAME_QUEUE.with(|q| q.borrow_mut().push(frame));
}

/// Pop the next pending committed frame from the queue (called by RAF tick).
pub fn pop_committed_frame() -> Option<CommittedFrame> {
    COMMITTED_FRAME_QUEUE.with(|q| q.borrow_mut().pop())
}

/// Apply a committed frame: update DOM scroll, box, and transform.
///
/// This is the core of ADR-0002's committed frame contract:
/// - Updates `last_rendered_zoom`簿记
/// - Applies SurfaceOps (SetBox/SetTransform) to the container
/// - Adjusts `LAST_APPLIED_SCALE` for the next transform computation
pub fn apply_committed_frame(frame: CommittedFrame, _current_visual_zoom: f32) {
    cancel_settle_cleanup();

    let display_zoom = if frame.display_zoom.is_finite() && frame.display_zoom > 0.0 {
        frame.display_zoom
    } else {
        return;
    };

    let (anchor_page, cursor, visual_zoom, target_zoom) = ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.last_rendered_zoom = if frame.render_zoom.is_finite() && frame.render_zoom > 0.0 {
            frame.render_zoom
        } else {
            display_zoom
        };
        s.recompute_css_scale();
        let (ap, cur) = match s.pending_anchor.as_ref() {
            Some(a) => ((a.anchor_page_x, a.anchor_page_y), (a.viewport_x, a.viewport_y)),
            None => ((0.0, 0.0), (0.0, 0.0)),
        };
        (ap, cur, s.visual_zoom, s.target_zoom)
    });
    let settled = (visual_zoom - target_zoom).abs() < 0.001;

    let committed = pdf_viewer_core::render::zoom::presentation::CommittedLayout {
        display_zoom,
        left: frame.content_left,
        top: frame.content_top,
        width: frame.host_width,
        height: frame.host_height,
        scroll_left: frame.scroll_left,
        scroll_top: frame.scroll_top,
    };
    let (ops, _new_layout) = pdf_viewer_core::render::zoom::presentation::committed_frame_ops(
        &committed, anchor_page, visual_zoom, cursor,
    );

    with_dom_cache(|dom| {
        let dom = match dom {
            Some(d) => d,
            None => return,
        };

        let _ = dom.scroll_container.set_scroll_left(frame.scroll_left as i32);
        let _ = dom.scroll_container.set_scroll_top(frame.scroll_top as i32);

        for op in ops {
            match op {
                pdf_viewer_core::render::zoom::presentation::SurfaceOp::SetBox {
                    surface: _,
                    left,
                    top,
                    width,
                    height,
                } => {
                    let style = dom.container.style();
                    let _ = style.set_property("width", &format!("{}px", width));
                    let _ = style.set_property("height", &format!("{}px", height));
                    let _ = style.set_property("left", &format!("{}px", left));
                    let _ = style.set_property("top", &format!("{}px", top));
                }
                pdf_viewer_core::render::zoom::presentation::SurfaceOp::SetTransform { transform, .. } => {
                    let _ = dom.container.style().set_property("transform", &transform);
                }
                _ => {}
            }
        }
    });

    if settled {
        LAST_APPLIED_SCALE.with(|s| *s.borrow_mut() = (f32::NAN, (0.0, 0.0)));
        if with_dom_cache(|d| d.is_some()) {
            schedule_settle_cleanup();
        }
    } else {
        let s_new = visual_zoom / display_zoom;
        LAST_APPLIED_SCALE.with(|s| *s.borrow_mut() = (s_new, cursor));
    }
}
