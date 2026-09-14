//! 未编辑态的 source 布局重建。

use crate::geometry::layout_engine::{ParagraphLayout, VisualLine};
use crate::edit::document_plan::EditorDocumentPlan;
use crate::models::LayoutRun;

use super::styles::{
    paragraph_preserve_underline, resolve_draft_template_run, sanitize_draft_run_style,
};

pub(super) fn same_existing_layout_line(reference_baseline_y: f32, run: &LayoutRun, anchor_top: f32) -> bool {
    let baseline_y = (run.origin_y - anchor_top).max(0.0);
    let tolerance = (run.style.font_size * 0.45).max(2.0);
    (reference_baseline_y - baseline_y).abs() <= tolerance
}

pub(super) fn build_source_layout(document_plan: &EditorDocumentPlan) -> ParagraphLayout {
    let session = &document_plan.body_session;
    let anchor_left = session.anchor_bbox.left;
    let anchor_top = session.anchor_bbox.top;
    let mut lines: Vec<VisualLine> = Vec::new();
    let preserve_underline = paragraph_preserve_underline(&document_plan.body_session.paragraph);

    for run in session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
    {
        let mut normalized_run: LayoutRun = run.clone();
        sanitize_draft_run_style(&mut normalized_run, preserve_underline);
        normalized_run.origin_x = (run.origin_x - anchor_left).max(0.0);
        normalized_run.origin_y = 0.0;
        normalized_run.bbox.left = (run.bbox.left - anchor_left).max(0.0);
        normalized_run.bbox.right = (run.bbox.right - anchor_left).max(normalized_run.bbox.left);
        normalized_run.bbox.top = (run.bbox.top - anchor_top).max(0.0);
        normalized_run.bbox.bottom = (run.bbox.bottom - anchor_top).max(normalized_run.bbox.top);

        if let Some(line) = lines
            .last_mut()
            .filter(|line| same_existing_layout_line(line.baseline_y, run, anchor_top))
        {
            line.text.push_str(&run.text);
            line.width = line.width.max((run.bbox.right - anchor_left).max(0.0));
            line.height = line.height.max(run.style.font_size.max(1.0));
            line.runs.push(normalized_run);
        } else {
            lines.push(VisualLine {
                runs: vec![normalized_run],
                width: (run.bbox.right - anchor_left).max(0.0),
                height: run.style.font_size.max(1.0),
                baseline_y: (run.origin_y - anchor_top).max(0.0),
                offset_x: 0.0,
                text: run.text.clone(),
            });
        }
    }

    let height = lines
        .last()
        .map(|line| line.baseline_y + line.height)
        .unwrap_or(0.0);

    ParagraphLayout { lines, height }
}
pub(super) fn source_baseline_y(document_plan: &EditorDocumentPlan) -> f32 {
    document_plan
        .body_session
        .paragraph
        .runs
        .iter()
        .find(|run| !run.text.is_empty())
        .map(|run| (run.origin_y - document_plan.body_session.anchor_bbox.top).max(0.0))
        .unwrap_or_else(|| {
            resolve_draft_template_run(document_plan)
                .style
                .font_size
                .max(1.0)
        })
}
