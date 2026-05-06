use pdf_viewer_core::models::BoundingBox;

use crate::editor::source_geometry::source_session_visual_bbox;
use crate::editor::session::ActiveEditorTarget;
use crate::utils::bbox::{bbox_height, bbox_width, union_bbox};

#[derive(Debug, Clone, Copy)]
pub struct ParagraphReplacementRegion {
    pub shell_bbox: BoundingBox,
    pub source_bbox: BoundingBox,
    pub text_clear_bbox: BoundingBox,
    pub path_suppression_bbox: BoundingBox,
    pub row_band_top: f32,
    pub row_band_bottom: f32,
}

impl ParagraphReplacementRegion {
    pub fn row_path_suppression_bbox_for_page_width(&self, page_width: f32) -> BoundingBox {
        let right = page_width
            .max(self.shell_bbox.right)
            .max(self.text_clear_bbox.right)
            .max(self.path_suppression_bbox.right)
            .max(1.0);
        BoundingBox {
            left: 0.0,
            top: self.row_band_top,
            right,
            bottom: self.row_band_bottom,
        }
    }

    pub fn viewport_cull_bbox_for_page_width(&self, page_width: f32) -> BoundingBox {
        union_bbox(
            &union_bbox(&self.shell_bbox, &self.text_clear_bbox),
            &self.row_path_suppression_bbox_for_page_width(page_width),
        )
    }

    pub fn cache_invalidation_bbox_for_page_width(&self, page_width: f32) -> BoundingBox {
        self.viewport_cull_bbox_for_page_width(page_width)
    }
}

pub fn paragraph_replacement_region(
    target: &ActiveEditorTarget,
) -> ParagraphReplacementRegion {
    let shell_bbox = target.scene.shell_bbox;
    let source_bbox = preferred_source_bbox(target);
    let row_height = bbox_height(&source_bbox).max(1.0);

    let text_x_pad = 4.0;
    let text_top_pad = (row_height * 0.08).clamp(0.5, 1.5);
    let text_bottom_pad = (row_height * 0.25).clamp(2.0, 4.0);
    let text_clear_bbox = BoundingBox {
        left: source_bbox.left - text_x_pad,
        top: source_bbox.top - text_top_pad,
        right: source_bbox.right + text_x_pad,
        bottom: source_bbox.bottom + text_bottom_pad,
    };

    let path_x_pad = 2.0;
    let path_top_pad = (row_height * 0.05).clamp(0.5, 1.0);
    let path_bottom_pad = (row_height * 0.12).clamp(1.0, 2.0);
    let path_suppression_bbox = BoundingBox {
        left: source_bbox.left - path_x_pad,
        top: source_bbox.top - path_top_pad,
        right: source_bbox.right + path_x_pad,
        bottom: source_bbox.bottom + path_bottom_pad,
    };

    let row_band_top = source_bbox.top - (row_height * 0.06).clamp(0.5, 1.0);
    let row_band_bottom = source_bbox.bottom + (row_height * 0.08).clamp(0.5, 1.25);

    ParagraphReplacementRegion {
        shell_bbox,
        source_bbox,
        text_clear_bbox,
        path_suppression_bbox,
        row_band_top,
        row_band_bottom,
    }
}

fn preferred_source_bbox(target: &ActiveEditorTarget) -> BoundingBox {
    if let Some(source_bbox) = source_session_visual_bbox(&target.scene.body_session) {
        if bbox_has_area(&source_bbox) {
            return source_bbox;
        }
    }
    let body_bbox = target.scene.body_session.anchor_bbox;
    if bbox_has_area(&body_bbox) {
        return body_bbox;
    }
    if bbox_has_area(&target.scene.shell_bbox) {
        return target.scene.shell_bbox;
    }
    BoundingBox {
        left: target.bbox_left,
        top: target.bbox_top,
        right: target.bbox_right,
        bottom: target.bbox_bottom,
    }
}

