//! 矢量对象 bbox 与调试摘要（text/path/image 三类对象）。

use crate::models::{BoundingBox, VectorRenderObject};
use crate::render::viewport_culling::{path_object_bbox, styled_run_bbox};

use super::PreparedOverlay;

pub(super) fn vector_object_bbox(object: &VectorRenderObject) -> Option<BoundingBox> {
    match object {
        VectorRenderObject::Text(text) => {
            let mut combined: Option<BoundingBox> = None;
            for run in &text.runs {
                let run_bbox = styled_run_bbox(run);
                combined = Some(match combined {
                    Some(current) => BoundingBox {
                        left: current.left.min(run_bbox.left),
                        top: current.top.min(run_bbox.top),
                        right: current.right.max(run_bbox.right),
                        bottom: current.bottom.max(run_bbox.bottom),
                    },
                    None => run_bbox,
                });
            }
            combined
        }
        VectorRenderObject::Path(path) => path_object_bbox(path),
        VectorRenderObject::Image(image) => Some(BoundingBox {
            left: image.x,
            top: image.y,
            right: image.x + image.width.max(0.0),
            bottom: image.y + image.height.max(0.0),
        }),
    }
}

pub(super) fn vector_object_summary(object: &VectorRenderObject, object_index: usize) -> String {
    match object {
        VectorRenderObject::Text(text) => {
            let bbox = vector_object_bbox(object).unwrap_or_default();
            format!(
                "idx={} type=text id={} runs={} bbox={:.1},{:.1},{:.1},{:.1}",
                object_index,
                text.id,
                text.runs.len(),
                bbox.left,
                bbox.top,
                bbox.right,
                bbox.bottom,
            )
        }
        VectorRenderObject::Path(path) => {
            let bbox = vector_object_bbox(object).unwrap_or_default();
            format!(
                "idx={} type=path id={} stroke={} fill={} bbox={:.1},{:.1},{:.1},{:.1}",
                object_index,
                path.id,
                path.stroke_color.as_deref().unwrap_or("none"),
                path.fill_color.as_deref().unwrap_or("none"),
                bbox.left,
                bbox.top,
                bbox.right,
                bbox.bottom,
            )
        }
        VectorRenderObject::Image(image) => {
            let bbox = vector_object_bbox(object).unwrap_or_default();
            format!(
                "idx={} type=image id={} bbox={:.1},{:.1},{:.1},{:.1}",
                object_index, image.id, bbox.left, bbox.top, bbox.right, bbox.bottom,
            )
        }
    }
}

pub(super) fn record_overlay_object_summary(overlay: &mut PreparedOverlay, summary: String) {
    if overlay.object_summary_1.is_none() {
        overlay.object_summary_1 = Some(summary);
    } else if overlay.object_summary_2.is_none() {
        overlay.object_summary_2 = Some(summary);
    } else if overlay.object_summary_3.is_none() {
        overlay.object_summary_3 = Some(summary);
    }
}
