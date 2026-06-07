use crate::models::BoundingBox;

pub fn bbox_width(bbox: &BoundingBox) -> f32 {
    (bbox.right - bbox.left).max(0.0)
}

pub fn bbox_height(bbox: &BoundingBox) -> f32 {
    (bbox.bottom - bbox.top).max(0.0)
}

pub fn bbox_intersects(a: &BoundingBox, b: &BoundingBox) -> bool {
    a.left <= b.right && a.right >= b.left && a.top <= b.bottom && a.bottom >= b.top
}

pub fn union_bbox(a: &BoundingBox, b: &BoundingBox) -> BoundingBox {
    BoundingBox {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}