fn bbox_has_area(bbox: &BoundingBox) -> bool {
    bbox_width(bbox) > 0.0 && bbox_height(bbox) > 0.0
}

#[cfg(test)]
mod tests {
    use super::paragraph_replacement_region;
    use crate::editor::session::ActiveEditorTarget;
    use pdf_viewer_core::models::{
        BoundingBox, EditorSession, LayoutParagraph, LayoutRun, RunStyle,
    };

    fn target_for_body(body_bbox: BoundingBox) -> ActiveEditorTarget {
        let mut target = ActiveEditorTarget::default();
        target.scene.shell_bbox = BoundingBox {
            left: 50.0,
            top: 100.0,
            right: 180.0,
            bottom: 112.0,
        };
        target.scene.body_session = EditorSession {
            anchor_bbox: body_bbox,
            paragraph: LayoutParagraph::default(),
        };
        target
    }

    fn target_with_baseline_down_body_run() -> ActiveEditorTarget {
        let mut target = target_for_body(BoundingBox {
            left: 70.0,
            top: 112.0,
            right: 170.0,
            bottom: 124.0,
        });
        target.scene.body_session.paragraph.runs = vec![LayoutRun {
            id: "body-run".to_string(),
            text: "Anchor Framework".to_string(),
            style: RunStyle {
                font_name: "Arial".to_string(),
                font_size: 12.0,
                color: "#111111".to_string(),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                char_spacing: 0.0,
                scale_x: 1.0,
            },
            bbox: BoundingBox {
                left: 70.0,
                top: 112.0,
                right: 170.0,
                bottom: 124.0,
            },
            origin_x: 70.0,
            origin_y: 112.0,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: Vec::new(),
            object_indices: Vec::new(),
        }];
        target
    }

    #[test]
    fn text_clear_region_stays_near_editable_text() {
        let target = target_for_body(BoundingBox {
            left: 70.0,
            top: 100.0,
            right: 170.0,
            bottom: 112.0,
        });

        let region = paragraph_replacement_region(&target);

        assert!(region.text_clear_bbox.left > target.scene.shell_bbox.left);
        assert!(region.text_clear_bbox.right < target.scene.shell_bbox.right);
        assert!(region.text_clear_bbox.bottom > target.scene.shell_bbox.bottom);
        assert!(region.text_clear_bbox.top >= target.scene.body_session.anchor_bbox.top - 2.0);
    }

    #[test]
    fn path_suppression_is_tighter_than_source_replacement() {
        let target = target_for_body(BoundingBox {
            left: 70.0,
            top: 100.0,
            right: 170.0,
            bottom: 112.0,
        });

        let region = paragraph_replacement_region(&target);

        assert!(region.path_suppression_bbox.left >= region.text_clear_bbox.left);
        assert!(region.path_suppression_bbox.right <= region.text_clear_bbox.right);
        assert!(region.path_suppression_bbox.top >= region.text_clear_bbox.top);
        assert!(region.path_suppression_bbox.bottom < region.text_clear_bbox.bottom);
    }

    #[test]
    fn viewport_cull_region_covers_whole_row_for_tiled_path_suppression() {
        let target = target_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });

        let region = paragraph_replacement_region(&target);
        let cull_bbox = region.viewport_cull_bbox_for_page_width(595.0);

        assert_eq!(cull_bbox.left, 0.0);
        assert!(cull_bbox.right >= 595.0);
        assert!(cull_bbox.top <= region.row_band_top);
        assert!(cull_bbox.bottom >= region.row_band_bottom);
    }

    #[test]
    fn replacement_region_uses_baseline_font_source_geometry() {
        let target = target_with_baseline_down_body_run();

        let region = paragraph_replacement_region(&target);

        assert_eq!(region.source_bbox.top, 100.0);
        assert_eq!(region.source_bbox.bottom, 112.0);
        assert!(region.text_clear_bbox.top < 100.0);
        assert!(region.path_suppression_bbox.bottom > 112.0);
    }
}
