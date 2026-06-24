//! 装饰性路径/图像抑制判断 — 从 ui::render::path_suppression 迁入。

use crate::edit::replacement_region::ParagraphReplacementRegion;
use crate::geometry::bbox_ops::bbox_intersects;
use crate::models::{BoundingBox, VectorImageObject, VectorRenderObject};
use crate::render::viewport_culling::path_bbox;

pub fn should_suppress(
    object: &VectorRenderObject,
    replacement_region: &ParagraphReplacementRegion,
    suppression_bbox: &BoundingBox,
) -> Option<String> {
    match object {
        VectorRenderObject::Path(path) => {
            let path_bbox = path_bbox(path)?;
            let stroke_pad = if path.stroke {
                (path.stroke_width.max(0.0) * 0.5).max(0.5)
            } else {
                0.0
            };
            let visual_bbox = BoundingBox {
                left: path_bbox.left - stroke_pad,
                top: path_bbox.top - stroke_pad,
                right: path_bbox.right + stroke_pad,
                bottom: path_bbox.bottom + stroke_pad,
            };

            if !source_row_decoration_matches(
                &visual_bbox,
                replacement_region,
                suppression_bbox,
                allowed_path_height(object),
            ) {
                return None;
            }

            Some(format!(
                "type=path id={} {} stroke={} strokeColor={} fillColor={}",
                path.id,
                source_row_decoration_summary(&visual_bbox, replacement_region, suppression_bbox),
                path.stroke_width,
                path.stroke_color.as_deref().unwrap_or("none"),
                path.fill_color.as_deref().unwrap_or("none")
            ))
        }
        VectorRenderObject::Image(image) => {
            let image_bbox = image_object_bbox(image);
            if !source_row_decoration_matches(
                &image_bbox,
                replacement_region,
                suppression_bbox,
                30.0_f32,
            ) {
                return None;
            }

            Some(format!(
                "type=image id={} {}",
                image.id,
                source_row_decoration_summary(&image_bbox, replacement_region, suppression_bbox)
            ))
        }
        VectorRenderObject::Text(_) => None,
    }
}

fn image_object_bbox(image: &VectorImageObject) -> BoundingBox {
    BoundingBox {
        left: image.x,
        top: image.y,
        right: image.x + image.width.max(0.0),
        bottom: image.y + image.height.max(0.0),
    }
}

fn allowed_path_height(object: &VectorRenderObject) -> f32 {
    let VectorRenderObject::Path(path) = object else {
        return 0.0;
    };

    let has_fill = path.fill && path.fill_color.is_some();
    if has_fill && path.stroke_width <= 0.5 {
        30.0_f32
    } else {
        (path.stroke_width.max(0.0) * 6.0).max(12.0)
    }
}

fn source_row_decoration_matches(
    object_bbox: &BoundingBox,
    replacement_region: &ParagraphReplacementRegion,
    suppression_bbox: &BoundingBox,
    max_line_height: f32,
) -> bool {
    if !bbox_intersects(object_bbox, suppression_bbox) {
        return false;
    }

    let object_width = (object_bbox.right - object_bbox.left).max(0.0);
    let object_height = (object_bbox.bottom - object_bbox.top).max(0.0);
    if object_width < 12.0 && object_height < 12.0 {
        return false;
    }

    let overlap_height = bbox_overlap_height(object_bbox, suppression_bbox);
    let overlaps_row_band = object_bbox.bottom >= replacement_region.row_band_top
        && object_bbox.top <= replacement_region.row_band_bottom
        && overlap_height > 0.0;
    let row_overlap_height = row_overlap_height(object_bbox, replacement_region);
    let row_overlap_ratio = row_overlap_height / object_height.max(1.0);
    let object_has_material_row_overlap = row_overlap_height >= 1.0 || row_overlap_ratio >= 0.25;
    let suppression_height = (suppression_bbox.bottom - suppression_bbox.top).max(1.0);
    let allowed_object_height = max_line_height
        .max(1.0)
        .min((suppression_height * 1.6).max(4.0));
    let overlaps_shell_materially =
        bbox_overlap_width(object_bbox, suppression_bbox) >= 8.0 && overlap_height > 0.0;

    object_width >= 12.0
        && object_height <= allowed_object_height
        && overlaps_shell_materially
        && overlaps_row_band
        && object_has_material_row_overlap
}

