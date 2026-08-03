use crate::models::{BoundingBox, LayoutRun, ParagraphEditContext};

pub fn compute_session_bbox(session: &ParagraphEditContext) -> Option<BoundingBox> {
    compute_bbox_from_runs(&session.paragraph.runs)
}

pub fn compute_bbox_from_runs(runs: &[LayoutRun]) -> Option<BoundingBox> {
    let mut combined: Option<BoundingBox> = None;
    for run in runs.iter().filter(|run| !run.text.is_empty()) {
        let Some(run_bbox) = compute_run_bbox(run) else {
            continue;
        };
        combined = Some(match combined {
            Some(current) => union_bbox(current, run_bbox),
            None => run_bbox,
        });
    }
    combined
}

pub fn compute_caret_line_bbox(
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
        let Some(run_bbox) = compute_run_bbox(run) else {
            continue;
        };
        combined = Some(match combined {
            Some(current) => union_bbox(current, run_bbox),
            None => run_bbox,
        });
    }
    combined
}

pub fn compute_run_bbox(run: &LayoutRun) -> Option<BoundingBox> {
    if run.text.is_empty() {
        return None;
    }
    let text_run = run.to_text_run();
    let bbox = text_run.compute_bbox();
    if bbox.left.is_finite() && bbox.right.is_finite() && bbox.right > bbox.left {
        Some(bbox)
    } else if run.bbox.left.is_finite()
        && run.bbox.right.is_finite()
        && run.bbox.right > run.bbox.left
    {
        // 回退到 LayoutRun 存储的 bbox
        Some(run.bbox)
    } else {
        None
    }
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
    use super::{compute_bbox_from_runs, compute_caret_line_bbox};
    use crate::models::{BoundingBox, LayoutParagraph, LayoutRun, ParagraphEditContext, RunStyle};

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
            font_weight_numeric: 400,
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
    fn uses_baseline_bbox() {
        let run = test_run("r1", 70.0, 112.0, 12.0);

        let bbox = compute_bbox_from_runs(&[run]).expect("source bbox");

        assert_eq!(bbox.left, 70.0);
        assert_eq!(bbox.right, 130.0);
        assert_eq!(bbox.top, 100.0);
        assert_eq!(bbox.bottom, 112.0);
    }

    #[test]
    fn uses_source_geometry() {
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

        let line_bbox = compute_caret_line_bbox(&session, 12.0).expect("line bbox");

        assert_eq!(line_bbox.top, 100.0);
        assert_eq!(line_bbox.bottom, 112.0);
    }
}
