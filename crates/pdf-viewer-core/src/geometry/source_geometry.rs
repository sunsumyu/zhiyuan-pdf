use crate::models::{BoundingBox, ParagraphEditContext, LayoutRun};

pub fn source_session_visual_bbox(session: &ParagraphEditContext) -> Option<BoundingBox> {
    source_visual_bbox_from_runs(&session.paragraph.runs)
}

pub fn source_visual_bbox_from_runs(runs: &[LayoutRun]) -> Option<BoundingBox> {
    let mut combined: Option<BoundingBox> = None;
    for run in runs.iter().filter(|run| !run.text.is_empty()) {
        let Some(run_bbox) = source_run_visual_bbox(run) else {
            continue;
        };
        combined = Some(match combined {
            Some(current) => union_bbox(current, run_bbox),
            None => run_bbox,
        });
    }
    combined
}

pub fn source_line_visual_bbox_for_caret(
    session: &ParagraphEditContext,
    caret_baseline_y: f32,
) -> Option<BoundingBox> {
    let anchor_top = session.anchor_bbox.top;
    let mut combined: Option<BoundingBox> = None;
    for run in session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
    {
        let run_baseline_y = (run.origin_y - anchor_top).max(0.0);
        let tolerance = (run.style.font_size * 0.45).max(2.0);
        if (run_baseline_y - caret_baseline_y).abs() > tolerance {
            continue;
        }
        let Some(run_bbox) = source_run_visual_bbox(run) else {
            continue;
        };
        combined = Some(match combined {
            Some(current) => union_bbox(current, run_bbox),
            None => run_bbox,
        });
    }
    combined
}

pub fn source_run_visual_bbox(run: &LayoutRun) -> Option<BoundingBox> {
    if run.text.is_empty() {
        return None;
    }
    let (left, right) = source_run_horizontal_span(run)?;
    if run.origin_y.is_finite() && run.style.font_size.is_finite() && run.style.font_size > 0.0 {
        let font_size = run.style.font_size.max(1.0);
        return Some(BoundingBox {
            left,
            top: run.origin_y - font_size,
            right,
            bottom: run.origin_y,
        });
    }
    if bbox_has_area(run.bbox) {
        return Some(BoundingBox {
            left,
            top: run.bbox.top,
            right,
            bottom: run.bbox.bottom,
        });
    }
    None
}

fn source_run_horizontal_span(run: &LayoutRun) -> Option<(f32, f32)> {
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;

    if run.bbox.left.is_finite() && run.bbox.right.is_finite() && run.bbox.right > run.bbox.left {
        left = left.min(run.bbox.left);
        right = right.max(run.bbox.right);
    }

    if run.origin_x.is_finite() {
        let inferred_width = inferred_run_width(run)
            .or_else(|| bbox_width(run.bbox).filter(|width| *width > 0.0))
            .unwrap_or(1.0);
        left = left.min(run.origin_x);
        right = right.max(run.origin_x + inferred_width.max(1.0));
    }

    if left.is_finite() && right.is_finite() && right > left {
        Some((left, right))
    } else {
        None
    }
}

fn inferred_run_width(run: &LayoutRun) -> Option<f32> {
    if run.char_origins.is_empty() {
        return None;
    }
    let glyph_count = run.text.chars().count().min(run.char_origins.len());
    if glyph_count == 0 {
        return None;
    }

    let mut right = f32::NEG_INFINITY;
    for glyph_index in 0..glyph_count {
        let origin = run.char_origins[glyph_index];
        if !origin.is_finite() {
            continue;
        }
        let glyph_right = if let Some(width) = run.char_widths.get(glyph_index).copied() {
            origin + width.max(0.0)
        } else if let Some(next_origin) = run.char_origins.get(glyph_index + 1).copied() {
            next_origin
        } else {
            origin + (run.style.font_size.max(1.0) * 0.5)
        };
        if glyph_right.is_finite() {
            right = right.max(glyph_right);
        }
    }
    right.is_finite().then_some(right.max(1.0))
}

fn bbox_width(bbox: BoundingBox) -> Option<f32> {
    let width = bbox.right - bbox.left;
    width.is_finite().then_some(width)
}

fn bbox_height(bbox: BoundingBox) -> Option<f32> {
    let height = bbox.bottom - bbox.top;
    height.is_finite().then_some(height)
}

fn bbox_has_area(bbox: BoundingBox) -> bool {
    bbox_width(bbox).is_some_and(|width| width > 0.0)
        && bbox_height(bbox).is_some_and(|height| height > 0.0)
}

fn union_bbox(left: BoundingBox, right: BoundingBox) -> BoundingBox {
    BoundingBox {
        left: left.left.min(right.left),
        top: left.top.min(right.top),
        right: left.right.max(right.right),
        bottom: left.bottom.max(right.bottom),
    }
}

#[cfg(test)]
mod tests {
    use super::{source_line_visual_bbox_for_caret, source_visual_bbox_from_runs};
    use crate::models::{
        BoundingBox, ParagraphEditContext, LayoutParagraph, LayoutRun, RunStyle,
    };

    fn test_run(id: &str, left: f32, baseline_y: f32, font_size: f32) -> LayoutRun {
        LayoutRun {
            id: id.to_string(),
            text: "Anchor".to_string(),
            style: RunStyle {
                font_name: "Arial".to_string(),
                font_size,
                color: "#111111".to_string(),
                is_bold: false,
                is_italic: false,
                is_underline: false,
                char_spacing: 0.0,
                scale_x: 1.0,
            },
            bbox: BoundingBox {
                left,
                top: baseline_y,
                right: left + 60.0,
                bottom: baseline_y + font_size,
            },
            origin_x: left,
            origin_y: baseline_y,
            char_origins: Vec::new(),
            char_widths: Vec::new(),
            object_ids: Vec::new(),
            object_indices: Vec::new(),
        }
    }

    #[test]
    fn visual_bbox_uses_baseline_font_geometry_when_stored_bbox_is_baseline_down() {
        let run = test_run("r1", 70.0, 112.0, 12.0);

        let bbox = source_visual_bbox_from_runs(&[run]).expect("source bbox");

        assert_eq!(bbox.left, 70.0);
        assert_eq!(bbox.right, 130.0);
        assert_eq!(bbox.top, 100.0);
        assert_eq!(bbox.bottom, 112.0);
    }

    #[test]
    fn caret_line_bbox_uses_same_source_visual_geometry() {
        let run = test_run("r1", 70.0, 112.0, 12.0);
        let session = ParagraphEditContext {
            anchor_bbox: BoundingBox {
                left: 70.0,
                top: 100.0,
                right: 130.0,
                bottom: 112.0,
            },
            paragraph: LayoutParagraph {
                runs: vec![run],
                ..LayoutParagraph::default()
            },
        };

        let line_bbox = source_line_visual_bbox_for_caret(&session, 12.0).expect("line bbox");

        assert_eq!(line_bbox.top, 100.0);
        assert_eq!(line_bbox.bottom, 112.0);
    }
}