fn source_row_decoration_summary(
    object_bbox: &BoundingBox,
    replacement_region: &ParagraphReplacementRegion,
    suppression_bbox: &BoundingBox,
) -> String {
    let object_width = (object_bbox.right - object_bbox.left).max(0.0);
    let object_height = (object_bbox.bottom - object_bbox.top).max(0.0);
    let suppression_width = (suppression_bbox.right - suppression_bbox.left).max(1.0);
    let overlap_width = bbox_overlap_width(object_bbox, suppression_bbox);
    let overlap_height = bbox_overlap_height(object_bbox, suppression_bbox);
    let row_overlap_height = row_overlap_height(object_bbox, replacement_region);

    format!(
        "bbox={:.1},{:.1},{:.1},{:.1} size={:.1}x{:.1} overlap={:.1}x{:.1} suppressionWidth={:.1} rowBand={:.1}-{:.1} rowOverlap={:.1}",
        object_bbox.left,
        object_bbox.top,
        object_bbox.right,
        object_bbox.bottom,
        object_width,
        object_height,
        overlap_width,
        overlap_height,
        suppression_width,
        replacement_region.row_band_top,
        replacement_region.row_band_bottom,
        row_overlap_height,
    )
}

fn bbox_overlap_width(left: &BoundingBox, right: &BoundingBox) -> f32 {
    (left.right.min(right.right) - left.left.max(right.left)).max(0.0)
}

fn bbox_overlap_height(left: &BoundingBox, right: &BoundingBox) -> f32 {
    (left.bottom.min(right.bottom) - left.top.max(right.top)).max(0.0)
}

fn row_overlap_height(
    object_bbox: &BoundingBox,
    replacement_region: &ParagraphReplacementRegion,
) -> f32 {
    (object_bbox.bottom.min(replacement_region.row_band_bottom)
        - object_bbox.top.max(replacement_region.row_band_top))
    .max(0.0)
}

#[cfg(test)]
mod tests {
    use super::should_suppress;
    use crate::edit::active_target::ActiveEditorTarget;
    use crate::edit::replacement_region::build_region;
    use crate::models::{
        BoundingBox, LayoutParagraph, ParagraphEditContext, VectorImageObject, VectorRenderObject,
    };

    fn replacement_target() -> ActiveEditorTarget {
        let mut target = ActiveEditorTarget::default();
        target.scene.shell_bbox = BoundingBox {
            left: 40.0,
            top: 96.0,
            right: 360.0,
            bottom: 116.0,
        };
        *target.scene.body_session_mut() = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 90.0,
                top: 100.0,
                right: 330.0,
                bottom: 112.0,
            },
            paragraph: LayoutParagraph::default(),
        };
        target
    }

    fn row_image(id: &str, y: f32, height: f32) -> VectorRenderObject {
        VectorRenderObject::Image(VectorImageObject {
            id: id.to_string(),
            x: 0.0,
            y,
            width: 420.0,
            height,
            z_index: 1,
        })
    }

    #[test]
    fn suppresses_thin_decoration() {
        let target = replacement_target();
        let region = build_region(&target);
        let suppression_bbox = region.row_suppression_bbox(420.0);
        let object = row_image("blue-image-row", 101.0, 8.0);

        assert!(should_suppress(
            &object,
            &region,
            &suppression_bbox
        )
        .is_some());
    }

    #[test]
    fn keeps_normal_image() {
        let target = replacement_target();
        let region = build_region(&target);
        let suppression_bbox = region.row_suppression_bbox(420.0);
        let object = row_image("normal-image", 96.0, 42.0);

        assert!(should_suppress(
            &object,
            &region,
            &suppression_bbox
        )
        .is_none());
    }
}
