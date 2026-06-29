//! 段落替换区域计算 — 从 ui::editor::replacement_region 迁入。
//! 纯几何计算，无 wasm 依赖。

use crate::edit::active_target::ActiveEditorTarget;
use crate::edit::document_plan::bbox_from_runs;
use crate::geometry::bbox_ops::{bbox_height, bbox_width, union_bbox};
use crate::geometry::source_geometry::compute_session_bbox;
use crate::models::BoundingBox;

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
    pub fn row_suppression_bbox(&self, page_width: f32) -> BoundingBox {
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

    pub fn viewport_cull_bbox(&self, page_width: f32) -> BoundingBox {
        union_bbox(
            &union_bbox(&self.shell_bbox, &self.text_clear_bbox),
            &self.row_suppression_bbox(page_width),
        )
    }

    pub fn cache_invalidation_bbox(&self, page_width: f32) -> BoundingBox {
        self.viewport_cull_bbox(page_width)
    }
}

pub fn build_region(target: &ActiveEditorTarget) -> ParagraphReplacementRegion {
    let shell_bbox = target.scene.shell_bbox;
    let source_bbox = resolve_preferred_bbox(target);
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

fn resolve_preferred_bbox(target: &ActiveEditorTarget) -> BoundingBox {
    // 遮盖区域必须覆盖整行（marker + body），而非仅 body 部分
    // 因为 PDF 原文包含 marker 和 body，如果只遮盖 body，marker 区域的原文会透出来
    let shell_bbox = target.scene.shell_bbox;

    // 先尝试完整 session 的 bbox（如果有 editor_session，它包含整行数据）
    let body_bbox = if let Some(source_bbox) = compute_session_bbox(target.scene.body_session()) {
        if bbox_has_area(&source_bbox) {
            source_bbox
        } else {
            target.scene.body_session().anchor_bbox
        }
    } else {
        target.scene.body_session().anchor_bbox
    };

    // 如果有 marker，扩展 bbox 覆盖 marker 的真实源区域。
    let full_bbox = if let Some(marker) = target.scene.marker() {
        if let Some(marker_bbox) = bbox_from_runs(&marker.runs).filter(bbox_has_area) {
            union_bbox(&body_bbox, &marker_bbox)
        } else {
            body_bbox
        }
    } else {
        body_bbox
    };

    if bbox_has_area(&full_bbox) {
        return full_bbox;
    }
    if bbox_has_area(&shell_bbox) {
        return shell_bbox;
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
    use super::build_region;
    use crate::edit::active_target::ActiveEditorTarget;
    use crate::edit::document_plan::ParagraphEditorMarker;
    use crate::models::{BoundingBox, LayoutParagraph, LayoutRun, ParagraphEditContext, RunStyle};
    use crate::text::list_semantics::ListMarkerKind;

    fn target_for_body(body_bbox: BoundingBox) -> ActiveEditorTarget {
        let mut target = ActiveEditorTarget::default();
        target.scene.shell_bbox = BoundingBox {
            left: 50.0,
            top: 100.0,
            right: 180.0,
            bottom: 112.0,
        };
        *target.scene.body_session_mut() = ParagraphEditContext {
            anchor_bbox: body_bbox,
            paragraph: LayoutParagraph::default(),
        };
        target
    }

    fn find_target() -> ActiveEditorTarget {
        let mut target = target_for_body(BoundingBox {
            left: 70.0,
            top: 112.0,
            right: 170.0,
            bottom: 124.0,
        });
        target.scene.body_session_mut().paragraph.runs = vec![LayoutRun {
            id: "body-run".to_string(),
            text: "Anchor Framework".to_string(),
            style: test_style(),
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

    fn test_style() -> RunStyle {
        RunStyle {
            font_name: "Arial".to_string(),
            font_size: 12.0,
            color: "#111111".to_string(),
            is_bold: false,
            is_italic: false,
            is_underline: false,
            char_spacing: 0.0,
            scale_x: 1.0,
        }
    }

    fn layout_run(id: &str, text: &str, left: f32, baseline_y: f32, width: f32) -> LayoutRun {
        LayoutRun {
            id: id.to_string(),
            text: text.to_string(),
            style: test_style(),
            bbox: BoundingBox {
                left,
                top: baseline_y - 12.0,
                right: left + width,
                bottom: baseline_y,
            },
            origin_x: left,
            origin_y: baseline_y,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: Vec::new(),
            object_indices: Vec::new(),
        }
    }

    fn list_target(marker_left: f32, body_left: f32, baseline_y: f32) -> ActiveEditorTarget {
        let body_run = layout_run("body-run", "Body", body_left, baseline_y, 80.0);
        let marker_run = layout_run("marker-run", "•", marker_left, baseline_y, 8.0);
        let mut target = target_for_body(BoundingBox {
            left: body_left,
            top: baseline_y - 12.0,
            right: body_left + 80.0,
            bottom: baseline_y,
        });
        target.scene.shell_bbox = BoundingBox {
            left: body_left,
            top: baseline_y - 12.0,
            right: body_left + 80.0,
            bottom: baseline_y,
        };
        target.scene.body_session_mut().paragraph.runs = vec![body_run];
        *target.scene.marker_mut() = Some(ParagraphEditorMarker {
            kind: ListMarkerKind::Bullet,
            text: "•".to_string(),
            advance: 0.0,
            runs: vec![marker_run],
        });
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

        let region = build_region(&target);

        assert!(region.text_clear_bbox.left > target.scene.shell_bbox.left);
        assert!(region.text_clear_bbox.right < target.scene.shell_bbox.right);
        assert!(region.text_clear_bbox.bottom > target.scene.shell_bbox.bottom);
        assert!(region.text_clear_bbox.top >= target.scene.body_session().anchor_bbox.top - 2.0);
    }

    #[test]
    fn tightens_path_suppression() {
        let target = target_for_body(BoundingBox {
            left: 70.0,
            top: 100.0,
            right: 170.0,
            bottom: 112.0,
        });

        let region = build_region(&target);

        assert!(region.path_suppression_bbox.left >= region.text_clear_bbox.left);
        assert!(region.path_suppression_bbox.right <= region.text_clear_bbox.right);
        assert!(region.path_suppression_bbox.top >= region.text_clear_bbox.top);
        assert!(region.path_suppression_bbox.bottom < region.text_clear_bbox.bottom);
    }

    #[test]
    fn covers_tiled_row() {
        let target = target_for_body(BoundingBox {
            left: 90.0,
            top: 100.0,
            right: 330.0,
            bottom: 112.0,
        });

        let region = build_region(&target);
        let cull_bbox = region.viewport_cull_bbox(595.0);

        assert_eq!(cull_bbox.left, 0.0);
        assert!(cull_bbox.right >= 595.0);
        assert!(cull_bbox.top <= region.row_band_top);
        assert!(cull_bbox.bottom >= region.row_band_bottom);
    }

    #[test]
    fn list_marker_region_uses_real_marker_bbox() {
        let target = list_target(42.0, 70.0, 112.0);

        let region = build_region(&target);

        assert_eq!(region.source_bbox.left, 42.0);
        assert_eq!(region.source_bbox.right, 150.0);
        assert!(region.text_clear_bbox.left < 42.0);
        assert!(region.path_suppression_bbox.left < 42.0);
    }

    #[test]
    fn list_marker_region_tracks_each_rows_own_geometry() {
        let first_row = list_target(42.0, 70.0, 112.0);
        let second_row = list_target(80.0, 110.0, 132.0);

        let first_region = build_region(&first_row);
        let second_region = build_region(&second_row);

        assert_eq!(first_region.source_bbox.left, 42.0);
        assert_eq!(first_region.source_bbox.right, 150.0);
        assert_eq!(second_region.source_bbox.left, 80.0);
        assert_eq!(second_region.source_bbox.right, 190.0);
        assert_eq!(first_region.row_suppression_bbox(595.0).left, 0.0);
        assert!(second_region.row_suppression_bbox(595.0).right >= 595.0);
    }

    #[test]
    fn uses_baseline_geometry() {
        let target = find_target();

        let region = build_region(&target);

        assert_eq!(region.source_bbox.top, 100.0);
        assert_eq!(region.source_bbox.bottom, 112.0);
        assert!(region.text_clear_bbox.top < 100.0);
        assert!(region.path_suppression_bbox.bottom > 112.0);
    }
}
