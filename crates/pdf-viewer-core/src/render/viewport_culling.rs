use crate::geometry::bbox_ops::bbox_intersects;
use crate::models::{
    BoundingBox, GlyphPaintParagraph, GlyphPaintRegion, GlyphPaintRun, PageState, StyledRun,
    VectorImageObject, VectorPathObject, VectorRenderObject, VectorTextObject,
};

pub fn resolve_page_viewport_bbox(
    state: &PageState,
    page_width: f32,
    page_height: f32,
) -> BoundingBox {
    let zoom = if state.zoom.is_finite() && state.zoom > 0.0 {
        state.zoom
    } else {
        1.0
    };
    let safe_page_width = if page_width.is_finite() && page_width > 0.0 {
        page_width
    } else {
        1.0
    };
    let safe_page_height = if page_height.is_finite() && page_height > 0.0 {
        page_height
    } else {
        1.0
    };

    let left = (state.viewport_left.max(0.0) / zoom).clamp(0.0, safe_page_width);
    let right =
        ((state.viewport_left + state.viewport_width).max(0.0) / zoom).clamp(left, safe_page_width);
    let top = (state.viewport_top.max(0.0) / zoom).clamp(0.0, safe_page_height);
    let bottom =
        ((state.viewport_top + state.viewport_height).max(0.0) / zoom).clamp(top, safe_page_height);

    BoundingBox {
        left,
        top,
        right,
        bottom,
    }
}

pub fn glyph_run_intersects_viewport(run: &GlyphPaintRun, viewport: &BoundingBox) -> bool {
    bbox_intersects(&run.bbox, viewport)
}

pub fn paragraph_intersects_viewport(
    paragraph: &GlyphPaintParagraph,
    viewport: &BoundingBox,
) -> bool {
    bbox_intersects(&paragraph.bbox, viewport)
}

pub fn region_intersects_viewport(region: &GlyphPaintRegion, viewport: &BoundingBox) -> bool {
    bbox_intersects(&region.bbox, viewport)
}

pub fn vector_object_intersects_viewport(
    object: &VectorRenderObject,
    viewport: &BoundingBox,
) -> bool {
    match object {
        VectorRenderObject::Text(text) => text_object_intersects_viewport(text, viewport),
        VectorRenderObject::Path(path) => path_object_intersects_viewport(path, viewport),
        VectorRenderObject::Image(image) => image_object_intersects_viewport(image, viewport),
    }
}

fn text_object_intersects_viewport(text: &VectorTextObject, viewport: &BoundingBox) -> bool {
    text.runs
        .iter()
        .any(|run| bbox_intersects(&run_bbox(run), viewport))
}

fn path_object_intersects_viewport(path: &VectorPathObject, viewport: &BoundingBox) -> bool {
    path_bbox(path)
        .map(|bbox| bbox_intersects(&bbox, viewport))
        .unwrap_or(false)
}

fn image_object_intersects_viewport(image: &VectorImageObject, viewport: &BoundingBox) -> bool {
    let bbox = BoundingBox {
        left: image.x,
        top: image.y,
        right: image.x + image.width.max(0.0),
        bottom: image.y + image.height.max(0.0),
    };
    bbox_intersects(&bbox, viewport)
}

pub fn run_bbox(run: &StyledRun) -> BoundingBox {
    BoundingBox {
        left: run.tx,
        top: run.ty - run.font_size.max(0.0),
        right: run.tx + run.width.max(0.0),
        bottom: run.ty,
    }
}

pub fn path_bbox(path: &VectorPathObject) -> Option<BoundingBox> {
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

    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
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

#[cfg(test)]
mod tests {
    use super::resolve_page_viewport_bbox;
    use crate::geometry::bbox_ops::bbox_intersects;
    use crate::models::{BoundingBox, PageState};

    #[test]
    fn resolves_viewport_bbox_from_display_space() {
        let state = PageState {
            zoom: 2.0,
            viewport_left: 100.0,
            viewport_top: 200.0,
            viewport_width: 300.0,
            viewport_height: 400.0,
            ..Default::default()
        };

        let bbox = resolve_page_viewport_bbox(&state, 595.0, 842.0);
        assert!((bbox.left - 50.0).abs() < 0.01);
        assert!((bbox.right - 200.0).abs() < 0.01);
        assert!((bbox.top - 100.0).abs() < 0.01);
        assert!((bbox.bottom - 300.0).abs() < 0.01);
    }

    #[test]
    fn detects_bbox_intersection() {
        let a = BoundingBox {
            left: 10.0,
            top: 20.0,
            right: 50.0,
            bottom: 60.0,
        };
        let b = BoundingBox {
            left: 40.0,
            top: 30.0,
            right: 80.0,
            bottom: 70.0,
        };
        assert!(bbox_intersects(&a, &b));
    }
}
