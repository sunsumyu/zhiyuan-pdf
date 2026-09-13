//! bbox 工具与图形 marker 检测（vector 对象 bbox、候选判定、detect_graphic_markers）。

use crate::edit::debug_trace::{
    editor_debug_field as dbg_field, record_editor_debug_event as dbg_event,
};
use crate::models::{
    BoundingBox, GraphicType, ParagraphEditContext, VectorPageModel, VectorRenderObject,
    VisualMarker,
};

fn vector_object_bbox(object: &VectorRenderObject) -> Option<BoundingBox> {
    match object {
        VectorRenderObject::Text(_) => None,
        VectorRenderObject::Path(path) => {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;

            for segment in &path.segments {
                for [x, y] in &segment.points {
                    min_x = min_x.min(*x);
                    min_y = min_y.min(*y);
                    max_x = max_x.max(*x);
                    max_y = max_y.max(*y);
                }
            }

            if min_x.is_finite()
                && min_y.is_finite()
                && max_x.is_finite()
                && max_y.is_finite()
                && max_x > min_x
                && max_y > min_y
            {
                Some(BoundingBox {
                    left: min_x,
                    top: min_y,
                    right: max_x,
                    bottom: max_y,
                })
            } else {
                None
            }
        }
        VectorRenderObject::Image(image) => {
            let width = image.width.max(0.0);
            let height = image.height.max(0.0);
            if width > 0.0 && height > 0.0 {
                Some(BoundingBox {
                    left: image.x,
                    top: image.y,
                    right: image.x + width,
                    bottom: image.y + height,
                })
            } else {
                None
            }
        }
    }
}

fn bbox_width(bbox: &BoundingBox) -> f32 {
    (bbox.right - bbox.left).max(0.0)
}

fn bbox_height(bbox: &BoundingBox) -> f32 {
    (bbox.bottom - bbox.top).max(0.0)
}

fn vertical_overlap_height(left: &BoundingBox, right: &BoundingBox) -> f32 {
    (left.bottom.min(right.bottom) - left.top.max(right.top)).max(0.0)
}

fn object_marker_kind(object: &VectorRenderObject) -> Option<GraphicType> {
    match object {
        VectorRenderObject::Image(_) => Some(GraphicType::Image),
        VectorRenderObject::Path(_) => Some(GraphicType::Path),
        VectorRenderObject::Text(_) => None,
    }
}

fn object_id(object: &VectorRenderObject) -> Option<&str> {
    match object {
        VectorRenderObject::Image(image) => Some(image.id.as_str()),
        VectorRenderObject::Path(path) => Some(path.id.as_str()),
        VectorRenderObject::Text(_) => None,
    }
}

fn graphic_marker_candidate(
    object_bbox: &BoundingBox,
    body_bbox: &BoundingBox,
    shell_bbox: &BoundingBox,
) -> bool {
    let width = bbox_width(object_bbox);
    let height = bbox_height(object_bbox);
    let body_height = bbox_height(body_bbox).max(1.0);
    if width < 2.0 || height < 2.0 || width > body_height * 2.5 || height > body_height * 2.5 {
        return false;
    }

    let object_center_y = (object_bbox.top + object_bbox.bottom) * 0.5;
    let body_center_y = (body_bbox.top + body_bbox.bottom) * 0.5;
    let center_tolerance = (body_height * 0.7).max(height * 0.7).max(3.0);
    if (object_center_y - body_center_y).abs() > center_tolerance {
        return false;
    }

    let overlap = vertical_overlap_height(object_bbox, body_bbox);
    if overlap < height.min(body_height) * 0.25 {
        return false;
    }

    let max_right = body_bbox.left + (body_height * 0.35).max(4.0);
    let min_left = shell_bbox.left - (body_height * 2.0).max(24.0);
    object_bbox.left >= min_left && object_bbox.right <= max_right
}

pub(super) fn detect_graphic_markers(
    vector_model: Option<&VectorPageModel>,
    body_session: &ParagraphEditContext,
    shell_bbox: &BoundingBox,
) -> Vec<VisualMarker> {
    let Some(vector_model) = vector_model else {
        return Vec::new();
    };
    let body_bbox = body_session.paragraph.bbox;
    if bbox_width(&body_bbox) <= 0.0 || bbox_height(&body_bbox) <= 0.0 {
        return Vec::new();
    }

    let markers: Vec<VisualMarker> = vector_model
        .objects
        .iter()
        .enumerate()
        .filter_map(|(object_index, object)| {
            let object_type = object_marker_kind(object)?;
            let bbox = vector_object_bbox(object)?;
            if !graphic_marker_candidate(&bbox, &body_bbox, shell_bbox) {
                return None;
            }
            Some(VisualMarker::from_graphic(
                object_index,
                object_type,
                object_id(object).unwrap_or_default().to_string(),
                bbox,
            ))
        })
        .collect();

    if !markers.is_empty() {
        dbg_event(
            "document-plan.graphic-marker",
            "detected",
            vec![
                dbg_field("paragraphId", body_session.paragraph.id.as_str()),
                dbg_field("count", markers.len()),
                dbg_field(
                    "objectIndices",
                    format!(
                        "{:?}",
                        markers
                            .iter()
                            .flat_map(|marker| marker.object_indices.iter().copied())
                            .collect::<Vec<_>>()
                    ),
                ),
            ],
        );
    }

    markers
}
