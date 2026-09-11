//! Committed frame management — queue and application of render pipeline frames.
//!
//! When the render pipeline commits a frame (new layout zoom + scroll),
//! this module queues it for the RAF loop to apply atomically. If the
//! RAF loop is not running (post-settle), the frame is applied immediately.
//!
//! CSS transform zoom has been removed. Container dimensions are set directly
//! via SetBox — no translate/scale compensation needed.

use std::cell::RefCell;

use super::raf_dom_cache::{init_dom_cache, with_dom_cache};
use super::raf_settle::{cancel_settle_cleanup, schedule_settle_cleanup};
use crate::zoom::zoom_store::ZOOM_STATE;

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
pub fn commit_rendered_frame(frame: CommittedFrame) {
    if !super::raf_loop::is_raf_loop_running() {
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

/// Apply a committed frame: set container dimensions and scroll position directly.
///
/// No CSS transform — container dimensions match the rendered zoom exactly.
pub fn apply_committed_frame(frame: CommittedFrame, _current_visual_zoom: f32) {
    cancel_settle_cleanup();

    let display_zoom = if frame.display_zoom.is_finite() && frame.display_zoom > 0.0 {
        frame.display_zoom
    } else {
        return;
    };

    // Check gesture state BEFORE mutating ZOOM_STATE — the visual_zoom/target_zoom
    // gap determines whether on_wheel_event or the RAF loop owns geometry.
    // Uses the same GESTURE_THRESHOLD as raf_loop.rs tick().
    let gap = ZOOM_STATE.with(|s| {
        let st = s.borrow();
        (st.visual_zoom - st.target_zoom).abs()
    });
    let settled = gap < 0.0008; // Matches advance_zoom_animation_state's settled threshold
    let in_gesture = gap > 0.001; // Matches GESTURE_THRESHOLD in raf_loop.rs

    ZOOM_STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.last_rendered_zoom = if frame.render_zoom.is_finite() && frame.render_zoom > 0.0 {
            frame.render_zoom
        } else {
            display_zoom
        };
        let (ap, cur) = match s.pending_anchor.as_ref() {
            Some(a) => (
                (a.anchor_page_x, a.anchor_page_y),
                (a.viewport_x, a.viewport_y),
            ),
            None => ((0.0, 0.0), (0.0, 0.0)),
        };
        // Only update visual_layout when settled — during gesture, on_wheel_event
        // owns visual_layout and the RAF loop must not overwrite it with
        // visualZoom-based layout (different zoom → different content_left → jump).
        if settled {
            s.visual_layout = Some(crate::zoom::zoom_store::VisualLayoutState {
                display_zoom,
                content_left: frame.content_left,
                content_top: frame.content_top,
            });
        }
        let _ = (ap, cur); // anchor available for future scroll computation
    });

    with_dom_cache(|dom| {
        let dom = match dom {
            Some(d) => d,
            None => return,
        };

        // Always set scroll position
        let _ = dom
            .scroll_container
            .set_scroll_left(frame.scroll_left as i32);
        let _ = dom.scroll_container.set_scroll_top(frame.scroll_top as i32);

        // During an active wheel gesture, on_wheel_event already positioned
        // the container using target_zoom. The render pipeline renders at
        // visualZoom (interpolated), so its width/height/content_left/content_top
        // would use a different zoom and fight the anchor position — observed as
        // the page jumping mid-gesture. Skip ALL geometry writes during gesture.
        if settled || !in_gesture {
            let style = dom.container.style();
            let _ = style.set_property("width", &format!("{}px", frame.host_width));
            let _ = style.set_property("height", &format!("{}px", frame.host_height));
            let _ = style.set_property("left", &format!("{}px", frame.content_left));
            let _ = style.set_property("top", &format!("{}px", frame.content_top));
        }
    });

    if settled {
        if with_dom_cache(|d| d.is_some()) {
            schedule_settle_cleanup();
        }
    }
}
