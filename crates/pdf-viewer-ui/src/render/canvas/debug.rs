//! Debug tracing helpers for canvas draw calls.
//!
//! These helpers log canvas drawing operations only when they intersect the
//! active editor shell bbox, providing targeted debug instrumentation
//! without per-draw overhead in the common case.

use crate::common::bbox::bbox_intersects;
use crate::editor::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event, EditorDebugField,
};
use crate::editor::mode::read_active_editor_state;
use crate::editor::replacement_region::paragraph_replacement_region;
use pdf_viewer_core::models::BoundingBox;

pub(crate) fn active_shell_bbox_for_debug() -> Option<BoundingBox> {
    read_active_editor_state()
        .map(|state| paragraph_replacement_region(&state.target).text_clear_bbox)
}

pub(crate) fn debug_bbox_intersects_active_shell(bbox: &BoundingBox) -> bool {
    active_shell_bbox_for_debug()
        .map(|shell| bbox_intersects(bbox, &shell))
        .unwrap_or(false)
}

pub(crate) fn debug_log_canvas_method(
    action: &str,
    object_type: &str,
    object_index: Option<usize>,
    object_id: Option<&str>,
    bbox: Option<BoundingBox>,
    extra: Vec<EditorDebugField>,
) {
    let intersects_shell = bbox
        .as_ref()
        .map(debug_bbox_intersects_active_shell)
        .unwrap_or(false);
    if !intersects_shell {
        return;
    }
    let mut details = vec![
        dbg_field("objectType", object_type),
        dbg_field("intersectsShell", intersects_shell),
    ];
    if let Some(index) = object_index {
        details.push(dbg_field("objectIndex", index));
    }
    if let Some(id) = object_id {
        details.push(dbg_field("objectId", id));
    }
    if let Some(bounds) = bbox {
        details.push(dbg_field(
            "bbox",
            format!(
                "{:.1},{:.1},{:.1},{:.1}",
                bounds.left, bounds.top, bounds.right, bounds.bottom
            ),
        ));
    }
    details.extend(extra);
    dbg_event("canvas.draw", action, details);
}
