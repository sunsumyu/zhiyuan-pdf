//! body 行计划构建与草稿模板 run 选择。

use crate::geometry::source_geometry::source_visual_bbox_from_runs;
use crate::models::{BoundingBox, LayoutRun, ParagraphEditContext, RunStyle};
use crate::text::glyph_layout::{infer_run_advance, is_decorative_text, EditorSessionTextPlan};
use crate::typography::font_resolver::looks_like_symbolic_font;

use super::EditorDocumentLinePlan;

pub(super) fn split_run_at_char_index(
    run: &LayoutRun,
    split_index: usize,
) -> (Option<LayoutRun>, Option<LayoutRun>) {
    let chars: Vec<char> = run.text.chars().collect();
    if split_index == 0 {
        return (None, Some(run.clone()));
    }
    if split_index >= chars.len() {
        return (Some(run.clone()), None);
    }

    let x_offset = if split_index < run.char_origins.len() {
        run.char_origins[split_index]
    } else {
        infer_run_advance(run) * split_index as f32
    };

    // marker 侧保留原始（相对 anchor 的绝对）char_origins，不归零；
    // body 侧以 body 首字符 origin 为基线转相对坐标，两条路径互不干扰。
    let mut marker_run = run.clone();
    marker_run.text = chars[..split_index].iter().collect();
    marker_run.char_origins = run.char_origins.iter().take(split_index).copied().collect();
    marker_run.char_widths = run.char_widths.iter().take(split_index).copied().collect();
    marker_run.bbox.right = (run.bbox.left + x_offset).max(marker_run.bbox.left);

    let mut body_run = run.clone();
    body_run.text = chars[split_index..].iter().collect();
    body_run.origin_x += x_offset;
    body_run.bbox.left = (run.bbox.left + x_offset).min(body_run.bbox.right);
    let base_origin = run
        .char_origins
        .get(split_index)
        .copied()
        .unwrap_or(x_offset);
    body_run.char_origins = run
        .char_origins
        .iter()
        .skip(split_index)
        .map(|origin| origin - base_origin)
        .collect();
    body_run.char_widths = run.char_widths.iter().skip(split_index).copied().collect();

    (Some(marker_run), Some(body_run))
}

pub(super) fn bbox_from_runs(runs: &[LayoutRun]) -> Option<BoundingBox> {
    if let Some(source_bbox) = source_visual_bbox_from_runs(runs) {
        return Some(source_bbox);
    }
    let first = runs.first()?;
    let mut bbox = first.bbox;
    for run in runs.iter().skip(1) {
        bbox.left = bbox.left.min(run.bbox.left);
        bbox.top = bbox.top.min(run.bbox.top);
        bbox.right = bbox.right.max(run.bbox.right);
        bbox.bottom = bbox.bottom.max(run.bbox.bottom);
    }
    Some(bbox)
}

fn normalize_draft_template_run(run: &LayoutRun) -> LayoutRun {
    let mut normalized = run.clone();
    normalized.char_origins.clear();
    normalized.char_widths.clear();
    normalized.object_ids.clear();
    normalized.object_indices.clear();
    normalized.origin_x = 0.0;
    normalized.origin_y = 0.0;
    normalized.bbox = BoundingBox::default();
    normalized
}

pub(super) fn select_draft_template_run(
    session: &ParagraphEditContext,
    body_lines: &[EditorDocumentLinePlan],
) -> LayoutRun {
    let source_candidate = body_lines
        .iter()
        .flat_map(|line| line.source_runs.iter())
        .find(|run| {
            !run.text.trim().is_empty()
                && !is_decorative_text(&run.text)
                && !looks_like_symbolic_font(&run.style.font_name)
        });
    if let Some(run) = source_candidate {
        return normalize_draft_template_run(run);
    }

    let template_candidate = body_lines
        .iter()
        .flat_map(|line| line.template_runs.iter())
        .find(|run| {
            !run.text.trim().is_empty()
                && !is_decorative_text(&run.text)
                && !looks_like_symbolic_font(&run.style.font_name)
        });
    if let Some(run) = template_candidate {
        return normalize_draft_template_run(run);
    }

    if let Some(run) = session
        .paragraph
        .runs
        .iter()
        .find(|run| !run.text.trim().is_empty())
    {
        return normalize_draft_template_run(run);
    }

    LayoutRun {
        id: format!("editor-draft-template-{}", session.paragraph.id),
        style: RunStyle {
            font_size: 12.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn normalize_template_run_for_draft(run: &LayoutRun) -> LayoutRun {
    let mut normalized = run.clone();
    normalized.char_origins.clear();
    normalized.char_widths.clear();
    normalized.object_ids.clear();
    normalized.object_indices.clear();
    normalized.origin_x = 0.0;
    normalized.origin_y = 0.0;
    normalized.bbox = BoundingBox::default();
    normalized
}

pub(super) fn build_body_line_plans(
    session: &ParagraphEditContext,
    _text_plan: &EditorSessionTextPlan,
) -> Vec<EditorDocumentLinePlan> {
    let mut rebuilt_lines: Vec<EditorDocumentLinePlan> = Vec::new();
    let mut raw_line_start = 0usize;
    let mut current_runs: Vec<LayoutRun> = Vec::new();
    let mut current_source_runs: Vec<LayoutRun> = Vec::new();
    let mut current_origin_y: Option<f32> = None;
    let mut raw_consumed_again = 0usize;
    for run in session
        .paragraph
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
    {
        let glyph_count = run.text.chars().count();
        if let Some(origin_y) = current_origin_y {
            if !same_document_line(origin_y, run) {
                rebuilt_lines.push(EditorDocumentLinePlan {
                    template_runs: std::mem::take(&mut current_runs),
                    source_runs: std::mem::take(&mut current_source_runs),
                    reconstructed_char_count: raw_consumed_again.saturating_sub(raw_line_start),
                });
                raw_line_start = raw_consumed_again;
            }
        }
        current_origin_y = Some(run.origin_y);
        current_runs.push(normalize_template_run_for_draft(run));
        current_source_runs.push(run.clone());
        raw_consumed_again += glyph_count;
    }
    if !current_runs.is_empty() {
        rebuilt_lines.push(EditorDocumentLinePlan {
            template_runs: current_runs,
            source_runs: current_source_runs,
            reconstructed_char_count: raw_consumed_again.saturating_sub(raw_line_start),
        });
    }
    rebuilt_lines
}

fn same_document_line(reference_origin_y: f32, run: &LayoutRun) -> bool {
    let tolerance = (run.style.font_size * 0.45).max(2.0);
    (reference_origin_y - run.origin_y).abs() <= tolerance
}
