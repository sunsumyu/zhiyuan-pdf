use crate::infrastructure::multimedia::pdf::models::{LightPageKind, RenderObject, StyledRun};
pub fn classify_page(
    objects: &[RenderObject],
    text_runs: &[StyledRun],
    page_width: f32,
    page_height: f32,
) -> LightPageKind {
    let mut image_count = 0usize;
    let mut image_area_max = 0.0f32;

    for obj in objects {
        if let RenderObject::Image(img) = obj {
            image_count += 1;
            image_area_max = image_area_max.max(img.width.abs() * img.height.abs());
        }
    }

    let page_area = (page_width.abs() * page_height.abs()).max(1.0);
    let image_coverage = image_area_max / page_area;
    let text_count = text_runs.len();

    if image_count > 0 && image_coverage >= 0.55 && text_count <= 8 {
        LightPageKind::Scanned
    } else if image_count > 0 {
        LightPageKind::Mixed
    } else {
        LightPageKind::Text
    }
}
